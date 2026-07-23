//! Configuration of module search paths, debug mode, and strict mode.

use super::Interpreter;

impl Interpreter {
    /// Sets the base directory for imports.
    pub fn set_base_dir(&mut self, dir: impl Into<std::path::PathBuf>) {
        if let Ok(mut importer) = self.file_importer.write() {
            importer.set_base_dir(dir);
        }
    }

    /// Adds a module search directory.
    pub fn add_module_path(&mut self, path: impl Into<std::path::PathBuf>) {
        if let Ok(mut importer) = self.file_importer.write() {
            importer.add_search_path(path);
        }
    }

    /// Enables or disables debug mode.
    pub fn set_debug_mode(&mut self, enabled: bool) {
        self.debug_mode = enabled;
        self.env.set_debug_mode(enabled);
    }

    /// [W0] Enables or disables strict mode.
    ///
    /// In strict mode, assignment to a previously undeclared variable (e.g., a typo
    /// `xyz := 5`) becomes a runtime error instead of silently creating the variable.
    /// Disabled by default — existing program behavior and output are unchanged.
    pub fn set_strict(&mut self, enabled: bool) {
        self.env.set_strict(enabled);
    }

    /// [W0] Checks whether strict mode is enabled.
    pub fn is_strict(&self) -> bool {
        self.env.is_strict()
    }
}
