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
    ("AutoType", "авто", &["auto", "var", "пусть"]),
    ("AnyType", "любой", &[]),
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
    // Арифметический оператор остатка как ключевое слово (как и/или/не).
    // Лексится в Token::Percent, далее обрабатывается общей логикой (modulus).
    ("Percent", "мод", &[]),
    // Оператор целочисленного деления как ключевое слово (как и `мод`).
    // Лексится в Token::IntDiv, далее обрабатывается общей логикой (int_div).
    // Только русское `див`: английское `div` — имя встроенной функции (не трогаем).
    ("IntDiv", "див", &[]),
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
    (
        "None",
        "Пусто",
        &["пусто", "ничего", "None", "none", "nil", "null"],
    ),
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
//
// ИНВАРИАНТ (проверяется тестами в interpreter/src/interpreter/builtins/tests.rs):
//   * каждое имя таблицы РЕАЛЬНО диспетчеризуется `Builtins::try_call`;
//   * каждое имя, диспетчеризуемое интерпретатором и НЕ являющееся ключевым
//     словом языка, присутствует в таблице.
// Имена, совпадающие с ключевыми словами (`int`, `print`, `цел`, `mod`, `таб`,
// `ввод`, `утв` …), в таблицу НЕ входят: лексер превращает их в keyword-токен,
// поэтому `имя(x)` никогда не доходит до диспетчера встроенных функций.
// Категория = модуль реализации: math.rs -> Math, string.rs -> String,
// io.rs -> Io, array.rs / types.rs -> Other.
const BUILTINS: &[(&str, &str)] = &[
    // ----- Math -----
    ("abs", "Math"),
    ("sqrt", "Math"),
    ("корень", "Math"),
    ("квадратный_корень", "Math"),
    ("sin", "Math"),
    ("cos", "Math"),
    ("tan", "Math"),
    ("tg", "Math"),
    ("asin", "Math"),
    ("arcsin", "Math"),
    ("acos", "Math"),
    ("arccos", "Math"),
    ("atan", "Math"),
    ("arctg", "Math"),
    ("arctan", "Math"),
    ("ctg", "Math"),
    ("cot", "Math"),
    ("arcctg", "Math"),
    ("arccot", "Math"),
    ("sh", "Math"),
    ("sinh", "Math"),
    ("ch", "Math"),
    ("cosh", "Math"),
    ("th", "Math"),
    ("tanh", "Math"),
    ("atan2", "Math"),
    ("arctg2", "Math"),
    ("ln", "Math"),
    ("log", "Math"),
    ("log10", "Math"),
    ("lg", "Math"),
    ("exp", "Math"),
    ("pow", "Math"),
    ("степень", "Math"),
    ("floor", "Math"),
    ("пол", "Math"),
    ("ceil", "Math"),
    ("потолок", "Math"),
    ("цел_часть", "Math"),
    ("trunc", "Math"),
    ("frac", "Math"),
    ("дробь", "Math"),
    ("div", "Math"),
    ("цел_деление", "Math"),
    ("остаток", "Math"),
    ("rem", "Math"),
    ("round", "Math"),
    ("округлить", "Math"),
    ("округл", "Math"),
    ("min", "Math"),
    ("минимум", "Math"),
    ("мин", "Math"),
    ("max", "Math"),
    ("максимум", "Math"),
    ("макс", "Math"),
    ("sign", "Math"),
    ("знак", "Math"),
    ("sgn", "Math"),
    ("пи", "Math"),
    ("pi", "Math"),
    ("е", "Math"),
    ("e", "Math"),
    ("случайное", "Math"),
    ("случ", "Math"),
    ("random", "Math"),
    ("rand", "Math"),
    // ----- Functional (higher-order over tables) -----
    ("отобразить", "Other"),
    ("map", "Other"),
    ("отобрать", "Other"),
    ("отфильтровать", "Other"),
    ("filter", "Other"),
    ("свернуть", "Other"),
    ("fold", "Other"),
    ("reduce", "Other"),
    ("любой_из", "Other"),
    ("any", "Other"),
    ("все_из", "Other"),
    ("all", "Other"),
    ("найти_первый", "Other"),
    ("find_first", "Other"),
    ("количество", "Other"),
    ("count", "Other"),
    ("сортировать_по", "Other"),
    ("sort_by", "Other"),
    // ----- String -----
    ("длина", "String"),
    ("длин", "String"),
    ("len", "String"),
    ("length", "String"),
    ("размер", "String"),
    ("size", "String"),
    ("байты", "String"),
    ("bytes", "String"),
    ("строка_из_байт", "String"),
    ("байты_в_строку", "String"),
    ("bytes_to_string", "String"),
    ("символ", "String"),
    ("chr", "String"),
    ("код", "String"),
    ("ord", "String"),
    ("code", "String"),
    ("подстрока", "String"),
    ("substring", "String"),
    ("substr", "String"),
    ("копировать_строку", "String"),
    ("вырезка", "String"),
    ("позиция", "String"),
    ("position", "String"),
    ("pos", "String"),
    ("найти", "String"),
    ("find", "String"),
    ("верхний_регистр", "String"),
    ("to_upper", "String"),
    ("upper", "String"),
    ("верхний", "String"),
    ("uppercase", "String"),
    ("нижний_регистр", "String"),
    ("to_lower", "String"),
    ("lower", "String"),
    ("нижний", "String"),
    ("lowercase", "String"),
    ("слева", "String"),
    ("left", "String"),
    ("справа", "String"),
    ("right", "String"),
    ("повторить", "String"),
    ("repeat", "String"),
    ("обрезать", "String"),
    ("trim", "String"),
    ("заменить", "String"),
    ("replace", "String"),
    ("разделить", "String"),
    ("split", "String"),
    ("разбить", "String"),
    ("соединить", "String"),
    ("join", "String"),
    // ----- Io -----
    ("печать", "Io"),
    ("печатьстр", "Io"),
    ("println", "Io"),
    ("вывод_строки", "Io"),
    ("нс", "Io"),
    ("newline", "Io"),
    ("nl", "Io"),
    ("новая_строка", "Io"),
    ("время", "Io"),
    ("time", "Io"),
    ("sleep", "Io"),
    // ----- Other -----
    ("массив", "Other"),
    ("добавить", "Other"),
    ("push", "Other"),
    ("append", "Other"),
    ("удалить_последний", "Other"),
    ("pop", "Other"),
    ("первый", "Other"),
    ("first", "Other"),
    ("head", "Other"),
    ("последний", "Other"),
    ("last", "Other"),
    ("сумма", "Other"),
    ("sum", "Other"),
    ("среднее", "Other"),
    ("avg", "Other"),
    ("average", "Other"),
    ("обратить", "Other"),
    ("reverse", "Other"),
    ("перевернуть", "Other"),
    ("сортировать", "Other"),
    ("sort", "Other"),
    ("содержит", "Other"),
    ("contains", "Other"),
    ("индекс", "Other"),
    ("index_of", "Other"),
    ("empty", "Other"),
    ("is_empty", "Other"),
    ("целое", "Other"),
    ("to_int", "Other"),
    ("вещественное", "Other"),
    ("to_float", "Other"),
    ("строка", "Other"),
    ("to_string", "Other"),
    ("логическое", "Other"),
    ("to_bool", "Other"),
    ("это_число", "Other"),
    ("is_number", "Other"),
    ("это_строка", "Other"),
    ("is_string", "Other"),
    ("это_массив", "Other"),
    ("is_array", "Other"),
    ("это_пусто", "Other"),
    ("is_null", "Other"),
    ("тип", "Other"),
    ("typeof", "Other"),
    ("type_of", "Other"),
    ("ошибка", "Other"),
    ("error", "Other"),
    ("пара", "Other"),
    ("pair", "Other"),
    ("тройка", "Other"),
    ("triple", "Other"),
    ("кортеж", "Other"),
    ("tuple", "Other"),
    ("некоторое", "Other"),
    ("some", "Other"),
    ("есть", "Other"),
    ("is_some", "Other"),
    ("извлечь", "Other"),
    ("unwrap", "Other"),
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
    ("??", "QuestionQuestion"),
    ("?.", "QuestionDot"),
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
