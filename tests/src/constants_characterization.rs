//! Характеризация констант: снимок текущего поведения публичного API.
//! Снимается ДО рефакторинга и гарантирует 100% паритет после.
//!
//! Перегенерация снимков (только при намеренном изменении поведения):
//!   BLESS=1 cargo test -p tests --  constants_characterization

use shared::constants::*;
use shared::lexer::tokenize;

/// Сравнивает actual со снимком на диске; при BLESS=1 — записывает снимок.
fn assert_snapshot(rel_path: &str, actual: &str) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(rel_path);
    // Канонический текстовый файл — с финальным переводом строки.
    let actual = format!("{actual}\n");
    if std::env::var("BLESS").is_ok() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, &actual).unwrap();
        eprintln!(
            "[BLESS] перезаписан снимок {}; перезапустите БЕЗ BLESS для проверки",
            path.display()
        );
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "missing snapshot {}; run with BLESS=1 to create",
            path.display()
        )
    });
    assert_eq!(actual, expected, "snapshot drift in {rel_path}");
}

#[test]
fn keyword_map_snapshot() {
    let mut pairs: Vec<String> = all_keywords()
        .iter()
        .map(|k| {
            format!(
                "{k}\t{:?}",
                get_keyword_token(k).expect("keyword must resolve")
            )
        })
        .collect();
    pairs.sort();
    assert_snapshot("src/golden/keywords.snapshot", &pairs.join("\n"));
}

/// Все символы операторов (3/2/1-символьные), порядок не важен — сортируется.
const OPERATOR_SYMBOLS: &[&str] = &[
    "...", "..=", "<<=", ">>=", // 3-char
    "<>", "!=", "<=", ">=", "==", ":=", "+=", "-=", "*=", "/=", "%=", "**", "->", "=>", "::", "|>",
    ">>", "..", "&&", "||", // 2-char
    "+", "-", "*", "/", "%", "=", "<", ">", "(", ")", "[", "]", "{", "}", ",", ":", ";", ".", "@",
    "&", "^", "?", "!", "~", // 1-char
];

#[test]
fn operator_map_snapshot() {
    let mut pairs: Vec<String> = OPERATOR_SYMBOLS
        .iter()
        .map(|sym| {
            let first = tokenize(sym)
                .expect("tokenize op")
                .into_iter()
                .next()
                .unwrap();
            format!("{sym}\t{:?}", first.token)
        })
        .collect();
    pairs.sort();
    assert_snapshot("src/golden/operators.snapshot", &pairs.join("\n"));
}

#[test]
fn builtin_function_snapshot() {
    // Паритет двух вещей: множества is_builtin_function и полного get_all_builtin_names.
    let mut all: Vec<&str> = get_all_builtin_names();
    all.sort();
    all.dedup();
    let lines: Vec<String> = all
        .iter()
        .map(|n| format!("{n}\tis_builtin={}", is_builtin_function(n)))
        .collect();
    assert_snapshot("src/golden/builtins.snapshot", &lines.join("\n"));
}
