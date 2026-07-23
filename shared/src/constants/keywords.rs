//! Kumir language keywords.
//!
//! Single source of truth: `KEYWORDS` table in `shared/build.rs`. This module
//! provides generated `phf` maps for forward/reverse lookup and thin wrappers.
//! No runtime initialization.

use crate::types::Token;

include!(concat!(env!("OUT_DIR"), "/keywords_gen.rs"));

/// Returns the token for a keyword string, if it is one.
#[inline]
pub fn get_keyword_token(s: &str) -> Option<Token> {
    KEYWORD_INDEX.get(s).cloned()
}

/// Checks if a string is a keyword.
#[inline]
pub fn is_keyword(s: &str) -> bool {
    KEYWORD_INDEX.contains_key(s)
}

/// All keyword spellings (canonical and aliases), for documentation and tools.
#[inline]
pub fn all_keywords() -> &'static [&'static str] {
    ALL_KEYWORDS
}

/// Reverse lookup: the canonical spelling of a keyword token.
#[inline]
pub fn keyword_for(token: &Token) -> Option<&'static str> {
    keyword_canonical(token)
}
