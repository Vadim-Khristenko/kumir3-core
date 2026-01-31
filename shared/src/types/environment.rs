//! Виртуальные окружения для Kumir 3
//!
//! Система виртуальных окружений позволяет:
//! - Изолировать зависимости проекта
//! - Использовать разные версии библиотек в разных проектах
//! - Работать с несколькими версиями Kumir 3 одновременно

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::version::{Version, VersionSpec};
use super::library::LibraryDef;

// ============================================================================
//                         ПУТИ ОКРУЖЕНИЯ
// ============================================================================

/// Структура путей для виртуального окружения
#[derive(Debug, Clone)]
pub struct EnvPaths {
    /// Корневая директория окружения (~/.kumir)
    pub root: PathBuf,
    /// Директория глобального кэша библиотек
    pub global_cache: PathBuf,
    /// Директория локальных библиотек проекта
    pub local_libs: PathBuf,
    /// Файл конфигурации проекта (kumir.toml)
    pub config_file: PathBuf,
    /// Файл блокировки версий (kumir.lock)
    pub lock_file: PathBuf,
}

impl EnvPaths {
    /// Создаёт пути для глобального окружения
    pub fn global() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        // Новый целевой путь: ~/.kumir
        let root = home.join(".kumir");
        
        Self {
            global_cache: root.join("registry"),
            local_libs: root.join("libs"),
            config_file: root.join("config.toml"),
            // Глобальный lock для registry пока не используется, храним рядом
            lock_file: root.join("registry.lock"),
            root,
        }
    }

    /// Создаёт пути для локального окружения проекта
    pub fn local(project_root: impl AsRef<Path>) -> Self {
        let project_root = project_root.as_ref();
        let root = project_root.to_path_buf();
        
        Self {
            global_cache: Self::global().global_cache,
            // Локальные библиотеки лежат в корне проекта: ./libs
            local_libs: root.join("libs"),
            config_file: project_root.join("kumir.toml"),
            lock_file: project_root.join("kumir.lock"),
            root,
        }
    }

    /// Проверяет, существует ли окружение
    pub fn exists(&self) -> bool {
        self.root.exists()
    }

    /// Возвращает путь к кэшу конкретной версии библиотеки
    /// Формат: <global_cache>/<name>-<version>/
    pub fn library_cache_path(&self, name: &str, version: &Version) -> PathBuf {
        self.global_cache.join(format!("{}-{}", name, version))
    }

    /// Возвращает путь к локальной библиотеке
    pub fn local_library_path(&self, name: &str) -> PathBuf {
        self.local_libs.join(name)
    }
}

// ============================================================================
//                         ИСТОЧНИК БИБЛИОТЕКИ
// ============================================================================

/// Откуда загружена библиотека
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LibrarySource {
    /// Встроенная в интерпретатор
    Builtin,
    /// Из глобального кэша
    GlobalCache(PathBuf),
    /// Из локальной директории проекта
    Local(PathBuf),
    /// Из удалённого репозитория (URL)
    Remote(String),
    /// Относительный путь
    Path(PathBuf),
}

impl LibrarySource {
    /// Является ли источник локальным
    pub fn is_local(&self) -> bool {
        matches!(self, LibrarySource::Local(_) | LibrarySource::Path(_))
    }

    /// Является ли источник кэшируемым
    pub fn is_cacheable(&self) -> bool {
        matches!(self, LibrarySource::GlobalCache(_) | LibrarySource::Remote(_))
    }
}

// ============================================================================
//                    ВЕРСИОНИРОВАННАЯ БИБЛИОТЕКА
// ============================================================================

/// Библиотека с информацией о версии и источнике
#[derive(Debug, Clone)]
pub struct VersionedLibrary {
    /// Определение библиотеки
    pub def: LibraryDef,
    /// Версия (из определения или переопределённая)
    pub version: Version,
    /// Откуда загружена
    pub source: LibrarySource,
    /// Полный путь к библиотеке (если есть)
    pub path: Option<PathBuf>,
    /// Хеш содержимого для проверки целостности
    pub checksum: Option<String>,
}

impl VersionedLibrary {
    /// Создаёт из встроенной библиотеки
    pub fn from_builtin(def: LibraryDef) -> Self {
        let version = Version::new(
            def.version.major,
            def.version.minor,
            def.version.patch,
        );
        Self {
            def,
            version,
            source: LibrarySource::Builtin,
            path: None,
            checksum: None,
        }
    }

    /// Уникальный ключ для кэширования
    pub fn cache_key(&self) -> String {
        format!("{}@{}", self.def.id, self.version)
    }

    /// Полное имя с версией
    pub fn full_name(&self) -> String {
        format!("{}:{}", self.def.name, self.version)
    }
}

// ============================================================================
//                         ВИРТУАЛЬНОЕ ОКРУЖЕНИЕ
// ============================================================================

/// Разрешённая зависимость
#[derive(Debug, Clone)]
pub struct ResolvedDependency {
    /// Имя библиотеки
    pub name: String,
    /// Разрешённая версия
    pub version: Version,
    /// Источник
    pub source: LibrarySource,
    /// Запрошенная спецификация версии
    pub requested: VersionSpec,
}

/// Виртуальное окружение проекта
#[derive(Debug)]
pub struct VirtualEnvironment {
    /// Имя окружения
    pub name: String,
    /// Пути окружения
    pub paths: EnvPaths,
    /// Загруженные библиотеки (имя -> версия -> библиотека)
    libraries: HashMap<String, HashMap<String, Arc<VersionedLibrary>>>,
    /// Разрешённые зависимости (для lock файла)
    resolved: HashMap<String, ResolvedDependency>,
    /// Активные версии библиотек (имя -> активная версия)
    active_versions: HashMap<String, Version>,
    /// Родительское окружение (для наследования)
    parent: Option<Box<VirtualEnvironment>>,
}

impl VirtualEnvironment {
    /// Создаёт новое виртуальное окружение
    pub fn new(name: impl Into<String>, paths: EnvPaths) -> Self {
        Self {
            name: name.into(),
            paths,
            libraries: HashMap::new(),
            resolved: HashMap::new(),
            active_versions: HashMap::new(),
            parent: None,
        }
    }

    /// Создаёт глобальное окружение
    pub fn global() -> Self {
        Self::new("global", EnvPaths::global())
    }

    /// Создаёт локальное окружение для проекта
    pub fn for_project(project_root: impl AsRef<Path>) -> Self {
        let paths = EnvPaths::local(&project_root);
        let name = project_root.as_ref()
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("project")
            .to_string();
        
        Self::new(name, paths)
    }

    /// Создаёт дочернее окружение с наследованием
    pub fn child(parent: VirtualEnvironment, name: impl Into<String>) -> Self {
        let paths = parent.paths.clone();
        Self {
            name: name.into(),
            paths,
            libraries: HashMap::new(),
            resolved: HashMap::new(),
            active_versions: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    // ========================================================================
    //                         РЕГИСТРАЦИЯ
    // ========================================================================

    /// Регистрирует библиотеку в окружении
    pub fn register(&mut self, lib: VersionedLibrary) {
        let name = lib.def.name.to_string();
        let version_str = lib.version.to_string();
        
        // Добавляем в библиотеки
        self.libraries
            .entry(name.clone())
            .or_insert_with(HashMap::new)
            .insert(version_str, Arc::new(lib.clone()));

        // Если это первая версия, делаем её активной
        if !self.active_versions.contains_key(&name) {
            self.active_versions.insert(name, lib.version);
        }
    }

    /// Регистрирует встроенную библиотеку
    pub fn register_builtin(&mut self, def: LibraryDef) {
        self.register(VersionedLibrary::from_builtin(def));
    }

    /// Устанавливает активную версию библиотеки
    pub fn set_active_version(&mut self, name: &str, version: Version) -> Result<(), String> {
        let version_str = version.to_string();
        
        // Проверяем, что версия существует
        if !self.libraries
            .get(name)
            .map(|versions| versions.contains_key(&version_str))
            .unwrap_or(false)
        {
            return Err(format!(
                "Версия {} библиотеки '{}' не найдена в окружении",
                version, name
            ));
        }

        self.active_versions.insert(name.to_string(), version);
        Ok(())
    }

    // ========================================================================
    //                         ПОИСК
    // ========================================================================

    /// Нормализует имя библиотеки (ищет основное имя по алиасам)
    fn normalize_name(&self, name: &str) -> Option<String> {
        // Если это уже основное имя
        if self.libraries.contains_key(name) {
            return Some(name.to_string());
        }
        
        // Ищем по алиасам в текущем окружении
        for (main_name, versions) in &self.libraries {
            if let Some(lib) = versions.values().next() {
                if lib.def.matches_name(name) {
                    return Some(main_name.clone());
                }
            }
        }
        
        // Ищем в родительском окружении
        if let Some(parent) = &self.parent {
            return parent.normalize_name(name);
        }
        
        None
    }

    /// Ищет библиотеку по имени (активная версия)
    pub fn find(&self, name: &str) -> Option<Arc<VersionedLibrary>> {
        // Нормализуем имя (учитывая алиасы)
        let main_name = self.normalize_name(name)?;
        
        // Сначала ищем в текущем окружении
        if let Some(version) = self.active_versions.get(&main_name) {
            if let Some(lib) = self.find_version_by_main_name(&main_name, version) {
                return Some(lib);
            }
        }

        // Ищем любую версию
        if let Some(versions) = self.libraries.get(&main_name) {
            if let Some(lib) = versions.values().next() {
                return Some(Arc::clone(lib));
            }
        }

        // Ищем в родительском окружении
        if let Some(parent) = &self.parent {
            return parent.find(name);
        }

        None
    }
    
    /// Вспомогательный метод для поиска по основному имени
    fn find_version_by_main_name(&self, main_name: &str, version: &Version) -> Option<Arc<VersionedLibrary>> {
        let version_str = version.to_string();
        
        if let Some(versions) = self.libraries.get(main_name) {
            if let Some(lib) = versions.get(&version_str) {
                return Some(Arc::clone(lib));
            }
        }
        
        None
    }

    /// Ищет конкретную версию библиотеки
    pub fn find_version(&self, name: &str, version: &Version) -> Option<Arc<VersionedLibrary>> {
        // Нормализуем имя
        let main_name = self.normalize_name(name)?;
        
        if let Some(lib) = self.find_version_by_main_name(&main_name, version) {
            return Some(lib);
        }

        // Ищем в родительском окружении
        if let Some(parent) = &self.parent {
            return parent.find_version(name, version);
        }

        None
    }

    /// Ищет библиотеку по спецификации версии
    pub fn find_matching(&self, name: &str, spec: &VersionSpec) -> Option<Arc<VersionedLibrary>> {
        // Нормализуем имя
        let main_name = self.normalize_name(name);
        
        // Собираем все подходящие версии
        let mut matching: Vec<Arc<VersionedLibrary>> = Vec::new();

        if let Some(main_name) = &main_name {
            if let Some(versions) = self.libraries.get(main_name) {
                for lib in versions.values() {
                    if spec.matches(&lib.version) {
                        matching.push(Arc::clone(lib));
                    }
                }
            }
        }

        // Ищем в родительском окружении
        if let Some(parent) = &self.parent {
            if let Some(lib) = parent.find_matching(name, spec) {
                if spec.matches(&lib.version) {
                    matching.push(lib);
                }
            }
        }

        // Возвращаем самую новую подходящую версию
        matching.into_iter().max_by_key(|lib| lib.version.clone())
    }

    /// Проверяет, существует ли библиотека
    pub fn exists(&self, name: &str) -> bool {
        self.normalize_name(name).is_some()
    }

    /// Возвращает все доступные версии библиотеки
    pub fn available_versions(&self, name: &str) -> Vec<Version> {
        let mut versions: Vec<Version> = Vec::new();
        
        // Нормализуем имя
        if let Some(main_name) = self.normalize_name(name) {
            if let Some(libs) = self.libraries.get(&main_name) {
                for lib in libs.values() {
                    versions.push(lib.version.clone());
                }
            }
        }

        if let Some(parent) = &self.parent {
            versions.extend(parent.available_versions(name));
        }

        versions.sort();
        versions.dedup();
        versions
    }

    // ========================================================================
    //                         ИНФОРМАЦИЯ
    // ========================================================================

    /// Возвращает все библиотеки в окружении
    pub fn all_libraries(&self) -> Vec<Arc<VersionedLibrary>> {
        let mut result: Vec<Arc<VersionedLibrary>> = Vec::new();

        for versions in self.libraries.values() {
            for lib in versions.values() {
                result.push(Arc::clone(lib));
            }
        }

        if let Some(parent) = &self.parent {
            result.extend(parent.all_libraries());
        }

        result
    }

    /// Возвращает разрешённые зависимости
    pub fn resolved_dependencies(&self) -> &HashMap<String, ResolvedDependency> {
        &self.resolved
    }

    /// Добавляет разрешённую зависимость
    pub fn add_resolved(&mut self, dep: ResolvedDependency) {
        self.resolved.insert(dep.name.clone(), dep);
    }

    /// Очищает окружение
    pub fn clear(&mut self) {
        self.libraries.clear();
        self.resolved.clear();
        self.active_versions.clear();
    }
}

// ============================================================================
//                         МЕНЕДЖЕР ОКРУЖЕНИЙ
// ============================================================================

/// Менеджер виртуальных окружений
pub struct EnvironmentManager {
    /// Глобальное окружение
    global: VirtualEnvironment,
    /// Активное окружение
    active: Option<VirtualEnvironment>,
    /// Стек окружений (для вложенных контекстов)
    stack: Vec<VirtualEnvironment>,
}

impl EnvironmentManager {
    /// Создаёт менеджер с глобальным окружением
    pub fn new() -> Self {
        Self {
            global: VirtualEnvironment::global(),
            active: None,
            stack: Vec::new(),
        }
    }

    /// Возвращает глобальное окружение
    pub fn global(&self) -> &VirtualEnvironment {
        &self.global
    }

    /// Возвращает глобальное окружение (мутабельно)
    pub fn global_mut(&mut self) -> &mut VirtualEnvironment {
        &mut self.global
    }

    /// Возвращает активное окружение (или глобальное)
    pub fn active(&self) -> &VirtualEnvironment {
        self.active.as_ref().unwrap_or(&self.global)
    }

    /// Возвращает активное окружение (мутабельно)
    pub fn active_mut(&mut self) -> &mut VirtualEnvironment {
        self.active.as_mut().unwrap_or(&mut self.global)
    }

    /// Активирует окружение проекта
    pub fn activate_project(&mut self, project_root: impl AsRef<Path>) {
        let mut env = VirtualEnvironment::for_project(project_root);
        
        // Наследуем библиотеки из глобального окружения
        for lib in self.global.all_libraries() {
            env.register((*lib).clone());
        }
        
        self.active = Some(env);
    }

    /// Деактивирует текущее окружение
    pub fn deactivate(&mut self) {
        self.active = None;
    }

    /// Сохраняет текущее окружение в стек и активирует новое
    pub fn push_context(&mut self, env: VirtualEnvironment) {
        if let Some(current) = self.active.take() {
            self.stack.push(current);
        }
        self.active = Some(env);
    }

    /// Восстанавливает предыдущее окружение из стека
    pub fn pop_context(&mut self) -> Option<VirtualEnvironment> {
        let popped = self.active.take();
        self.active = self.stack.pop();
        popped
    }

    /// Ищет библиотеку в активном окружении
    pub fn find_library(&self, name: &str) -> Option<Arc<VersionedLibrary>> {
        self.active().find(name)
    }

    /// Ищет библиотеку с конкретной версией
    pub fn find_library_version(&self, name: &str, version: &Version) -> Option<Arc<VersionedLibrary>> {
        self.active().find_version(name, version)
    }

    /// Ищет библиотеку по спецификации
    pub fn find_library_matching(&self, name: &str, spec: &VersionSpec) -> Option<Arc<VersionedLibrary>> {
        self.active().find_matching(name, spec)
    }
}

impl Default for EnvironmentManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
//                         ТЕСТЫ
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::library::{LibraryDef, LibVersion};

    fn test_library(name: &'static str, major: u32, minor: u32, patch: u32) -> LibraryDef {
        LibraryDef {
            id: name,
            name,
            aliases: &[],
            description: "",
            version: LibVersion::new(major, minor, patch),
            author: "",
            dependencies: Vec::new(),
            classes: Vec::new(),
            functions: Vec::new(),
            types: Vec::new(),
            constants: Vec::new(),
            kumir_version: "3.0",
            stable: true,
        }
    }

    #[test]
    fn test_versioned_library() {
        let def = test_library("test", 1, 2, 3);
        let lib = VersionedLibrary::from_builtin(def);
        
        assert_eq!(lib.version.major, 1);
        assert_eq!(lib.version.minor, 2);
        assert_eq!(lib.version.patch, 3);
        assert_eq!(lib.cache_key(), "test@1.2.3");
    }

    #[test]
    fn test_environment_registration() {
        let mut env = VirtualEnvironment::new("test", EnvPaths::global());
        
        let lib1 = VersionedLibrary::from_builtin(test_library("mylib", 1, 0, 0));
        let lib2 = VersionedLibrary::from_builtin(test_library("mylib", 2, 0, 0));
        
        env.register(lib1);
        env.register(lib2);
        
        let versions = env.available_versions("mylib");
        assert_eq!(versions.len(), 2);
        
        // Первая зарегистрированная версия становится активной
        let active = env.find("mylib").unwrap();
        assert_eq!(active.version, Version::new(1, 0, 0));
    }

    #[test]
    fn test_version_matching() {
        let mut env = VirtualEnvironment::new("test", EnvPaths::global());
        
        env.register(VersionedLibrary::from_builtin(test_library("lib", 1, 0, 0)));
        env.register(VersionedLibrary::from_builtin(test_library("lib", 1, 5, 0)));
        env.register(VersionedLibrary::from_builtin(test_library("lib", 2, 0, 0)));
        
        // ^1.0 должен найти 1.5.0 (самая новая совместимая)
        let spec: VersionSpec = "^1.0.0".parse().unwrap();
        let lib = env.find_matching("lib", &spec).unwrap();
        assert_eq!(lib.version, Version::new(1, 5, 0));
    }
}
