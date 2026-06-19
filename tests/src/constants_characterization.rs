//! Характеризация констант: снимок текущего поведения публичного API.
//! Снимается ДО рефакторинга и гарантирует 100% паритет после.
//!
//! Перегенерация снимков (только при намеренном изменении поведения):
//!   BLESS=1 cargo test -p tests --  constants_characterization

use shared::constants::*;

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
