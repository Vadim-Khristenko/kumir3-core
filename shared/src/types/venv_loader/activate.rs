//! Project environment activation and lock file application.

use std::path::{Path, PathBuf};

use crate::types::config::LockFile;
use crate::types::environment::{LibrarySource, ResolvedDependency};
use crate::types::version::VersionSpec;

use super::IntegratedLoader;
use super::error::{LoaderError, LoaderResult};

impl IntegratedLoader {
    /// Activate project environment.
    pub fn activate_project(&mut self, project_root: impl AsRef<Path>) -> LoaderResult<()> {
        let project_root = project_root.as_ref();

        // Activate environment
        self.env_manager.activate_project(project_root);

        // Load kumir.lock if it exists
        let lock_path = project_root.join("kumir.lock");
        if lock_path.exists() {
            self.load_lock_file(&lock_path)?;
        }

        Ok(())
    }

    /// Load lock file.
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

    /// Deactivate project.
    pub fn deactivate_project(&mut self) {
        self.env_manager.deactivate();
    }
}
