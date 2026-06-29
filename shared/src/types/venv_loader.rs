//! Интегрированный загрузчик библиотек с поддержкой виртуальных окружений
//!
//! Объединяет функциональность загрузки библиотек и управления окружениями.
//!
//! # Структура хранения
//!
//! ```text
//! ~/.kumir/
//! ├── registry/                    # Глобальный реестр библиотек
//! │   ├── sockets-1.0.0/
//! │   │   ├── manifest.toml        # Метаданные библиотеки
//! │   │   ├── lib.kum              # Исходный код (Kumir-библиотека)
//! │   │   └── native/              # Нативные модули (опционально)
//! │   │       └── sockets.dll
//! │   └── http-2.1.0/
//! │       └── ...
//! ├── cache/                       # Кэш скачанных пакетов
//! │   └── downloads/
//! └── config.toml                  # Глобальная конфигурация
//!
//! project/
//! ├── kumir.toml                   # Конфигурация проекта
//! ├── kumir.lock                   # Lock-файл с разрешёнными версиями
//! ├── libs/                        # Локальные библиотеки проекта
//! │   └── mylib/
//! │       └── lib.kum
//! └── main.kum
//! ```
//!
//! # Приоритет загрузки
//!
//! 1. Встроенные библиотеки (compiled-in)
//! 2. Локальные библиотеки проекта (./libs/)
//! 3. Lock-файл (kumir.lock) - точные версии
//! 4. Глобальный реестр (~/.kumir/registry/)
//! 5. Удалённые репозитории (в будущем)

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use once_cell::sync::Lazy;
use toml::Value;

use super::config::LockFile;
use super::environment::{EnvironmentManager, LibrarySource, ResolvedDependency, VersionedLibrary};
use super::library::{LibVersion, LibraryDef};
use super::version::{Version, VersionSpec};

// =============================================================================
//                         ОШИБКИ
// =============================================================================

/// Ошибки загрузчика
#[derive(Debug, Clone)]
pub enum LoaderError {
    /// Библиотека не найдена
    NotFound {
        name: String,
        searched_paths: Vec<PathBuf>,
    },
    /// Версия не найдена
    VersionNotFound {
        name: String,
        requested: VersionSpec,
        available: Vec<Version>,
    },
    /// Несовместимая версия
    VersionMismatch {
        name: String,
        required: VersionSpec,
        found: Version,
    },
    /// Ошибка чтения файла
    IoError(String),
    /// Ошибка парсинга библиотеки
    ParseError { path: PathBuf, message: String },
    /// Ошибка парсинга манифеста
    ManifestError { path: PathBuf, message: String },
    /// Циклическая зависимость
    CyclicDependency(Vec<String>),
    /// Конфликт версий
    VersionConflict {
        name: String,
        required_by: Vec<(String, VersionSpec)>,
    },
    /// Ошибка загрузки нативного модуля
    NativeLoadError { path: PathBuf, message: String },
    /// Окружение не инициализировано
    EnvironmentNotInitialized,
    /// Блокировка занята
    LockError(String),
}

impl std::fmt::Display for LoaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoaderError::NotFound {
                name,
                searched_paths,
            } => {
                write!(f, "Библиотека '{}' не найдена.\nПути поиска:\n", name)?;
                for path in searched_paths {
                    writeln!(f, "  - {}", path.display())?;
                }
                Ok(())
            }
            LoaderError::VersionNotFound {
                name,
                requested,
                available,
            } => {
                write!(
                    f,
                    "Версия {} библиотеки '{}' не найдена.\nДоступные версии: {}",
                    requested,
                    name,
                    available
                        .iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            LoaderError::VersionMismatch {
                name,
                required,
                found,
            } => {
                write!(
                    f,
                    "Несовместимая версия библиотеки '{}': требуется {}, найдена {}",
                    name, required, found
                )
            }
            LoaderError::IoError(e) => write!(f, "Ошибка чтения: {}", e),
            LoaderError::ParseError { path, message } => {
                write!(f, "Ошибка парсинга {}: {}", path.display(), message)
            }
            LoaderError::ManifestError { path, message } => {
                write!(f, "Ошибка манифеста {}: {}", path.display(), message)
            }
            LoaderError::CyclicDependency(chain) => {
                write!(f, "Циклическая зависимость: {}", chain.join(" → "))
            }
            LoaderError::VersionConflict { name, required_by } => {
                writeln!(f, "Конфликт версий библиотеки '{}':", name)?;
                for (dep, spec) in required_by {
                    writeln!(f, "  - {} требует {}", dep, spec)?;
                }
                Ok(())
            }
            LoaderError::NativeLoadError { path, message } => {
                write!(
                    f,
                    "Ошибка загрузки нативного модуля {}: {}",
                    path.display(),
                    message
                )
            }
            LoaderError::EnvironmentNotInitialized => {
                write!(f, "Окружение не инициализировано")
            }
            LoaderError::LockError(e) => write!(f, "Ошибка блокировки: {}", e),
        }
    }
}

impl std::error::Error for LoaderError {}

impl From<io::Error> for LoaderError {
    fn from(e: io::Error) -> Self {
        LoaderError::IoError(e.to_string())
    }
}

pub type LoaderResult<T> = Result<T, LoaderError>;

// =============================================================================
//                         МАНИФЕСТ БИБЛИОТЕКИ
// =============================================================================

/// Манифест библиотеки (manifest.toml)
#[derive(Debug, Clone)]
pub struct LibraryManifest {
    /// Идентификатор библиотеки
    pub id: String,
    /// Отображаемое имя
    pub name: String,
    /// Алиасы (альтернативные имена)
    pub aliases: Vec<String>,
    /// Описание
    pub description: String,
    /// Версия
    pub version: Version,
    /// Автор
    pub author: String,
    /// Минимальная версия Kumir
    pub kumir_version: String,
    /// Зависимости
    pub dependencies: Vec<ManifestDependency>,
    /// Точка входа (главный файл)
    pub entry_point: String,
    /// Нативные модули
    pub native_modules: Vec<String>,
    /// Стабильная ли библиотека
    pub stable: bool,
}

/// Зависимость в манифесте
#[derive(Debug, Clone)]
pub struct ManifestDependency {
    /// Имя библиотеки
    pub name: String,
    /// Спецификация версии
    pub version: VersionSpec,
    /// Опциональная зависимость
    pub optional: bool,
}

impl LibraryManifest {
    /// Парсит манифест из TOML строки
    pub fn parse(content: &str) -> LoaderResult<Self> {
        let value: Value =
            content
                .parse::<Value>()
                .map_err(|e: toml::de::Error| LoaderError::ParseError {
                    path: PathBuf::new(),
                    message: e.to_string(),
                })?;

        let table = value.as_table().ok_or_else(|| LoaderError::ParseError {
            path: PathBuf::new(),
            message: "manifest.toml должен быть TOML-объектом".to_string(),
        })?;

        let mut manifest = LibraryManifest {
            id: String::new(),
            name: String::new(),
            aliases: Vec::new(),
            description: String::new(),
            version: Version::new(0, 1, 0),
            author: String::new(),
            kumir_version: "3.0".to_string(),
            dependencies: Vec::new(),
            entry_point: "lib.kum".to_string(),
            native_modules: Vec::new(),
            stable: true,
        };

        let get_str = |t: &toml::value::Table, key: &str| -> Option<String> {
            t.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
        };

        manifest.name = get_str(table, "name").unwrap_or_default();
        manifest.id = get_str(table, "id").unwrap_or_else(|| manifest.name.clone());

        if manifest.id.is_empty() && manifest.name.is_empty() {
            return Err(LoaderError::ParseError {
                path: PathBuf::new(),
                message: "Не указан id или name в manifest.toml".to_string(),
            });
        }

        if let Some(ver) = get_str(table, "version") {
            manifest.version = ver.parse().map_err(|e| LoaderError::ParseError {
                path: PathBuf::new(),
                message: format!("Неверный формат version: {}", e),
            })?;
        } else {
            return Err(LoaderError::ParseError {
                path: PathBuf::new(),
                message: "Отсутствует обязательное поле version".to_string(),
            });
        }

        manifest.description = get_str(table, "description").unwrap_or_default();
        manifest.author = get_str(table, "author").unwrap_or_default();
        manifest.kumir_version =
            get_str(table, "kumir_version").unwrap_or_else(|| "3.0".to_string());
        manifest.entry_point =
            get_str(table, "entry_point").unwrap_or_else(|| "lib.kum".to_string());
        manifest.stable = table
            .get("stable")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        if let Some(arr) = table.get("aliases").and_then(|v| v.as_array()) {
            manifest.aliases = arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }

        if let Some(arr) = table.get("native_modules").and_then(|v| v.as_array()) {
            manifest.native_modules = arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }

        if let Some(deps) = table.get("dependencies").and_then(|v| v.as_table()) {
            for (dep_name, dep_val) in deps {
                match dep_val {
                    Value::String(req) => {
                        let version = req.parse().map_err(|e| LoaderError::ParseError {
                            path: PathBuf::new(),
                            message: format!("Неверная версия для зависимости {}: {}", dep_name, e),
                        })?;
                        manifest.dependencies.push(ManifestDependency {
                            name: dep_name.to_string(),
                            version,
                            optional: false,
                        });
                    }
                    Value::Table(dep_table) => {
                        let version_str = dep_table
                            .get("version")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| LoaderError::ParseError {
                                path: PathBuf::new(),
                                message: format!(
                                    "Зависимость {} должна содержать version",
                                    dep_name
                                ),
                            })?;

                        let version = version_str.parse().map_err(|e| LoaderError::ParseError {
                            path: PathBuf::new(),
                            message: format!("Неверная версия для зависимости {}: {}", dep_name, e),
                        })?;

                        let optional = dep_table
                            .get("optional")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);

                        manifest.dependencies.push(ManifestDependency {
                            name: dep_name.to_string(),
                            version,
                            optional,
                        });
                    }
                    _ => {
                        return Err(LoaderError::ParseError {
                            path: PathBuf::new(),
                            message: format!("Неверный формат зависимости {}", dep_name),
                        });
                    }
                }
            }
        }

        // Если name не указан, но есть id, используем id как name
        if manifest.name.is_empty() {
            manifest.name = manifest.id.clone();
        }

        Ok(manifest)
    }

    /// Загружает манифест из файла
    pub fn load(path: &Path) -> LoaderResult<Self> {
        let content = fs::read_to_string(path)?;
        Self::parse(&content).map_err(|e| LoaderError::ManifestError {
            path: path.to_path_buf(),
            message: e.to_string(),
        })
    }
}

// =============================================================================
//                         ЗАГРУЖЕННАЯ БИБЛИОТЕКА
// =============================================================================

/// Информация о загруженной библиотеке
#[derive(Debug, Clone)]
pub struct LoadedLibrary {
    /// Определение библиотеки
    pub def: LibraryDef,
    /// Версия
    pub version: Version,
    /// Источник загрузки
    pub source: LibrarySource,
    /// Путь к библиотеке (если есть)
    pub path: Option<PathBuf>,
    /// Исходный код (для Kumir-библиотек)
    pub source_code: Option<String>,
    /// Манифест (если есть)
    pub manifest: Option<LibraryManifest>,
}

impl LoadedLibrary {
    /// Конвертирует в VersionedLibrary для окружения
    pub fn into_versioned(self) -> VersionedLibrary {
        VersionedLibrary {
            def: self.def,
            version: self.version,
            source: self.source,
            path: self.path,
            checksum: None,
        }
    }
}

// =============================================================================
//                    ИНТЕГРИРОВАННЫЙ ЗАГРУЗЧИК
// =============================================================================

/// Глобальный загрузчик
pub static LOADER: Lazy<RwLock<IntegratedLoader>> =
    Lazy::new(|| RwLock::new(IntegratedLoader::new()));

/// Интегрированный загрузчик с поддержкой виртуальных окружений
pub struct IntegratedLoader {
    /// Менеджер окружений
    pub env_manager: EnvironmentManager,
    /// Встроенные библиотеки
    builtins: HashMap<String, LibraryDef>,
    /// Стек загрузки (для обнаружения циклов)
    loading_stack: Vec<String>,
    /// Кэш загруженных библиотек (путь -> библиотека)
    file_cache: HashMap<PathBuf, LoadedLibrary>,
}

impl IntegratedLoader {
    /// Создаёт новый загрузчик
    pub fn new() -> Self {
        Self {
            env_manager: EnvironmentManager::new(),
            builtins: HashMap::new(),
            loading_stack: Vec::new(),
            file_cache: HashMap::new(),
        }
    }

    // =========================================================================
    //                         РЕГИСТРАЦИЯ ВСТРОЕННЫХ
    // =========================================================================

    /// Регистрирует встроенную библиотеку
    pub fn register_builtin(&mut self, def: LibraryDef) {
        // Сохраняем по всем именам
        let name = def.name.to_string();
        self.builtins.insert(def.id.to_string(), def.clone());
        self.builtins.insert(name, def.clone());
        for alias in &def.aliases {
            self.builtins.insert(alias.to_string(), def.clone());
        }

        // Регистрируем в глобальном окружении
        self.env_manager.global_mut().register_builtin(def);
    }

    /// Проверяет, является ли библиотека встроенной
    pub fn is_builtin(&self, name: &str) -> bool {
        self.builtins.contains_key(name)
    }

    // =========================================================================
    //                         АКТИВАЦИЯ ПРОЕКТА
    // =========================================================================

    /// Активирует окружение проекта
    pub fn activate_project(&mut self, project_root: impl AsRef<Path>) -> LoaderResult<()> {
        let project_root = project_root.as_ref();

        // Активируем окружение
        self.env_manager.activate_project(project_root);

        // Загружаем kumir.lock если есть
        let lock_path = project_root.join("kumir.lock");
        if lock_path.exists() {
            self.load_lock_file(&lock_path)?;
        }

        Ok(())
    }

    /// Загружает lock-файл
    fn load_lock_file(&mut self, path: &Path) -> LoaderResult<()> {
        let lock = LockFile::load(path).map_err(|e| LoaderError::ManifestError {
            path: path.to_path_buf(),
            message: e.to_string(),
        })?;

        let paths = self.env_manager.active().paths.clone();

        let map_source = |src: &str| -> LibrarySource {
            match src {
                "builtin" => LibrarySource::Builtin,
                "local" => LibrarySource::Local(paths.local_libs.clone()),
                "registry" | "global" => LibrarySource::GlobalCache(paths.global_cache.clone()),
                s if s.starts_with("path:") => {
                    let p = s.trim_start_matches("path:");
                    LibrarySource::Path(PathBuf::from(p))
                }
                other => LibrarySource::Remote(other.to_string()),
            }
        };

        for entry in lock.entries.values() {
            let name = entry.name.clone();
            let version = entry.version.clone();
            let source = map_source(&entry.source);

            let lib = self.load_version(&name, &version)?;

            let env = self.env_manager.active_mut();
            env.register(lib.into_versioned());
            env.add_resolved(ResolvedDependency {
                name: name.clone(),
                version: version.clone(),
                source,
                requested: VersionSpec::exact(version),
            });
        }

        Ok(())
    }

    /// Деактивирует проект
    pub fn deactivate_project(&mut self) {
        self.env_manager.deactivate();
    }

    // =========================================================================
    //                         ЗАГРУЗКА БИБЛИОТЕК
    // =========================================================================

    /// Загружает библиотеку по имени
    pub fn load(&mut self, name: &str) -> LoaderResult<LoadedLibrary> {
        self.load_with_spec(name, &VersionSpec::any())
    }

    /// Загружает библиотеку с проверкой версии
    pub fn load_with_spec(
        &mut self,
        name: &str,
        spec: &VersionSpec,
    ) -> LoaderResult<LoadedLibrary> {
        // Проверяем на цикл
        if self.loading_stack.contains(&name.to_string()) {
            let mut chain = self.loading_stack.clone();
            chain.push(name.to_string());
            return Err(LoaderError::CyclicDependency(chain));
        }

        self.loading_stack.push(name.to_string());
        let result = self.load_impl(name, spec);
        self.loading_stack.pop();

        result
    }

    /// Загружает конкретную версию
    pub fn load_version(&mut self, name: &str, version: &Version) -> LoaderResult<LoadedLibrary> {
        self.load_with_spec(name, &VersionSpec::exact(version.clone()))
    }

    /// Внутренняя реализация загрузки
    fn load_impl(&mut self, name: &str, spec: &VersionSpec) -> LoaderResult<LoadedLibrary> {
        // 1. Проверяем в активном окружении
        if let Some(lib) = self.env_manager.find_library_matching(name, spec) {
            return Ok(LoadedLibrary {
                def: lib.def.clone(),
                version: lib.version.clone(),
                source: lib.source.clone(),
                path: lib.path.clone(),
                source_code: None,
                manifest: None,
            });
        }

        // 2. Встроенные библиотеки
        if let Some(def) = self.builtins.get(name) {
            let version = Version::new(def.version.major, def.version.minor, def.version.patch);

            if spec.matches(&version) {
                return Ok(LoadedLibrary {
                    def: def.clone(),
                    version,
                    source: LibrarySource::Builtin,
                    path: None,
                    source_code: None,
                    manifest: None,
                });
            }
        }

        // 3. Локальные библиотеки проекта
        let paths = self.env_manager.active().paths.clone();
        if let Some(lib) = self.try_load_local(&paths.local_libs, name, spec)? {
            return Ok(lib);
        }

        // 4. Глобальный реестр
        if let Some(lib) = self.try_load_from_registry(&paths.global_cache, name, spec)? {
            return Ok(lib);
        }

        // 5. Не найдено
        let searched_paths = vec![paths.local_libs, paths.global_cache];

        Err(LoaderError::NotFound {
            name: name.to_string(),
            searched_paths,
        })
    }

    /// Пытается загрузить из локальной папки
    fn try_load_local(
        &mut self,
        libs_dir: &Path,
        name: &str,
        spec: &VersionSpec,
    ) -> LoaderResult<Option<LoadedLibrary>> {
        // Новый целевой формат: libs/<name>/manifest.toml + entry_point
        let manifest_path = libs_dir.join(name).join("manifest.toml");
        if manifest_path.exists() {
            let manifest = LibraryManifest::load(&manifest_path)?;
            if !spec.matches(&manifest.version) {
                return Err(LoaderError::VersionMismatch {
                    name: name.to_string(),
                    required: spec.clone(),
                    found: manifest.version,
                });
            }

            let entry_path = libs_dir.join(name).join(&manifest.entry_point);
            if !entry_path.exists() {
                return Err(LoaderError::ParseError {
                    path: entry_path,
                    message: "entry_point не найден".to_string(),
                });
            }

            let mut lib =
                self.load_from_file(&entry_path, LibrarySource::Local(entry_path.clone()))?;
            lib.manifest = Some(manifest.clone());
            lib.version = manifest.version;
            return Ok(Some(lib));
        }

        // Fallback для старых путей
        let variants = [
            libs_dir.join(format!("{}.kum", name)),
            libs_dir.join(format!("{}/lib.kum", name)),
            libs_dir.join(format!("{}/mod.kum", name)),
        ];

        for path in variants {
            if path.exists() {
                let lib = self.load_from_file(&path, LibrarySource::Local(path.clone()))?;

                if spec.matches(&lib.version) {
                    return Ok(Some(lib));
                }
            }
        }

        Ok(None)
    }

    /// Пытается загрузить из глобального реестра
    fn try_load_from_registry(
        &mut self,
        registry_dir: &Path,
        name: &str,
        spec: &VersionSpec,
    ) -> LoaderResult<Option<LoadedLibrary>> {
        if !registry_dir.exists() {
            return Ok(None);
        }

        // Ищем все версии библиотеки
        let mut versions: Vec<(Version, PathBuf)> = Vec::new();

        if let Ok(entries) = fs::read_dir(registry_dir) {
            for entry in entries.flatten() {
                let dir_name = entry.file_name().to_string_lossy().to_string();

                // Формат: name-version (например sockets-1.0.0)
                if let Some(suffix) = dir_name.strip_prefix(&format!("{}-", name))
                    && let Ok(version) = suffix.parse::<Version>()
                {
                    versions.push((version, entry.path()));
                }
            }
        }

        // Сортируем по убыванию версии
        versions.sort_by(|a, b| b.0.cmp(&a.0));

        // Ищем подходящую версию
        for (version, path) in versions {
            if spec.matches(&version) {
                // Ищем manifest.toml или lib.kum
                let manifest_path = path.join("manifest.toml");
                let lib_path = path.join("lib.kum");

                if manifest_path.exists() {
                    let manifest = LibraryManifest::load(&manifest_path)?;

                    // Проверяем, что версия из имени каталога совпадает с манифестом
                    if manifest.version != version {
                        return Err(LoaderError::VersionMismatch {
                            name: name.to_string(),
                            required: VersionSpec::exact(version),
                            found: manifest.version,
                        });
                    }

                    let entry_path = path.join(&manifest.entry_point);

                    if entry_path.exists() {
                        let mut lib =
                            self.load_from_file(&entry_path, LibrarySource::GlobalCache(path))?;
                        lib.manifest = Some(manifest);
                        lib.version = version;
                        return Ok(Some(lib));
                    }
                } else if lib_path.exists() {
                    let mut lib =
                        self.load_from_file(&lib_path, LibrarySource::GlobalCache(path))?;
                    lib.version = version;
                    return Ok(Some(lib));
                }
            }
        }

        Ok(None)
    }

    /// Загружает библиотеку из файла
    pub fn load_from_file(
        &mut self,
        path: &Path,
        source: LibrarySource,
    ) -> LoaderResult<LoadedLibrary> {
        // Проверяем кэш
        if let Some(lib) = self.file_cache.get(path) {
            return Ok(lib.clone());
        }

        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        let lib = match extension {
            "kum" | "kumir" => self.parse_kumir_file(path, source)?,
            _ => {
                return Err(LoaderError::ParseError {
                    path: path.to_path_buf(),
                    message: format!("Неизвестный тип файла: .{}", extension),
                });
            }
        };

        // Кэшируем
        self.file_cache.insert(path.to_path_buf(), lib.clone());

        Ok(lib)
    }

    /// Парсит Kumir-файл библиотеки
    fn parse_kumir_file(&self, path: &Path, source: LibrarySource) -> LoaderResult<LoadedLibrary> {
        let content = fs::read_to_string(path)?;

        // Извлекаем метаданные из комментариев и директив
        let mut name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        let mut description = String::new();
        let mut version = Version::new(1, 0, 0);
        let mut author = String::new();
        let mut aliases: Vec<Arc<str>> = Vec::new();

        for line in content.lines() {
            let line = line.trim();

            // | библиотека ИмяБиблиотеки
            if (line.starts_with("библиотека ") || line.starts_with("library "))
                && let Some(lib_name) = line.split_whitespace().nth(1)
            {
                name = lib_name.to_string();
            }

            // @-директивы в комментариях
            // | @описание Текст
            if let Some(desc) = line
                .strip_prefix("@описание ")
                .or(line.strip_prefix("@description "))
            {
                description = desc.to_string();
            }

            // | @версия 1.2.3
            if let Some(ver) = line
                .strip_prefix("@версия ")
                .or(line.strip_prefix("@version "))
                && let Ok(v) = ver.trim().parse()
            {
                version = v;
            }

            // | @автор Имя
            if let Some(auth) = line
                .strip_prefix("@автор ")
                .or(line.strip_prefix("@author "))
            {
                author = auth.to_string();
            }

            // | @алиас ДругоеИмя
            if let Some(alias) = line
                .strip_prefix("@алиас ")
                .or(line.strip_prefix("@alias "))
            {
                aliases.push(Arc::from(alias.trim()));
            }
        }

        // Создаём LibraryDef
        let def = LibraryDef {
            id: Arc::from(name.as_str()),
            name: Arc::from(name.as_str()),
            aliases,
            description: if description.is_empty() {
                None
            } else {
                Some(Arc::from(description.as_str()))
            },
            version: LibVersion::new(version.major, version.minor, version.patch),
            author: Arc::from(author.as_str()),
            dependencies: Vec::new(),
            classes: Vec::new(),
            functions: Vec::new(),
            types: Vec::new(),
            constants: Vec::new(),
            kumir_version: Some(Version::new(3, 0, 0)),
            stable: true,
        };

        Ok(LoadedLibrary {
            def,
            version,
            source,
            path: Some(path.to_path_buf()),
            source_code: Some(content),
            manifest: None,
        })
    }

    // =========================================================================
    //                    ЗАГРУЗКА С ЗАВИСИМОСТЯМИ
    // =========================================================================

    /// Загружает библиотеку со всеми зависимостями
    pub fn load_with_dependencies(&mut self, name: &str) -> LoaderResult<Vec<LoadedLibrary>> {
        self.load_with_dependencies_spec(name, &VersionSpec::any())
    }

    /// Загружает библиотеку с зависимостями и проверкой версии
    pub fn load_with_dependencies_spec(
        &mut self,
        name: &str,
        spec: &VersionSpec,
    ) -> LoaderResult<Vec<LoadedLibrary>> {
        let mut loaded: Vec<LoadedLibrary> = Vec::new();
        let mut to_load: Vec<(String, VersionSpec)> = vec![(name.to_string(), spec.clone())];

        while let Some((lib_name, lib_spec)) = to_load.pop() {
            // Пропускаем уже загруженные
            if loaded
                .iter()
                .any(|l| l.def.name.as_ref() == lib_name || l.def.id.as_ref() == lib_name)
            {
                continue;
            }

            let lib = self.load_with_spec(&lib_name, &lib_spec)?;

            // Добавляем зависимости в очередь
            if let Some(ref manifest) = lib.manifest {
                for dep in &manifest.dependencies {
                    if !dep.optional {
                        to_load.push((dep.name.clone(), dep.version.clone()));
                    }
                }
            }

            // Регистрируем в окружении
            let versioned = lib.clone().into_versioned();
            self.env_manager.active_mut().register(versioned);

            loaded.push(lib);
        }

        Ok(loaded)
    }

    // =========================================================================
    //                         ИНФОРМАЦИЯ
    // =========================================================================

    /// Возвращает список доступных библиотек
    pub fn available_libraries(&self) -> Vec<String> {
        let mut libs: Vec<String> = self.builtins.keys().cloned().collect();

        // Добавляем из окружения
        for lib in self.env_manager.active().all_libraries() {
            if !libs.contains(&lib.def.name.to_string()) {
                libs.push(lib.def.name.to_string());
            }
        }

        // Сканируем реестр
        let paths = self.env_manager.active().paths.clone();
        if let Ok(entries) = fs::read_dir(&paths.global_cache) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                // Формат: name-version
                if let Some(pos) = name.rfind('-') {
                    let lib_name = &name[..pos];
                    if !libs.contains(&lib_name.to_string()) {
                        libs.push(lib_name.to_string());
                    }
                }
            }
        }

        libs.sort();
        libs.dedup();
        libs
    }

    /// Возвращает все версии библиотеки в реестре
    pub fn available_versions(&self, name: &str) -> Vec<Version> {
        let mut versions: Vec<Version> = Vec::new();

        // Из окружения
        versions.extend(self.env_manager.active().available_versions(name));

        // Из реестра
        let paths = self.env_manager.active().paths.clone();
        if let Ok(entries) = fs::read_dir(&paths.global_cache) {
            for entry in entries.flatten() {
                let dir_name = entry.file_name().to_string_lossy().to_string();
                if let Some(suffix) = dir_name.strip_prefix(&format!("{}-", name))
                    && let Ok(version) = suffix.parse::<Version>()
                {
                    versions.push(version);
                }
            }
        }

        versions.sort();
        versions.dedup();
        versions
    }

    /// Очищает кэш файлов
    pub fn clear_cache(&mut self) {
        self.file_cache.clear();
    }
}

impl Default for IntegratedLoader {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
//                         ПУБЛИЧНОЕ API
// =============================================================================

/// Получает глобальный загрузчик
pub fn loader() -> &'static RwLock<IntegratedLoader> {
    &LOADER
}

/// Регистрирует встроенную библиотеку
pub fn register_builtin(def: LibraryDef) {
    if let Ok(mut loader) = LOADER.write() {
        loader.register_builtin(def);
    }
}

/// Загружает библиотеку
pub fn load_library(name: &str) -> LoaderResult<LoadedLibrary> {
    LOADER
        .write()
        .map_err(|_| LoaderError::LockError("Не удалось получить блокировку".to_string()))?
        .load(name)
}

/// Загружает библиотеку с проверкой версии
pub fn load_library_versioned(name: &str, spec: &VersionSpec) -> LoaderResult<LoadedLibrary> {
    LOADER
        .write()
        .map_err(|_| LoaderError::LockError("Не удалось получить блокировку".to_string()))?
        .load_with_spec(name, spec)
}

/// Загружает библиотеку со всеми зависимостями
pub fn load_library_with_deps(name: &str) -> LoaderResult<Vec<LoadedLibrary>> {
    LOADER
        .write()
        .map_err(|_| LoaderError::LockError("Не удалось получить блокировку".to_string()))?
        .load_with_dependencies(name)
}

/// Активирует проект
pub fn activate_project(path: impl AsRef<Path>) -> LoaderResult<()> {
    LOADER
        .write()
        .map_err(|_| LoaderError::LockError("Не удалось получить блокировку".to_string()))?
        .activate_project(path)
}

/// Деактивирует проект
pub fn deactivate_project() {
    if let Ok(mut loader) = LOADER.write() {
        loader.deactivate_project();
    }
}

/// Возвращает список доступных библиотек
pub fn list_available() -> Vec<String> {
    LOADER
        .read()
        .map(|l| l.available_libraries())
        .unwrap_or_default()
}

// =============================================================================
//                         ТЕСТЫ
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_parse() {
        let toml = r#"
id = "sockets"
name = "Сокеты"
description = "Библиотека для работы с сетью"
version = "1.2.3"
author = "Kumir Team"
aliases = ["сокеты", "net"]

[dependencies]
http = "^2.0"

[dependencies.json]
version = ">=1.0"
optional = true
"#;

        let manifest = LibraryManifest::parse(toml).unwrap();

        assert_eq!(manifest.id, "sockets");
        assert_eq!(manifest.name, "Сокеты");
        assert_eq!(manifest.version, Version::new(1, 2, 3));
        assert_eq!(manifest.aliases, vec!["сокеты", "net"]);
        assert_eq!(manifest.dependencies.len(), 2);

        let http_dep = manifest
            .dependencies
            .iter()
            .find(|d| d.name == "http")
            .unwrap();
        assert!(!http_dep.optional);

        let json_dep = manifest
            .dependencies
            .iter()
            .find(|d| d.name == "json")
            .unwrap();
        assert!(json_dep.optional);
    }

    #[test]
    fn test_loader_creation() {
        let loader = IntegratedLoader::new();
        assert!(loader.builtins.is_empty());
    }

    #[test]
    fn test_builtin_registration() {
        let mut loader = IntegratedLoader::new();

        let def = LibraryDef::new("test", "Тест");
        loader.register_builtin(def);

        assert!(loader.is_builtin("test"));
        assert!(loader.is_builtin("Тест"));
    }

    #[test]
    fn test_load_builtin() {
        let mut loader = IntegratedLoader::new();

        let def = LibraryDef::new("mylib", "МояБиблиотека");
        loader.register_builtin(def);

        let result = loader.load("mylib");
        assert!(result.is_ok());

        let lib = result.unwrap();
        assert!(matches!(lib.source, LibrarySource::Builtin));
    }

    #[test]
    fn test_load_not_found() {
        let mut loader = IntegratedLoader::new();
        let result = loader.load("nonexistent");
        assert!(matches!(result, Err(LoaderError::NotFound { .. })));
    }
}
