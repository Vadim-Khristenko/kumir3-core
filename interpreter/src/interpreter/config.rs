// =============================================================================
//                  МОДУЛЬ: НАСТРОЙКА ИНТЕРПРЕТАТОРА
// =============================================================================
// Пути поиска модулей, режим отладки и строгий режим [W0].
use super::Interpreter;

impl Interpreter {
    /// Устанавливает базовую директорию для импортов.
    pub fn set_base_dir(&mut self, dir: impl Into<std::path::PathBuf>) {
        if let Ok(mut importer) = self.file_importer.write() {
            importer.set_base_dir(dir);
        }
    }

    /// Добавляет директорию поиска модулей.
    pub fn add_module_path(&mut self, path: impl Into<std::path::PathBuf>) {
        if let Ok(mut importer) = self.file_importer.write() {
            importer.add_search_path(path);
        }
    }

    /// Включает/выключает режим отладки.
    pub fn set_debug_mode(&mut self, enabled: bool) {
        self.debug_mode = enabled;
        self.env.set_debug_mode(enabled);
    }

    /// [W0] Включает/выключает строгий режим.
    ///
    /// В строгом режиме присваивание ранее необъявленной переменной
    /// (например, опечатка `хyz := 5`) становится ошибкой выполнения вместо
    /// молчаливого создания переменной. По умолчанию выключен — поведение и
    /// вывод существующих программ не меняются.
    pub fn set_strict(&mut self, enabled: bool) {
        self.env.set_strict(enabled);
    }

    /// [W0] Проверяет, включён ли строгий режим.
    pub fn is_strict(&self) -> bool {
        self.env.is_strict()
    }
}
