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

use std::collections::HashSet;

use super::pattern::Pattern;
use super::stmt::Stmt;
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
    /// Null-coalescing operator: a ?? b
    Coalesce(Box<Expr>, Box<Expr>),

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

// =============================================================================
//         SECTION: FREE VARIABLE ANALYSIS
// =============================================================================

impl Expr {
    /// Returns the set of variable names referenced in this expression that are
    /// not in the supplied bound set (parameters, locals, etc.).
    pub fn free_vars(&self, bound: &HashSet<String>) -> HashSet<String> {
        let mut result = HashSet::new();
        match self {
            Expr::Literal(_) => {}
            Expr::Variable(name) => {
                if !bound.contains(name) {
                    result.insert(name.clone());
                }
            }
            Expr::BinaryOp(left, _, right) => {
                result.extend(left.free_vars(bound));
                result.extend(right.free_vars(bound));
            }
            Expr::UnaryOp(_, operand) => {
                result.extend(operand.free_vars(bound));
            }
            Expr::Call(name, args) => {
                if !bound.contains(name) {
                    result.insert(name.clone());
                }
                for arg in args {
                    result.extend(arg.free_vars(bound));
                }
            }
            Expr::ArrayAccess(name, indices) => {
                if !bound.contains(name) {
                    result.insert(name.clone());
                }
                for idx in indices {
                    result.extend(idx.free_vars(bound));
                }
            }
            Expr::FieldAccess(object, _) => {
                result.extend(object.free_vars(bound));
            }
            Expr::MethodCall { object, args, .. } => {
                result.extend(object.free_vars(bound));
                for arg in args {
                    result.extend(arg.free_vars(bound));
                }
            }
            Expr::NewInstance { args, .. } => {
                for arg in args {
                    result.extend(arg.free_vars(bound));
                }
            }
            Expr::SelfRef | Expr::SuperRef => {}
            Expr::Cast { expr, .. } => result.extend(expr.free_vars(bound)),
            Expr::TypeCheck { expr, .. } => result.extend(expr.free_vars(bound)),
            Expr::ModuleAccess(_, _) => {}
            Expr::EnumConstruct { data, .. } => {
                if let Some(d) = data {
                    result.extend(d.free_vars(bound));
                }
            }
            Expr::Ref(inner) => result.extend(inner.free_vars(bound)),
            Expr::Deref(inner) => result.extend(inner.free_vars(bound)),
            Expr::New(inner) => result.extend(inner.free_vars(bound)),
            Expr::Lambda { params, body, .. } => {
                let mut lambda_bound = bound.clone();
                for p in params {
                    lambda_bound.insert(p.clone());
                }
                result.extend(body.free_vars(&lambda_bound));
            }
            Expr::Closure {
                params,
                captures,
                body,
            } => {
                let mut closure_bound = bound.clone();
                for p in params {
                    closure_bound.insert(p.clone());
                }
                for c in captures {
                    closure_bound.insert(c.clone());
                }
                result.extend(body.free_vars(&closure_bound));
            }
            Expr::Pipe(left, right) => {
                result.extend(left.free_vars(bound));
                result.extend(right.free_vars(bound));
            }
            Expr::Compose(left, right) => {
                result.extend(left.free_vars(bound));
                result.extend(right.free_vars(bound));
            }
            Expr::PartialApp { func, args } => {
                result.extend(func.free_vars(bound));
                for a in args.iter().flatten() {
                    result.extend(a.free_vars(bound));
                }
            }
            Expr::Coalesce(left, right) => {
                result.extend(left.free_vars(bound));
                result.extend(right.free_vars(bound));
            }
            Expr::IfExpr {
                condition,
                then_expr,
                else_expr,
            } => {
                result.extend(condition.free_vars(bound));
                result.extend(then_expr.free_vars(bound));
                result.extend(else_expr.free_vars(bound));
            }
            Expr::MatchExpr { expr, arms } => {
                result.extend(expr.free_vars(bound));
                for (pattern, arm_expr) in arms {
                    let mut arm_bound = bound.clone();
                    arm_bound.extend(pattern.bindings());
                    result.extend(pattern_free_vars(pattern, bound));
                    result.extend(arm_expr.free_vars(&arm_bound));
                }
            }
            Expr::Await(inner) => result.extend(inner.free_vars(bound)),
            Expr::Spawn(inner) => result.extend(inner.free_vars(bound)),
            Expr::YieldExpr(inner) => result.extend(inner.free_vars(bound)),
            Expr::ChannelSend { channel, value } => {
                result.extend(channel.free_vars(bound));
                result.extend(value.free_vars(bound));
            }
            Expr::ChannelReceive(inner) => result.extend(inner.free_vars(bound)),
            Expr::Move(inner) => result.extend(inner.free_vars(bound)),
            Expr::Borrow { expr, .. } => result.extend(expr.free_vars(bound)),
            Expr::Clone(inner) => result.extend(inner.free_vars(bound)),
            Expr::Copy(inner) => result.extend(inner.free_vars(bound)),
            Expr::RustExpr(_) => {}
            Expr::None => {}
            Expr::NotImplemented(_) => {}
            Expr::NotAvailable(_) => {}
            Expr::Deprecated(_) => {}
            Expr::TupleExpr(elems) => {
                for e in elems {
                    result.extend(e.free_vars(bound));
                }
            }
            Expr::Range {
                start, end, step, ..
            } => {
                if let Some(s) = start {
                    result.extend(s.free_vars(bound));
                }
                if let Some(e) = end {
                    result.extend(e.free_vars(bound));
                }
                if let Some(s) = step {
                    result.extend(s.free_vars(bound));
                }
            }
            Expr::TypeOf(inner) => result.extend(inner.free_vars(bound)),
            Expr::Block { stmts, expr } => {
                let mut block_bound = bound.clone();
                for stmt in stmts {
                    collect_stmt_bound_names(stmt, &mut block_bound);
                }
                for stmt in stmts {
                    result.extend(stmt_free_vars(stmt, &block_bound));
                }
                if let Some(e) = expr {
                    result.extend(e.free_vars(&block_bound));
                }
            }
        }
        result
    }
}

/// Collects names declared by a statement into the provided set.
fn collect_stmt_bound_names(stmt: &Stmt, out: &mut HashSet<String>) {
    match stmt {
        Stmt::AutoVarDecl { name, .. } => {
            out.insert(name.clone());
        }
        Stmt::VarDecl { names, .. } => {
            for n in names {
                out.insert(n.clone());
            }
        }
        Stmt::Destructure { pattern, .. } => {
            out.extend(pattern.bindings());
        }
        Stmt::PointerNew { name, .. } => {
            out.insert(name.clone());
        }
        Stmt::LoopFor { variable, .. } => {
            out.insert(variable.clone());
        }
        Stmt::LoopForEach { variable, .. } => {
            out.insert(variable.clone());
        }
        Stmt::TryCatch {
            catch_var: Some(v), ..
        } => {
            out.insert(v.clone());
        }
        Stmt::Block(stmts) => {
            for s in stmts {
                collect_stmt_bound_names(s, out);
            }
        }
        _ => {}
    }
}

/// Returns free variables referenced inside a statement.
fn stmt_free_vars(stmt: &Stmt, bound: &HashSet<String>) -> HashSet<String> {
    let mut result = HashSet::new();
    match stmt {
        Stmt::Assignment(name, expr) => {
            if !bound.contains(name) {
                result.insert(name.clone());
            }
            result.extend(expr.free_vars(bound));
        }
        Stmt::ArrayAssignment(name, indices, expr) => {
            if !bound.contains(name) {
                result.insert(name.clone());
            }
            for idx in indices {
                result.extend(idx.free_vars(bound));
            }
            result.extend(expr.free_vars(bound));
        }
        Stmt::If {
            condition,
            then_branch,
            else_branch,
        } => {
            result.extend(condition.free_vars(bound));
            for s in then_branch {
                result.extend(stmt_free_vars(s, bound));
            }
            if let Some(else_stmts) = else_branch {
                for s in else_stmts {
                    result.extend(stmt_free_vars(s, bound));
                }
            }
        }
        Stmt::LoopWhile { condition, body } => {
            result.extend(condition.free_vars(bound));
            for s in body {
                result.extend(stmt_free_vars(s, bound));
            }
        }
        Stmt::LoopFor {
            variable,
            from,
            to,
            step,
            body,
        } => {
            result.extend(from.free_vars(bound));
            result.extend(to.free_vars(bound));
            if let Some(s) = step {
                result.extend(s.free_vars(bound));
            }
            let mut loop_bound = bound.clone();
            loop_bound.insert(variable.clone());
            for s in body {
                result.extend(stmt_free_vars(s, &loop_bound));
            }
        }
        Stmt::LoopInfinite { body } => {
            for s in body {
                result.extend(stmt_free_vars(s, bound));
            }
        }
        Stmt::LoopDoWhile { body, condition } => {
            for s in body {
                result.extend(stmt_free_vars(s, bound));
            }
            result.extend(condition.free_vars(bound));
        }
        Stmt::LoopForEach {
            variable,
            iterable,
            body,
            ..
        } => {
            result.extend(iterable.free_vars(bound));
            let mut loop_bound = bound.clone();
            loop_bound.insert(variable.clone());
            for s in body {
                result.extend(stmt_free_vars(s, &loop_bound));
            }
        }
        Stmt::Input(vars) => {
            for v in vars {
                if !bound.contains(v) {
                    result.insert(v.clone());
                }
            }
        }
        Stmt::Output(exprs) => {
            for e in exprs {
                result.extend(e.free_vars(bound));
            }
        }
        Stmt::OutputFormatted { args, .. } => {
            for e in args {
                result.extend(e.free_vars(bound));
            }
        }
        Stmt::Assert(expr)
        | Stmt::ReturnValue(expr)
        | Stmt::ResultAssign(expr)
        | Stmt::Throw(expr)
        | Stmt::Await(expr)
        | Stmt::Debug(expr)
        | Stmt::Propagate(expr) => {
            result.extend(expr.free_vars(bound));
        }
        Stmt::StaticAssert { condition, .. } => result.extend(condition.free_vars(bound)),
        Stmt::ExprStmt(expr) => result.extend(expr.free_vars(bound)),
        Stmt::Return
        | Stmt::Break
        | Stmt::Continue
        | Stmt::Pause
        | Stmt::Nop
        | Stmt::EnumDecl { .. }
        | Stmt::Import { .. }
        | Stmt::ModuleDecl { .. }
        | Stmt::Export { .. }
        | Stmt::TypeAlias { .. }
        | Stmt::ClassDecl(_)
        | Stmt::StructDecl(_)
        | Stmt::InterfaceDecl(_)
        | Stmt::TraitDecl(_)
        | Stmt::ImplBlock(_) => {}
        Stmt::AutoVarDecl { init, .. } => result.extend(init.free_vars(bound)),
        Stmt::VarDecl { init, .. } => {
            if let Some(e) = init {
                result.extend(e.free_vars(bound));
            }
        }
        Stmt::Destructure { pattern, value, .. } => {
            result.extend(value.free_vars(bound));
            result.extend(pattern_free_vars(pattern, bound));
        }
        Stmt::Match { expr, arms, .. } => {
            result.extend(expr.free_vars(bound));
            for arm in arms {
                let mut arm_bound = bound.clone();
                arm_bound.extend(arm.pattern.bindings());
                result.extend(pattern_free_vars(&arm.pattern, bound));
                if let Some(g) = &arm.guard {
                    result.extend(g.free_vars(&arm_bound));
                }
                for s in &arm.body {
                    result.extend(stmt_free_vars(s, &arm_bound));
                }
            }
        }
        Stmt::PointerNew { value, .. } => result.extend(value.free_vars(bound)),
        Stmt::PointerDelete { name } => {
            if !bound.contains(name) {
                result.insert(name.clone());
            }
        }
        Stmt::PointerAssign { pointer, value } => {
            result.extend(pointer.free_vars(bound));
            result.extend(value.free_vars(bound));
        }
        Stmt::TryCatch {
            try_block,
            catch_var,
            catch_block,
            finally_block,
            ..
        } => {
            for s in try_block {
                result.extend(stmt_free_vars(s, bound));
            }
            let mut catch_bound = bound.clone();
            if let Some(v) = catch_var {
                catch_bound.insert(v.clone());
            }
            for s in catch_block {
                result.extend(stmt_free_vars(s, &catch_bound));
            }
            if let Some(f) = finally_block {
                for s in f {
                    result.extend(stmt_free_vars(s, bound));
                }
            }
        }
        Stmt::RustBlock { captured_vars, .. } => {
            for v in captured_vars {
                if !bound.contains(v) {
                    result.insert(v.clone());
                }
            }
        }
        Stmt::SpawnTask { body, .. } => {
            for s in body {
                result.extend(stmt_free_vars(s, bound));
            }
        }
        Stmt::AwaitAll(exprs) | Stmt::AwaitAny(exprs) => {
            for e in exprs {
                result.extend(e.free_vars(bound));
            }
        }
        Stmt::Yield(param) => result.extend(param.value.free_vars(bound)),
        Stmt::ChannelSend(op) | Stmt::ChannelReceive { channel: op, .. } => {
            if !bound.contains(&op.channel) {
                result.insert(op.channel.clone());
            }
            if let Some(v) = &op.value {
                result.extend(v.free_vars(bound));
            }
        }
        Stmt::ChannelSelect { .. } => {}
        Stmt::Move { from, to }
        | Stmt::Borrow {
            source: from,
            target: to,
            ..
        }
        | Stmt::Clone {
            source: from,
            target: to,
        } => {
            if !bound.contains(from) {
                result.insert(from.clone());
            }
            if !bound.contains(to) {
                result.insert(to.clone());
            }
        }
        Stmt::FieldAssignment { object, value, .. } => {
            result.extend(object.free_vars(bound));
            result.extend(value.free_vars(bound));
        }
        Stmt::MethodCall { object, args, .. } => {
            result.extend(object.free_vars(bound));
            for a in args {
                result.extend(a.free_vars(bound));
            }
        }
        Stmt::Block(stmts) => {
            let mut block_bound = bound.clone();
            for s in stmts {
                collect_stmt_bound_names(s, &mut block_bound);
            }
            for s in stmts {
                result.extend(stmt_free_vars(s, &block_bound));
            }
        }
    }
    result
}

/// Returns free variables appearing inside a pattern (range expressions, guards).
fn pattern_free_vars(pattern: &Pattern, bound: &HashSet<String>) -> HashSet<String> {
    let mut result = HashSet::new();
    match pattern {
        Pattern::Range {
            start, end, step, ..
        } => {
            if let Some(s) = start {
                result.extend(s.free_vars(bound));
            }
            if let Some(e) = end {
                result.extend(e.free_vars(bound));
            }
            if let Some(s) = step {
                result.extend(s.free_vars(bound));
            }
        }
        Pattern::Guard { pattern, condition } => {
            result.extend(pattern_free_vars(pattern, bound));
            result.extend(condition.free_vars(bound));
        }
        Pattern::Binding { pattern, .. }
        | Pattern::Typed { pattern, .. }
        | Pattern::Ref { pattern, .. }
        | Pattern::Deref(pattern) => {
            result.extend(pattern_free_vars(pattern, bound));
        }
        Pattern::EnumVariant { bindings, .. } => {
            for b in bindings {
                result.extend(pattern_free_vars(b, bound));
            }
        }
        Pattern::Struct { fields, .. } => {
            for (_, p) in fields {
                result.extend(pattern_free_vars(p, bound));
            }
        }
        Pattern::Tuple(patterns) | Pattern::Or(patterns) => {
            for p in patterns {
                result.extend(pattern_free_vars(p, bound));
            }
        }
        Pattern::Array { elements, .. } => {
            for p in elements {
                result.extend(pattern_free_vars(p, bound));
            }
        }
        Pattern::Pair(a, b) => {
            result.extend(pattern_free_vars(a, bound));
            result.extend(pattern_free_vars(b, bound));
        }
        Pattern::Triple(a, b, c) => {
            result.extend(pattern_free_vars(a, bound));
            result.extend(pattern_free_vars(b, bound));
            result.extend(pattern_free_vars(c, bound));
        }
        Pattern::OptionSome(p) | Pattern::ResultOk(p) | Pattern::ResultErr(p) => {
            result.extend(pattern_free_vars(p, bound));
        }
        Pattern::Wildcard
        | Pattern::Literal(_)
        | Pattern::Variable(_)
        | Pattern::MutableVariable(_)
        | Pattern::Rest
        | Pattern::OptionNone => {}
    }
    result
}
