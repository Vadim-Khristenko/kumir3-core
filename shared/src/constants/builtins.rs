//! Kumir built-in functions.
//!
//! Single source of truth: `BUILTINS` table in `shared/build.rs`. This module
//! provides the generated index with categories and thin wrappers.

/// Category of a built-in function. Math, String, Io count as `is_builtin_function`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinCategory {
    Math,
    String,
    Io,
    /// Conversions, collections, and other utilities: in the name list but not `is_builtin_function`.
    Other,
}

include!(concat!(env!("OUT_DIR"), "/builtins_gen.rs"));

/// Checks if a string is a built-in function (Math, String, or Io category).
#[inline]
pub fn is_builtin_function(s: &str) -> bool {
    matches!(
        BUILTIN_INDEX.get(s),
        Some(BuiltinCategory::Math | BuiltinCategory::String | BuiltinCategory::Io)
    )
}

/// The category of a built-in name, if known.
#[inline]
pub fn builtin_category(s: &str) -> Option<BuiltinCategory> {
    BUILTIN_INDEX.get(s).copied()
}

/// All built-in function names, across all categories.
#[inline]
pub fn get_all_builtin_names() -> Vec<&'static str> {
    ALL_BUILTIN_NAMES.to_vec()
}
