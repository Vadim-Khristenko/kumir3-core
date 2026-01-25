// ============================================================================
//                    МОДУЛЬ RUST-ВСТАВОК
// ============================================================================
//
// Реализует выполнение Rust-кода из КуМир программы.
//
// Поддерживаемые синтаксисы:
//   1. РастВставкаНЦ ... РастВставкаКЦ
//   2. ржавчина нач ... кон
//
// Режимы работы:
//   - Компиляция (для release): rustc → dylib → загрузка
//   - Интерпретация (упрощённый): eval простых выражений
//
// ============================================================================

use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

use crate::interpreter::RuntimeError;
use crate::shared::types::{Value, Number};

/// Результат выполнения Rust-блока
#[derive(Debug, Clone)]
pub struct RustBlockResult {
    /// Выходные данные (stdout)
    pub stdout: String,
    /// Ошибки (stderr)
    pub stderr: String,
    /// Код возврата
    pub exit_code: Option<i32>,
    /// Возвращаемое значение (если есть)
    pub return_value: Option<Value>,
}

impl Default for RustBlockResult {
    fn default() -> Self {
        Self {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: Some(0),
            return_value: None,
        }
    }
}

/// Конфигурация выполнения Rust-блоков
#[derive(Debug, Clone)]
pub struct RustBlockConfig {
    /// Путь к rustc компилятору
    pub rustc_path: Option<String>,
    /// Временная директория для компиляции
    pub temp_dir: PathBuf,
    /// Режим выполнения
    pub execution_mode: RustExecutionMode,
    /// Дополнительные зависимости (crate)
    pub extra_deps: Vec<String>,
    /// Флаги компиляции
    pub compile_flags: Vec<String>,
}

impl Default for RustBlockConfig {
    fn default() -> Self {
        Self {
            rustc_path: None,
            temp_dir: env::temp_dir().join("kumir3_rust_blocks"),
            execution_mode: RustExecutionMode::Compile,
            extra_deps: vec![],
            compile_flags: vec!["-O".to_string()],
        }
    }
}

/// Режим выполнения Rust-кода
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustExecutionMode {
    /// Компиляция через rustc (полная поддержка)
    Compile,
    /// Простая интерпретация (только базовые выражения)
    Interpret,
    /// Только проверка синтаксиса (без выполнения)
    CheckOnly,
}

/// Исполнитель Rust-блоков
pub struct RustBlockExecutor {
    config: RustBlockConfig,
    /// Кеш скомпилированных блоков (хеш кода → путь к бинарнику)
    cache: HashMap<u64, PathBuf>,
    /// Счётчик для уникальных имён
    counter: u64,
}

impl RustBlockExecutor {
    /// Создаёт новый исполнитель с конфигурацией по умолчанию
    pub fn new() -> Self {
        Self::with_config(RustBlockConfig::default())
    }

    /// Создаёт новый исполнитель с заданной конфигурацией
    pub fn with_config(config: RustBlockConfig) -> Self {
        // Создаём временную директорию если нужно
        if !config.temp_dir.exists() {
            let _ = fs::create_dir_all(&config.temp_dir);
        }

        Self {
            config,
            cache: HashMap::new(),
            counter: 0,
        }
    }

    /// Выполняет Rust-код с переданными переменными
    pub fn execute(
        &mut self,
        code: &str,
        captured_vars: &HashMap<String, Value>,
    ) -> Result<RustBlockResult, RuntimeError> {
        match self.config.execution_mode {
            RustExecutionMode::Compile => self.execute_compiled(code, captured_vars),
            RustExecutionMode::Interpret => self.execute_interpreted(code, captured_vars),
            RustExecutionMode::CheckOnly => self.check_syntax(code),
        }
    }

    /// Компилирует и выполняет Rust-код
    fn execute_compiled(
        &mut self,
        code: &str,
        captured_vars: &HashMap<String, Value>,
    ) -> Result<RustBlockResult, RuntimeError> {
        // Проверяем наличие rustc
        let rustc = self.find_rustc()?;

        // Генерируем уникальное имя для файла
        self.counter += 1;
        let file_name = format!("kumir_rust_block_{}", self.counter);

        // Пути к файлам
        let source_path = self.config.temp_dir.join(format!("{}.rs", file_name));
        let output_path = if cfg!(windows) {
            self.config.temp_dir.join(format!("{}.exe", file_name))
        } else {
            self.config.temp_dir.join(&file_name)
        };

        // Генерируем полный исходный код с main()
        let full_code = self.generate_wrapper(code, captured_vars);

        // Записываем исходный код
        let mut file = fs::File::create(&source_path).map_err(|e| {
            RuntimeError::io_error(&format!("Не удалось создать файл: {}", e))
        })?;
        file.write_all(full_code.as_bytes()).map_err(|e| {
            RuntimeError::io_error(&format!("Не удалось записать файл: {}", e))
        })?;

        // Компилируем
        let compile_output = Command::new(&rustc)
            .args(&self.config.compile_flags)
            .arg("-o")
            .arg(&output_path)
            .arg(&source_path)
            .output()
            .map_err(|e| {
                RuntimeError::io_error(&format!("Не удалось запустить rustc: {}", e))
            })?;

        if !compile_output.status.success() {
            let stderr = String::from_utf8_lossy(&compile_output.stderr);
            return Err(RuntimeError::rust_block_error(&format!(
                "Ошибка компиляции Rust-блока:\n{}",
                self.format_rustc_error(&stderr, code)
            )));
        }

        // Выполняем скомпилированный код
        let run_output = Command::new(&output_path).output().map_err(|e| {
            RuntimeError::io_error(&format!("Не удалось выполнить Rust-блок: {}", e))
        })?;

        // Очищаем временные файлы
        let _ = fs::remove_file(&source_path);
        let _ = fs::remove_file(&output_path);

        Ok(RustBlockResult {
            stdout: String::from_utf8_lossy(&run_output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&run_output.stderr).to_string(),
            exit_code: run_output.status.code(),
            return_value: None,
        })
    }

    /// Интерпретирует простой Rust-код (без компиляции)
    fn execute_interpreted(
        &self,
        code: &str,
        captured_vars: &HashMap<String, Value>,
    ) -> Result<RustBlockResult, RuntimeError> {
        // Упрощённая интерпретация для базовых операций
        let code = code.trim();

        // Обработка простого println!
        if let Some(content) = self.extract_println(code) {
            let expanded = self.expand_variables(content, captured_vars);
            return Ok(RustBlockResult {
                stdout: format!("{}\n", expanded),
                stderr: String::new(),
                exit_code: Some(0),
                return_value: None,
            });
        }

        // Обработка eprintln!
        if let Some(content) = self.extract_eprintln(code) {
            let expanded = self.expand_variables(content, captured_vars);
            return Ok(RustBlockResult {
                stdout: String::new(),
                stderr: format!("{}\n", expanded),
                exit_code: Some(0),
                return_value: None,
            });
        }

        // Обработка простых арифметических выражений
        if let Some(value) = self.eval_simple_expr(code, captured_vars) {
            return Ok(RustBlockResult {
                stdout: String::new(),
                stderr: String::new(),
                exit_code: Some(0),
                return_value: Some(value),
            });
        }

        // Если не удалось интерпретировать, пытаемся скомпилировать
        if self.find_rustc().is_ok() {
            // Fallback к компиляции
            let mut executor = RustBlockExecutor::with_config(RustBlockConfig {
                execution_mode: RustExecutionMode::Compile,
                ..self.config.clone()
            });
            return executor.execute_compiled(code, captured_vars);
        }

        Err(RuntimeError::rust_block_error(
            "Не удалось интерпретировать Rust-код. \
             Для сложных выражений требуется rustc."
        ))
    }

    /// Проверяет синтаксис без выполнения
    fn check_syntax(&self, code: &str) -> Result<RustBlockResult, RuntimeError> {
        let rustc = self.find_rustc()?;

        // Создаём временный файл для проверки
        let source_path = self.config.temp_dir.join("syntax_check.rs");
        let full_code = format!(
            "fn main() {{\n{}\n}}",
            code
        );

        fs::write(&source_path, &full_code).map_err(|e| {
            RuntimeError::io_error(&format!("Не удалось создать файл: {}", e))
        })?;

        // Запускаем rustc --emit=metadata (только проверка)
        let output = Command::new(&rustc)
            .args(["--emit=metadata", "-o", "/dev/null"])
            .arg(&source_path)
            .output()
            .map_err(|e| {
                RuntimeError::io_error(&format!("Не удалось запустить rustc: {}", e))
            })?;

        let _ = fs::remove_file(&source_path);

        if output.status.success() {
            Ok(RustBlockResult::default())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(RuntimeError::rust_block_error(&format!(
                "Синтаксическая ошибка:\n{}",
                self.format_rustc_error(&stderr, code)
            )))
        }
    }

    /// Находит путь к rustc
    fn find_rustc(&self) -> Result<String, RuntimeError> {
        // Сначала проверяем конфиг
        if let Some(ref path) = self.config.rustc_path {
            return Ok(path.clone());
        }

        // Ищем в PATH
        let rustc_name = if cfg!(windows) { "rustc.exe" } else { "rustc" };

        // Проверяем наличие rustc
        let output = Command::new(rustc_name)
            .arg("--version")
            .output();

        match output {
            Ok(out) if out.status.success() => Ok(rustc_name.to_string()),
            _ => Err(RuntimeError::rust_block_error(
                "Компилятор Rust (rustc) не найден в системе. \
                 Установите Rust: https://rustup.rs/"
            ))
        }
    }

    /// Генерирует обёртку main() для пользовательского кода
    fn generate_wrapper(&self, code: &str, captured_vars: &HashMap<String, Value>) -> String {
        let mut wrapper = String::new();

        // Добавляем необходимые use
        wrapper.push_str("// Автоматически сгенерировано КуМир 3\n");
        wrapper.push_str("#![allow(unused_variables, unused_imports, dead_code)]\n\n");

        // Генерируем константы из захваченных переменных
        if !captured_vars.is_empty() {
            wrapper.push_str("// Захваченные переменные из КуМир\n");
            for (name, value) in captured_vars {
                wrapper.push_str(&self.value_to_rust_const(name, value));
            }
            wrapper.push('\n');
        }

        // Основная функция
        wrapper.push_str("fn main() {\n");

        // Вставляем пользовательский код с отступом
        for line in code.lines() {
            wrapper.push_str("    ");
            wrapper.push_str(line);
            wrapper.push('\n');
        }

        wrapper.push_str("}\n");

        wrapper
    }

    /// Преобразует Value в Rust-константу
    fn value_to_rust_const(&self, name: &str, value: &Value) -> String {
        match value {
            Value::Number(Number::I64(i)) => format!("const {}: i64 = {};\n", name, i),
            Value::Number(Number::F64(f)) => format!("const {}: f64 = {};\n", name, f),
            Value::Number(Number::F128(f)) => format!("const {}: f64 = {};\n", name, f),
            Value::Boolean(b) => format!("const {}: bool = {};\n", name, b),
            Value::String(s) => format!("const {}: &str = {:?};\n", name, s),
            Value::Char(c) => format!("const {}: char = {:?};\n", name, c),
            _ => format!("// {} имеет неподдерживаемый тип\n", name),
        }
    }

    /// Извлекает содержимое println!
    fn extract_println<'a>(&self, code: &'a str) -> Option<&'a str> {
        let code = code.trim();
        if code.starts_with("println!(") && code.ends_with(");") {
            let inner = &code[9..code.len()-2];
            // Убираем кавычки если есть
            if inner.starts_with('"') && inner.ends_with('"') {
                return Some(&inner[1..inner.len()-1]);
            }
            return Some(inner);
        }
        None
    }

    /// Извлекает содержимое eprintln!
    fn extract_eprintln<'a>(&self, code: &'a str) -> Option<&'a str> {
        let code = code.trim();
        if code.starts_with("eprintln!(") && code.ends_with(");") {
            let inner = &code[10..code.len()-2];
            if inner.starts_with('"') && inner.ends_with('"') {
                return Some(&inner[1..inner.len()-1]);
            }
            return Some(inner);
        }
        None
    }

    /// Раскрывает переменные в строке (формат {var})
    fn expand_variables(&self, template: &str, vars: &HashMap<String, Value>) -> String {
        let mut result = template.to_string();
        for (name, value) in vars {
            let placeholder = format!("{{{}}}", name);
            let value_str = match value {
                Value::Number(n) => n.to_string(),
                Value::Boolean(b) => b.to_string(),
                Value::String(s) => s.clone(),
                Value::Char(c) => c.to_string(),
                _ => format!("{:?}", value),
            };
            result = result.replace(&placeholder, &value_str);
        }
        result
    }

    /// Вычисляет простое выражение
    fn eval_simple_expr(&self, code: &str, vars: &HashMap<String, Value>) -> Option<Value> {
        let code = code.trim();

        // Простое число
        if let Ok(i) = code.parse::<i64>() {
            return Some(Value::Number(Number::I64(i)));
        }
        if let Ok(f) = code.parse::<f64>() {
            return Some(Value::Number(Number::F64(f)));
        }

        // Простая переменная
        if let Some(value) = vars.get(code) {
            return Some(value.clone());
        }

        // Простые бинарные операции
        for op in ["+", "-", "*", "/", "%"] {
            if let Some(pos) = code.rfind(op) {
                let left = code[..pos].trim();
                let right = code[pos+1..].trim();

                let left_val = self.eval_simple_expr(left, vars)?;
                let right_val = self.eval_simple_expr(right, vars)?;

                return self.apply_binary_op(op, &left_val, &right_val);
            }
        }

        None
    }

    /// Применяет бинарную операцию
    fn apply_binary_op(&self, op: &str, left: &Value, right: &Value) -> Option<Value> {
        match (left, right) {
            (Value::Number(Number::I64(a)), Value::Number(Number::I64(b))) => {
                let result = match op {
                    "+" => a + b,
                    "-" => a - b,
                    "*" => a * b,
                    "/" => a / b,
                    "%" => a % b,
                    _ => return None,
                };
                Some(Value::Number(Number::I64(result)))
            }
            (Value::Number(Number::F64(a)), Value::Number(Number::F64(b))) => {
                let result = match op {
                    "+" => a + b,
                    "-" => a - b,
                    "*" => a * b,
                    "/" => a / b,
                    "%" => a % b,
                    _ => return None,
                };
                Some(Value::Number(Number::F64(result)))
            }
            _ => None,
        }
    }

    /// Форматирует ошибку rustc, показывая только релевантные строки
    fn format_rustc_error(&self, stderr: &str, original_code: &str) -> String {
        // Упрощаем вывод ошибки
        let mut result = String::new();
        let lines: Vec<&str> = stderr.lines().collect();

        for line in lines {
            // Пропускаем строки с путём к временному файлу
            if line.contains("kumir_rust_block_") || line.contains("syntax_check.rs") {
                continue;
            }
            // Показываем только ошибки
            if line.contains("error") || line.contains("ошибка") {
                result.push_str(line);
                result.push('\n');
            }
        }

        if result.is_empty() {
            stderr.to_string()
        } else {
            result
        }
    }
}

impl Default for RustBlockExecutor {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Расширение RuntimeError для Rust-блоков
// ============================================================================

impl RuntimeError {
    /// Создаёт ошибку Rust-блока
    pub fn rust_block_error(message: &str) -> Self {
        use crate::interpreter::RuntimeErrorKind;
        Self::new(&format!("[Rust-вставка] {}", message), RuntimeErrorKind::Other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpret_println() {
        let executor = RustBlockExecutor::with_config(RustBlockConfig {
            execution_mode: RustExecutionMode::Interpret,
            ..Default::default()
        });

        let result = executor.execute_interpreted(
            r#"println!("Hello, КуМир!");"#,
            &HashMap::new()
        );

        assert!(result.is_ok());
        let res = result.unwrap();
        assert_eq!(res.stdout, "Hello, КуМир!\n");
    }

    #[test]
    fn test_interpret_with_variables() {
        let executor = RustBlockExecutor::with_config(RustBlockConfig {
            execution_mode: RustExecutionMode::Interpret,
            ..Default::default()
        });

        let mut vars = HashMap::new();
        vars.insert("x".to_string(), Value::Number(Number::I64(42)));

        let result = executor.execute_interpreted(
            r#"println!("x = {x}");"#,
            &vars
        );

        assert!(result.is_ok());
        let res = result.unwrap();
        assert_eq!(res.stdout, "x = 42\n");
    }

    #[test]
    fn test_simple_arithmetic() {
        let executor = RustBlockExecutor::with_config(RustBlockConfig {
            execution_mode: RustExecutionMode::Interpret,
            ..Default::default()
        });

        let result = executor.eval_simple_expr("2 + 3", &HashMap::new());
        assert_eq!(result, Some(Value::Number(Number::I64(5))));

        let result = executor.eval_simple_expr("10 - 4", &HashMap::new());
        assert_eq!(result, Some(Value::Number(Number::I64(6))));
    }

    #[test]
    fn test_generate_wrapper() {
        let executor = RustBlockExecutor::new();
        let mut vars = HashMap::new();
        vars.insert("count".to_string(), Value::Number(Number::I64(10)));

        let wrapper = executor.generate_wrapper("println!(\"{}\", count);", &vars);

        assert!(wrapper.contains("const count: i64 = 10;"));
        assert!(wrapper.contains("fn main()"));
        assert!(wrapper.contains("println!"));
    }
}
