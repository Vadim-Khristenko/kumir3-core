//! Конфигурация проекта Kumir 3 (kumir.toml)
//!
//! Формат файла конфигурации:
//! ```toml
//! [проект]
//! имя = "мой_проект"
//! версия = "1.0.0"
//! авторы = ["Иван Иванов"]
//! 
//! [зависимости]
//! Сокеты = "^2.0"
//! Графика = { версия = "1.5", опционально = true }
//! МойМодуль = { путь = "./libs/мой_модуль" }
//! 
//! [сборка]
//! главный = "main.kum"
//! выход = "./build"
//! ```

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use toml::Value;

use super::version::{Version, VersionSpec, VersionParseError};

// ============================================================================
//                         МЕТАДАННЫЕ ПРОЕКТА
// ============================================================================

/// Метаданные проекта
#[derive(Debug, Clone, Default)]
pub struct ProjectMetadata {
    /// Имя проекта
    pub name: String,
    /// Версия проекта
    pub version: Version,
    /// Авторы
    pub authors: Vec<String>,
    /// Описание
    pub description: Option<String>,
    /// Лицензия
    pub license: Option<String>,
    /// Домашняя страница
    pub homepage: Option<String>,
    /// Репозиторий
    pub repository: Option<String>,
    /// Ключевые слова
    pub keywords: Vec<String>,
}

// ============================================================================
//                         ЗАВИСИМОСТИ
// ============================================================================

/// Спецификация зависимости
#[derive(Debug, Clone)]
pub struct DependencySpec {
    /// Имя библиотеки
    pub name: String,
    /// Спецификация версии
    pub version: VersionSpec,
    /// Путь к локальной библиотеке (вместо версии)
    pub path: Option<PathBuf>,
    /// Git репозиторий
    pub git: Option<GitSource>,
    /// URL для скачивания
    pub url: Option<String>,
    /// Опциональная зависимость
    pub optional: bool,
    /// Фичи библиотеки
    pub features: Vec<String>,
    /// Отключённые фичи по умолчанию
    pub default_features: bool,
}

/// Источник Git
#[derive(Debug, Clone)]
pub struct GitSource {
    /// URL репозитория
    pub url: String,
    /// Ветка
    pub branch: Option<String>,
    /// Тег
    pub tag: Option<String>,
    /// Коммит
    pub rev: Option<String>,
}

impl DependencySpec {
    /// Создаёт зависимость с версией
    pub fn version(name: impl Into<String>, version: VersionSpec) -> Self {
        Self {
            name: name.into(),
            version,
            path: None,
            git: None,
            url: None,
            optional: false,
            features: Vec::new(),
            default_features: true,
        }
    }

    /// Создаёт зависимость с путём
    pub fn path(name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            name: name.into(),
            version: VersionSpec::any(),
            path: Some(path.into()),
            git: None,
            url: None,
            optional: false,
            features: Vec::new(),
            default_features: true,
        }
    }

    /// Создаёт зависимость с git
    pub fn git(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: VersionSpec::any(),
            path: None,
            git: Some(GitSource {
                url: url.into(),
                branch: None,
                tag: None,
                rev: None,
            }),
            url: None,
            optional: false,
            features: Vec::new(),
            default_features: true,
        }
    }

    /// Помечает как опциональную
    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }

    /// Добавляет фичу
    pub fn with_feature(mut self, feature: impl Into<String>) -> Self {
        self.features.push(feature.into());
        self
    }

    /// Является ли зависимость локальной
    pub fn is_local(&self) -> bool {
        self.path.is_some()
    }

    /// Является ли зависимость из git
    pub fn is_git(&self) -> bool {
        self.git.is_some()
    }
}

// ============================================================================
//                         НАСТРОЙКИ СБОРКИ
// ============================================================================

/// Настройки сборки
#[derive(Debug, Clone)]
pub struct BuildSettings {
    /// Главный файл программы
    pub main_file: PathBuf,
    /// Директория вывода
    pub output_dir: PathBuf,
    /// Уровень оптимизации (0-3)
    pub optimization_level: u8,
    /// Включить отладочную информацию
    pub debug_info: bool,
    /// Включить проверки границ
    pub bounds_check: bool,
    /// Строгий режим типизации
    pub strict_mode: bool,
}

impl Default for BuildSettings {
    fn default() -> Self {
        Self {
            main_file: PathBuf::from("главный.kum"),
            output_dir: PathBuf::from("./сборка"),
            optimization_level: 1,
            debug_info: true,
            bounds_check: true,
            strict_mode: false,
        }
    }
}

// ============================================================================
//                         ПРОФИЛИ
// ============================================================================

/// Профиль сборки
#[derive(Debug, Clone)]
pub struct BuildProfile {
    /// Имя профиля
    pub name: String,
    /// Уровень оптимизации
    pub optimization_level: Option<u8>,
    /// Отладочная информация
    pub debug_info: Option<bool>,
    /// Проверки границ
    pub bounds_check: Option<bool>,
    /// Дополнительные определения
    pub defines: HashMap<String, String>,
}

impl BuildProfile {
    /// Профиль разработки
    pub fn dev() -> Self {
        Self {
            name: "разработка".to_string(),
            optimization_level: Some(0),
            debug_info: Some(true),
            bounds_check: Some(true),
            defines: HashMap::new(),
        }
    }

    /// Профиль выпуска
    pub fn release() -> Self {
        Self {
            name: "выпуск".to_string(),
            optimization_level: Some(3),
            debug_info: Some(false),
            bounds_check: Some(false),
            defines: HashMap::new(),
        }
    }

    /// Профиль тестирования
    pub fn test() -> Self {
        Self {
            name: "тест".to_string(),
            optimization_level: Some(0),
            debug_info: Some(true),
            bounds_check: Some(true),
            defines: HashMap::from([
                ("ТЕСТ".to_string(), "да".to_string()),
            ]),
        }
    }
}

// ============================================================================
//                         КОНФИГУРАЦИЯ ПРОЕКТА
// ============================================================================

/// Полная конфигурация проекта
#[derive(Debug, Clone)]
pub struct KumirConfig {
    /// Путь к файлу конфигурации
    pub config_path: PathBuf,
    /// Корень проекта
    pub project_root: PathBuf,
    /// Метаданные проекта
    pub metadata: ProjectMetadata,
    /// Зависимости
    pub dependencies: HashMap<String, DependencySpec>,
    /// Зависимости для разработки
    pub dev_dependencies: HashMap<String, DependencySpec>,
    /// Настройки сборки
    pub build: BuildSettings,
    /// Профили сборки
    pub profiles: HashMap<String, BuildProfile>,
    /// Рабочие пространства (для монорепозиториев)
    pub workspaces: Vec<PathBuf>,
}

impl KumirConfig {
    /// Создаёт новую конфигурацию
    pub fn new(project_root: impl AsRef<Path>) -> Self {
        let root = project_root.as_ref().to_path_buf();
        Self {
            config_path: root.join("kumir.toml"),
            project_root: root,
            metadata: ProjectMetadata::default(),
            dependencies: HashMap::new(),
            dev_dependencies: HashMap::new(),
            build: BuildSettings::default(),
            profiles: HashMap::from([
                ("разработка".to_string(), BuildProfile::dev()),
                ("выпуск".to_string(), BuildProfile::release()),
                ("тест".to_string(), BuildProfile::test()),
            ]),
            workspaces: Vec::new(),
        }
    }

    /// Загружает конфигурацию из файла
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .map_err(|e| ConfigError::IoError(e.to_string()))?;
        
        Self::parse(&content, path)
    }

    /// Ищет и загружает конфигурацию из текущей или родительских директорий
    pub fn find(start_dir: impl AsRef<Path>) -> Option<Self> {
        let mut current = start_dir.as_ref().to_path_buf();
        
        loop {
            let config_path = current.join("kumir.toml");
            if config_path.exists() {
                return Self::load(&config_path).ok();
            }
            
            if !current.pop() {
                return None;
            }
        }
    }

    /// Парсит конфигурацию из строки
    pub fn parse(content: &str, config_path: &Path) -> Result<Self, ConfigError> {
        let project_root = config_path.parent()
            .ok_or_else(|| ConfigError::ParseError("Не удалось определить корень проекта".into()))?
            .to_path_buf();
        
        let mut config = Self::new(&project_root);
        config.config_path = config_path.to_path_buf();

        // Простой парсер TOML (в реальности лучше использовать библиотеку toml)
        let mut current_section = String::new();
        
        for line in content.lines() {
            let line = line.trim();
            
            // Пропускаем пустые строки и комментарии
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            // Секция
            if line.starts_with('[') && line.ends_with(']') {
                current_section = line[1..line.len()-1].to_string();
                continue;
            }
            
            // Ключ = значение
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"');
                
                match current_section.as_str() {
                    "проект" | "project" => {
                        match key {
                            "имя" | "name" => config.metadata.name = value.to_string(),
                            "версия" | "version" => {
                                config.metadata.version = value.parse()
                                    .map_err(|e: VersionParseError| ConfigError::ParseError(e.message))?;
                            }
                            "описание" | "description" => {
                                config.metadata.description = Some(value.to_string());
                            }
                            "лицензия" | "license" => {
                                config.metadata.license = Some(value.to_string());
                            }
                            _ => {}
                        }
                    }
                    "зависимости" | "dependencies" => {
                        // Простая версия: зависимость = "версия"
                        let spec = if value.starts_with('^') || value.starts_with('~') 
                            || value.starts_with('>') || value.starts_with('<')
                            || value.starts_with('=') || value.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
                        {
                            let version: VersionSpec = value.parse()
                                .map_err(|e: VersionParseError| ConfigError::ParseError(e.message))?;
                            DependencySpec::version(key, version)
                        } else if value.starts_with("./") || value.starts_with("../") {
                            DependencySpec::path(key, value)
                        } else {
                            // По умолчанию любая версия
                            DependencySpec::version(key, VersionSpec::any())
                        };
                        config.dependencies.insert(key.to_string(), spec);
                    }
                    "сборка" | "build" => {
                        match key {
                            "главный" | "main" => {
                                config.build.main_file = PathBuf::from(value);
                            }
                            "выход" | "output" => {
                                config.build.output_dir = PathBuf::from(value);
                            }
                            "оптимизация" | "optimization" => {
                                config.build.optimization_level = value.parse().unwrap_or(1);
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(config)
    }

    /// Сохраняет конфигурацию в файл
    pub fn save(&self) -> Result<(), ConfigError> {
        let content = self.to_toml();
        fs::write(&self.config_path, content)
            .map_err(|e| ConfigError::IoError(e.to_string()))
    }

    /// Генерирует TOML представление
    pub fn to_toml(&self) -> String {
        let mut output = String::new();

        // Секция проекта
        output.push_str("[проект]\n");
        output.push_str(&format!("имя = \"{}\"\n", self.metadata.name));
        output.push_str(&format!("версия = \"{}\"\n", self.metadata.version));
        if let Some(desc) = &self.metadata.description {
            output.push_str(&format!("описание = \"{}\"\n", desc));
        }
        if !self.metadata.authors.is_empty() {
            output.push_str(&format!("авторы = {:?}\n", self.metadata.authors));
        }
        output.push('\n');

        // Зависимости
        if !self.dependencies.is_empty() {
            output.push_str("[зависимости]\n");
            for (name, spec) in &self.dependencies {
                if let Some(path) = &spec.path {
                    output.push_str(&format!("{} = {{ путь = \"{}\" }}\n", name, path.display()));
                } else {
                    output.push_str(&format!("{} = \"{}\"\n", name, spec.version));
                }
            }
            output.push('\n');
        }

        // Сборка
        output.push_str("[сборка]\n");
        output.push_str(&format!("главный = \"{}\"\n", self.build.main_file.display()));
        output.push_str(&format!("выход = \"{}\"\n", self.build.output_dir.display()));
        output.push_str(&format!("оптимизация = {}\n", self.build.optimization_level));

        output
    }

    /// Добавляет зависимость
    pub fn add_dependency(&mut self, spec: DependencySpec) {
        self.dependencies.insert(spec.name.clone(), spec);
    }

    /// Удаляет зависимость
    pub fn remove_dependency(&mut self, name: &str) -> Option<DependencySpec> {
        self.dependencies.remove(name)
    }

    /// Возвращает абсолютный путь к главному файлу
    pub fn main_file_path(&self) -> PathBuf {
        if self.build.main_file.is_absolute() {
            self.build.main_file.clone()
        } else {
            self.project_root.join(&self.build.main_file)
        }
    }

    /// Возвращает абсолютный путь к директории вывода
    pub fn output_dir_path(&self) -> PathBuf {
        if self.build.output_dir.is_absolute() {
            self.build.output_dir.clone()
        } else {
            self.project_root.join(&self.build.output_dir)
        }
    }
}

// ============================================================================
//                         ОШИБКИ
// ============================================================================

/// Ошибки конфигурации
#[derive(Debug, Clone)]
pub enum ConfigError {
    /// Ошибка ввода-вывода
    IoError(String),
    /// Ошибка парсинга
    ParseError(String),
    /// Файл не найден
    NotFound(PathBuf),
    /// Неверный формат
    InvalidFormat(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::IoError(e) => write!(f, "Ошибка IO: {}", e),
            ConfigError::ParseError(e) => write!(f, "Ошибка парсинга: {}", e),
            ConfigError::NotFound(p) => write!(f, "Файл не найден: {}", p.display()),
            ConfigError::InvalidFormat(e) => write!(f, "Неверный формат: {}", e),
        }
    }
}

impl std::error::Error for ConfigError {}

// ============================================================================
//                         LOCK ФАЙЛ
// ============================================================================

/// Запись в lock файле
#[derive(Debug, Clone)]
pub struct LockEntry {
    /// Имя библиотеки
    pub name: String,
    /// Точная версия
    pub version: Version,
    /// Хеш содержимого
    pub checksum: Option<String>,
    /// Источник
    pub source: String,
    /// Зависимости
    pub dependencies: Vec<String>,
}

/// Lock файл (kumir.lock)
///
/// Строгий формат TOML:
/// ```toml
/// # Этот файл генерируется автоматически. Не редактируйте вручную.
/// format_version = 1
///
/// [[package]]
/// name = "sockets"
/// version = "1.2.3"
/// source = "registry"
/// checksum = "sha256:..."
/// dependencies = ["http", "json"]
/// ```
#[derive(Debug, Clone, Default)]
pub struct LockFile {
    /// Путь к файлу
    pub path: PathBuf,
    /// Версия формата
    pub format_version: u32,
    /// Записи
    pub entries: HashMap<String, LockEntry>,
}

impl LockFile {
    /// Создаёт новый lock файл
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            format_version: 1,
            entries: HashMap::new(),
        }
    }

    /// Загружает lock файл
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::new(path));
        }

        let content = fs::read_to_string(path)
            .map_err(|e| ConfigError::IoError(e.to_string()))?;

        let value: Value = content
            .parse::<Value>()
            .map_err(|e: toml::de::Error| ConfigError::ParseError(e.to_string()))?;

        let table = value.as_table().ok_or_else(|| ConfigError::InvalidFormat(
            "Корневой объект kumir.lock должен быть TOML-таблицей".to_string(),
        ))?;

        let mut lock = Self::new(path);

        lock.format_version = table
            .get("format_version")
            .or_else(|| table.get("версия_формата"))
            .and_then(|v| v.as_integer())
            .map(|v| v as u32)
            .unwrap_or(1);

        if let Some(packages) = table.get("package").and_then(|v| v.as_array())
            .or_else(|| table.get("пакет").and_then(|v| v.as_array()))
            .or_else(|| table.get("packages").and_then(|v| v.as_array()))
        {
            for pkg in packages {
                let pkg_table = pkg.as_table().ok_or_else(|| ConfigError::InvalidFormat(
                    "[[package]] должен быть таблицей".to_string(),
                ))?;

                let get_str = |keys: &[&str]| -> Option<String> {
                    keys.iter()
                        .find_map(|k| pkg_table.get(*k).and_then(|v| v.as_str()))
                        .map(|s| s.to_string())
                };

                let name = get_str(&["name", "имя"]).ok_or_else(|| ConfigError::InvalidFormat(
                    "Поле name обязательно для записи [[package]]".to_string(),
                ))?;

                let version_str = get_str(&["version", "версия"]).ok_or_else(|| ConfigError::InvalidFormat(
                    format!("Поле version обязательно для пакета {}", name),
                ))?;

                let version = version_str.parse().map_err(|e: VersionParseError| ConfigError::InvalidFormat(
                    format!("Неверная версия для {}: {}", name, e.message),
                ))?;

                let source = get_str(&["source", "источник"]).ok_or_else(|| ConfigError::InvalidFormat(
                    format!("Поле source обязательно для пакета {}", name),
                ))?;

                let checksum = get_str(&["checksum", "хеш"]);

                let dependencies = pkg_table.get("dependencies")
                    .or_else(|| pkg_table.get("зависимости"))
                    .map(|v| v.as_array()
                        .ok_or_else(|| ConfigError::InvalidFormat(format!("dependencies {} должны быть массивом", name))))
                    .transpose()? // Result<Option<_>>
                    .map(|arr| {
                        arr.iter()
                            .map(|v| v.as_str()
                                .ok_or_else(|| ConfigError::InvalidFormat(format!("dependency {} должна быть строкой", name))))
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .transpose()? // Result<Option<Vec<&str>>>
                    .map(|vec| vec.into_iter().map(|s| s.to_string()).collect())
                    .unwrap_or_default();

                let entry = LockEntry {
                    name: name.clone(),
                    version,
                    checksum,
                    source,
                    dependencies,
                };

                lock.entries.insert(name, entry);
            }
        }

        Ok(lock)
    }

    /// Сохраняет lock файл
    pub fn save(&self) -> Result<(), ConfigError> {
        let mut output = String::new();
        
        output.push_str("# Этот файл генерируется автоматически. Не редактируйте вручную.\n");
        output.push_str(&format!("format_version = {}\n\n", self.format_version));
        
        for entry in self.entries.values() {
            output.push_str("[[package]]\n");
            output.push_str(&format!("name = \"{}\"\n", entry.name));
            output.push_str(&format!("version = \"{}\"\n", entry.version));
            if let Some(checksum) = &entry.checksum {
                output.push_str(&format!("checksum = \"{}\"\n", checksum));
            }
            output.push_str(&format!("source = \"{}\"\n", entry.source));
            if !entry.dependencies.is_empty() {
                let deps = entry
                    .dependencies
                    .iter()
                    .map(|d| format!("\"{}\"", d))
                    .collect::<Vec<_>>()
                    .join(", ");
                output.push_str(&format!("dependencies = [{}]\n", deps));
            }
            output.push('\n');
        }

        fs::write(&self.path, output)
            .map_err(|e| ConfigError::IoError(e.to_string()))
    }

    /// Добавляет или обновляет запись
    pub fn update(&mut self, entry: LockEntry) {
        self.entries.insert(entry.name.clone(), entry);
    }

    /// Проверяет, заблокирована ли библиотека
    pub fn is_locked(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }

    /// Получает заблокированную версию
    pub fn locked_version(&self, name: &str) -> Option<&Version> {
        self.entries.get(name).map(|e| &e.version)
    }
}

// ============================================================================
//                         ТЕСТЫ
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_parse() {
        let toml = r#"
[проект]
имя = "тест"
версия = "1.0.0"
описание = "Тестовый проект"

[зависимости]
Сокеты = "^2.0"
Графика = "1.5.0"

[сборка]
главный = "main.kum"
выход = "./build"
"#;
        
        let config = KumirConfig::parse(toml, Path::new("/test/kumir.toml")).unwrap();
        
        assert_eq!(config.metadata.name, "тест");
        assert_eq!(config.metadata.version, Version::new(1, 0, 0));
        assert!(config.dependencies.contains_key("Сокеты"));
        assert!(config.dependencies.contains_key("Графика"));
    }

    #[test]
    fn test_dependency_spec() {
        let dep = DependencySpec::version("test", VersionSpec::compatible(Version::new(1, 0, 0)));
        assert!(!dep.is_local());
        
        let local_dep = DependencySpec::path("local", "./libs/local");
        assert!(local_dep.is_local());
    }

    #[test]
    fn test_to_toml() {
        let mut config = KumirConfig::new("/test");
        config.metadata.name = "мой_проект".to_string();
        config.metadata.version = Version::new(2, 0, 0);
        config.add_dependency(DependencySpec::version(
            "Сокеты",
            VersionSpec::compatible(Version::new(1, 5, 0))
        ));
        
        let toml = config.to_toml();
        assert!(toml.contains("мой_проект"));
        assert!(toml.contains("2.0.0"));
        assert!(toml.contains("Сокеты"));
    }
}
