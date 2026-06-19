//! Встроенные функции Кумир.
//!
//! Источник истины — таблица `BUILTINS` в `shared/build.rs`; здесь — сгенерированный
//! индекс с категориями и тонкие обёртки.

/// Категория встроенной функции. Math/String/Io считаются `is_builtin_function`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinCategory {
    Math,
    String,
    Io,
    /// Конверсии/коллекции/прочее: входят в перечень имён, но не в is_builtin_function.
    Other,
}

include!(concat!(env!("OUT_DIR"), "/builtins_gen.rs"));

/// Проверяет, является ли строка встроенной функцией (Math/String/Io).
#[inline]
pub fn is_builtin_function(s: &str) -> bool {
    matches!(
        BUILTIN_INDEX.get(s),
        Some(BuiltinCategory::Math | BuiltinCategory::String | BuiltinCategory::Io)
    )
}

/// Категория встроенной функции, если известна.
#[inline]
pub fn builtin_category(s: &str) -> Option<BuiltinCategory> {
    BUILTIN_INDEX.get(s).copied()
}

/// Все имена встроенных функций (всех категорий).
#[inline]
pub fn get_all_builtin_names() -> Vec<&'static str> {
    ALL_BUILTIN_NAMES.to_vec()
}
