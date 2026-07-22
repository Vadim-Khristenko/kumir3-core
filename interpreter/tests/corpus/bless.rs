// =============================================================================
//     МОДУЛЬ: РЕЖИМ ПЕРЕЗАПИСИ ЭТАЛОНОВ (KITE 18 § 3.11, последний абзац)
// =============================================================================
// Раннер МОЖЕТ перезаписывать значения `| ОЖИДАЕМЫЙ ВЫВОД:` фактическим
// выводом. Режим НЕ включается по умолчанию: он требует переменной окружения
//
//     KUMIR3_BLESS=1 cargo test -p kumir3-interpreter --test corpus
//
// Обычный прогон файлы корпуса не трогает.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Имя переменной окружения, включающей режим перезаписи.
pub const ENV_VAR: &str = "KUMIR3_BLESS";

/// Включён ли режим перезаписи.
pub fn enabled() -> bool {
    matches!(
        std::env::var(ENV_VAR).ok().as_deref(),
        Some("1") | Some("да") | Some("true")
    )
}

/// Одна замена: полуинтервал строк файла и новый текст эталона.
#[derive(Debug)]
pub struct Edit {
    pub span: (usize, usize),
    pub actual: String,
}

/// Накопитель замен по файлам корпуса.
#[derive(Debug, Default)]
pub struct Blessings {
    edits: BTreeMap<PathBuf, Vec<Edit>>,
}

impl Blessings {
    pub fn record(&mut self, path: &Path, edit: Edit) {
        self.edits.entry(path.to_path_buf()).or_default().push(edit);
    }

    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }

    /// Записывает накопленные замены на диск, возвращая отчёт по файлам.
    pub fn apply(
        self,
        source_lines: &BTreeMap<PathBuf, Vec<String>>,
    ) -> std::io::Result<Vec<String>> {
        let mut report = Vec::new();
        for (path, mut edits) in self.edits {
            let Some(lines) = source_lines.get(&path) else {
                continue;
            };
            let mut lines = lines.clone();
            // Замены применяются с конца, чтобы индексы строк не смещались.
            edits.sort_by_key(|e| std::cmp::Reverse(e.span.0));
            for edit in &edits {
                let indent: String = lines[edit.span.0]
                    .chars()
                    .take_while(|c| c.is_whitespace())
                    .collect();
                let mut block = vec![format!("{indent}| ОЖИДАЕМЫЙ ВЫВОД:")];
                let text = edit.actual.replace("\r\n", "\n");
                let text = text.trim_end_matches('\n');
                if !text.is_empty() {
                    for line in text.split('\n') {
                        if line.is_empty() {
                            block.push(format!("{indent}|"));
                        } else {
                            block.push(format!("{indent}| {line}"));
                        }
                    }
                }
                lines.splice(edit.span.0..edit.span.1, block);
            }
            std::fs::write(&path, lines.join("\n"))?;
            report.push(format!(
                "{}: перезаписано эталонов {}",
                path.display(),
                edits.len()
            ));
        }
        Ok(report)
    }
}
