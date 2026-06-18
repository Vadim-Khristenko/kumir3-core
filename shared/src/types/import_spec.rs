//! Спецификация импорта для Kumir 3
//!
//! Поддерживает различные формы импорта:
//! ```kumir
//! подключить Сокеты
//! подключить из Сокеты (TCP_Сервер, UDP_Клиент)
//! подключить из Сокеты:2.0.0 (TCP_Сервер)
//! подключить из ./локальный_модуль (функция)
//! подключить Сокеты:1.0 как СокетыV1
//! ```

use std::path::PathBuf;

use super::version::{VersionParseError, VersionSpec};

// ============================================================================
//                         ТИП ИМПОРТА
// ============================================================================

/// Что именно импортируется
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportItem {
    /// Вся библиотека целиком
    All,
    /// Конкретный элемент (функция, тип, константа)
    Named(String),
    /// Элемент с переименованием: `функция как новое_имя`
    Aliased { name: String, alias: String },
    /// Групповой импорт: `(элемент1, элемент2)`
    Group(Vec<ImportItem>),
}

impl ImportItem {
    /// Создаёт именованный импорт
    pub fn named(name: impl Into<String>) -> Self {
        ImportItem::Named(name.into())
    }

    /// Создаёт импорт с псевдонимом
    pub fn aliased(name: impl Into<String>, alias: impl Into<String>) -> Self {
        ImportItem::Aliased {
            name: name.into(),
            alias: alias.into(),
        }
    }

    /// Создаёт групповой импорт
    pub fn group(items: Vec<ImportItem>) -> Self {
        ImportItem::Group(items)
    }

    /// Возвращает все импортируемые имена
    pub fn names(&self) -> Vec<&str> {
        match self {
            ImportItem::All => vec!["*"],
            ImportItem::Named(name) => vec![name.as_str()],
            ImportItem::Aliased { name, .. } => vec![name.as_str()],
            ImportItem::Group(items) => items.iter().flat_map(|i| i.names()).collect(),
        }
    }

    /// Возвращает имя для использования в коде (с учётом алиаса)
    pub fn use_name(&self) -> Option<&str> {
        match self {
            ImportItem::All => None,
            ImportItem::Named(name) => Some(name.as_str()),
            ImportItem::Aliased { alias, .. } => Some(alias.as_str()),
            ImportItem::Group(_) => None,
        }
    }
}

// ============================================================================
//                         ИСТОЧНИК ИМПОРТА
// ============================================================================

/// Откуда импортировать
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportSource {
    /// Библиотека по имени: `подключить Сокеты`
    Library(String),
    /// Библиотека с версией: `подключить из Сокеты:2.0.0`
    VersionedLibrary { name: String, version: VersionSpec },
    /// Относительный путь: `подключить из ./модуль`
    RelativePath(PathBuf),
    /// Абсолютный путь: `подключить из /путь/к/модулю`
    AbsolutePath(PathBuf),
    /// URL (для будущего): `подключить из https://kumir.dev/libs/sockets`
    Url(String),
}

impl ImportSource {
    /// Создаёт источник из имени библиотеки
    pub fn library(name: impl Into<String>) -> Self {
        ImportSource::Library(name.into())
    }

    /// Создаёт версионированный источник
    pub fn versioned(name: impl Into<String>, version: VersionSpec) -> Self {
        ImportSource::VersionedLibrary {
            name: name.into(),
            version,
        }
    }

    /// Создаёт источник из пути
    pub fn path(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        if path.is_absolute() {
            ImportSource::AbsolutePath(path)
        } else {
            ImportSource::RelativePath(path)
        }
    }

    /// Возвращает имя библиотеки (если это библиотека)
    pub fn library_name(&self) -> Option<&str> {
        match self {
            ImportSource::Library(name) => Some(name),
            ImportSource::VersionedLibrary { name, .. } => Some(name),
            _ => None,
        }
    }

    /// Возвращает спецификацию версии (если есть)
    pub fn version_spec(&self) -> Option<&VersionSpec> {
        match self {
            ImportSource::VersionedLibrary { version, .. } => Some(version),
            _ => None,
        }
    }

    /// Является ли источник локальным путём
    pub fn is_path(&self) -> bool {
        matches!(
            self,
            ImportSource::RelativePath(_) | ImportSource::AbsolutePath(_)
        )
    }
}

// ============================================================================
//                         СПЕЦИФИКАЦИЯ ИМПОРТА
// ============================================================================

/// Полная спецификация импорта
#[derive(Debug, Clone)]
pub struct ImportSpec {
    /// Источник импорта
    pub source: ImportSource,
    /// Что импортировать
    pub items: ImportItem,
    /// Псевдоним для всей библиотеки: `подключить Сокеты как С`
    pub library_alias: Option<String>,
    /// Позиция в исходном коде (для ошибок)
    pub location: Option<SourceLocation>,
}

/// Позиция в исходном коде
#[derive(Debug, Clone, Copy)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
    pub offset: usize,
}

impl ImportSpec {
    /// Создаёт простой импорт всей библиотеки
    pub fn library(name: impl Into<String>) -> Self {
        Self {
            source: ImportSource::library(name),
            items: ImportItem::All,
            library_alias: None,
            location: None,
        }
    }

    /// Создаёт импорт с конкретными элементами
    pub fn from_library(name: impl Into<String>, items: Vec<ImportItem>) -> Self {
        Self {
            source: ImportSource::library(name),
            items: ImportItem::Group(items),
            library_alias: None,
            location: None,
        }
    }

    /// Создаёт версионированный импорт
    pub fn versioned(name: impl Into<String>, version: VersionSpec, items: ImportItem) -> Self {
        Self {
            source: ImportSource::versioned(name, version),
            items,
            library_alias: None,
            location: None,
        }
    }

    /// Добавляет псевдоним библиотеки
    pub fn with_alias(mut self, alias: impl Into<String>) -> Self {
        self.library_alias = Some(alias.into());
        self
    }

    /// Добавляет позицию в коде
    pub fn at(mut self, line: usize, column: usize, offset: usize) -> Self {
        self.location = Some(SourceLocation {
            line,
            column,
            offset,
        });
        self
    }

    /// Возвращает имя для использования (с учётом алиаса)
    pub fn effective_name(&self) -> Option<&str> {
        self.library_alias
            .as_deref()
            .or_else(|| self.source.library_name())
    }
}

// ============================================================================
//                         ПАРСИНГ ИМПОРТА
// ============================================================================

/// Ошибка парсинга импорта
#[derive(Debug, Clone)]
pub struct ImportParseError {
    pub message: String,
    pub position: Option<usize>,
}

impl ImportParseError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
            position: None,
        }
    }

    pub fn at(mut self, pos: usize) -> Self {
        self.position = Some(pos);
        self
    }
}

impl std::fmt::Display for ImportParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ошибка импорта: {}", self.message)
    }
}

impl std::error::Error for ImportParseError {}

impl From<VersionParseError> for ImportParseError {
    fn from(e: VersionParseError) -> Self {
        ImportParseError::new(e.message)
    }
}

/// Парсер импортов
pub struct ImportParser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> ImportParser<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    /// Парсит спецификацию импорта
    pub fn parse(&mut self) -> Result<ImportSpec, ImportParseError> {
        self.skip_whitespace();

        // Проверяем ключевое слово "подключить"
        if !self.consume_keyword("подключить") && !self.consume_keyword("использовать")
        {
            return Err(ImportParseError::new(
                "Ожидалось 'подключить' или 'использовать'",
            ));
        }

        self.skip_whitespace();

        // Проверяем "из" для селективного импорта
        let has_from = self.consume_keyword("из");
        self.skip_whitespace();

        // Парсим источник (библиотека или путь)
        let source = self.parse_source()?;
        self.skip_whitespace();

        // Парсим элементы импорта
        let items = if has_from {
            self.parse_items()?
        } else {
            ImportItem::All
        };

        self.skip_whitespace();

        // Проверяем псевдоним "как"
        let library_alias = if self.consume_keyword("как") {
            self.skip_whitespace();
            Some(self.parse_identifier()?)
        } else {
            None
        };

        Ok(ImportSpec {
            source,
            items,
            library_alias,
            location: None,
        })
    }

    fn parse_source(&mut self) -> Result<ImportSource, ImportParseError> {
        // Проверяем путь (начинается с ./ или /)
        if self.peek_char() == Some('.') || self.peek_char() == Some('/') {
            let path = self.parse_path()?;
            return Ok(ImportSource::path(path));
        }

        // Парсим имя библиотеки
        let name = self.parse_identifier()?;

        // Проверяем версию после ':'
        if self.consume_char(':') {
            let version_str = self.parse_until(|c| c.is_whitespace() || c == '(' || c == ')');
            let version: VersionSpec = version_str.parse()?;
            Ok(ImportSource::versioned(name, version))
        } else {
            Ok(ImportSource::library(name))
        }
    }

    fn parse_items(&mut self) -> Result<ImportItem, ImportParseError> {
        // Проверяем групповой импорт (в скобках)
        if self.consume_char('(') {
            let mut items = Vec::new();

            loop {
                self.skip_whitespace();

                if self.consume_char(')') {
                    break;
                }

                let item = self.parse_single_item()?;
                items.push(item);

                self.skip_whitespace();

                if !self.consume_char(',') {
                    if !self.consume_char(')') {
                        return Err(ImportParseError::new("Ожидалось ',' или ')'"));
                    }
                    break;
                }
            }

            if items.is_empty() {
                return Err(ImportParseError::new("Пустой список импорта"));
            }

            Ok(ImportItem::group(items))
        } else {
            // Одиночный элемент
            self.parse_single_item()
        }
    }

    fn parse_single_item(&mut self) -> Result<ImportItem, ImportParseError> {
        let name = self.parse_identifier()?;
        self.skip_whitespace();

        if self.consume_keyword("как") {
            self.skip_whitespace();
            let alias = self.parse_identifier()?;
            Ok(ImportItem::aliased(name, alias))
        } else {
            Ok(ImportItem::named(name))
        }
    }

    fn parse_identifier(&mut self) -> Result<String, ImportParseError> {
        let start = self.pos;

        while let Some(c) = self.peek_char() {
            if c.is_alphanumeric()
                || c == '_'
                || c == 'ё'
                || c == 'Ё'
                || ('\u{0400}'..='\u{04FF}').contains(&c)
            // Кириллица
            {
                self.advance();
            } else {
                break;
            }
        }

        if self.pos == start {
            return Err(ImportParseError::new("Ожидался идентификатор").at(self.pos));
        }

        Ok(self.input[start..self.pos].to_string())
    }

    fn parse_path(&mut self) -> Result<String, ImportParseError> {
        let start = self.pos;

        while let Some(c) = self.peek_char() {
            if c.is_whitespace() || c == '(' || c == ')' {
                break;
            }
            self.advance();
        }

        if self.pos == start {
            return Err(ImportParseError::new("Ожидался путь"));
        }

        Ok(self.input[start..self.pos].to_string())
    }

    fn parse_until(&mut self, predicate: impl Fn(char) -> bool) -> String {
        let start = self.pos;

        while let Some(c) = self.peek_char() {
            if predicate(c) {
                break;
            }
            self.advance();
        }

        self.input[start..self.pos].to_string()
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek_char() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn consume_keyword(&mut self, keyword: &str) -> bool {
        if self.input[self.pos..].starts_with(keyword) {
            // Проверяем, что за ключевым словом не идёт буква
            let after = self.pos + keyword.len();
            if after < self.input.len() {
                let next_char = self.input[after..].chars().next();
                if let Some(c) = next_char
                    && (c.is_alphanumeric() || c == '_')
                {
                    return false;
                }
            }
            self.pos += keyword.len();
            true
        } else {
            false
        }
    }

    fn consume_char(&mut self, ch: char) -> bool {
        if self.peek_char() == Some(ch) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn advance(&mut self) {
        if let Some(c) = self.peek_char() {
            self.pos += c.len_utf8();
        }
    }
}

/// Парсит строку импорта
pub fn parse_import(input: &str) -> Result<ImportSpec, ImportParseError> {
    ImportParser::new(input).parse()
}

// ============================================================================
//                         ТЕСТЫ
// ============================================================================

#[cfg(test)]
mod tests {
    use super::super::version::Version;
    use super::*;

    #[test]
    fn test_simple_import() {
        let spec = parse_import("подключить Сокеты").unwrap();
        assert_eq!(spec.source.library_name(), Some("Сокеты"));
        assert!(matches!(spec.items, ImportItem::All));
    }

    #[test]
    fn test_import_with_items() {
        let spec = parse_import("подключить из Сокеты (TCP_Сервер, UDP_Клиент)").unwrap();
        assert_eq!(spec.source.library_name(), Some("Сокеты"));

        if let ImportItem::Group(items) = spec.items {
            assert_eq!(items.len(), 2);
        } else {
            panic!("Expected group import");
        }
    }

    #[test]
    fn test_versioned_import() {
        let spec = parse_import("подключить из Сокеты:2.0.0 (TCP_Сервер)").unwrap();

        if let ImportSource::VersionedLibrary { name, version } = &spec.source {
            assert_eq!(name, "Сокеты");
            assert!(version.matches(&Version::new(2, 0, 0)));
        } else {
            panic!("Expected versioned library");
        }
    }

    #[test]
    fn test_import_with_alias() {
        let spec = parse_import("подключить Сокеты как С").unwrap();
        assert_eq!(spec.library_alias, Some("С".to_string()));
    }

    #[test]
    fn test_import_item_alias() {
        let spec = parse_import("подключить из Сокеты (TCP_Сервер как Сервер)").unwrap();

        if let ImportItem::Group(items) = spec.items {
            if let ImportItem::Aliased { name, alias } = &items[0] {
                assert_eq!(name, "TCP_Сервер");
                assert_eq!(alias, "Сервер");
            } else {
                panic!("Expected aliased item");
            }
        }
    }

    #[test]
    fn test_path_import() {
        let spec = parse_import("подключить из ./мой_модуль (функция)").unwrap();
        assert!(spec.source.is_path());
    }

    #[test]
    fn test_version_spec_import() {
        let spec = parse_import("подключить из Сокеты:^1.5 (TCP_Сервер)").unwrap();

        if let ImportSource::VersionedLibrary { version, .. } = &spec.source {
            assert!(version.matches(&Version::new(1, 5, 0)));
            assert!(version.matches(&Version::new(1, 9, 9)));
            assert!(!version.matches(&Version::new(2, 0, 0)));
        }
    }
}
