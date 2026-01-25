//! Динамический реестр библиотек Kumir 3
//!
//! Поддерживает:
//! - Виртуальные окружения
//! - Версионирование библиотек
//! - Глобальный кэш
//! - Локальные библиотеки проекта

use once_cell::sync::Lazy;
use std::sync::RwLock;
use std::path::Path;

use crate::shared::types::library::LibraryDef;
use crate::shared::types::version::{Version, VersionSpec};
use crate::shared::types::environment::{
    VersionedLibrary, EnvironmentManager, EnvPaths, LibrarySource,
};
use crate::shared::types::config::{KumirConfig, DependencySpec};
use crate::shared::types::resolver::{DependencyResolver, LibraryProvider};
use crate::shared::types::venv_loader;
use crate::shared::libraries::time::create_time_library;
use crate::shared::libraries::syscall::create_syscall_library;
use crate::shared::libraries::files::create_files_library;
use crate::shared::libraries::net::create_net_library;

// Единоразовая регистрация встроенных библиотек в интегрированном загрузчике
static BUILTINS_LOADER_INIT: Lazy<()> = Lazy::new(|| {
    venv_loader::register_builtin(create_time_library());
    venv_loader::register_builtin(create_syscall_library());
    venv_loader::register_builtin(create_files_library());
    venv_loader::register_builtin(create_net_library());
});

// ============================================================================
//                    ГЛОБАЛЬНЫЙ МЕНЕДЖЕР ОКРУЖЕНИЙ
// ============================================================================

/// Глобальный менеджер окружений
pub static ENVIRONMENT_MANAGER: Lazy<RwLock<EnvironmentManager>> = Lazy::new(|| {
    let mut manager = EnvironmentManager::new();
    
    // Регистрируем встроенные библиотеки в глобальном окружении
    let global = manager.global_mut();
    global.register_builtin(create_time_library());
    global.register_builtin(create_syscall_library());
    global.register_builtin(create_files_library());
    global.register_builtin(create_net_library());
    // И одновременно регистрируем их в интегрированном загрузчике (compiled-in)
    Lazy::force(&BUILTINS_LOADER_INIT);
    
    RwLock::new(manager)
});

// ============================================================================
//                    ПРОВАЙДЕР БИБЛИОТЕК
// ============================================================================

/// Провайдер библиотек из глобального реестра
pub struct GlobalLibraryProvider;

impl LibraryProvider for GlobalLibraryProvider {
    fn available_versions(&self, name: &str) -> Vec<Version> {
        ENVIRONMENT_MANAGER.read()
            .map(|m| m.global().available_versions(name))
            .unwrap_or_default()
    }

    fn get_library(&self, name: &str, version: &Version) -> Option<LibraryDef> {
        ENVIRONMENT_MANAGER.read().ok()?
            .global()
            .find_version(name, version)
            .map(|v| v.def.clone())
    }

    fn get_dependencies(&self, name: &str, version: &Version) -> Vec<DependencySpec> {
        ENVIRONMENT_MANAGER.read().ok()
            .and_then(|m| m.global().find_version(name, version))
            .map(|v| {
                v.def.dependencies
                    .iter()
                    .map(|d| d.to_dependency_spec())
                    .collect()
            })
            .unwrap_or_default()
    }
}

// ============================================================================
//                    ПУБЛИЧНОЕ API
// ============================================================================

/// Активирует окружение для проекта
pub fn activate_project_environment(project_root: impl AsRef<Path>) -> Result<(), String> {
    let mut manager = ENVIRONMENT_MANAGER.write()
        .map_err(|_| "Не удалось получить доступ к менеджеру окружений")?;
    
    manager.activate_project(project_root.as_ref());
    
    // Загружаем зависимости из kumir.toml если есть
    let project_root_path = manager.active().paths.root.parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| manager.active().paths.root.clone());
    if let Some(config) = KumirConfig::find(&project_root_path) {
        let provider = GlobalLibraryProvider;
        let mut resolver = DependencyResolver::new(&provider);
        
        let deps: Vec<DependencySpec> = config.dependencies.values().cloned().collect();
        
        if let Ok(result) = resolver.resolve(&deps) {
            result.apply_to_environment(manager.active_mut(), &provider);
        }
    }
    
    Ok(())
}

/// Деактивирует текущее окружение проекта
pub fn deactivate_project_environment() {
    if let Ok(mut manager) = ENVIRONMENT_MANAGER.write() {
        manager.deactivate();
    }
}

/// Ищет библиотеку по имени (активная версия в текущем окружении)
pub fn find_library(name: &str) -> Option<LibraryDef> {
    ENVIRONMENT_MANAGER.read().ok()?
        .active()
        .find(name)
        .map(|v| v.def.clone())
}

/// Ищет библиотеку по имени и версии
pub fn find_library_version(name: &str, version: &Version) -> Option<LibraryDef> {
    ENVIRONMENT_MANAGER.read().ok()?
        .active()
        .find_version(name, version)
        .map(|v| v.def.clone())
}

/// Ищет библиотеку по спецификации версии
pub fn find_library_matching(name: &str, spec: &VersionSpec) -> Option<LibraryDef> {
    ENVIRONMENT_MANAGER.read().ok()?
        .active()
        .find_matching(name, spec)
        .map(|v| v.def.clone())
}

/// Проверяет, существует ли библиотека
pub fn is_known_library(name: &str) -> bool {
    ENVIRONMENT_MANAGER.read()
        .map(|m| m.active().exists(name))
        .unwrap_or(false)
}

/// Получает список всех библиотек в текущем окружении
pub fn all_libraries() -> Vec<LibraryDef> {
    ENVIRONMENT_MANAGER.read()
        .map(|m| {
            m.active()
                .all_libraries()
                .into_iter()
                .map(|v| v.def.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// Получает все доступные версии библиотеки
pub fn available_versions(name: &str) -> Vec<Version> {
    ENVIRONMENT_MANAGER.read()
        .map(|m| m.active().available_versions(name))
        .unwrap_or_default()
}

/// Регистрирует пользовательскую библиотеку в текущем окружении
pub fn register_library(lib: LibraryDef) -> Result<(), String> {
    let mut manager = ENVIRONMENT_MANAGER.write()
        .map_err(|_| "Не удалось получить доступ к реестру библиотек")?;
    
    manager.active_mut().register_builtin(lib);
    Ok(())
}

/// Регистрирует версионированную библиотеку
pub fn register_versioned_library(lib: VersionedLibrary) -> Result<(), String> {
    let mut manager = ENVIRONMENT_MANAGER.write()
        .map_err(|_| "Не удалось получить доступ к реестру библиотек")?;
    
    manager.active_mut().register(lib);
    Ok(())
}

/// Устанавливает активную версию библиотеки
pub fn set_active_version(name: &str, version: Version) -> Result<(), String> {
    let mut manager = ENVIRONMENT_MANAGER.write()
        .map_err(|_| "Не удалось получить доступ к реестру библиотек")?;
    
    manager.active_mut().set_active_version(name, version)
}

// ============================================================================
//                    ВСПОМОГАТЕЛЬНЫЕ ФУНКЦИИ
// ============================================================================

/// Создаёт версионированную библиотеку из определения
pub fn versioned_from_def(def: LibraryDef) -> VersionedLibrary {
    VersionedLibrary::from_builtin(def)
}

/// Создаёт локальную версионированную библиотеку
pub fn versioned_local(def: LibraryDef, path: impl AsRef<Path>) -> VersionedLibrary {
    let version = Version::new(
        def.version.major,
        def.version.minor,
        def.version.patch,
    );
    VersionedLibrary {
        def,
        version,
        source: LibrarySource::Local(path.as_ref().to_path_buf()),
        path: Some(path.as_ref().to_path_buf()),
        checksum: None,
    }
}

/// Информация об окружении
pub struct EnvironmentInfo {
    pub name: String,
    pub is_global: bool,
    pub library_count: usize,
    pub paths: EnvPaths,
}

/// Получает информацию о текущем окружении
pub fn environment_info() -> Option<EnvironmentInfo> {
    let manager = ENVIRONMENT_MANAGER.read().ok()?;
    let env = manager.active();
    
    Some(EnvironmentInfo {
        name: env.name.clone(),
        is_global: env.name == "global",
        library_count: env.all_libraries().len(),
        paths: env.paths.clone(),
    })
}
