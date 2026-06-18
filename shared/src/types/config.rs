//! Kumir 3 project configuration (kumir.toml).
//!
//! [STABLE] Modern, extensible configuration model for projects, workspaces,
//! dependencies, registries, and build profiles. Replaces legacy RTC-style
//! config with richer metadata, safer parsing, and lockfile support.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use toml::Value;

use super::version::{Version, VersionParseError, VersionSpec};

// =============================================================================
//         SECTION: PROJECT METADATA
// =============================================================================

#[derive(Debug, Clone, Default)]
pub struct ProjectMetadata {
    pub name: String,
    pub version: Version,
    pub authors: Vec<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub keywords: Vec<String>,
    pub readme: Option<String>,
    pub kumir_version: Option<Version>,
    pub edition: Option<String>,
}

// =============================================================================
//         SECTION: DEPENDENCIES
// =============================================================================

#[derive(Debug, Clone)]
pub struct GitSource {
    pub url: String,
    pub branch: Option<String>,
    pub tag: Option<String>,
    pub rev: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DependencySpec {
    pub name: String,
    pub version: VersionSpec,
    pub path: Option<PathBuf>,
    pub git: Option<GitSource>,
    pub registry: Option<String>,
    pub url: Option<String>,
    pub optional: bool,
    pub features: Vec<String>,
    pub default_features: bool,
    pub target: Option<String>,
    pub package: Option<String>,
}

impl DependencySpec {
    pub fn version(name: impl Into<String>, version: VersionSpec) -> Self {
        Self {
            name: name.into(),
            version,
            path: None,
            git: None,
            registry: None,
            url: None,
            optional: false,
            features: Vec::new(),
            default_features: true,
            target: None,
            package: None,
        }
    }

    pub fn path(name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        let mut dep = Self::version(name, VersionSpec::any());
        dep.path = Some(path.into());
        dep
    }

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
            registry: None,
            url: None,
            optional: false,
            features: Vec::new(),
            default_features: true,
            target: None,
            package: None,
        }
    }

    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }
    pub fn with_feature(mut self, feature: impl Into<String>) -> Self {
        self.features.push(feature.into());
        self
    }
    pub fn is_local(&self) -> bool {
        self.path.is_some()
    }
    pub fn is_git(&self) -> bool {
        self.git.is_some()
    }
}

// =============================================================================
//         SECTION: BUILD SETTINGS AND PROFILES
// =============================================================================

#[derive(Debug, Clone)]
pub struct BuildSettings {
    pub main_file: PathBuf,
    pub output_dir: PathBuf,
    pub optimization_level: u8,
    pub debug_info: bool,
    pub bounds_check: bool,
    pub strict_mode: bool,
    pub incremental: bool,
    pub lto: bool,
    pub target_triple: Option<String>,
    pub emit_ir: bool,
}

impl Default for BuildSettings {
    fn default() -> Self {
        Self {
            main_file: PathBuf::from("main.kum"),
            output_dir: PathBuf::from("./build"),
            optimization_level: 0,
            debug_info: true,
            bounds_check: true,
            strict_mode: false,
            incremental: true,
            lto: false,
            target_triple: None,
            emit_ir: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BuildProfile {
    pub name: String,
    pub optimization_level: Option<u8>,
    pub debug_info: Option<bool>,
    pub bounds_check: Option<bool>,
    pub incremental: Option<bool>,
    pub lto: Option<bool>,
    pub defines: HashMap<String, String>,
}

impl BuildProfile {
    pub fn dev() -> Self {
        Self {
            name: "dev".into(),
            optimization_level: Some(0),
            debug_info: Some(true),
            bounds_check: Some(true),
            incremental: Some(true),
            lto: Some(false),
            defines: HashMap::new(),
        }
    }

    pub fn release() -> Self {
        Self {
            name: "release".into(),
            optimization_level: Some(3),
            debug_info: Some(false),
            bounds_check: Some(false),
            incremental: Some(false),
            lto: Some(true),
            defines: HashMap::new(),
        }
    }

    pub fn test() -> Self {
        Self {
            name: "test".into(),
            optimization_level: Some(0),
            debug_info: Some(true),
            bounds_check: Some(true),
            incremental: Some(true),
            lto: Some(false),
            defines: HashMap::new(),
        }
    }
}

// =============================================================================
//         SECTION: WORKSPACE & REGISTRIES
// =============================================================================

#[derive(Debug, Clone, Default)]
pub struct Workspace {
    pub members: Vec<PathBuf>,
    pub exclude: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct Registry {
    pub name: String,
    pub index: String,
    pub priority: u32,
}

// =============================================================================
//         SECTION: FULL CONFIG
// =============================================================================

#[derive(Debug, Clone)]
pub struct KumirConfig {
    pub config_path: PathBuf,
    pub project_root: PathBuf,
    pub metadata: ProjectMetadata,
    pub dependencies: HashMap<String, DependencySpec>,
    pub dev_dependencies: HashMap<String, DependencySpec>,
    pub build_dependencies: HashMap<String, DependencySpec>,
    pub build: BuildSettings,
    pub profiles: HashMap<String, BuildProfile>,
    pub workspace: Workspace,
    pub registries: HashMap<String, Registry>,
    pub env: HashMap<String, String>,
}

impl KumirConfig {
    pub fn new(project_root: impl AsRef<Path>) -> Self {
        let root = project_root.as_ref().to_path_buf();
        Self {
            config_path: root.join("kumir.toml"),
            project_root: root,
            metadata: ProjectMetadata::default(),
            dependencies: HashMap::new(),
            dev_dependencies: HashMap::new(),
            build_dependencies: HashMap::new(),
            build: BuildSettings::default(),
            profiles: HashMap::new(),
            workspace: Workspace::default(),
            registries: HashMap::new(),
            env: HashMap::new(),
        }
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let content = fs::read_to_string(path).map_err(|e| ConfigError::IoError(e.to_string()))?;
        Self::parse(&content, path)
    }

    pub fn find(start_dir: impl AsRef<Path>) -> Option<Self> {
        let mut current = start_dir.as_ref().to_path_buf();
        loop {
            let candidate = current.join("kumir.toml");
            if candidate.exists() {
                return Self::load(&candidate).ok();
            }
            if !current.pop() {
                break;
            }
        }
        None
    }

    pub fn parse(content: &str, config_path: &Path) -> Result<Self, ConfigError> {
        let value: Value = content
            .parse::<Value>()
            .map_err(|e| ConfigError::ParseError(format!("TOML parse error: {e}")))?;

        let project_root = config_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        let mut cfg = Self::new(&project_root);
        cfg.config_path = config_path.to_path_buf();

        let table = value
            .as_table()
            .ok_or_else(|| ConfigError::InvalidFormat("Root must be a TOML table".into()))?;

        if let Some(project) = table
            .get("project")
            .or_else(|| table.get("проект"))
            .or_else(|| table.get("package"))
            && let Some(t) = project.as_table()
        {
            cfg.metadata.name = t
                .get("name")
                .or_else(|| t.get("имя"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            // Главный файл может быть указан в [package]/[project].
            if let Some(v) = t
                .get("main")
                .or_else(|| t.get("главный"))
                .and_then(|v| v.as_str())
            {
                cfg.build.main_file = PathBuf::from(v);
            }
            cfg.metadata.version = t
                .get("version")
                .or_else(|| t.get("версия"))
                .and_then(|v| v.as_str())
                .map(Version::parse)
                .transpose()
                .map_err(ConfigError::Version)?
                .unwrap_or_else(|| Version::new(0, 0, 0));
            cfg.metadata.description = t
                .get("description")
                .or_else(|| t.get("описание"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            cfg.metadata.license = t
                .get("license")
                .or_else(|| t.get("лицензия"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            cfg.metadata.homepage = t
                .get("homepage")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            cfg.metadata.repository = t
                .get("repository")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            cfg.metadata.readme = t
                .get("readme")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            cfg.metadata.kumir_version = t
                .get("kumir_version")
                .and_then(|v| v.as_str())
                .and_then(|s| Version::parse(s).ok());
            cfg.metadata.edition = t
                .get("edition")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            if let Some(authors) = t
                .get("authors")
                .or_else(|| t.get("авторы"))
                .and_then(|v| v.as_array())
            {
                cfg.metadata.authors = authors
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
            }
            if let Some(keys) = t.get("keywords").or_else(|| t.get("ключевые"))
                && let Some(arr) = keys.as_array()
            {
                cfg.metadata.keywords = arr
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
            }
        }

        if let Some(deps) = table
            .get("dependencies")
            .or_else(|| table.get("зависимости"))
        {
            cfg.dependencies = parse_deps_table(deps, &project_root)?;
        }
        if let Some(dev) = table
            .get("dev-dependencies")
            .or_else(|| table.get("dev_dependencies"))
        {
            cfg.dev_dependencies = parse_deps_table(dev, &project_root)?;
        }
        if let Some(build_deps) = table
            .get("build-dependencies")
            .or_else(|| table.get("build_dependencies"))
        {
            cfg.build_dependencies = parse_deps_table(build_deps, &project_root)?;
        }

        if let Some(build) = table.get("build").or_else(|| table.get("сборка"))
            && let Some(t) = build.as_table()
        {
            if let Some(v) = t
                .get("main")
                .or_else(|| t.get("главный"))
                .and_then(|v| v.as_str())
            {
                cfg.build.main_file = PathBuf::from(v);
            }
            if let Some(v) = t
                .get("output")
                .or_else(|| t.get("выход"))
                .and_then(|v| v.as_str())
            {
                cfg.build.output_dir = PathBuf::from(v);
            }
            cfg.build.optimization_level =
                t.get("optimization")
                    .or_else(|| t.get("оптимизация"))
                    .and_then(|v| v.as_integer())
                    .unwrap_or(cfg.build.optimization_level as i64) as u8;
            cfg.build.debug_info = t
                .get("debug_info")
                .and_then(|v| v.as_bool())
                .unwrap_or(cfg.build.debug_info);
            cfg.build.bounds_check = t
                .get("bounds_check")
                .and_then(|v| v.as_bool())
                .unwrap_or(cfg.build.bounds_check);
            cfg.build.strict_mode = t
                .get("strict_mode")
                .and_then(|v| v.as_bool())
                .unwrap_or(cfg.build.strict_mode);
            cfg.build.incremental = t
                .get("incremental")
                .and_then(|v| v.as_bool())
                .unwrap_or(cfg.build.incremental);
            cfg.build.lto = t
                .get("lto")
                .and_then(|v| v.as_bool())
                .unwrap_or(cfg.build.lto);
            cfg.build.target_triple = t
                .get("target")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            cfg.build.emit_ir = t
                .get("emit_ir")
                .and_then(|v| v.as_bool())
                .unwrap_or(cfg.build.emit_ir);
        }

        if let Some(ws) = table
            .get("workspace")
            .or_else(|| table.get("рабочее_пространство"))
            && let Some(t) = ws.as_table()
        {
            if let Some(members) = t.get("members").and_then(|v| v.as_array()) {
                cfg.workspace.members = members
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| project_root.join(s)))
                    .collect();
            }
            if let Some(exclude) = t.get("exclude").and_then(|v| v.as_array()) {
                cfg.workspace.exclude = exclude
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| project_root.join(s)))
                    .collect();
            }
        }

        if let Some(regs) = table.get("registries")
            && let Some(t) = regs.as_table()
        {
            for (name, v) in t {
                if let Some(rt) = v.as_table() {
                    let index = rt
                        .get("index")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let priority =
                        rt.get("priority").and_then(|v| v.as_integer()).unwrap_or(0) as u32;
                    cfg.registries.insert(
                        name.clone(),
                        Registry {
                            name: name.clone(),
                            index,
                            priority,
                        },
                    );
                }
            }
        }

        if let Some(env) = table.get("env")
            && let Some(t) = env.as_table()
        {
            for (k, v) in t {
                if let Some(val) = v.as_str() {
                    cfg.env.insert(k.clone(), val.to_string());
                }
            }
        }

        Ok(cfg)
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        let content = self.to_toml();
        fs::write(&self.config_path, content).map_err(|e| ConfigError::IoError(e.to_string()))
    }

    pub fn to_toml(&self) -> String {
        let mut out = String::new();
        out.push_str("[project]\n");
        out.push_str(&format!("name = \"{}\"\n", self.metadata.name));
        out.push_str(&format!("version = \"{}\"\n", self.metadata.version));
        if let Some(desc) = &self.metadata.description {
            out.push_str(&format!("description = \"{}\"\n", desc));
        }
        if !self.metadata.authors.is_empty() {
            out.push_str("authors = [");
            for (i, a) in self.metadata.authors.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&format!("\"{}\"", a));
            }
            out.push_str("]\n");
        }
        if let Some(home) = &self.metadata.homepage {
            out.push_str(&format!("homepage = \"{}\"\n", home));
        }
        if let Some(repo) = &self.metadata.repository {
            out.push_str(&format!("repository = \"{}\"\n", repo));
        }
        out.push_str("\n[build]\n");
        out.push_str(&format!("main = \"{}\"\n", self.build.main_file.display()));
        out.push_str(&format!(
            "output = \"{}\"\n",
            self.build.output_dir.display()
        ));
        out.push_str(&format!(
            "optimization = {}\n",
            self.build.optimization_level
        ));
        out.push_str(&format!("debug_info = {}\n", self.build.debug_info));
        out.push_str(&format!("bounds_check = {}\n", self.build.bounds_check));
        out.push_str(&format!("strict_mode = {}\n", self.build.strict_mode));
        out
    }

    pub fn main_file_path(&self) -> PathBuf {
        if self.build.main_file.is_absolute() {
            self.build.main_file.clone()
        } else {
            self.project_root.join(&self.build.main_file)
        }
    }

    pub fn output_dir_path(&self) -> PathBuf {
        if self.build.output_dir.is_absolute() {
            self.build.output_dir.clone()
        } else {
            self.project_root.join(&self.build.output_dir)
        }
    }
}

// =============================================================================
//         SECTION: LOCK FILES
// =============================================================================

#[derive(Debug, Clone)]
pub struct LockEntry {
    pub name: String,
    pub version: Version,
    pub checksum: Option<String>,
    pub source: String,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct LockFile {
    pub path: PathBuf,
    pub format_version: u32,
    pub entries: HashMap<String, LockEntry>,
}

impl LockFile {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            format_version: 1,
            entries: HashMap::new(),
        }
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(ConfigError::NotFound(path.to_path_buf()));
        }
        let content = fs::read_to_string(path).map_err(|e| ConfigError::IoError(e.to_string()))?;
        let value: Value = content
            .parse::<Value>()
            .map_err(|e| ConfigError::ParseError(format!("TOML parse error: {e}")))?;
        let table = value
            .as_table()
            .ok_or_else(|| ConfigError::InvalidFormat("Lock root must be a table".into()))?;
        let mut lock = Self::new(path);
        lock.format_version = table
            .get("format_version")
            .and_then(|v| v.as_integer())
            .unwrap_or(1) as u32;
        if let Some(pkgs) = table
            .get("package")
            .and_then(|v| v.as_array())
            .or_else(|| table.get("packages").and_then(|v| v.as_array()))
        {
            for pkg in pkgs {
                if let Some(t) = pkg.as_table() {
                    let name = t
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let version = t
                        .get("version")
                        .and_then(|v| v.as_str())
                        .map(Version::parse)
                        .transpose()
                        .map_err(ConfigError::Version)?
                        .unwrap_or_else(|| Version::new(0, 0, 0));
                    let source = t
                        .get("source")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let checksum = t
                        .get("checksum")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let deps = t
                        .get("dependencies")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();
                    lock.entries.insert(
                        name.clone(),
                        LockEntry {
                            name,
                            version,
                            checksum,
                            source,
                            dependencies: deps,
                        },
                    );
                }
            }
        }
        Ok(lock)
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        let mut out = String::new();
        out.push_str(&format!("format_version = {}\n\n", self.format_version));
        for entry in self.entries.values() {
            out.push_str("[[package]]\n");
            out.push_str(&format!("name = \"{}\"\n", entry.name));
            out.push_str(&format!("version = \"{}\"\n", entry.version));
            out.push_str(&format!("source = \"{}\"\n", entry.source));
            if let Some(cs) = &entry.checksum {
                out.push_str(&format!("checksum = \"{}\"\n", cs));
            }
            if !entry.dependencies.is_empty() {
                out.push_str("dependencies = [");
                for (i, d) in entry.dependencies.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    out.push_str(&format!("\"{}\"", d));
                }
                out.push_str("]\n");
            }
            out.push('\n');
        }
        fs::write(&self.path, out).map_err(|e| ConfigError::IoError(e.to_string()))
    }

    pub fn update(&mut self, entry: LockEntry) {
        self.entries.insert(entry.name.clone(), entry);
    }

    /// Returns the locked version for a package name
    pub fn locked_version(&self, name: &str) -> Option<&Version> {
        self.entries.get(name).map(|e| &e.version)
    }

    /// Checks if a package is locked
    pub fn is_locked(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }

    /// Gets the lock entry for a package
    pub fn get(&self, name: &str) -> Option<&LockEntry> {
        self.entries.get(name)
    }
}

// =============================================================================
//         SECTION: ERRORS
// =============================================================================

#[derive(Debug, Clone)]
pub enum ConfigError {
    IoError(String),
    ParseError(String),
    Version(VersionParseError),
    InvalidFormat(String),
    NotFound(PathBuf),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::IoError(e) => write!(f, "IO error: {e}"),
            ConfigError::ParseError(e) => write!(f, "Parse error: {e}"),
            ConfigError::Version(e) => write!(f, "Version parse error: {e}"),
            ConfigError::InvalidFormat(e) => write!(f, "Invalid format: {e}"),
            ConfigError::NotFound(p) => write!(f, "Config not found: {}", p.display()),
        }
    }
}

impl std::error::Error for ConfigError {}

// =============================================================================
//         SECTION: HELPERS
// =============================================================================

fn parse_deps_table(
    value: &Value,
    root: &Path,
) -> Result<HashMap<String, DependencySpec>, ConfigError> {
    let mut map = HashMap::new();
    let tbl = value
        .as_table()
        .ok_or_else(|| ConfigError::InvalidFormat("dependencies must be a table".into()))?;
    for (name, val) in tbl {
        let spec = if let Some(ver) = val.as_str() {
            DependencySpec::version(
                name.clone(),
                VersionSpec::parse(ver).map_err(ConfigError::ParseError)?,
            )
        } else if let Some(t) = val.as_table() {
            let version_str = t
                .get("version")
                .or_else(|| t.get("версия"))
                .and_then(|v| v.as_str())
                .unwrap_or("*");
            let mut dep = DependencySpec::version(
                name.clone(),
                VersionSpec::parse(version_str).map_err(ConfigError::ParseError)?,
            );
            dep.optional = t.get("optional").and_then(|v| v.as_bool()).unwrap_or(false);
            dep.default_features = t
                .get("default_features")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            if let Some(arr) = t.get("features").and_then(|v| v.as_array()) {
                dep.features = arr
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
            }
            if let Some(p) = t.get("path").and_then(|v| v.as_str()) {
                dep.path = Some(root.join(p));
            }
            if let Some(url) = t.get("git").and_then(|v| v.as_str()) {
                dep.git = Some(GitSource {
                    url: url.to_string(),
                    branch: None,
                    tag: None,
                    rev: None,
                });
            }
            if let Some(reg) = t.get("registry").and_then(|v| v.as_str()) {
                dep.registry = Some(reg.to_string());
            }
            if let Some(pkg) = t.get("package").and_then(|v| v.as_str()) {
                dep.package = Some(pkg.to_string());
            }
            dep
        } else {
            return Err(ConfigError::InvalidFormat(format!(
                "Invalid dependency spec for {name}"
            )));
        };
        map.insert(name.clone(), spec);
    }
    Ok(map)
}
