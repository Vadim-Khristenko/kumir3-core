// =============================================================================
//        МОДУЛЬ: ИСПОЛНЕНИЕ КЕЙСА НАСТОЯЩИМ ИНТЕРПРЕТАТОРОМ (KITE 18 § 3.11)
// =============================================================================
// Кейс исполняется собранным двоичным файлом `interpreter-cli`: только так
// доступны настоящий поток ввода (директива `| ВВОД:`), код возврата
// (`| КОД ВОЗВРАТА:`) и жёсткое ограничение времени (`| ТАЙМАУТ:`) —
// зависший процесс снимается, чего нельзя сделать с потоком внутри теста.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Значение по умолчанию директивы `| ТАЙМАУТ:` (KITE 18 § 3.3).
///
/// Предел нужен, чтобы снять зависшую программу, а не чтобы измерять скорость.
/// Один запуск отладочной сборки стоит около секунды ещё до первой инструкции
/// программы, и когда раннер идёт рядом с остальными тестами, этой секунде есть
/// куда вырасти. При пороге в пять секунд запаса не хватало: кейс из двух
/// строк арифметики изредка не успевал — и корпус мигал, что хуже, чем
/// медленный корпус.
pub const DEFAULT_TIMEOUT_MS: u64 = 15_000;

/// Префикс машиночитаемой строки вида ошибки, которую печатает
/// `interpreter-cli --error-kind`.
pub const ERROR_KIND_PREFIX: &str = "[вид ошибки] ";

/// Итог одного запуска программы.
#[derive(Debug)]
pub struct Outcome {
    pub stdout: String,
    pub stderr: String,
    /// Код завершения; `None`, если процесс снят по таймауту.
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    /// Вид ошибки исполнения, если программа завершилась ошибкой.
    pub error_kind: Option<String>,
    /// Человекочитаемое сообщение об ошибке без служебных префиксов.
    pub error_message: Option<String>,
    pub elapsed: Duration,
}

impl Outcome {
    pub fn failed(&self) -> bool {
        self.timed_out || self.exit_code != Some(0)
    }
}

/// Записывает собранный исходный текст во временный файл.
///
/// Файлы корпуса раннер не изменяет (KITE 18 § 3.11, последний абзац).
pub fn write_temp_source(source: &str) -> std::io::Result<std::path::PathBuf> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!("kumir3-korpus-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = dir.join(format!("kejs-{n}.kum"));
    std::fs::write(&path, source)?;
    Ok(path)
}

/// Запускает программу, подавая `input` во входной поток.
pub fn run_program(
    binary: &Path,
    source_path: &Path,
    input: &str,
    args: &[String],
    timeout: Duration,
) -> std::io::Result<Outcome> {
    let mut command = Command::new(binary);
    command
        .arg(source_path)
        .arg("--error-kind")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if !args.is_empty() {
        command.arg("--");
        command.args(args);
    }

    let started = Instant::now();
    let mut child = command.spawn()?;

    // Ввод и оба выходных потока обслуживаются отдельными нитями: иначе
    // заполнение буфера канала заблокирует и процесс, и тест.
    let mut stdin = child.stdin.take().expect("канал ввода запрошен");
    let input_owned = input.to_string();
    let writer = std::thread::spawn(move || {
        if !input_owned.is_empty() {
            let _ = stdin.write_all(input_owned.as_bytes());
        }
        drop(stdin);
    });
    let mut out_pipe = child.stdout.take().expect("канал вывода запрошен");
    let reader_out = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = std::io::Read::read_to_end(&mut out_pipe, &mut buf);
        buf
    });
    let mut err_pipe = child.stderr.take().expect("канал ошибок запрошен");
    let reader_err = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = std::io::Read::read_to_end(&mut err_pipe, &mut buf);
        buf
    });

    let deadline = started + timeout;
    let mut timed_out = false;
    let status = loop {
        match child.try_wait()? {
            Some(status) => break Some(status),
            None => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    timed_out = true;
                    break None;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        }
    };

    let _ = writer.join();
    let stdout = String::from_utf8_lossy(&reader_out.join().unwrap_or_default()).into_owned();
    let stderr = String::from_utf8_lossy(&reader_err.join().unwrap_or_default()).into_owned();

    let mut error_kind = None;
    let mut error_message = None;
    for line in stderr.lines() {
        if let Some(kind) = line.trim().strip_prefix(ERROR_KIND_PREFIX) {
            error_kind = Some(kind.trim().to_string());
        } else if let Some(msg) = line.trim().strip_prefix("Ошибка выполнения: ") {
            error_message = Some(msg.trim().to_string());
        }
    }

    Ok(Outcome {
        stdout,
        stderr,
        exit_code: status.and_then(|s| s.code()),
        timed_out,
        error_kind,
        error_message,
        elapsed: started.elapsed(),
    })
}
