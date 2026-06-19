use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

// (Token variant, canonical spelling, other spellings). ОДНА строка на вариант Token.
// Источник истины для ключевых слов языка Кумир (прямой и обратный поиск).
const KEYWORDS: &[(&str, &str, &[&str])] = &[
    // ---- ALGORITHM STRUCTURE (KUMIR 2) ----
    ("Alg", "алг", &["alg", "algorithm"]),
    ("Begin", "нач", &["begin"]),
    ("End", "кон", &["end"]),
    ("Given", "дано", &["given"]),
    ("Need", "надо", &["need"]),
    ("Arg", "арг", &["arg"]),
    ("Res", "рез", &["res"]),
    ("ArgRes", "аргрез", &["argres"]),
    // ---- PRIMITIVE TYPES (KUMIR 2) ----
    ("IntType", "цел", &["int", "integer"]),
    ("FloatType", "вещ", &["float", "real"]),
    ("BoolType", "лог", &["bool", "boolean"]),
    ("CharType", "сим", &["char"]),
    ("StringType", "лит", &["string", "str"]),
    ("ArrayType", "таб", &["array", "tab"]),
    // ---- ADVANCED TYPES (KUMIR 3) ----
    ("PointerType", "указатель", &["pointer", "ptr"]),
    ("EnumType", "перечисление", &["enum"]),
    ("AutoType", "авто", &["auto", "var"]),
    ("NoneType", "пустота", &["void", "unit"]),
    (
        "OptionalType",
        "необязательно",
        &["Необязательно", "optional", "Optional", "может"],
    ),
    // ---- MUTABILITY & OWNERSHIP (KUMIR 3) ----
    ("Mut", "измен", &["изменяемый", "mut", "mutable"]),
    ("Const", "конст", &["константа", "const", "constant"]),
    ("Move", "перемещение", &["переместить", "move"]),
    ("Borrow", "заимствовать", &["заимств", "borrow"]),
    ("Clone", "клонировать", &["клон", "clone"]),
    ("Copy", "копировать", &["копия", "copy"]),
    // ---- GENERICS & TYPE ALIASES (KUMIR 3) ----
    ("Where", "где", &["where"]),
    ("TypeAlias", "типалиас", &["type", "typedef"]),
    // ---- LOGIC OPERATORS & CONSTANTS ----
    ("And", "и", &["and"]),
    ("Or", "или", &["or"]),
    ("Not", "не", &["not"]),
    ("True", "да", &["true", "истина"]),
    ("False", "нет", &["false", "ложь"]),
    // ---- CONTROL FLOW (KUMIR 2) ----
    ("If", "если", &["if"]),
    ("Then", "то", &["then"]),
    ("Else", "иначе", &["else"]),
    ("Fi", "все", &["всё", "fi", "endif"]),
    ("Switch", "выбор", &["switch"]),
    ("Case", "при", &["case"]),
    ("Loop", "нц", &["loop"]),
    ("EndLoop", "кц", &["endloop"]),
    ("For", "для", &["for"]),
    ("From", "от", &["from"]),
    ("To", "до", &["to"]),
    ("Step", "шаг", &["step"]),
    ("While", "пока", &["while"]),
    // ---- IO & RUNTIME CONTROL (KUMIR 2) ----
    ("Input", "ввод", &["input", "read"]),
    ("Output", "вывод", &["output", "print", "write"]),
    ("Assert", "утв", &["assert"]),
    ("Pause", "пауза", &["pause"]),
    ("Halt", "выход", &["halt", "exit"]),
    ("Use", "использовать", &["use"]),
    ("Return", "вернуть", &["return"]),
    ("ResultValue", "знач", &["result"]),
    // ---- MODULE SYSTEM (KUMIR 3) ----
    ("Import", "подключить", &["import", "include"]),
    ("Module", "модуль", &["module", "mod"]),
    ("Export", "экспорт", &["export", "pub"]),
    // ---- MEMORY & POINTERS (KUMIR 3) ----
    ("New", "новый", &["new", "создать"]),
    ("Delete", "удалить", &["delete", "free"]),
    ("Ref", "ссылка", &["ref"]),
    ("Deref", "разыменовать", &["deref"]),
    // ---- ENUMS & PATTERN MATCHING (KUMIR 3) ----
    ("EnumDecl", "объявить_перечисление", &["enum_decl"]),
    ("Match", "совпадение", &["match"]),
    ("When", "когда", &["when", "guard"]),
    // ---- OBJECT MODEL (KUMIR 3) ----
    ("Class", "класс", &["Класс", "class", "Class"]),
    (
        "Struct",
        "структура",
        &["Структура", "struct", "Struct", "запись"],
    ),
    (
        "Interface",
        "интерфейс",
        &["Интерфейс", "interface", "Interface"],
    ),
    (
        "Trait",
        "свойство",
        &["Свойство", "trait", "Trait", "типаж"],
    ),
    ("Impl", "реализация", &["Реализация", "impl", "Impl"]),
    ("Self_", "я", &["себя", "self", "Self"]),
    ("This", "это", &["this"]),
    ("Super", "предок", &["родитель", "супер", "super"]),
    (
        "Constructor",
        "конструктор",
        &["констр", "constructor", "init"],
    ),
    (
        "Destructor",
        "деструктор",
        &["дестр", "destructor", "deinit", "drop"],
    ),
    ("Public", "открытый", &["публичное", "публ", "public"]),
    ("Private", "закрытый", &["приватное", "приват", "private"]),
    (
        "Protected",
        "защищённый",
        &["защищённое", "защищ", "protected"],
    ),
    ("Static", "статический", &["статическое", "стат", "static"]),
    (
        "Virtual",
        "виртуальный",
        &["виртуальное", "вирт", "virtual"],
    ),
    ("Override", "переопределить", &["переопр", "override"]),
    (
        "Abstract",
        "абстрактный",
        &["абстрактное", "абстр", "abstract"],
    ),
    ("Final", "финальный", &["финал", "final", "sealed"]),
    ("Extends", "расширяет", &["extends", "наследует"]),
    ("Implements", "реализует", &["implements"]),
    // ---- RUST EMBEDS (KUMIR 3) ----
    ("RustBlockStart", "РастВставкаНЦ", &[]),
    ("RustBlockEnd", "РастВставкаКЦ", &[]),
    ("Rust", "ржавчина", &["Ржавчина", "rust"]),
    // ---- FUNCTIONAL PROGRAMMING (KUMIR 3) ----
    ("Lambda", "лямбда", &["lambda", "fn", "func"]),
    // ---- ASYNC & CONCURRENCY (KUMIR 3) ----
    ("Async", "асинх", &["async", "асинхронный"]),
    ("Await", "ждать", &["await", "ожидать"]),
    ("Spawn", "запустить", &["spawn"]),
    ("Yield", "уступить", &["yield"]),
    // ---- ERROR HANDLING (KUMIR 3) ----
    ("Try", "попытка", &["try"]),
    ("Catch", "перехват", &["catch", "except"]),
    ("Throw", "бросить", &["throw", "raise"]),
    ("Finally", "наконец", &["finally"]),
    // ---- RESOURCE GUARDING (KUMIR 3) ----
    ("Defer", "отложить", &["defer"]),
    // ---- SPECIAL VALUES (KUMIR 3) ----
    ("None", "Пусто", &["пусто", "None", "none", "nil", "null"]),
    (
        "NotImplemented",
        "НеРеализовано",
        &[
            "не_реализовано",
            "NotImplemented",
            "not_implemented",
            "todo",
            "TODO",
        ],
    ),
    (
        "NotAvailable",
        "НеДоступно",
        &[
            "не_доступно",
            "NotAvailable",
            "not_available",
            "unavailable",
        ],
    ),
    (
        "Deprecated",
        "Устарело",
        &["устарело", "Deprecated", "deprecated"],
    ),
];

fn write_keywords(out_dir: &str) {
    let path = Path::new(out_dir).join("keywords_gen.rs");
    let mut w = BufWriter::new(File::create(&path).unwrap());

    // Значения (`Token::Variant`) должны пережить карту до вызова `build()`,
    // поэтому собираем их заранее во владеющий Vec.
    let entries: Vec<(&str, String)> = KEYWORDS
        .iter()
        .flat_map(|(variant, canonical, aliases)| {
            let value = format!("Token::{variant}");
            std::iter::once((*canonical, value.clone()))
                .chain(aliases.iter().map(move |alias| (*alias, value.clone())))
        })
        .collect();

    let mut map = phf_codegen::Map::new();
    for (key, value) in &entries {
        map.entry(*key, value);
    }
    writeln!(
        w,
        "static KEYWORD_INDEX: ::phf::Map<&'static str, Token> = {};",
        map.build()
    )
    .unwrap();

    writeln!(
        w,
        "pub fn keyword_canonical(t: &Token) -> Option<&'static str> {{"
    )
    .unwrap();
    writeln!(w, "    match t {{").unwrap();
    for (variant, canonical, _) in KEYWORDS {
        writeln!(w, "        Token::{variant} => Some(\"{canonical}\"),").unwrap();
    }
    writeln!(w, "        _ => None,").unwrap();
    writeln!(w, "    }}").unwrap();
    writeln!(w, "}}").unwrap();

    write!(w, "static ALL_KEYWORDS: &[&str] = &[").unwrap();
    for (_, canonical, aliases) in KEYWORDS {
        write!(w, "\"{canonical}\",").unwrap();
        for alias in *aliases {
            write!(w, "\"{alias}\",").unwrap();
        }
    }
    writeln!(w, "];").unwrap();
}

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    write_keywords(&out_dir);
    println!("cargo:rerun-if-changed=build.rs");
}
