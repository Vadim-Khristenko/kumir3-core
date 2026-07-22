// =============================================================================
//        МОДУЛЬ: ПРОВЕРКИ САМОГО РАННЕРА (сканер директив и допуски)
// =============================================================================

use super::compare::{Comparison, ExpectedError, Tolerance, compare, normalize};
use super::directives::{Key, parse_case_file, parse_continuation_line, parse_directive_line};
use std::path::Path;

// -----------------------------------------------------------------------------
//                                   СКАНЕР
// -----------------------------------------------------------------------------

#[test]
fn direktiva_razbiraetsya_s_klyuchom_iz_dvuh_slov() {
    let (key, value) = parse_directive_line("| ОЖИДАЕМЫЙ ВЫВОД: 12").expect("это директива");
    assert_eq!(key, Key::ExpectedOutput);
    assert_eq!(value, "12");
}

#[test]
fn direktiva_bez_znacheniya_daet_pustuyu_stroku() {
    let (key, value) = parse_directive_line("| ВВОД:").expect("это директива");
    assert_eq!(key, Key::Input);
    assert!(value.is_empty());
}

#[test]
fn stroka_prodolzheniya_s_dvoetochiem_ne_stanovitsya_direktivoj() {
    // Пример KITE 18 § 3.4: текст продолжения содержит двоеточие.
    assert!(parse_directive_line("| Строка 3 с ведущими пробелами:    хвост").is_none());
    assert_eq!(
        parse_continuation_line("| Строка 3 с ведущими пробелами:    хвост").as_deref(),
        Some("Строка 3 с ведущими пробелами:    хвост")
    );
}

#[test]
fn odinokaya_palochka_daet_pustuyu_stroku_znacheniya() {
    assert_eq!(parse_continuation_line("|").as_deref(), Some(""));
}

#[test]
fn tolko_pervyj_probel_sluzhit_razdelitelem() {
    assert_eq!(
        parse_continuation_line("|     отступ значим").as_deref(),
        Some("    отступ значим")
    );
}

#[test]
fn mnogostrochnyj_etalon_sobiraetsya_iz_blokov() {
    let source = "| краткое описание\n\
                  | КЕЙС: пример\n\
                  | ОЖИДАЕМЫЙ ВЫВОД:\n\
                  | Привет, мир!\n\
                  |\n\
                  | хвост\n\
                  алг главный\n";
    let file = parse_case_file(Path::new("пример.kum"), source);
    assert!(file.problems.is_empty(), "{:?}", file.problems);
    assert_eq!(file.cases.len(), 1);
    assert_eq!(
        file.cases[0].meta.expected_output.as_deref(),
        Some("Привет, мир!\n\nхвост")
    );
}

#[test]
fn prelyudiya_dostaetsya_kazhdomu_kejsu() {
    let source = "| описание\n\
                  | УРОВЕНЬ: 1\n\
                  алг цел ф(цел н)\n\
                  кон\n\
                  | КЕЙС: первый\n\
                  | ОЖИДАЕМЫЙ ВЫВОД: 1\n\
                  алг главный первый\n\
                  | КЕЙС: второй\n\
                  | ОЖИДАЕМЫЙ ВЫВОД: 2\n\
                  алг главный второй\n";
    let file = parse_case_file(Path::new("пример.kum"), source);
    assert_eq!(file.cases.len(), 2);
    assert_eq!(file.meta.level.as_deref(), Some("1"));
    // Файловая директива видна обоим кейсам (§ 3.1, п. 3).
    assert_eq!(file.cases[1].meta.level.as_deref(), Some("1"));
    let assembled = file.assemble(&file.cases[0]);
    assert!(assembled.contains("алг цел ф(цел н)"));
    assert!(assembled.contains("алг главный первый"));
    assert!(!assembled.contains("алг главный второй"));
}

#[test]
fn skaner_rabotaet_na_nerazbiraemoj_programme() {
    // § 3.11, п. 2: метаданные кейсов доступны и для «сломанной» программы.
    let source = "| описание\n\
                  | КЕЙС: мусор\n\
                  | ОЖИДАЕМЫЙ ВЫВОД: 1\n\
                  ))) это не программа (((\n\
                  нц кц кон алг\n";
    let file = parse_case_file(Path::new("сломано.kum"), source);
    assert_eq!(file.cases.len(), 1);
    assert_eq!(file.cases[0].name, "мусор");
    assert_eq!(file.cases[0].meta.expected_output.as_deref(), Some("1"));
}

#[test]
fn neizvestnyj_klyuch_soobshchaetsya() {
    let source = "| описание\n| ОЖИДАЕМЫЙ ВЫВД: 1\nалг главный\n";
    let file = parse_case_file(Path::new("опечатка.kum"), source);
    assert_eq!(file.problems.len(), 1);
    assert!(file.problems[0].message.contains("ОЖИДАЕМЫЙ ВЫВД"));
}

#[test]
fn direktiva_kejsa_vne_kejsa_soobshchaetsya() {
    let source = "| описание\n| ОЖИДАЕМЫЙ ВЫВОД: 1\nалг главный\n";
    let file = parse_case_file(Path::new("вне.kum"), source);
    assert!(
        file.problems
            .iter()
            .any(|p| p.message.contains("должна стоять после"))
    );
}

#[test]
fn direktiva_tolko_otbiraet_kejsy() {
    let source = "| описание\n\
                  | КЕЙС: первый\n\
                  | ОЖИДАЕМЫЙ ВЫВОД: 1\n\
                  | КЕЙС: второй\n\
                  | ТОЛЬКО:\n\
                  | ОЖИДАЕМЫЙ ВЫВОД: 2\n";
    let file = parse_case_file(Path::new("отбор.kum"), source);
    let selected = file.selected_cases();
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].name, "второй");
}

// -----------------------------------------------------------------------------
//                                  СРАВНЕНИЕ
// -----------------------------------------------------------------------------

fn ravno(expected: &str, actual: &str, tolerance: &Tolerance) -> bool {
    matches!(compare(expected, actual, tolerance), Comparison::Equal)
}

#[test]
fn normalizaciya_ubiraet_crlf_i_hvostovoj_perevod() {
    assert_eq!(normalize("а\r\nб\n\n"), "а\nб");
}

#[test]
fn rezhim_tochno() {
    assert!(ravno("12", "12\n", &Tolerance::Exact));
    assert!(!ravno("12", "13", &Tolerance::Exact));
    assert!(!ravno("а б", "а  б", &Tolerance::Exact));
}

#[test]
fn rezhim_probely() {
    assert!(ravno("а б", "а      б   ", &Tolerance::Whitespace));
    assert!(ravno("а\tб", "а б", &Tolerance::Whitespace));
    assert!(!ravno("а б", "аб", &Tolerance::Whitespace));
}

#[test]
fn rezhim_veshchestvennyj() {
    let t = Tolerance::Float(1e-9);
    assert!(ravno("x = 1.414213562", "x = 1.4142135623", &t));
    assert!(!ravno("x = 1.414213562", "x = 1.415", &t));
    // Нечисловой текст сравнивается точно.
    assert!(!ravno("x = 1.0", "y = 1.0", &t));
    // Экспоненциальная запись и знак.
    assert!(ravno("-1e-3", "-0.001", &t));
}

#[test]
fn rezhim_regeksp() {
    assert!(ravno(r"время: \d+ мс", "время: 1234 мс", &Tolerance::Regex));
    assert!(!ravno(
        r"время: \d+ мс",
        "время: много мс",
        &Tolerance::Regex
    ));
    // Совпадение обязано быть целиком.
    assert!(!ravno(r"\d+", "12 и хвост", &Tolerance::Regex));
}

#[test]
fn dopusk_razbiraetsya() {
    assert_eq!(Tolerance::parse("точно"), Ok(Tolerance::Exact));
    assert_eq!(Tolerance::parse("пробелы"), Ok(Tolerance::Whitespace));
    assert_eq!(Tolerance::parse("регэксп"), Ok(Tolerance::Regex));
    assert_eq!(Tolerance::parse("вещ:1e-9"), Ok(Tolerance::Float(1e-9)));
    assert!(Tolerance::parse("вещ:абв").is_err());
    assert!(Tolerance::parse("приблизительно").is_err());
}

#[test]
fn raznica_nazyvaet_nesovpavshuyu_stroku() {
    let Comparison::Different(text) = compare("а\nб", "а\nв", &Tolerance::Exact) else {
        panic!("ожидалось расхождение");
    };
    assert!(text.contains("строка   2"), "{text}");
    assert!(text.contains("«б»"), "{text}");
    assert!(text.contains("«в»"), "{text}");
}

// -----------------------------------------------------------------------------
//                              ОЖИДАЕМАЯ ОШИБКА
// -----------------------------------------------------------------------------

#[test]
fn ozhidaemaya_oshibka_razbiraetsya() {
    let e = ExpectedError::parse("DivisionByZero / делитель равен нулю").expect("вид известен");
    assert_eq!(e.kind, "DivisionByZero");
    assert_eq!(e.message_part.as_deref(), Some("делитель равен нулю"));

    let e = ExpectedError::parse("Other").expect("вид известен");
    assert_eq!(e.kind, "Other");
    assert!(e.message_part.is_none());

    assert!(ExpectedError::parse("НетТакогоВида").is_err());
}
