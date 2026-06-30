//! Expressions of the Kumir language.
//!
//! [STABLE] Expressions compute values and can be arbitrarily nested.
//! This module covers Kumir 2 basics plus Kumir 3 extensions: OOP, modules,
//! pointers, functional constructs, async, and ownership semantics.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                        Expr (Expressions)                       │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  Kumir 2: Literal, Variable, BinaryOp, UnaryOp, Call, Array     │
//! │  Kumir 3: OOP, Modules, Enums, Pointers, Lambdas, Async         │
//! │  Kumir 3+: Ownership (Move, Borrow, Clone), Generators, Channels│
//! └─────────────────────────────────────────────────────────────────┘
//! ```

use super::pattern::Pattern;
use super::token::Token;
use super::value::{TypeKind, Value};

// =============================================================================
//         SECTION: EXPRESSION ENUM
// =============================================================================

/// [STABLE] Expression in the Kumir language.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    // =========================================================================
    //         KUMIR 2: BASIC EXPRESSIONS
    // =========================================================================

    // -------------------------------------------------------------------------
    // Literals and Variables
    // -------------------------------------------------------------------------
    /// Literal value (number, string, boolean, etc.)
    Literal(Value),
    /// Variable reference
    Variable(String),

    // -------------------------------------------------------------------------
    // Operators
    // -------------------------------------------------------------------------
    /// Binary operation: a + b, a * b, a и b
    BinaryOp(Box<Expr>, Token, Box<Expr>),
    /// Unary operation: -x, не x
    UnaryOp(Token, Box<Expr>),

    // -------------------------------------------------------------------------
    // Calls and Access
    // -------------------------------------------------------------------------
    /// Algorithm call: foo(x, y)
    Call(String, Vec<Expr>),
    /// Array element access: arr[i, j]
    ArrayAccess(String, Vec<Expr>),

    // =========================================================================
    //         KUMIR 3: OOP EXPRESSIONS
    // =========================================================================

    // -------------------------------------------------------------------------
    // Field and Method Access
    // -------------------------------------------------------------------------
    /// Field access: object.field
    FieldAccess(Box<Expr>, String),

    /// Method call: object.method(args)
    MethodCall {
        object: Box<Expr>,
        method: String,
        args: Vec<Expr>,
    },

    /// Class instantiation: new Class(args)
    ///
    /// Semantics of `new` keyword:
    /// - `new Class()` — creates a new instance
    /// - Multiple assignment `a, b := new X()` — each variable gets a separate object
    /// - Without `new`: `a, b := Class.Method()` — all variables reference the same object
    NewInstance { class_name: String, args: Vec<Expr> },

    // -------------------------------------------------------------------------
    // Self and Super References
    // -------------------------------------------------------------------------
    /// Self reference: я / self
    SelfRef,

    /// Super/parent reference: предок / super
    SuperRef,

    // -------------------------------------------------------------------------
    // Type Operations
    // -------------------------------------------------------------------------
    /// Type cast: x as Type
    Cast {
        expr: Box<Expr>,
        target_type: TypeKind,
    },

    /// Type check: x is Type
    TypeCheck {
        expr: Box<Expr>,
        check_type: TypeKind,
    },

    // =========================================================================
    //         KUMIR 3: MODULES AND ENUMS
    // =========================================================================
    /// Module access: Module::function
    ModuleAccess(String, String),

    /// Enum variant construction: Color::Red or Option::Some(42)
    EnumConstruct {
        enum_name: String,
        variant: String,
        data: Option<Box<Expr>>,
    },

    // =========================================================================
    //         KUMIR 3: POINTERS AND REFERENCES
    // =========================================================================
    /// Create reference: &x
    Ref(Box<Expr>),

    /// Dereference pointer: *ptr or ^ptr
    Deref(Box<Expr>),

    /// Allocate new pointer: new int(42)
    New(Box<Expr>),

    // =========================================================================
    //         KUMIR 3: FUNCTIONAL PROGRAMMING
    // =========================================================================
    /// Lambda expression: lambda(x, y) -> x + y
    Lambda {
        params: Vec<String>,
        param_types: Option<Vec<TypeKind>>,
        return_type: Option<TypeKind>,
        body: Box<Expr>,
    },

    /// Closure with captured variables
    Closure {
        params: Vec<String>,
        captures: Vec<String>,
        body: Box<Expr>,
    },

    /// Pipe expression: x |> f |> g (equivalent to g(f(x)))
    Pipe(Box<Expr>, Box<Expr>),

    /// Function composition: f >> g (equivalent to |x| g(f(x)))
    Compose(Box<Expr>, Box<Expr>),

    /// Partial application: f(_, 2) creates a function waiting for first arg
    PartialApp {
        func: Box<Expr>,
        args: Vec<Option<Expr>>,
    },

    // =========================================================================
    //         KUMIR 3: CONDITIONAL EXPRESSIONS
    // =========================================================================
    /// Ternary operator: if x > 0 then x else -x end
    IfExpr {
        condition: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
    },

    /// Match expression (returns value)
    MatchExpr {
        expr: Box<Expr>,
        arms: Vec<(Pattern, Expr)>,
    },

    // =========================================================================
    //         KUMIR 3+: ASYNC & CONCURRENCY
    // =========================================================================
    /// [EXPERIMENTAL] Await async expression: await promise
    Await(Box<Expr>),

    /// [EXPERIMENTAL] Spawn async task: spawn { expr }
    Spawn(Box<Expr>),

    /// [EXPERIMENTAL] Generator yield expression
    YieldExpr(Box<Expr>),

    /// [EXPERIMENTAL] Channel send: channel <- value
    ChannelSend {
        channel: Box<Expr>,
        value: Box<Expr>,
    },

    /// [EXPERIMENTAL] Channel receive: <- channel
    ChannelReceive(Box<Expr>),

    // =========================================================================
    //         KUMIR 3+: OWNERSHIP EXPRESSIONS
    // =========================================================================
    /// [EXPERIMENTAL] Move value: move x
    Move(Box<Expr>),

    /// [EXPERIMENTAL] Borrow value: borrow x or borrow mut x
    Borrow { expr: Box<Expr>, mutable: bool },

    /// [EXPERIMENTAL] Clone value: clone x
    Clone(Box<Expr>),

    /// [EXPERIMENTAL] Copy value: copy x
    Copy(Box<Expr>),

    // =========================================================================
    //         KUMIR 3: RUST EMBEDS
    // =========================================================================
    /// Inline Rust expression returning a value
    RustExpr(String),

    // =========================================================================
    //         KUMIR 3: SPECIAL EXPRESSIONS
    // =========================================================================
    /// None / null — absence of value
    None,

    /// Not implemented placeholder
    /// Evaluates to runtime error: "Not yet implemented: {name}"
    NotImplemented(Option<String>),

    /// Not available placeholder
    /// Evaluates to runtime error: "Value not available: {name}"
    NotAvailable(Option<String>),

    /// Deprecated placeholder
    /// Evaluates to runtime warning: "Deprecated value used: {name}"
    Deprecated(Option<String>),

    /// Tuple expression: (a, b, c)
    TupleExpr(Vec<Expr>),

    /// Range expression: start..end or start..=end, optionally with step.
    Range {
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
        inclusive: bool,
        step: Option<Box<Expr>>,
    },

    /// Type expression (for reflection): typeof x
    TypeOf(Box<Expr>),

    /// Block expression with statements and final expression
    Block {
        stmts: Vec<super::stmt::Stmt>,
        expr: Option<Box<Expr>>,
    },
}
