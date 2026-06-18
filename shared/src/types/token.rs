//! Lexical tokens for Kumir 2/3.
//!
//! Kumir 3 expands the classic Kumir surface with modules, pointers, enums
//! plus pattern matching, type inference, Rust embeds, lambdas, async flows,
//! and structured error handling.

// =============================================================================
//         SECTION: TYPES
// =============================================================================

/// [STABLE] Lexical token produced by the Kumir lexer (v2 and v3 dialects).
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // =============================================================================
    //         SECTION: ALGORITHM STRUCTURE (KUMIR 2)
    // =============================================================================
    Alg,    // alg      — algorithm start
    Begin,  // нач      — body start
    End,    // кон      — algorithm end
    Given,  // дано     — precondition
    Need,   // надо     — postcondition
    Arg,    // арг      — input argument
    Res,    // рез      — output result
    ArgRes, // аргрез   — input/output argument

    // =============================================================================
    //         SECTION: PRIMITIVE TYPES (KUMIR 2)
    // =============================================================================
    IntType,    // цел      — integer type
    FloatType,  // вещ      — floating-point type
    BoolType,   // лог      — boolean type
    CharType,   // сим      — character type
    StringType, // лит      — string literal type
    ArrayType,  // таб      — array/table type

    // =============================================================================
    //         SECTION: ADVANCED TYPES (KUMIR 3)
    // =============================================================================
    PointerType,  // указатель — pointer to a value/memory cell
    EnumType,     // перечисление — enum type declaration
    AutoType,     // авто     — compiler-driven type inference
    NoneType,     // пустота  — void/absence of value
    OptionalType, // необязательно — optional value container

    // =============================================================================
    //         SECTION: MUTABILITY & OWNERSHIP (KUMIR 3)
    // =============================================================================
    Mut,    // измен    — [EXPERIMENTAL] mutability marker for bindings/fields
    Const,  // конст    — [EXPERIMENTAL] constant/immutable marker
    Move,   // перемещение — [EXPERIMENTAL] move semantics and ownership transfer
    Borrow, // заимствовать — [EXPERIMENTAL] borrow reference without ownership transfer
    Clone,  // клонировать  — [EXPERIMENTAL] create deep copy of value
    Copy,   // копировать   — [EXPERIMENTAL] shallow copy for copyable types

    // =============================================================================
    //         SECTION: GENERICS & TYPE ALIASES (KUMIR 3)
    // =============================================================================
    Where,     // где      — [EXPERIMENTAL] generic constraint clause
    TypeAlias, // типалиас — [EXPERIMENTAL] type alias declaration

    // =============================================================================
    //         SECTION: LOGIC OPERATORS & CONSTANTS
    // =============================================================================
    And,   // и        — logical AND
    Or,    // или      — logical OR
    Not,   // не       — logical NOT
    True,  // да       — boolean true
    False, // нет      — boolean false

    // =============================================================================
    //         SECTION: CONTROL FLOW (KUMIR 2)
    // =============================================================================
    If,      // если     — conditional start
    Then,    // то       — then branch
    Else,    // иначе    — else branch
    Fi,      // все      — conditional end
    Switch,  // выбор    — switch statement
    Case,    // при      — switch branch
    Loop,    // нц       — loop start
    EndLoop, // кц       — loop end
    For,     // для      — for-loop keyword
    From,    // от       — from bound
    To,      // до       — to bound
    Step,    // шаг      — step value
    While,   // пока     — while loop

    // =============================================================================
    //         SECTION: IO & RUNTIME CONTROL (KUMIR 2)
    // =============================================================================
    Input,       // ввод     — input
    Output,      // вывод    — output
    Assert,      // утв      — assertion
    Pause,       // пауза    — pause execution
    Halt,        // выход    — terminate program
    Use,         // использовать — connect performer/driver
    Return,      // вернуть  — return value from algorithm
    ResultValue, // знач     — algorithm result accessor

    // =============================================================================
    //         SECTION: MODULE SYSTEM (KUMIR 3)
    // =============================================================================
    Import, // подключить   — import libraries and .kum files
    Module, // модуль       — module declaration
    Export, // экспорт      — module export control

    // =============================================================================
    //         SECTION: MEMORY & POINTERS (KUMIR 3)
    // =============================================================================
    New,    // новый        — allocate new object/pointer
    Delete, // удалить      — free allocated memory
    Ref,    // ссылка       — create reference to value
    Deref,  // разыменовать — dereference pointer

    // =============================================================================
    //         SECTION: ENUMS & PATTERN MATCHING (KUMIR 3)
    // =============================================================================
    EnumDecl, // перечисление — enum declaration
    Match,    // совпадение   — pattern matching construct
    When,     // когда        — [EXPERIMENTAL] guard/condition within match arms

    // =============================================================================
    //         SECTION: OBJECT MODEL (KUMIR 3)
    // =============================================================================
    Class,       // класс        — class declaration
    Struct,      // структура    — structure declaration
    Interface,   // интерфейс    — interface declaration
    Trait,       // свойство     — trait declaration
    Impl,        // реализация   — implementation block
    Self_,       // я / себя     — self reference
    This,        // это / this   — alternative self keyword
    Super,       // предок       — parent reference
    Constructor, // конструктор  — initializer method
    Destructor,  // деструктор   — finalizer method
    Public,      // открытый     — public visibility
    Private,     // закрытый     — private visibility
    Protected,   // защищённый   — protected visibility
    Static,      // статический  — static member
    Virtual,     // виртуальный  — virtual method
    Override,    // переопределить — method override
    Abstract,    // абстрактный  — abstract class/method
    Final,       // финальный    — final/non-overridable
    Extends,     // расширяет    — inheritance
    Implements,  // реализует    — interface implementation

    // =============================================================================
    //         SECTION: RUST EMBEDS (KUMIR 3)
    // =============================================================================
    RustBlockStart,     // РастВставкаНЦ — Rust block start
    RustBlockEnd,       // РастВставкаКЦ — Rust block end
    RustInline(String), // inline Rust snippet (single-line embed)
    RustCode,           // multi-line Rust block contents
    Rust,               // ржавчина   — short Rust keyword

    // =============================================================================
    //         SECTION: FUNCTIONAL PROGRAMMING (KUMIR 3)
    // =============================================================================
    Lambda,  // лямбда       — anonymous function
    Pipe,    // |>           — pipe operator
    Compose, // >>           — function composition

    // =============================================================================
    //         SECTION: ASYNC & CONCURRENCY (KUMIR 3)
    // =============================================================================
    Async, // асинх        — async algorithm
    Await, // ждать        — await result
    Spawn, // [EXPERIMENTAL] spawn concurrent task
    Yield, // [EXPERIMENTAL] cooperative yield/generator step

    // =============================================================================
    //         SECTION: ERROR HANDLING (KUMIR 3)
    // =============================================================================
    Try,     // попытка      — try block start
    Catch,   // перехват     — exception handler
    Throw,   // бросить      — raise exception
    Finally, // наконец      — cleanup/finally block

    // =============================================================================
    //         SECTION: RESOURCE GUARDING (KUMIR 3)
    // =============================================================================
    Defer, // [EXPERIMENTAL] defer execution until scope exit

    // =============================================================================
    //         SECTION: SPECIAL VALUES (KUMIR 3)
    // =============================================================================
    None,           // Пусто        — absence of value
    NotImplemented, // НеРеализовано — placeholder for missing impl
    NotAvailable,   // НеДоступно   — unavailable value
    Deprecated,     // Устарело     — deprecated/obsolete value

    // =============================================================================
    //         SECTION: IDENTIFIERS (KUMIR 3 — typed identifiers)
    // =============================================================================
    /// Namespace/module identifier (e.g., Математика::, Файлы::)
    NamespaceIdent(String),
    /// Class/struct identifier (e.g., HTTPКлиент, Точка)
    ClassIdent(String),
    /// Function/algorithm identifier (e.g., вычислить, сортировать)
    FuncIdent(String),
    /// Type identifier (e.g., цел, вещ, Список)
    TypeIdent(String),
    /// Variable/field identifier (e.g., x, счётчик, результат)
    VarIdent(String),
    /// Generic identifier when category is unknown at lex time
    Ident(String),

    // =============================================================================
    //         SECTION: LITERALS
    // =============================================================================
    /// Integer literal (e.g., 42, -100, 0xFF)
    IntLiteral(i64),
    /// Float literal (e.g., 3.14, -0.5, 1e10)
    FloatLiteral(f64),
    /// String literal (e.g., "привет", "hello world")
    StringLiteral(String),
    /// Character literal (e.g., 'а', 'x', '\n')
    CharLiteral(char),
    /// Raw string literal (e.g., r#"..."#)
    RawStringLiteral(String),
    /// Interpolated string start (e.g., f"Hello {name}")
    InterpolatedStringStart,
    /// Interpolated string part (text between interpolations)
    InterpolatedStringPart(String),
    /// Interpolated string end
    InterpolatedStringEnd,

    // =============================================================================
    //         SECTION: ARITHMETIC OPERATORS
    // =============================================================================
    Plus,    // +        — addition
    Minus,   // -        — subtraction
    Star,    // *        — multiplication
    Slash,   // /        — division
    Percent, // %        — modulo
    Power,   // **       — exponentiation

    // =============================================================================
    //         SECTION: COMPARISON OPERATORS
    // =============================================================================
    Equal,        // =        — equality
    NotEqual,     // <>       — inequality
    Less,         // <        — less than
    Greater,      // >        — greater than
    LessEqual,    // <=       — less than or equal
    GreaterEqual, // >=       — greater than or equal

    // =============================================================================
    //         SECTION: ASSIGNMENT OPERATORS
    // =============================================================================
    Assign,      // :=       — assignment
    PlusAssign,  // +=       — add-assign
    MinusAssign, // -=       — subtract-assign
    StarAssign,  // *=       — multiply-assign
    SlashAssign, // /=       — divide-assign

    // =============================================================================
    //         SECTION: DELIMITERS & BRACKETS
    // =============================================================================
    LParen,      // (        — left parenthesis
    RParen,      // )        — right parenthesis
    LBracket,    // [        — left bracket
    RBracket,    // ]        — right bracket
    LBrace,      // {        — left brace
    RBrace,      // }        — right brace
    Comma,       // ,        — comma
    Colon,       // :        — colon
    SemiColon,   // ;        — semicolon
    Dot,         // .        — dot
    DoubleDot,   // ..       — range operator (exclusive)
    DoubleDotEq, // ..=      — inclusive range operator
    Ellipsis,    // ...      — variadic/multidot

    // =============================================================================
    //         SECTION: SPECIAL OPERATORS (KUMIR 3)
    // =============================================================================
    Arrow,       // ->       — arrow (lambdas/pointers)
    FatArrow,    // =>       — fat arrow (match)
    DoubleColon, // ::       — module/enum access
    At,          // @        — decorator/annotation
    Ampersand,   // &        — reference sigil
    Caret,       // ^        — dereference sigil
    Question,    // ?        — early-return operator

    // =============================================================================
    //         SECTION: META & TRIVIA TOKENS
    // =============================================================================
    Newline,                 // significant newline
    Comment(String),         // comment payload
    DocComment(Vec<String>), // doc comment payload (///)
    EOF,                     // end of file
}
