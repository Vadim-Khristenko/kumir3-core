//! Statements of the Kumir 3 language
//!
//! [STABLE] Statements perform actions and do not return values directly.
//! Works with the unified type system `TypeKind` and `Value`.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                        Stmt (Statements)                        │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  Kumir 2: Assignment, If, Loop*, Input, Output, Assert, Return  │
//! │  Kumir 3: VarDecl, Import, Enum, Match, Pointer*, TryCatch      │
//! │  Kumir 3+: Ownership, Async, OOP, Generators, Channels          │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

use super::class::{ClassDef, ImplDef, InterfaceDef, TraitDef};
use super::expr::Expr;
use super::pattern::Pattern;
use super::value::{Ownership, TypeKind};

// =============================================================================
//         SECTION: HELPER STRUCTURES
// =============================================================================

/// Enum variant.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    /// Variant name
    pub name: String,
    /// Type of associated data (if any)
    pub data: Option<TypeKind>,
    /// Variant documentation
    pub doc: Option<String>,
}

/// Match expression arm.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    /// Pattern for matching
    pub pattern: Pattern,
    /// Additional condition (when ... if ...)
    pub guard: Option<Expr>,
    /// Arm body
    pub body: Vec<Stmt>,
}

/// [EXPERIMENTAL] Variable modifier for ownership semantics.
#[derive(Debug, Clone, PartialEq)]
pub struct VarModifiers {
    /// Mutability: mutable or not
    pub mutable: bool,
    /// Ownership model
    pub ownership: Ownership,
    /// Public variable (for modules)
    pub public: bool,
    /// Constant (immutable value)
    pub constant: bool,
}

/// Generator yield parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct YieldParam {
    /// Expression for yield
    pub value: Expr,
    /// Delegation to another generator (yield*)
    pub delegate: bool,
}

/// Channel parameter for send/receive.
#[derive(Debug, Clone, PartialEq)]
pub struct ChannelOp {
    /// Channel name
    pub channel: String,
    /// Value to send (None for receive)
    pub value: Option<Expr>,
    /// Timeout in milliseconds (0 = no timeout)
    pub timeout_ms: u64,
}

// =============================================================================
//         SECTION: STATEMENT ENUM
// =============================================================================

/// [STABLE] Statement (operator) of the Kumir language.
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    // =========================================================================
    //         KUMIR 2: BASIC STATEMENTS
    // =========================================================================

    // -------------------------------------------------------------------------
    // Assignment
    // -------------------------------------------------------------------------
    /// Simple assignment: x := expression
    Assignment(String, Expr),
    /// Array element assignment: arr[i, j] := expression
    ArrayAssignment(String, Vec<Expr>, Expr),

    // -------------------------------------------------------------------------
    // Conditional operators
    // -------------------------------------------------------------------------
    /// Conditional operator: if ... then ... [else ...] end
    If {
        condition: Expr,
        then_branch: Vec<Stmt>,
        else_branch: Option<Vec<Stmt>>,
    },

    // -------------------------------------------------------------------------
    // Loops
    // -------------------------------------------------------------------------
    /// Pre-condition loop: loop while condition ... end
    LoopWhile { condition: Expr, body: Vec<Stmt> },
    /// Counter loop: loop for i from a to b [step c] ... end
    LoopFor {
        variable: String,
        from: Expr,
        to: Expr,
        step: Option<Expr>,
        body: Vec<Stmt>,
    },
    /// Infinite loop: loop ... end
    LoopInfinite { body: Vec<Stmt> },
    /// Post-condition loop: loop ... end_while condition
    LoopDoWhile { body: Vec<Stmt>, condition: Expr },
    /// [KUMIR 3] Collection loop: loop for element in collection ... end
    LoopForEach {
        variable: String,
        var_type: Option<TypeKind>,
        iterable: Expr,
        body: Vec<Stmt>,
    },

    // -------------------------------------------------------------------------
    // Input/Output
    // -------------------------------------------------------------------------
    /// Input: input x, y, z
    Input(Vec<String>),
    /// Output: output a, b, c
    Output(Vec<Expr>),
    /// [KUMIR 3] Formatted output: output_f "format", args...
    OutputFormatted { format: String, args: Vec<Expr> },

    // -------------------------------------------------------------------------
    // Control flow
    // -------------------------------------------------------------------------
    /// Assertion: assert condition
    Assert(Expr),
    /// Expression as statement
    ExprStmt(Expr),
    /// Exit algorithm without value
    Return,
    /// Return value: return expression
    ReturnValue(Expr),
    /// Result assignment: result := expression
    ResultAssign(Expr),
    /// Exit loop: break
    Break,
    /// Next iteration: continue
    Continue,

    // =========================================================================
    //         KUMIR 3: EXTENDED STATEMENTS
    // =========================================================================

    // -------------------------------------------------------------------------
    // Variable declarations
    // -------------------------------------------------------------------------
    /// Declaration with automatic type inference: auto x := 42
    AutoVarDecl {
        name: String,
        init: Expr,
        modifiers: VarModifiers,
    },

    /// Declaration with explicit type: int x or int x, y, z
    VarDecl {
        type_kind: TypeKind,
        names: Vec<String>,
        init: Option<Expr>,
        modifiers: VarModifiers,
    },

    /// [EXPERIMENTAL] Destructuring: let (a, b) := pair
    Destructure {
        pattern: Pattern,
        value: Expr,
        modifiers: VarModifiers,
    },

    // -------------------------------------------------------------------------
    // Modules and import
    // -------------------------------------------------------------------------
    /// Module import: use "file.kum" [as Alias]
    Import {
        path: String,
        alias: Option<String>,
        /// Selective import: use module { func1, func2 }
        items: Option<Vec<String>>,
    },

    /// Module declaration
    ModuleDecl {
        name: String,
        body: Vec<Stmt>,
        algorithms: Vec<crate::types::Algorithm>,
        /// Public API
        exports: Vec<String>,
    },

    /// Export from module: export name1, name2
    Export { names: Vec<String> },

    // -------------------------------------------------------------------------
    // Enums and types
    // -------------------------------------------------------------------------
    /// Enum declaration
    EnumDecl {
        name: String,
        variants: Vec<EnumVariant>,
        /// Generic parameters: enum Result<T, E>
        generics: Vec<String>,
    },

    /// Type alias declaration: type MyInt = int
    TypeAlias { name: String, target: TypeKind },

    /// Pattern matching: match expression ...
    Match {
        expr: Expr,
        arms: Vec<MatchArm>,
        /// Exhaustiveness check (all variants covered)
        exhaustive: bool,
    },

    // -------------------------------------------------------------------------
    // Pointers and memory
    // -------------------------------------------------------------------------
    /// Pointer creation: new x := 42
    PointerNew {
        name: String,
        value: Expr,
        type_kind: Option<TypeKind>,
    },

    /// Memory deallocation: delete x
    PointerDelete { name: String },

    /// [EXPERIMENTAL] Pointer assignment: *ptr := value
    PointerAssign { pointer: Expr, value: Expr },

    // -------------------------------------------------------------------------
    // Error handling
    // -------------------------------------------------------------------------
    /// Try-catch-finally block: try ... catch ... finally ... end
    TryCatch {
        try_block: Vec<Stmt>,
        catch_var: Option<String>,
        catch_type: Option<TypeKind>,
        catch_block: Vec<Stmt>,
        finally_block: Option<Vec<Stmt>>,
    },

    /// Exception throwing: throw expression
    Throw(Expr),

    /// [EXPERIMENTAL] Error propagation: expression?
    Propagate(Expr),

    // -------------------------------------------------------------------------
    // Rust embeds
    // -------------------------------------------------------------------------
    /// Rust code block: rust_block_start ... rust_block_end
    RustBlock {
        code: String,
        captured_vars: Vec<String>,
        /// Return value type
        return_type: Option<TypeKind>,
    },

    // =========================================================================
    //         KUMIR 3+: EXPERIMENTAL / ASYNC / OOP
    // =========================================================================

    // -------------------------------------------------------------------------
    // Asynchronous programming
    // -------------------------------------------------------------------------
    /// Await asynchronous result: await expression
    Await(Expr),

    /// [EXPERIMENTAL] Spawn parallel task: task { ... }
    SpawnTask {
        body: Vec<Stmt>,
        /// Name for result
        result_var: Option<String>,
    },

    /// [EXPERIMENTAL] Await multiple Promises: await_all [p1, p2, p3]
    AwaitAll(Vec<Expr>),

    /// [EXPERIMENTAL] Await first of Promises: await_any [p1, p2, p3]
    AwaitAny(Vec<Expr>),

    // -------------------------------------------------------------------------
    // Generators
    // -------------------------------------------------------------------------
    /// [EXPERIMENTAL] Yield value from generator
    Yield(YieldParam),

    // -------------------------------------------------------------------------
    // Channels (concurrency)
    // -------------------------------------------------------------------------
    /// [EXPERIMENTAL] Send to channel: channel <- value
    ChannelSend(ChannelOp),

    /// [EXPERIMENTAL] Receive from channel: value := <- channel
    ChannelReceive {
        variable: String,
        channel: ChannelOp,
    },

    /// [EXPERIMENTAL] Select from multiple channels
    ChannelSelect {
        cases: Vec<(ChannelOp, Vec<Stmt>)>,
        default: Option<Vec<Stmt>>,
    },

    // -------------------------------------------------------------------------
    // Ownership / Borrowing
    // -------------------------------------------------------------------------
    /// [EXPERIMENTAL] Move: move x to y
    Move { from: String, to: String },

    /// [EXPERIMENTAL] Borrow: borrow x as y
    Borrow {
        source: String,
        target: String,
        mutable: bool,
    },

    /// [EXPERIMENTAL] Clone: clone x to y
    Clone { source: String, target: String },

    // -------------------------------------------------------------------------
    // Classes and OOP
    // -------------------------------------------------------------------------
    /// Class declaration (includes structs via ClassKind::Struct)
    ClassDecl(ClassDef),

    /// Struct declaration (simple data-only struct parsed by decl.rs).
    /// Uses `ClassDef` with `ClassKind::Struct` for consistency.
    StructDecl(ClassDef),

    /// Interface declaration — wraps the rich `InterfaceDef` type.
    InterfaceDecl(InterfaceDef),

    /// Trait declaration — wraps the rich `TraitDef` type.
    TraitDecl(TraitDef),

    /// Trait implementation or inherent impl block.
    ImplBlock(ImplDef),

    /// Field assignment: object.field := value
    FieldAssignment {
        object: Expr,
        field: String,
        value: Expr,
    },

    /// [EXPERIMENTAL] Method call as statement
    MethodCall {
        object: Expr,
        method: String,
        args: Vec<Expr>,
    },

    // -------------------------------------------------------------------------
    // Debugging and metaprogramming
    // -------------------------------------------------------------------------
    /// [EXPERIMENTAL] Debug output: debug expression
    Debug(Expr),

    /// [EXPERIMENTAL] Static assertion (compile-time)
    StaticAssert { condition: Expr, message: String },

    /// Pause execution until user presses Enter (debugger break)
    Pause,

    /// No-op statement
    Nop,

    /// Statement block (for grouping)
    Block(Vec<Stmt>),
}

// =============================================================================
//         SECTION: DEFAULT IMPLEMENTATIONS
// =============================================================================

impl Default for VarModifiers {
    fn default() -> Self {
        Self {
            mutable: false,
            ownership: Ownership::Owned,
            public: false,
            constant: false,
        }
    }
}

impl VarModifiers {
    /// Creates modifiers for a mutable variable
    pub fn mutable() -> Self {
        Self {
            mutable: true,
            ..Default::default()
        }
    }

    /// Creates modifiers for a constant
    pub fn constant() -> Self {
        Self {
            constant: true,
            ..Default::default()
        }
    }

    /// Creates modifiers for a public variable
    pub fn public() -> Self {
        Self {
            public: true,
            ..Default::default()
        }
    }
}

// =============================================================================
//         SECTION: STATEMENT CONSTRUCTORS
// =============================================================================

impl Stmt {
    /// Creates a simple assignment
    pub fn assign(name: impl Into<String>, value: Expr) -> Self {
        Stmt::Assignment(name.into(), value)
    }

    /// Creates a variable declaration with type
    pub fn var_decl(type_kind: TypeKind, name: impl Into<String>) -> Self {
        Stmt::VarDecl {
            type_kind,
            names: vec![name.into()],
            init: None,
            modifiers: VarModifiers::default(),
        }
    }

    /// Creates a declaration with initialization
    pub fn var_decl_init(type_kind: TypeKind, name: impl Into<String>, init: Expr) -> Self {
        Stmt::VarDecl {
            type_kind,
            names: vec![name.into()],
            init: Some(init),
            modifiers: VarModifiers::default(),
        }
    }

    /// Creates an auto-declaration
    pub fn auto_var(name: impl Into<String>, init: Expr) -> Self {
        Stmt::AutoVarDecl {
            name: name.into(),
            init,
            modifiers: VarModifiers::default(),
        }
    }

    /// Creates an if-else statement
    pub fn if_else(condition: Expr, then_branch: Vec<Stmt>, else_branch: Vec<Stmt>) -> Self {
        Stmt::If {
            condition,
            then_branch,
            else_branch: Some(else_branch),
        }
    }

    /// Creates a simple if without else
    pub fn if_then(condition: Expr, then_branch: Vec<Stmt>) -> Self {
        Stmt::If {
            condition,
            then_branch,
            else_branch: None,
        }
    }

    /// Creates a while loop
    pub fn while_loop(condition: Expr, body: Vec<Stmt>) -> Self {
        Stmt::LoopWhile { condition, body }
    }

    /// Creates a for loop
    pub fn for_loop(var: impl Into<String>, from: Expr, to: Expr, body: Vec<Stmt>) -> Self {
        Stmt::LoopFor {
            variable: var.into(),
            from,
            to,
            step: None,
            body,
        }
    }

    /// Creates a foreach loop
    pub fn foreach(var: impl Into<String>, iterable: Expr, body: Vec<Stmt>) -> Self {
        Stmt::LoopForEach {
            variable: var.into(),
            var_type: None,
            iterable,
            body,
        }
    }

    /// Creates a return with value
    pub fn return_value(value: Expr) -> Self {
        Stmt::ReturnValue(value)
    }

    /// Creates a try-catch block
    pub fn try_catch(
        try_block: Vec<Stmt>,
        catch_var: impl Into<String>,
        catch_block: Vec<Stmt>,
    ) -> Self {
        Stmt::TryCatch {
            try_block,
            catch_var: Some(catch_var.into()),
            catch_type: None,
            catch_block,
            finally_block: None,
        }
    }
}
