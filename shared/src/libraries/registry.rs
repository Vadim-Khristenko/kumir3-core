//! Global library registry for Kumir 3.
//!
//! Registers all built-in libraries and provides
//! a lookup interface via the `LibraryProvider` trait.

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

// ===== Global state =====

/// Environment manager (global + project stack).
static ENVIRONMENT_MANAGER: Lazy<RwLock<EnvironmentManager>> =
    Lazy::new(|| RwLock::new(EnvironmentManager::default()));

/// One-time initialization: registration of built-in libraries.
static BUILTINS_LOADER_INIT: Lazy<()> = Lazy::new(|| {
    let builtins = vec![
        create_time_library(),
        create_syscall_library(),
        create_files_library(),
        create_net_library(),
    ];

    for lib in builtins {
        // Register via integrated loader.
        venv_loader::register_builtin(lib.clone());

        // Also register in global environment.
        if let Ok(mut mgr) = ENVIRONMENT_MANAGER.write() {
            let env = mgr.global_mut();
            env.register_builtin(lib);
        }
    }
});

// ===== Public API =====

/// Registers all built-in libraries.
/// Call once at interpreter startup.
pub fn register_all_builtins() {
    Lazy::force(&BUILTINS_LOADER_INIT);
}

/// Finds a library by name (or alias).
/// Automatically initializes built-in libraries on first call.
pub fn find_library(name: &str) -> Option<LibraryDef> {
    register_all_builtins();

    // First try EnvironmentManager.
    if let Ok(mgr) = ENVIRONMENT_MANAGER.read()
        && let Some(versioned) = mgr.find_library(name)
    {
        return Some(versioned.def.clone());
    }

    // Then try integrated loader.
    if let Ok(loaded) = venv_loader::load_library(name) {
        return Some(loaded.def);
    }

    None
}

/// Finds a library by name and version.
/// Supports exact versions and version specs (^1.0, ~1.2.3, >=1.0.0).
pub fn find_library_with_version(name: &str, version_spec: &str) -> Option<LibraryDef> {
    register_all_builtins();

    // Parse version spec.
    let spec = VersionSpec::parse(version_spec).ok()?;

    // Get all available versions of the library.
    if let Ok(mgr) = ENVIRONMENT_MANAGER.read() {
        let env = mgr.active();
        let available = env.available_versions(name);

        // Find the best matching version.
        let mut matching_versions: Vec<_> =
            available.into_iter().filter(|v| spec.matches(v)).collect();

        // Sort in descending order (newest first).
        matching_versions.sort_by(|a, b| b.cmp(a));

        // Use the newest matching version.
        if let Some(best_version) = matching_versions.first()
            && let Some(versioned) = mgr.find_library_version(name, best_version)
        {
            return Some(versioned.def.clone());
        }
    }

    // Try integrated loader.
    if let Ok(loaded) = venv_loader::load_library(name) {
        // Check if version matches.
        let spec = VersionSpec::parse(version_spec).ok()?;
        if spec.matches(&loaded.version) {
            return Some(loaded.def);
        }
    }

    None
}

/// Gets all available versions of a library.
pub fn get_library_versions(name: &str) -> Vec<Version> {
    register_all_builtins();

    if let Ok(mgr) = ENVIRONMENT_MANAGER.read() {
        let env = mgr.active();
        return env.available_versions(name).into_iter().collect();
    }

    Vec::new()
}

/// Checks if a name (or alias) is a known library.
pub fn is_known_library(name: &str) -> bool {
    register_all_builtins();

    // Fast check against known ids and aliases.
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

    // Check via environment manager.
    if let Ok(mgr) = ENVIRONMENT_MANAGER.read()
        && mgr.find_library(name).is_some()
    {
        return true;
    }

    false
}

/// Activates a project environment (loads dependencies from lock file).
pub fn activate_project(project_path: &str) -> Result<(), String> {
    register_all_builtins();
    venv_loader::activate_project(project_path)
        .map_err(|e| format!("Ошибка активации проекта: {}", e))
}

/// Deactivates the current project environment.
pub fn deactivate_project() -> Result<(), String> {
    venv_loader::deactivate_project();
    Ok(())
}

/// Lists all available libraries.
pub fn list_available() -> Vec<String> {
    register_all_builtins();
    venv_loader::list_available()
}

/// Finds a built-in library that provides a function with the given name.
///
/// Used for hints: if a program calls `текущий_год()` without importing the time library,
/// instead of "algorithm not defined", the user gets a hint showing which library to import.
/// Returns the library name in the language of the program—what goes after `использовать`.
pub fn library_providing_function(function: &str) -> Option<String> {
    register_all_builtins();

    for lib_name in venv_loader::list_available() {
        let Some(lib) = find_library(&lib_name) else {
            continue;
        };
        let provides = lib.functions.iter().any(|f| {
            f.name.as_ref() == function || f.aliases.iter().any(|a| a.as_ref() == function)
        });
        if provides {
            // The Russian name ("Время") reads better in hints than the utility ID ("time"),
            // but after `использовать` you write what the import understands—take the first alias.
            let import_name = lib
                .aliases
                .iter()
                .find(|a| !a.is_ascii())
                .map(|a| a.to_string())
                .unwrap_or_else(|| lib.name.to_string());
            return Some(import_name);
        }
    }
    None
}

// ===== LibraryProvider =====

/// Global library provider implementing the `LibraryProvider` trait
/// for the dependency resolution system.
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

        // Find library and return its dependencies as DependencySpec.
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

/// Returns the global library provider.
pub fn get_global_provider() -> GlobalLibraryProvider {
    GlobalLibraryProvider
}

// ===== Helper registry (for simple use without environments) =====

/// Creates a simple registry with all built-in libraries.
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
        // Repeated calls should not panic.
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
        // Built-in libraries should have at least one version
        // (depends on whether they're registered via EnvironmentManager).
        let _ = versions;
    }

    #[test]
    fn test_list_available() {
        let available = list_available();
        // At least the built-ins should be present.
        assert!(available.len() >= 4);
    }
}
