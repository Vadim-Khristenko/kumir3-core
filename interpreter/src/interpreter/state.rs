// =============================================================================
//                  МОДУЛЬ: СОСТОЯНИЕ ИНТЕРПРЕТАТОРА
// =============================================================================
// Доступ к среде выполнения, переменным, буферу вывода, предупреждениям [W0],
// менеджеру библиотек и async-runtime.
use super::{Environment, Interpreter, KumirRuntime, LibraryManager, RuntimeResult};
use shared::types::Value;

impl Interpreter {
    /// [W0] Возвращает предупреждения, собранные во время выполнения
    /// (например, об использовании необъявленных переменных). Они НЕ попадают
    /// в вывод программы (`вывод`/stdout).
    pub fn warnings(&self) -> &[String] {
        self.env.warnings()
    }

    /// Возвращает ссылку на среду выполнения.
    pub fn environment(&self) -> &Environment {
        &self.env
    }

    /// Возвращает изменяемую ссылку на среду выполнения.
    pub fn environment_mut(&mut self) -> &mut Environment {
        &mut self.env
    }

    // =========================================================================
    //                    ПЕРЕМЕННЫЕ
    // =========================================================================

    /// Устанавливает глобальную переменную.
    pub fn set_global(&mut self, name: impl Into<String>, value: Value) {
        self.env.define_global(name.into(), value);
    }

    /// Получает значение переменной.
    pub fn get_variable(&self, name: &str) -> RuntimeResult<&Value> {
        self.env.get_variable(name)
    }

    // =========================================================================
    //                    ВЫВОД
    // =========================================================================

    /// Получает вывод программы.
    pub fn get_output(&self) -> String {
        self.env.get_output()
    }

    /// Очищает буфер вывода.
    pub fn clear_output(&mut self) {
        self.env.clear_output();
    }

    // =========================================================================
    //                    RUNTIME И БИБЛИОТЕКИ
    // =========================================================================

    /// Получает менеджер библиотек.
    pub fn libraries(&self) -> &std::sync::Arc<std::sync::RwLock<LibraryManager>> {
        &self.libraries
    }

    /// Получает runtime (если инициализирован).
    pub fn runtime(&self) -> Option<&KumirRuntime> {
        self.runtime.as_ref()
    }
}
