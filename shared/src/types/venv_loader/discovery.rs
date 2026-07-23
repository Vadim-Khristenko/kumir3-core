//! Library discovery on disk: project-local directories, global registry,
//! library file reading, and available version enumeration.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::types::environment::LibrarySource;
use crate::types::library::{LibVersion, LibraryDef};
use crate::types::version::{Version, VersionSpec};

use super::error::{LoaderError, LoaderResult};
use super::manifest::LibraryManifest;
use super::{IntegratedLoader, LoadedLibrary};

impl IntegratedLoader {
    /// Try to load from project-local folder.
    pub(super) fn try_load_local(
        &mut self,
        libs_dir: &Path,
        name: &str,
        spec: &VersionSpec,
    ) -> LoaderResult<Option<LoadedLibrary>> {
        // New target format: libs/<name>/manifest.toml + entry_point
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

        // Fallback for old paths
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

    /// Try to load from global registry.
    pub(super) fn try_load_from_registry(
        &mut self,
        registry_dir: &Path,
        name: &str,
        spec: &VersionSpec,
    ) -> LoaderResult<Option<LoadedLibrary>> {
        if !registry_dir.exists() {
            return Ok(None);
        }

        // Search for all versions of the library
        let mut versions: Vec<(Version, PathBuf)> = Vec::new();

        if let Ok(entries) = fs::read_dir(registry_dir) {
            for entry in entries.flatten() {
                let dir_name = entry.file_name().to_string_lossy().to_string();

                // Format: name-version (e.g. sockets-1.0.0)
                if let Some(suffix) = dir_name.strip_prefix(&format!("{}-", name))
                    && let Ok(version) = suffix.parse::<Version>()
                {
                    versions.push((version, entry.path()));
                }
            }
        }

        // Sort by descending version
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
}
