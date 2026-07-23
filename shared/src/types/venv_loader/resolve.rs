//! Library resolution by name/version and loading with all dependencies.

use crate::types::environment::LibrarySource;
use crate::types::version::{Version, VersionSpec};

use super::error::{LoaderError, LoaderResult};
use super::{IntegratedLoader, LoadedLibrary};

impl IntegratedLoader {
    // =========================================================================
    //                         LIBRARY LOADING
    // =========================================================================

    /// Load a library by name.
    pub fn load(&mut self, name: &str) -> LoaderResult<LoadedLibrary> {
        self.load_with_spec(name, &VersionSpec::any())
    }

    /// Load a library with version verification.
    pub fn load_with_spec(
        &mut self,
        name: &str,
        spec: &VersionSpec,
    ) -> LoaderResult<LoadedLibrary> {
        // Check for cycles
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

    /// Load a specific version.
    pub fn load_version(&mut self, name: &str, version: &Version) -> LoaderResult<LoadedLibrary> {
        self.load_with_spec(name, &VersionSpec::exact(version.clone()))
    }

    /// Internal implementation of loading.
    fn load_impl(&mut self, name: &str, spec: &VersionSpec) -> LoaderResult<LoadedLibrary> {
        // 1. Check in active environment
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

        // 2. Built-in libraries
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

        // 3. Project-local libraries
        let paths = self.env_manager.active().paths.clone();
        if let Some(lib) = self.try_load_local(&paths.local_libs, name, spec)? {
            return Ok(lib);
        }

        // 4. Global registry
        if let Some(lib) = self.try_load_from_registry(&paths.global_cache, name, spec)? {
            return Ok(lib);
        }

        // 5. Not found
        let searched_paths = vec![paths.local_libs, paths.global_cache];

        Err(LoaderError::NotFound {
            name: name.to_string(),
            searched_paths,
        })
    }

    // =========================================================================
    //                    LOADING WITH DEPENDENCIES
    // =========================================================================

    /// Load a library with all dependencies.
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
}
