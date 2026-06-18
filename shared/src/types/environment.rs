//! Kumir 3 Virtual Environment System
//!
//! [STABLE] Modern virtual environment management for library isolation,
//! version resolution, and project-based dependency handling.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │ EnvironmentManager                                              │
//! │   - Global environment (singleton)                              │
//! │   - Project activation/deactivation                             │
//! │   - Environment stack for nested contexts                       │
//! └─────────────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │ VirtualEnvironment                                              │
//! │   - Library registry (name → version → VersionedLibrary)        │
//! │   - Active versions tracking                                    │
//! │   - Resolved dependencies for lock file                         │
//! │   - Parent environment inheritance                              │
//! └─────────────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │ VersionedLibrary                                                │
//! │   - LibraryDef (functions, types, classes, constants)           │
//! │   - Version info                                                │
//! │   - Source tracking (builtin, local, cache, remote)             │
//! │   - Checksum for integrity                                      │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::library::LibraryDef;
use super::version::{Version, VersionSpec};

// ============================================================================
//                         ENVIRONMENT PATHS
// ============================================================================

/// Paths structure for virtual environment
#[derive(Debug, Clone)]
pub struct EnvPaths {
    /// Root directory (~/.kumir)
    pub root: PathBuf,
    /// Global library cache directory
    pub global_cache: PathBuf,
    /// Local project libraries
    pub local_libs: PathBuf,
    /// Project config file (kumir.toml)
    pub config_file: PathBuf,
    /// Lock file (kumir.lock)
    pub lock_file: PathBuf,
}

impl EnvPaths {
    /// Creates paths for global environment
    pub fn global() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let root = home.join(".kumir");

        Self {
            global_cache: root.join("registry"),
            local_libs: root.join("libs"),
            config_file: root.join("config.toml"),
            lock_file: root.join("registry.lock"),
            root,
        }
    }

    /// Creates paths for local project environment
    pub fn local(project_root: impl AsRef<Path>) -> Self {
        let project_root = project_root.as_ref();
        let root = project_root.to_path_buf();

        Self {
            global_cache: Self::global().global_cache,
            local_libs: root.join("libs"),
            config_file: project_root.join("kumir.toml"),
            lock_file: project_root.join("kumir.lock"),
            root,
        }
    }

    /// Checks if environment exists
    pub fn exists(&self) -> bool {
        self.root.exists()
    }

    /// Returns path to library cache by name and version
    pub fn library_cache_path(&self, name: &str, version: &Version) -> PathBuf {
        self.global_cache.join(format!("{}-{}", name, version))
    }

    /// Returns path to local library
    pub fn local_library_path(&self, name: &str) -> PathBuf {
        self.local_libs.join(name)
    }
}

impl Default for EnvPaths {
    fn default() -> Self {
        Self::global()
    }
}

// ============================================================================
//                         LIBRARY SOURCE
// ============================================================================

/// Library source tracking
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum LibrarySource {
    /// Built into interpreter
    #[default]
    Builtin,
    /// From global cache
    GlobalCache(PathBuf),
    /// From local project directory
    Local(PathBuf),
    /// From remote repository
    Remote(String),
    /// Relative path
    Path(PathBuf),
}

impl LibrarySource {
    pub fn is_local(&self) -> bool {
        matches!(self, LibrarySource::Local(_) | LibrarySource::Path(_))
    }

    pub fn is_cacheable(&self) -> bool {
        matches!(
            self,
            LibrarySource::GlobalCache(_) | LibrarySource::Remote(_)
        )
    }

    /// Converts to string representation for lock files
    pub fn to_lock_string(&self) -> String {
        match self {
            LibrarySource::Builtin => "builtin".to_string(),
            LibrarySource::GlobalCache(_) => "registry".to_string(),
            LibrarySource::Local(_) => "local".to_string(),
            LibrarySource::Remote(url) => url.clone(),
            LibrarySource::Path(p) => format!("path:{}", p.display()),
        }
    }

    /// Parses from lock file string
    pub fn from_lock_string(s: &str, paths: &EnvPaths) -> Self {
        match s {
            "builtin" => LibrarySource::Builtin,
            "local" => LibrarySource::Local(paths.local_libs.clone()),
            "registry" | "global" => LibrarySource::GlobalCache(paths.global_cache.clone()),
            s if s.starts_with("path:") => {
                LibrarySource::Path(PathBuf::from(s.trim_start_matches("path:")))
            }
            url => LibrarySource::Remote(url.to_string()),
        }
    }
}

// ============================================================================
//                    VERSIONED LIBRARY
// ============================================================================

/// Library with version and source information
#[derive(Debug, Clone)]
pub struct VersionedLibrary {
    /// Library definition
    pub def: LibraryDef,
    /// Version
    pub version: Version,
    /// Source
    pub source: LibrarySource,
    /// Full path (if any)
    pub path: Option<PathBuf>,
    /// Content checksum for integrity
    pub checksum: Option<String>,
}

impl VersionedLibrary {
    /// Creates from builtin library
    pub fn from_builtin(def: LibraryDef) -> Self {
        let version = def.version.to_version();
        Self {
            def,
            version,
            source: LibrarySource::Builtin,
            path: None,
            checksum: None,
        }
    }

    /// Creates from local path
    pub fn from_local(def: LibraryDef, path: PathBuf) -> Self {
        let version = def.version.to_version();
        Self {
            def,
            version,
            source: LibrarySource::Local(path.clone()),
            path: Some(path),
            checksum: None,
        }
    }

    /// Creates from cache
    pub fn from_cache(def: LibraryDef, cache_path: PathBuf) -> Self {
        let version = def.version.to_version();
        Self {
            def,
            version,
            source: LibrarySource::GlobalCache(cache_path.clone()),
            path: Some(cache_path),
            checksum: None,
        }
    }

    /// With checksum
    pub fn with_checksum(mut self, checksum: impl Into<String>) -> Self {
        self.checksum = Some(checksum.into());
        self
    }

    /// Unique cache key
    pub fn cache_key(&self) -> String {
        format!("{}@{}", self.def.id, self.version)
    }

    /// Full name with version
    pub fn full_name(&self) -> String {
        format!("{}:{}", self.def.name, self.version)
    }
}

// ============================================================================
//                         RESOLVED DEPENDENCY
// ============================================================================

/// Resolved dependency for lock file
#[derive(Debug, Clone)]
pub struct ResolvedDependency {
    /// Library name
    pub name: String,
    /// Resolved version
    pub version: Version,
    /// Source
    pub source: LibrarySource,
    /// Requested version spec
    pub requested: VersionSpec,
}

impl ResolvedDependency {
    pub fn new(
        name: impl Into<String>,
        version: Version,
        source: LibrarySource,
        requested: VersionSpec,
    ) -> Self {
        Self {
            name: name.into(),
            version,
            source,
            requested,
        }
    }
}

// ============================================================================
//                         VIRTUAL ENVIRONMENT
// ============================================================================

/// Virtual environment for project
#[derive(Debug)]
pub struct VirtualEnvironment {
    /// Environment name
    pub name: String,
    /// Environment paths
    pub paths: EnvPaths,
    /// Libraries (name → version_string → library)
    libraries: HashMap<String, HashMap<String, Arc<VersionedLibrary>>>,
    /// Resolved dependencies
    resolved: HashMap<String, ResolvedDependency>,
    /// Active versions (name → version)
    active_versions: HashMap<String, Version>,
    /// Parent environment
    parent: Option<Box<VirtualEnvironment>>,
}

impl VirtualEnvironment {
    /// Creates new environment
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

    /// Creates global environment
    pub fn global() -> Self {
        Self::new("global", EnvPaths::global())
    }

    /// Creates project environment
    pub fn for_project(project_root: impl AsRef<Path>) -> Self {
        let paths = EnvPaths::local(&project_root);
        let name = project_root
            .as_ref()
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("project")
            .to_string();

        Self::new(name, paths)
    }

    /// Creates child environment with inheritance
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
    //                         REGISTRATION
    // ========================================================================

    /// Registers library
    pub fn register(&mut self, lib: VersionedLibrary) {
        let name = lib.def.name.to_string();
        let version_str = lib.version.to_string();

        self.libraries
            .entry(name.clone())
            .or_default()
            .insert(version_str, Arc::new(lib.clone()));

        // First version becomes active
        if let std::collections::hash_map::Entry::Vacant(e) = self.active_versions.entry(name) {
            e.insert(lib.version);
        }
    }

    /// Registers builtin library
    pub fn register_builtin(&mut self, def: LibraryDef) {
        self.register(VersionedLibrary::from_builtin(def));
    }

    /// Sets active version
    pub fn set_active_version(&mut self, name: &str, version: Version) -> Result<(), String> {
        let version_str = version.to_string();

        if !self
            .libraries
            .get(name)
            .is_some_and(|versions| versions.contains_key(&version_str))
        {
            return Err(format!(
                "Version {} of library '{}' not found in environment",
                version, name
            ));
        }

        self.active_versions.insert(name.to_string(), version);
        Ok(())
    }

    // ========================================================================
    //                         LOOKUP
    // ========================================================================

    /// Normalizes library name (finds main name by aliases)
    fn normalize_name(&self, name: &str) -> Option<String> {
        if self.libraries.contains_key(name) {
            return Some(name.to_string());
        }

        for (main_name, versions) in &self.libraries {
            if let Some(lib) = versions.values().next()
                && lib.def.matches_name(name)
            {
                return Some(main_name.clone());
            }
        }

        self.parent.as_ref().and_then(|p| p.normalize_name(name))
    }

    /// Finds library by name (active version)
    pub fn find(&self, name: &str) -> Option<Arc<VersionedLibrary>> {
        let main_name = self.normalize_name(name)?;

        if let Some(version) = self.active_versions.get(&main_name)
            && let Some(lib) = self.find_version_internal(&main_name, version)
        {
            return Some(lib);
        }

        if let Some(versions) = self.libraries.get(&main_name)
            && let Some(lib) = versions.values().next()
        {
            return Some(Arc::clone(lib));
        }

        self.parent.as_ref().and_then(|p| p.find(name))
    }

    fn find_version_internal(
        &self,
        main_name: &str,
        version: &Version,
    ) -> Option<Arc<VersionedLibrary>> {
        let version_str = version.to_string();
        self.libraries
            .get(main_name)?
            .get(&version_str)
            .map(Arc::clone)
    }

    /// Finds specific version
    pub fn find_version(&self, name: &str, version: &Version) -> Option<Arc<VersionedLibrary>> {
        let main_name = self.normalize_name(name)?;

        if let Some(lib) = self.find_version_internal(&main_name, version) {
            return Some(lib);
        }

        self.parent
            .as_ref()
            .and_then(|p| p.find_version(name, version))
    }

    /// Finds with version spec
    pub fn find_matching(&self, name: &str, spec: &VersionSpec) -> Option<Arc<VersionedLibrary>> {
        let main_name = self.normalize_name(name);
        let mut matching: Vec<Arc<VersionedLibrary>> = Vec::new();

        if let Some(ref main_name) = main_name
            && let Some(versions) = self.libraries.get(main_name)
        {
            for lib in versions.values() {
                if spec.matches(&lib.version) {
                    matching.push(Arc::clone(lib));
                }
            }
        }

        if let Some(parent) = &self.parent
            && let Some(lib) = parent.find_matching(name, spec)
            && spec.matches(&lib.version)
        {
            matching.push(lib);
        }

        matching.into_iter().max_by_key(|lib| lib.version.clone())
    }

    /// Checks if library exists
    pub fn exists(&self, name: &str) -> bool {
        self.normalize_name(name).is_some()
    }

    /// Returns available versions
    pub fn available_versions(&self, name: &str) -> Vec<Version> {
        let mut versions: Vec<Version> = Vec::new();

        if let Some(main_name) = self.normalize_name(name)
            && let Some(libs) = self.libraries.get(&main_name)
        {
            for lib in libs.values() {
                versions.push(lib.version.clone());
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
    //                         INFO
    // ========================================================================

    /// Returns all libraries
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

    /// Returns resolved dependencies
    pub fn resolved_dependencies(&self) -> &HashMap<String, ResolvedDependency> {
        &self.resolved
    }

    /// Adds resolved dependency
    pub fn add_resolved(&mut self, dep: ResolvedDependency) {
        self.resolved.insert(dep.name.clone(), dep);
    }

    /// Clears environment
    pub fn clear(&mut self) {
        self.libraries.clear();
        self.resolved.clear();
        self.active_versions.clear();
    }

    /// Returns library count
    pub fn library_count(&self) -> usize {
        self.libraries.values().map(|v| v.len()).sum()
    }
}

// ============================================================================
//                         ENVIRONMENT MANAGER
// ============================================================================

/// Virtual environment manager
pub struct EnvironmentManager {
    /// Global environment
    global: VirtualEnvironment,
    /// Active environment
    active: Option<VirtualEnvironment>,
    /// Environment stack
    stack: Vec<VirtualEnvironment>,
}

impl EnvironmentManager {
    /// Creates manager with global environment
    pub fn new() -> Self {
        Self {
            global: VirtualEnvironment::global(),
            active: None,
            stack: Vec::new(),
        }
    }

    /// Returns global environment
    pub fn global(&self) -> &VirtualEnvironment {
        &self.global
    }

    /// Returns global environment (mut)
    pub fn global_mut(&mut self) -> &mut VirtualEnvironment {
        &mut self.global
    }

    /// Returns active environment (or global)
    pub fn active(&self) -> &VirtualEnvironment {
        self.active.as_ref().unwrap_or(&self.global)
    }

    /// Returns active environment (mut)
    pub fn active_mut(&mut self) -> &mut VirtualEnvironment {
        self.active.as_mut().unwrap_or(&mut self.global)
    }

    /// Activates project environment
    pub fn activate_project(&mut self, project_root: impl AsRef<Path>) {
        let mut env = VirtualEnvironment::for_project(project_root);

        // Inherit from global
        for lib in self.global.all_libraries() {
            env.register((*lib).clone());
        }

        self.active = Some(env);
    }

    /// Deactivates current environment
    pub fn deactivate(&mut self) {
        self.active = None;
    }

    /// Pushes new context
    pub fn push_context(&mut self, env: VirtualEnvironment) {
        if let Some(current) = self.active.take() {
            self.stack.push(current);
        }
        self.active = Some(env);
    }

    /// Pops context
    pub fn pop_context(&mut self) -> Option<VirtualEnvironment> {
        let popped = self.active.take();
        self.active = self.stack.pop();
        popped
    }

    /// Finds library in active environment
    pub fn find_library(&self, name: &str) -> Option<Arc<VersionedLibrary>> {
        self.active().find(name)
    }

    /// Finds library with version
    pub fn find_library_version(
        &self,
        name: &str,
        version: &Version,
    ) -> Option<Arc<VersionedLibrary>> {
        self.active().find_version(name, version)
    }

    /// Finds library with spec
    pub fn find_library_matching(
        &self,
        name: &str,
        spec: &VersionSpec,
    ) -> Option<Arc<VersionedLibrary>> {
        self.active().find_matching(name, spec)
    }

    /// Checks if project is active
    pub fn is_project_active(&self) -> bool {
        self.active.is_some()
    }

    /// Returns active project name
    pub fn active_project_name(&self) -> Option<&str> {
        self.active.as_ref().map(|e| e.name.as_str())
    }
}

impl Default for EnvironmentManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
//                         TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::library::LibVersion;

    fn test_library(name: &str, major: u32, minor: u32, patch: u32) -> LibraryDef {
        LibraryDef {
            id: name.into(),
            name: name.into(),
            aliases: Vec::new(),
            description: None,
            version: LibVersion::new(major, minor, patch),
            author: "test".into(),
            dependencies: Vec::new(),
            classes: Vec::new(),
            functions: Vec::new(),
            types: Vec::new(),
            constants: Vec::new(),
            kumir_version: None,
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

        // First registered version is active
        let active = env.find("mylib").unwrap();
        assert_eq!(active.version, Version::new(1, 0, 0));
    }

    #[test]
    fn test_version_matching() {
        let mut env = VirtualEnvironment::new("test", EnvPaths::global());

        env.register(VersionedLibrary::from_builtin(test_library("lib", 1, 0, 0)));
        env.register(VersionedLibrary::from_builtin(test_library("lib", 1, 5, 0)));
        env.register(VersionedLibrary::from_builtin(test_library("lib", 2, 0, 0)));

        // ^1.0 should find 1.5.0 (newest compatible)
        let spec: VersionSpec = "^1.0.0".parse().unwrap();
        let lib = env.find_matching("lib", &spec).unwrap();
        assert_eq!(lib.version, Version::new(1, 5, 0));
    }

    #[test]
    fn test_environment_manager() {
        let mut manager = EnvironmentManager::new();

        assert!(!manager.is_project_active());

        let def = test_library("builtin_lib", 1, 0, 0);
        manager.global_mut().register_builtin(def);

        // Global library should be findable
        assert!(manager.find_library("builtin_lib").is_some());
    }

    #[test]
    fn test_library_source() {
        let paths = EnvPaths::global();

        let builtin = LibrarySource::Builtin;
        assert_eq!(builtin.to_lock_string(), "builtin");

        let parsed = LibrarySource::from_lock_string("builtin", &paths);
        assert_eq!(parsed, LibrarySource::Builtin);

        let path_src = LibrarySource::Path(PathBuf::from("/some/path"));
        assert!(path_src.to_lock_string().starts_with("path:"));
    }
}
