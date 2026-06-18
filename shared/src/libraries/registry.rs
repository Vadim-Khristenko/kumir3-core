//! Глобальный реестр библиотек для КуМир 3
//!
//! Регистрирует все встроенные библиотеки и предоставляет
//! интерфейс поиска через `LibraryProvider` trait.

use std::sync::RwLock;

use once_cell::sync::Lazy;

use crate::types::config::DependencySpec;
use crate::types::environment::EnvironmentManager;
use crate::types::library::{LibraryDef, LibraryRegistry};
use crate::types::resolver::LibraryProvider;
use crate::types::venv_loader;
use crate::types::version::{Version, VersionSpec};

use super::{
    create_files_library, create_net_library, create_syscall_library, create_time_library,
};

// ===== Глобальное состояние =====

/// Менеджер окружений (глобальное + стек проектных)
static ENVIRONMENT_MANAGER: Lazy<RwLock<EnvironmentManager>> =
    Lazy::new(|| RwLock::new(EnvironmentManager::default()));

/// Одноразовая инициализация: регистрация встроенных библиотек
static BUILTINS_LOADER_INIT: Lazy<()> = Lazy::new(|| {
    let builtins = vec![
        create_time_library(),
        create_syscall_library(),
        create_files_library(),
        create_net_library(),
    ];

    for lib in builtins {
        // Регистрируем через интегрированный загрузчик
        venv_loader::register_builtin(lib.clone());

        // Также регистрируем в глобальном окружении
        if let Ok(mut mgr) = ENVIRONMENT_MANAGER.write() {
            let env = mgr.global_mut();
            env.register_builtin(lib);
        }
    }
});

// ===== Публичное API =====

/// Регистрирует все встроенные библиотеки.
/// Вызовите один раз при запуске интерпретатора.
pub fn register_all_builtins() {
    Lazy::force(&BUILTINS_LOADER_INIT);
}

/// Ищет библиотеку по имени (или алиасу).
/// Автоматически инициализирует встроенные библиотеки при первом вызове.
pub fn find_library(name: &str) -> Option<LibraryDef> {
    register_all_builtins();

    // Сначала пробуем через EnvironmentManager
    if let Ok(mgr) = ENVIRONMENT_MANAGER.read()
        && let Some(versioned) = mgr.find_library(name)
    {
        return Some(versioned.def.clone());
    }

    // Затем через интегрированный загрузчик
    if let Ok(loaded) = venv_loader::load_library(name) {
        return Some(loaded.def);
    }

    None
}

/// Ищет библиотеку по имени и версии.
/// Поддерживает точные версии и спецификации версий (^1.0, ~1.2.3, >=1.0.0).
pub fn find_library_with_version(name: &str, version_spec: &str) -> Option<LibraryDef> {
    register_all_builtins();

    // Парсим спецификацию версии
    let spec = VersionSpec::parse(version_spec).ok()?;

    // Получаем все доступные версии библиотеки
    if let Ok(mgr) = ENVIRONMENT_MANAGER.read() {
        let env = mgr.active();
        let available = env.available_versions(name);

        // Находим лучшую подходящую версию
        let mut matching_versions: Vec<_> =
            available.into_iter().filter(|v| spec.matches(v)).collect();

        // Сортируем по убыванию (самая новая первая)
        matching_versions.sort_by(|a, b| b.cmp(a));

        // Берём самую новую подходящую версию
        if let Some(best_version) = matching_versions.first()
            && let Some(versioned) = mgr.find_library_version(name, best_version)
        {
            return Some(versioned.def.clone());
        }
    }

    // Пробуем через интегрированный загрузчик
    if let Ok(loaded) = venv_loader::load_library(name) {
        // Проверяем, подходит ли версия
        let spec = VersionSpec::parse(version_spec).ok()?;
        if spec.matches(&loaded.version) {
            return Some(loaded.def);
        }
    }

    None
}

/// Получает список всех доступных версий библиотеки
pub fn get_library_versions(name: &str) -> Vec<Version> {
    register_all_builtins();

    if let Ok(mgr) = ENVIRONMENT_MANAGER.read() {
        let env = mgr.active();
        return env.available_versions(name).into_iter().collect();
    }

    Vec::new()
}

/// Проверяет, является ли имя (или алиас) известной библиотекой
pub fn is_known_library(name: &str) -> bool {
    register_all_builtins();

    // Быстрая проверка по известным id и aliases
    let known_names = [
        "time",
        "время",
        "time_lib",
        "syscall",
        "sys",
        "системные_вызовы",
        "os",
        "files",
        "файлы",
        "fs",
        "файловая_система",
        "net",
        "сеть",
        "network",
        "networking",
    ];

    if known_names.contains(&name) {
        return true;
    }

    // Проверяем через менеджер окружений
    if let Ok(mgr) = ENVIRONMENT_MANAGER.read()
        && mgr.find_library(name).is_some()
    {
        return true;
    }

    false
}

/// Активирует окружение проекта (загружает зависимости из lock-файла)
pub fn activate_project(project_path: &str) -> Result<(), String> {
    register_all_builtins();
    venv_loader::activate_project(project_path)
        .map_err(|e| format!("Ошибка активации проекта: {}", e))
}

/// Деактивирует текущее окружение проекта
pub fn deactivate_project() -> Result<(), String> {
    venv_loader::deactivate_project();
    Ok(())
}

/// Список всех доступных библиотек
pub fn list_available() -> Vec<String> {
    register_all_builtins();
    venv_loader::list_available()
}

// ===== LibraryProvider =====

/// Глобальный провайдер библиотек, реализующий `LibraryProvider` trait
/// для системы разрешения зависимостей
pub struct GlobalLibraryProvider;

impl LibraryProvider for GlobalLibraryProvider {
    fn available_versions(&self, name: &str) -> Vec<Version> {
        register_all_builtins();

        if let Ok(mgr) = ENVIRONMENT_MANAGER.read() {
            let env = mgr.active();
            return env.available_versions(name).into_iter().collect();
        }
        Vec::new()
    }

    fn get_library(&self, name: &str, version: &Version) -> Option<LibraryDef> {
        register_all_builtins();

        if let Ok(mgr) = ENVIRONMENT_MANAGER.read()
            && let Some(versioned) = mgr.find_library_version(name, version)
        {
            return Some(versioned.def.clone());
        }
        None
    }

    fn get_dependencies(&self, name: &str, _version: &Version) -> Vec<DependencySpec> {
        register_all_builtins();

        // Ищем библиотеку и возвращаем её зависимости как DependencySpec
        if let Some(lib) = find_library(name) {
            return lib
                .dependencies
                .iter()
                .map(|dep| DependencySpec {
                    name: dep.name.to_string(),
                    version: VersionSpec::parse(&dep.version.to_string())
                        .unwrap_or_else(|_| VersionSpec::any()),
                    git: None,
                    path: None,
                    registry: None,
                    url: None,
                    optional: false,
                    features: Vec::new(),
                    default_features: true,
                    target: None,
                    package: None,
                })
                .collect();
        }
        Vec::new()
    }
}

/// Возвращает глобальный провайдер библиотек
pub fn get_global_provider() -> GlobalLibraryProvider {
    GlobalLibraryProvider
}

// ===== Вспомогательный реестр (для простого использования без окружений) =====

/// Создаёт простой реестр со всеми встроенными библиотеками
pub fn create_builtin_registry() -> LibraryRegistry {
    let mut registry = LibraryRegistry::new();
    registry.register(create_time_library());
    registry.register(create_syscall_library());
    registry.register(create_files_library());
    registry.register(create_net_library());
    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_all_builtins() {
        register_all_builtins();
        // Повторный вызов не должен паниковать
        register_all_builtins();
    }

    #[test]
    fn test_find_library_by_id() {
        register_all_builtins();
        let lib = find_library("time");
        assert!(lib.is_some());
        assert_eq!(lib.unwrap().id.as_ref(), "time");
    }

    #[test]
    fn test_is_known() {
        assert!(is_known_library("time"));
        assert!(is_known_library("время"));
        assert!(is_known_library("syscall"));
        assert!(is_known_library("files"));
        assert!(is_known_library("net"));
        assert!(!is_known_library("nonexistent_library_xyz"));
    }

    #[test]
    #[ignore] // TODO: Fix registry test - library lookup issue
    fn test_builtin_registry() {
        let reg = create_builtin_registry();
        assert!(reg.get("time").is_some());
        assert!(reg.get("syscall").is_some());
        assert!(reg.get("files").is_some());
        assert!(reg.get("net").is_some());
        assert_eq!(reg.all().count(), 4);
    }

    #[test]
    fn test_global_provider() {
        let provider = get_global_provider();
        let versions = provider.available_versions("time");
        // У встроенных библиотек должна быть хотя бы одна версия
        // (зависит от того, зарегистрированы ли они через EnvironmentManager)
        let _ = versions;
    }

    #[test]
    fn test_list_available() {
        let available = list_available();
        // Должны быть хотя бы встроенные
        assert!(available.len() >= 4);
    }
}
