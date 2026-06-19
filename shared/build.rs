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

// (имя, категория). Источник истины для встроенных функций.
// Math/String/Io -> is_builtin_function=true; Other -> false.
// Имена из слайсов Math/String/Io стоят с соответствующей категорией.
// Имена только из extra-списка стоят с Other.
// Имена, присутствующие в обоих (например "int"), стоят только один раз с категорией слайса.
const BUILTINS: &[(&str, &str)] = &[
    // ----- Math -----
    ("sin", "Math"),
    ("cos", "Math"),
    ("tg", "Math"),
    ("tan", "Math"),
    ("ctg", "Math"),
    ("cot", "Math"),
    ("arcsin", "Math"),
    ("arccos", "Math"),
    ("arctg", "Math"),
    ("arctan", "Math"),
    ("arcctg", "Math"),
    ("arccot", "Math"),
    ("sh", "Math"),
    ("sinh", "Math"),
    ("ch", "Math"),
    ("cosh", "Math"),
    ("th", "Math"),
    ("tanh", "Math"),
    ("sqrt", "Math"),
    ("корень", "Math"),
    ("exp", "Math"),
    ("ln", "Math"),
    ("lg", "Math"),
    ("log", "Math"),
    ("степень", "Math"),
    ("pow", "Math"),
    ("abs", "Math"),
    ("модуль", "Math"),
    ("sign", "Math"),
    ("знак", "Math"),
    ("int", "Math"),
    ("цел_часть", "Math"),
    ("frac", "Math"),
    ("дробь", "Math"),
    ("floor", "Math"),
    ("ceil", "Math"),
    ("round", "Math"),
    ("округл", "Math"),
    ("min", "Math"),
    ("мин", "Math"),
    ("max", "Math"),
    ("макс", "Math"),
    ("mod", "Math"),
    ("div", "Math"),
    ("случ", "Math"),
    ("rand", "Math"),
    ("random", "Math"),
    // ----- String -----
    ("длин", "String"),
    ("len", "String"),
    ("length", "String"),
    ("символ", "String"),
    ("char", "String"),
    ("chr", "String"),
    ("код", "String"),
    ("ord", "String"),
    ("code", "String"),
    ("вырезка", "String"),
    ("substr", "String"),
    ("substring", "String"),
    ("позиция", "String"),
    ("pos", "String"),
    ("position", "String"),
    ("find", "String"),
    ("заменить", "String"),
    ("replace", "String"),
    ("верхний", "String"),
    ("upper", "String"),
    ("uppercase", "String"),
    ("нижний", "String"),
    ("lower", "String"),
    ("lowercase", "String"),
    ("обрезать", "String"),
    ("trim", "String"),
    ("слева", "String"),
    ("left", "String"),
    ("справа", "String"),
    ("right", "String"),
    ("повторить", "String"),
    ("repeat", "String"),
    ("разбить", "String"),
    ("split", "String"),
    ("соединить", "String"),
    ("join", "String"),
    // ----- Io -----
    ("ввод", "Io"),
    ("input", "Io"),
    ("вывод", "Io"),
    ("output", "Io"),
    ("print", "Io"),
    ("вывод_строки", "Io"),
    ("println", "Io"),
    ("новая_строка", "Io"),
    ("newline", "Io"),
    ("нс", "Io"),
    // ----- Other (extra-only: conversions / collections / misc) -----
    ("цел", "Other"),
    ("to_int", "Other"),
    ("вещ", "Other"),
    ("float", "Other"),
    ("to_float", "Other"),
    ("строка", "Other"),
    ("str", "Other"),
    ("to_string", "Other"),
    ("лог", "Other"),
    ("bool", "Other"),
    ("to_bool", "Other"),
    ("размер", "Other"),
    ("size", "Other"),
    ("добавить", "Other"),
    ("push", "Other"),
    ("извлечь", "Other"),
    ("pop", "Other"),
    ("обратить", "Other"),
    ("reverse", "Other"),
    ("сортировать", "Other"),
    ("sort", "Other"),
    ("содержит", "Other"),
    ("contains", "Other"),
    ("индекс", "Other"),
    ("index_of", "Other"),
    ("утв", "Other"),
    ("assert", "Other"),
    ("тип", "Other"),
    ("type_of", "Other"),
    ("ошибка", "Other"),
    ("error", "Other"),
];

fn write_builtins(out_dir: &str) {
    let path = Path::new(out_dir).join("builtins_gen.rs");
    let mut w = BufWriter::new(File::create(&path).unwrap());

    let entries: Vec<(&str, String)> = BUILTINS
        .iter()
        .map(|(name, cat)| {
            assert!(
                !name.contains('"') && !name.contains('\\'),
                "builtin name needs escaping: {name}"
            );
            (*name, format!("BuiltinCategory::{cat}"))
        })
        .collect();
    let mut map = phf_codegen::Map::new();
    for (name, value) in &entries {
        map.entry(*name, value);
    }
    writeln!(
        w,
        "static BUILTIN_INDEX: ::phf::Map<&'static str, BuiltinCategory> = {};",
        map.build()
    )
    .unwrap();

    write!(w, "static ALL_BUILTIN_NAMES: &[&str] = &[").unwrap();
    for (name, _) in BUILTINS {
        write!(w, "\"{name}\",").unwrap();
    }
    writeln!(w, "];").unwrap();
}

// (символ, Token-вариант). Несколько символов могут давать один Token — это ок;
// обратный поиск для операторов НЕ генерируется.
const OPERATORS: &[(&str, &str)] = &[
    // 3-char
    ("...", "Ellipsis"),
    ("..=", "DoubleDotEq"),
    ("<<=", "Assign"),
    (">>=", "Assign"),
    // 2-char
    ("<>", "NotEqual"),
    ("!=", "NotEqual"),
    ("<=", "LessEqual"),
    (">=", "GreaterEqual"),
    ("==", "Equal"),
    (":=", "Assign"),
    ("+=", "PlusAssign"),
    ("-=", "MinusAssign"),
    ("*=", "StarAssign"),
    ("/=", "SlashAssign"),
    ("%=", "Assign"),
    ("**", "Power"),
    ("->", "Arrow"),
    ("=>", "FatArrow"),
    ("::", "DoubleColon"),
    ("|>", "Pipe"),
    (">>", "Compose"),
    ("..", "DoubleDot"),
    ("&&", "And"),
    ("||", "Or"),
    // 1-char
    ("+", "Plus"),
    ("-", "Minus"),
    ("*", "Star"),
    ("/", "Slash"),
    ("%", "Percent"),
    ("=", "Equal"),
    ("<", "Less"),
    (">", "Greater"),
    ("(", "LParen"),
    (")", "RParen"),
    ("[", "LBracket"),
    ("]", "RBracket"),
    ("{", "LBrace"),
    ("}", "RBrace"),
    (",", "Comma"),
    (":", "Colon"),
    (";", "SemiColon"),
    (".", "Dot"),
    ("@", "At"),
    ("&", "Ampersand"),
    ("^", "Caret"),
    ("?", "Question"),
    ("!", "Not"),
    ("~", "Not"),
];

fn write_operators(out_dir: &str) {
    let path = Path::new(out_dir).join("operators_gen.rs");
    let mut w = BufWriter::new(File::create(&path).unwrap());

    // Прямая карта: значения должны пережить карту до вызова `build()`,
    // поэтому собираем их заранее во владеющий Vec (как в write_keywords).
    let entries: Vec<(&str, String)> = OPERATORS
        .iter()
        .map(|(sym, variant)| {
            assert!(
                !sym.contains('"') && !sym.contains('\\'),
                "operator symbol needs escaping: {sym}"
            );
            (*sym, format!("Token::{variant}"))
        })
        .collect();
    let mut map = phf_codegen::Map::new();
    for (sym, value) in &entries {
        map.entry(*sym, value);
    }
    writeln!(
        w,
        "static OPERATOR_INDEX: ::phf::Map<&'static str, Token> = {};",
        map.build()
    )
    .unwrap();

    // Множество первых символов всех операторов (для is_operator_char).
    let mut first_chars: Vec<char> = OPERATORS
        .iter()
        .map(|(sym, _)| sym.chars().next().unwrap())
        .collect();
    first_chars.sort();
    first_chars.dedup();
    let mut set = phf_codegen::Set::new();
    for c in &first_chars {
        set.entry(*c);
    }
    writeln!(
        w,
        "static OPERATOR_FIRST_CHARS: ::phf::Set<char> = {};",
        set.build()
    )
    .unwrap();
}

fn write_keywords(out_dir: &str) {
    let path = Path::new(out_dir).join("keywords_gen.rs");
    let mut w = BufWriter::new(File::create(&path).unwrap());

    // Значения (`Token::Variant`) должны пережить карту до вызова `build()`,
    // поэтому собираем их заранее во владеющий Vec.
    let entries: Vec<(&str, String)> = KEYWORDS
        .iter()
        .flat_map(|(variant, canonical, aliases)| {
            let value = format!("Token::{variant}");
            assert!(
                !canonical.contains('"') && !canonical.contains('\\'),
                "keyword spelling needs escaping: {canonical}"
            );
            for alias in *aliases {
                assert!(
                    !alias.contains('"') && !alias.contains('\\'),
                    "keyword spelling needs escaping: {alias}"
                );
            }
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
        "fn keyword_canonical(t: &Token) -> Option<&'static str> {{"
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
    write_operators(&out_dir);
    write_builtins(&out_dir);
    println!("cargo:rerun-if-changed=build.rs");
}
