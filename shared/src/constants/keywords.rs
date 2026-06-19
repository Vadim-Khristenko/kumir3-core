//! Ключевые слова языка Кумир.
//!
//! Источник истины — таблица `KEYWORDS` в `shared/build.rs`; здесь — сгенерированные
//! `phf`-карты (прямой/обратный поиск) и тонкие обёртки. Рантайм-инициализации нет.

use crate::types::Token;

include!(concat!(env!("OUT_DIR"), "/keywords_gen.rs"));

/// Возвращает токен ключевого слова, если строка им является.
#[inline]
pub fn get_keyword_token(s: &str) -> Option<Token> {
    KEYWORD_INDEX.get(s).cloned()
}

/// Проверяет, является ли строка ключевым словом.
#[inline]
pub fn is_keyword(s: &str) -> bool {
    KEYWORD_INDEX.contains_key(s)
}

/// Все написания ключевых слов (для документации/инструментов).
#[inline]
pub fn all_keywords() -> &'static [&'static str] {
    ALL_KEYWORDS
}

/// Обратный поиск: каноничное написание для токена ключевого слова.
#[inline]
pub fn keyword_for(token: &Token) -> Option<&'static str> {
    keyword_canonical(token)
}
