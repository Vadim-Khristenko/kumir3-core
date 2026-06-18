//! Kumir 3 Keywords
//!
//! Complete keyword table mapping Russian/English keywords to Token variants.
//! Supports both Kumir 2 legacy and Kumir 3 modern syntax.

use once_cell::sync::Lazy;
use std::collections::HashMap;

use crate::types::Token;

// =============================================================================
//         SECTION: KEYWORDS TABLE
// =============================================================================

/// Complete keyword table for Kumir 2/3.
pub static KEYWORDS: Lazy<HashMap<&'static str, Token>> = Lazy::new(|| {
    let mut m = HashMap::with_capacity(300);

    // =========================================================================
    //         ALGORITHM STRUCTURE (KUMIR 2)
    // =========================================================================
    m.insert("алг", Token::Alg);
    m.insert("alg", Token::Alg);
    m.insert("algorithm", Token::Alg);
    m.insert("нач", Token::Begin);
    m.insert("begin", Token::Begin);
    m.insert("кон", Token::End);
    m.insert("end", Token::End);
    m.insert("дано", Token::Given);
    m.insert("given", Token::Given);
    m.insert("надо", Token::Need);
    m.insert("need", Token::Need);
    m.insert("арг", Token::Arg);
    m.insert("arg", Token::Arg);
    m.insert("рез", Token::Res);
    m.insert("res", Token::Res);
    m.insert("аргрез", Token::ArgRes);
    m.insert("argres", Token::ArgRes);

    // =========================================================================
    //         PRIMITIVE TYPES (KUMIR 2)
    // =========================================================================
    m.insert("цел", Token::IntType);
    m.insert("int", Token::IntType);
    m.insert("integer", Token::IntType);
    m.insert("вещ", Token::FloatType);
    m.insert("float", Token::FloatType);
    m.insert("real", Token::FloatType);
    m.insert("лог", Token::BoolType);
    m.insert("bool", Token::BoolType);
    m.insert("boolean", Token::BoolType);
    m.insert("сим", Token::CharType);
    m.insert("char", Token::CharType);
    m.insert("лит", Token::StringType);
    m.insert("string", Token::StringType);
    m.insert("str", Token::StringType);
    m.insert("таб", Token::ArrayType);
    m.insert("array", Token::ArrayType);
    m.insert("tab", Token::ArrayType);

    // =========================================================================
    //         ADVANCED TYPES (KUMIR 3)
    // =========================================================================
    m.insert("указатель", Token::PointerType);
    m.insert("pointer", Token::PointerType);
    m.insert("ptr", Token::PointerType);
    m.insert("перечисление", Token::EnumType);
    m.insert("enum", Token::EnumType);
    m.insert("авто", Token::AutoType);
    m.insert("auto", Token::AutoType);
    m.insert("var", Token::AutoType);
    m.insert("пустота", Token::NoneType);
    m.insert("void", Token::NoneType);
    m.insert("unit", Token::NoneType);
    m.insert("необязательно", Token::OptionalType);
    m.insert("Необязательно", Token::OptionalType);
    m.insert("optional", Token::OptionalType);
    m.insert("Optional", Token::OptionalType);
    m.insert("может", Token::OptionalType);

    // =========================================================================
    //         MUTABILITY & OWNERSHIP (KUMIR 3)
    // =========================================================================
    m.insert("измен", Token::Mut);
    m.insert("изменяемый", Token::Mut);
    m.insert("mut", Token::Mut);
    m.insert("mutable", Token::Mut);
    m.insert("конст", Token::Const);
    m.insert("константа", Token::Const);
    m.insert("const", Token::Const);
    m.insert("constant", Token::Const);
    m.insert("перемещение", Token::Move);
    m.insert("переместить", Token::Move);
    m.insert("move", Token::Move);
    m.insert("заимствовать", Token::Borrow);
    m.insert("заимств", Token::Borrow);
    m.insert("borrow", Token::Borrow);
    m.insert("клонировать", Token::Clone);
    m.insert("клон", Token::Clone);
    m.insert("clone", Token::Clone);
    m.insert("копировать", Token::Copy);
    m.insert("копия", Token::Copy);
    m.insert("copy", Token::Copy);

    // =========================================================================
    //         GENERICS & TYPE ALIASES (KUMIR 3)
    // =========================================================================
    m.insert("где", Token::Where);
    m.insert("where", Token::Where);
    m.insert("типалиас", Token::TypeAlias);
    m.insert("type", Token::TypeAlias);
    m.insert("typedef", Token::TypeAlias);

    // =========================================================================
    //         LOGIC OPERATORS & CONSTANTS
    // =========================================================================
    m.insert("и", Token::And);
    m.insert("and", Token::And);
    m.insert("или", Token::Or);
    m.insert("or", Token::Or);
    m.insert("не", Token::Not);
    m.insert("not", Token::Not);
    m.insert("да", Token::True);
    m.insert("true", Token::True);
    m.insert("истина", Token::True);
    m.insert("нет", Token::False);
    m.insert("false", Token::False);
    m.insert("ложь", Token::False);

    // =========================================================================
    //         CONTROL FLOW (KUMIR 2)
    // =========================================================================
    m.insert("если", Token::If);
    m.insert("if", Token::If);
    m.insert("то", Token::Then);
    m.insert("then", Token::Then);
    m.insert("иначе", Token::Else);
    m.insert("else", Token::Else);
    m.insert("все", Token::Fi);
    m.insert("всё", Token::Fi);
    m.insert("fi", Token::Fi);
    m.insert("endif", Token::Fi);
    m.insert("выбор", Token::Switch);
    m.insert("switch", Token::Switch);
    m.insert("при", Token::Case);
    m.insert("case", Token::Case);
    m.insert("нц", Token::Loop);
    m.insert("loop", Token::Loop);
    m.insert("кц", Token::EndLoop);
    m.insert("endloop", Token::EndLoop);
    m.insert("для", Token::For);
    m.insert("for", Token::For);
    m.insert("от", Token::From);
    m.insert("from", Token::From);
    m.insert("до", Token::To);
    m.insert("to", Token::To);
    m.insert("шаг", Token::Step);
    m.insert("step", Token::Step);
    m.insert("пока", Token::While);
    m.insert("while", Token::While);

    // =========================================================================
    //         IO & RUNTIME CONTROL (KUMIR 2)
    // =========================================================================
    m.insert("ввод", Token::Input);
    m.insert("input", Token::Input);
    m.insert("read", Token::Input);
    m.insert("вывод", Token::Output);
    m.insert("output", Token::Output);
    m.insert("print", Token::Output);
    m.insert("write", Token::Output);
    m.insert("утв", Token::Assert);
    m.insert("assert", Token::Assert);
    m.insert("пауза", Token::Pause);
    m.insert("pause", Token::Pause);
    m.insert("выход", Token::Halt);
    m.insert("halt", Token::Halt);
    m.insert("exit", Token::Halt);
    m.insert("использовать", Token::Use);
    m.insert("use", Token::Use);
    m.insert("вернуть", Token::Return);
    m.insert("return", Token::Return);
    m.insert("знач", Token::ResultValue);
    m.insert("result", Token::ResultValue);

    // =========================================================================
    //         MODULE SYSTEM (KUMIR 3)
    // =========================================================================
    m.insert("подключить", Token::Import);
    m.insert("import", Token::Import);
    m.insert("include", Token::Import);
    m.insert("модуль", Token::Module);
    m.insert("module", Token::Module);
    m.insert("mod", Token::Module);
    m.insert("экспорт", Token::Export);
    m.insert("export", Token::Export);
    m.insert("pub", Token::Export);

    // =========================================================================
    //         MEMORY & POINTERS (KUMIR 3)
    // =========================================================================
    m.insert("новый", Token::New);
    m.insert("new", Token::New);
    m.insert("создать", Token::New);
    m.insert("удалить", Token::Delete);
    m.insert("delete", Token::Delete);
    m.insert("free", Token::Delete);
    m.insert("ссылка", Token::Ref);
    m.insert("ref", Token::Ref);
    m.insert("разыменовать", Token::Deref);
    m.insert("deref", Token::Deref);

    // =========================================================================
    //         ENUMS & PATTERN MATCHING (KUMIR 3)
    // =========================================================================
    m.insert("объявить_перечисление", Token::EnumDecl);
    m.insert("enum_decl", Token::EnumDecl);
    m.insert("совпадение", Token::Match);
    m.insert("match", Token::Match);
    m.insert("когда", Token::When);
    m.insert("when", Token::When);
    m.insert("guard", Token::When);

    // =========================================================================
    //         OBJECT MODEL (KUMIR 3)
    // =========================================================================
    // class
    m.insert("класс", Token::Class);
    m.insert("Класс", Token::Class);
    m.insert("class", Token::Class);
    m.insert("Class", Token::Class);

    // struct
    m.insert("структура", Token::Struct);
    m.insert("Структура", Token::Struct);
    m.insert("struct", Token::Struct);
    m.insert("Struct", Token::Struct);
    m.insert("запись", Token::Struct);

    // interface
    m.insert("интерфейс", Token::Interface);
    m.insert("Интерфейс", Token::Interface);
    m.insert("interface", Token::Interface);
    m.insert("Interface", Token::Interface);

    // trait
    m.insert("свойство", Token::Trait);
    m.insert("Свойство", Token::Trait);
    m.insert("trait", Token::Trait);
    m.insert("Trait", Token::Trait);
    m.insert("типаж", Token::Trait);

    // impl
    m.insert("реализация", Token::Impl);
    m.insert("Реализация", Token::Impl);
    m.insert("impl", Token::Impl);
    m.insert("Impl", Token::Impl);

    // self
    m.insert("я", Token::Self_);
    m.insert("себя", Token::Self_);
    m.insert("self", Token::Self_);
    m.insert("Self", Token::Self_);

    // this
    m.insert("это", Token::This);
    m.insert("this", Token::This);

    // super
    m.insert("предок", Token::Super);
    m.insert("родитель", Token::Super);
    m.insert("супер", Token::Super);
    m.insert("super", Token::Super);

    // constructor
    m.insert("конструктор", Token::Constructor);
    m.insert("констр", Token::Constructor);
    m.insert("constructor", Token::Constructor);
    m.insert("init", Token::Constructor);

    // destructor
    m.insert("деструктор", Token::Destructor);
    m.insert("дестр", Token::Destructor);
    m.insert("destructor", Token::Destructor);
    m.insert("deinit", Token::Destructor);
    m.insert("drop", Token::Destructor);

    // visibility
    m.insert("открытый", Token::Public);
    m.insert("публичное", Token::Public);
    m.insert("публ", Token::Public);
    m.insert("public", Token::Public);

    m.insert("закрытый", Token::Private);
    m.insert("приватное", Token::Private);
    m.insert("приват", Token::Private);
    m.insert("private", Token::Private);

    m.insert("защищённый", Token::Protected);
    m.insert("защищённое", Token::Protected);
    m.insert("защищ", Token::Protected);
    m.insert("protected", Token::Protected);

    // modifiers
    m.insert("статический", Token::Static);
    m.insert("статическое", Token::Static);
    m.insert("стат", Token::Static);
    m.insert("static", Token::Static);

    m.insert("виртуальный", Token::Virtual);
    m.insert("виртуальное", Token::Virtual);
    m.insert("вирт", Token::Virtual);
    m.insert("virtual", Token::Virtual);

    m.insert("переопределить", Token::Override);
    m.insert("переопр", Token::Override);
    m.insert("override", Token::Override);

    m.insert("абстрактный", Token::Abstract);
    m.insert("абстрактное", Token::Abstract);
    m.insert("абстр", Token::Abstract);
    m.insert("abstract", Token::Abstract);

    m.insert("финальный", Token::Final);
    m.insert("финал", Token::Final);
    m.insert("final", Token::Final);
    m.insert("sealed", Token::Final);

    // inheritance
    m.insert("расширяет", Token::Extends);
    m.insert("extends", Token::Extends);
    m.insert("наследует", Token::Extends);

    m.insert("реализует", Token::Implements);
    m.insert("implements", Token::Implements);

    // =========================================================================
    //         RUST EMBEDS (KUMIR 3)
    // =========================================================================
    m.insert("РастВставкаНЦ", Token::RustBlockStart);
    m.insert("РастВставкаКЦ", Token::RustBlockEnd);
    m.insert("ржавчина", Token::Rust);
    m.insert("Ржавчина", Token::Rust);
    m.insert("rust", Token::Rust);

    // =========================================================================
    //         FUNCTIONAL PROGRAMMING (KUMIR 3)
    // =========================================================================
    m.insert("лямбда", Token::Lambda);
    m.insert("lambda", Token::Lambda);
    m.insert("fn", Token::Lambda);
    m.insert("func", Token::Lambda);

    // =========================================================================
    //         ASYNC & CONCURRENCY (KUMIR 3)
    // =========================================================================
    m.insert("асинх", Token::Async);
    m.insert("async", Token::Async);
    m.insert("асинхронный", Token::Async);
    m.insert("ждать", Token::Await);
    m.insert("await", Token::Await);
    m.insert("ожидать", Token::Await);
    m.insert("запустить", Token::Spawn);
    m.insert("spawn", Token::Spawn);
    m.insert("уступить", Token::Yield);
    m.insert("yield", Token::Yield);

    // =========================================================================
    //         ERROR HANDLING (KUMIR 3)
    // =========================================================================
    m.insert("попытка", Token::Try);
    m.insert("try", Token::Try);
    m.insert("перехват", Token::Catch);
    m.insert("catch", Token::Catch);
    m.insert("except", Token::Catch);
    m.insert("бросить", Token::Throw);
    m.insert("throw", Token::Throw);
    m.insert("raise", Token::Throw);
    m.insert("наконец", Token::Finally);
    m.insert("finally", Token::Finally);

    // =========================================================================
    //         RESOURCE GUARDING (KUMIR 3)
    // =========================================================================
    m.insert("отложить", Token::Defer);
    m.insert("defer", Token::Defer);

    // =========================================================================
    //         SPECIAL VALUES (KUMIR 3)
    // =========================================================================
    m.insert("Пусто", Token::None);
    m.insert("пусто", Token::None);
    m.insert("None", Token::None);
    m.insert("none", Token::None);
    m.insert("nil", Token::None);
    m.insert("null", Token::None);

    m.insert("НеРеализовано", Token::NotImplemented);
    m.insert("не_реализовано", Token::NotImplemented);
    m.insert("NotImplemented", Token::NotImplemented);
    m.insert("not_implemented", Token::NotImplemented);
    m.insert("todo", Token::NotImplemented);
    m.insert("TODO", Token::NotImplemented);

    m.insert("НеДоступно", Token::NotAvailable);
    m.insert("не_доступно", Token::NotAvailable);
    m.insert("NotAvailable", Token::NotAvailable);
    m.insert("not_available", Token::NotAvailable);
    m.insert("unavailable", Token::NotAvailable);

    m.insert("Устарело", Token::Deprecated);
    m.insert("устарело", Token::Deprecated);
    m.insert("Deprecated", Token::Deprecated);
    m.insert("deprecated", Token::Deprecated);

    m
});

// =============================================================================
//         SECTION: HELPER FUNCTIONS
// =============================================================================

/// Checks if a string is a keyword.
#[inline]
pub fn is_keyword(s: &str) -> bool {
    KEYWORDS.contains_key(s)
}

/// Returns the token for a keyword if it exists.
#[inline]
pub fn get_keyword_token(s: &str) -> Option<Token> {
    KEYWORDS.get(s).cloned()
}

/// Returns all keywords as a sorted vector (for documentation/debugging).
pub fn all_keywords() -> Vec<&'static str> {
    let mut keys: Vec<_> = KEYWORDS.keys().copied().collect();
    keys.sort();
    keys
}

/// Returns keywords grouped by token type.
pub fn keywords_by_token() -> HashMap<String, Vec<&'static str>> {
    let mut result: HashMap<String, Vec<&'static str>> = HashMap::new();
    for (kw, token) in KEYWORDS.iter() {
        let token_name = format!("{:?}", token);
        let token_name = token_name
            .split('(')
            .next()
            .unwrap_or(&token_name)
            .to_string();
        result.entry(token_name).or_default().push(kw);
    }
    for v in result.values_mut() {
        v.sort();
    }
    result
}
