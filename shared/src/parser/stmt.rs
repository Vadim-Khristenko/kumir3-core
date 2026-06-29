//! Kumir 3 Parser — Statement Parsing
//!
//! [STABLE] Parses all Kumir 3 statement forms: assignments, declarations,
//! control flow (if/loop/for/while/switch/match), I/O, error handling,
//! async, ownership, OOP field/method calls, Rust embeds, and more.
//!
//! ## Architecture
//!
//! ```text
//!   parse_stmt()                     ← single statement dispatch
//!     ├── parse_var_decl()           ← цел x := 42
//!     ├── parse_auto_decl()          ← авто x := expr
//!     ├── parse_if()                 ← если ... то ... [иначе ...] все
//!     ├── parse_loop()               ← нц [пока|для] ... кц
//!     ├── parse_for()                ← для i от a до b ... кц
//!     ├── parse_while()              ← пока cond ... кц
//!     ├── parse_switch()             ← выбор ... при ... все
//!     ├── parse_match()              ← совпадение ... при ... все
//!     ├── parse_input/output()       ← ввод/вывод
//!     ├── parse_try_catch()          ← попытка ... перехват ... кон
//!     ├── parse_rust_block()         ← РастВставкаНЦ ... РастВставкаКЦ
//!     ├── parse_assignment_or_call() ← x := expr | x(args)
//!     └── parse_field_assignment()   ← self.field := expr
//!
//!   parse_stmts_until(stop)          ← statement list up to stop tokens
//! ```
//!
//! ## Token → Stmt Dispatch
//!
//! | Token            | Stmt variant                    |
//! |------------------|---------------------------------|
//! | type keywords    | `VarDecl` (explicit type)       |
//! | `AutoType`       | `AutoVarDecl`                   |
//! | `If`             | `If { .. }`                     |
//! | `Loop`           | `LoopInfinite/While/For/DoWhile`|
//! | `For`            | `LoopFor`                       |
//! | `While`          | `LoopWhile`                     |
//! | `Switch`         | `Match { .. }` (desugared)      |
//! | `Match`          | `Match { .. }`                  |
//! | `Input`          | `Input(vars)`                   |
//! | `Output`         | `Output(exprs)`                 |
//! | `Assert`         | `Assert(expr)`                  |
//! | `Return`         | `ReturnValue(expr)`             |
//! | `ResultValue`    | `ResultAssign(expr)`            |
//! | `Halt`           | `Return` (exit)                 |
//! | `Throw`          | `Throw(expr)`                   |
//! | `Try`            | `TryCatch { .. }`               |
//! | `Await`          | `Await(expr)`                   |
//! | `Yield`          | `Yield(..)`                     |
//! | `Defer`          | `Defer(stmts)` = `Block(stmts)` |
//! | `Delete`         | `PointerDelete { name }`        |
//! | `RustBlockStart` | `RustBlock { .. }`              |
//! | `Self_`/`This`   | `FieldAssignment { .. }`        |
//! | identifiers      | assignment / call / method call  |

use super::core::Parser;
use super::error::{ParseError, ParseResult};
use crate::types::{Expr, MatchArm, Stmt, Token, VarModifiers, YieldParam};

impl Parser {
    // =========================================================================
    //         SECTION: STATEMENT LIST
    // =========================================================================

    /// Parses a sequence of statements until one of the `stop` tokens
    /// is encountered (the stop token is **not** consumed).
    ///
    /// Blank lines and comments between statements are skipped
    /// automatically.
    pub fn parse_stmts_until(&mut self, stop: &[Token]) -> ParseResult<Vec<Stmt>> {
        self.many_until(stop, |p| p.parse_stmt())
    }

    // =========================================================================
    //         SECTION: SINGLE STATEMENT DISPATCH
    // =========================================================================

    /// Parses a single statement.
    ///
    /// Dispatches on the current token to the appropriate sub-parser.
    pub fn parse_stmt(&mut self) -> ParseResult<Stmt> {
        match self.peek().clone() {
            // ── Variable declarations (explicit type) ───────────────
            Token::IntType
            | Token::FloatType
            | Token::BoolType
            | Token::CharType
            | Token::StringType
            | Token::ArrayType
            | Token::PointerType
            | Token::OptionalType => self.parse_var_decl(),

            // ── Auto-declaration: авто x := expr ────────────────────
            Token::AutoType => self.parse_auto_decl(),

            // ── Const declaration: конст цел X := 42 ────────────────
            Token::Const => self.parse_const_decl(),

            // ── Conditional: если ... то ... [иначе ...] все ────────
            Token::If => self.parse_if(),

            // ── Loop: нц ... кц ─────────────────────────────────────
            Token::Loop => self.parse_loop(),

            // ── For: для i от a до b ... кц ─────────────────────────
            Token::For => self.parse_for(),

            // ── While: пока condition ... кц ─────────────────────────
            Token::While => self.parse_while(),

            // ── Switch: выбор ... при ... все ────────────────────────
            Token::Switch => self.parse_switch(),

            // ── Input: ввод x, y, z ─────────────────────────────────
            Token::Input => self.parse_input(),

            // ── Output: вывод a, b, c ───────────────────────────────
            Token::Output => self.parse_output(),

            // ── Assert: утв condition ───────────────────────────────
            Token::Assert => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect_eol()?;
                Ok(Stmt::Assert(expr))
            }

            // ── Halt: выход ─────────────────────────────────────────
            Token::Halt => {
                self.advance();
                self.expect_eol()?;
                Ok(Stmt::Return)
            }

            // ── Pause: пауза ────────────────────────────────────────
            Token::Pause => {
                self.advance();
                self.expect_eol()?;
                Ok(Stmt::Nop) // Pause mapped to no-op at parse level
            }

            // ── Return: вернуть expr ────────────────────────────────
            Token::Return => {
                self.advance();
                if matches!(self.peek(), Token::Newline | Token::EOF | Token::Comment(_)) {
                    self.expect_eol()?;
                    return Ok(Stmt::Return);
                }
                let expr = self.parse_expr()?;
                self.expect_eol()?;
                Ok(Stmt::ReturnValue(expr))
            }

            // ── Result value: знач := expr ──────────────────────────
            Token::ResultValue => {
                self.advance();
                self.expect(&Token::Assign, ":=")?;
                let expr = self.parse_expr()?;
                self.expect_eol()?;
                Ok(Stmt::ResultAssign(expr))
            }

            // ── Throw: бросить expr ─────────────────────────────────
            Token::Throw => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect_eol()?;
                Ok(Stmt::Throw(expr))
            }

            // ── Match: совпадение expr ... все ──────────────────────
            Token::Match => self.parse_match(),

            // ── Try-catch: попытка ... перехват ... [наконец ...] кон
            Token::Try => self.parse_try_catch(),

            // ── Rust block: РастВставкаНЦ ... РастВставкаКЦ ─────────
            Token::RustBlockStart | Token::Rust => self.parse_rust_block(),

            // ── Delete: удалить x ───────────────────────────────────
            Token::Delete => {
                self.advance();
                let name = self.expect_ident("variable name")?;
                self.expect_eol()?;
                Ok(Stmt::PointerDelete { name })
            }

            // ── Await (statement form): ждать expr ──────────────────
            Token::Await => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect_eol()?;
                Ok(Stmt::Await(expr))
            }

            // ── Yield: yield expr ───────────────────────────────────
            Token::Yield => self.parse_yield(),

            // ── Defer: отложить { stmts } ───────────────────────────
            Token::Defer => self.parse_defer(),

            // ── Break / Continue keywords (via check_keyword) ───────
            Token::Ident(ref s) if s == "прервать" || s == "break" => {
                self.advance();
                self.expect_eol()?;
                Ok(Stmt::Break)
            }
            Token::Ident(ref s) if s == "продолжить" || s == "continue" => {
                self.advance();
                self.expect_eol()?;
                Ok(Stmt::Continue)
            }

            // ── Export: экспорт name1, name2 ────────────────────────
            Token::Export => self.parse_export(),

            // ── Move statement: перемещение x в y ───────────────────
            Token::Move => self.parse_move_stmt(),

            // ── Borrow statement: заимствовать x как y ──────────────
            Token::Borrow => self.parse_borrow_stmt(),

            // ── Clone statement: клонировать x в y ──────────────────
            Token::Clone => self.parse_clone_stmt(),

            // ── this/self field assignment or method ─────────────────
            Token::This | Token::Self_ => self.parse_field_assignment(),

            // ── Identifier — assignment, call, method call ──────────
            Token::Ident(_)
            | Token::VarIdent(_)
            | Token::FuncIdent(_)
            | Token::TypeIdent(_)
            | Token::ClassIdent(_)
            | Token::NamespaceIdent(_) => self.parse_assignment_or_call(),

            _ => Err(ParseError::unexpected("statement", self.peek(), self.span()).into()),
        }
    }

    // =========================================================================
    //         SECTION: VARIABLE DECLARATIONS
    // =========================================================================

    /// Parses a variable declaration with explicit type.
    ///
    /// ```text
    /// цел x
    /// цел x, y, z
    /// цел x := 42        (single variable only)
    /// ```
    fn parse_var_decl(&mut self) -> ParseResult<Stmt> {
        let type_kind = self.parse_type()?;

        let mut names = Vec::new();
        names.push(self.expect_ident("variable name")?);

        while self.match_token(&Token::Comma) {
            names.push(self.expect_ident("variable name")?);
        }

        // Initialisation (only for single variable)
        let init = if names.len() == 1 && self.match_token(&Token::Assign) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        self.expect_eol()?;
        Ok(Stmt::VarDecl {
            type_kind,
            names,
            init,
            modifiers: VarModifiers::default(),
        })
    }

    /// Parses an auto-declaration: `авто x := expr`.
    fn parse_auto_decl(&mut self) -> ParseResult<Stmt> {
        self.expect(&Token::AutoType, "авто")?;
        let name = self.expect_ident("variable name")?;
        self.expect(&Token::Assign, ":=")?;
        let init = self.parse_expr()?;
        self.expect_eol()?;
        Ok(Stmt::AutoVarDecl {
            name,
            init,
            modifiers: VarModifiers::default(),
        })
    }

    /// Parses a constant declaration: `конст цел X := 42`.
    fn parse_const_decl(&mut self) -> ParseResult<Stmt> {
        self.expect(&Token::Const, "конст")?;
        let type_kind = self.parse_type()?;
        let name = self.expect_ident("constant name")?;
        self.expect(&Token::Assign, ":=")?;
        let init = self.parse_expr()?;
        self.expect_eol()?;
        Ok(Stmt::VarDecl {
            type_kind,
            names: vec![name],
            init: Some(init),
            modifiers: VarModifiers::constant(),
        })
    }

    // =========================================================================
    //         SECTION: CONDITIONAL
    // =========================================================================

    /// Parses an if-else statement:
    /// ```text
    /// если condition то
    ///   stmts
    /// [иначе
    ///   stmts]
    /// все
    /// ```
    fn parse_if(&mut self) -> ParseResult<Stmt> {
        self.expect(&Token::If, "если")?;
        let condition = self.parse_expr()?;
        self.expect(&Token::Then, "то")?;
        self.skip_newlines();

        let then_branch = self.parse_stmts_until(&[Token::Else, Token::Fi])?;

        let else_branch = if self.match_token(&Token::Else) {
            self.skip_newlines();
            Some(self.parse_stmts_until(&[Token::Fi])?)
        } else {
            None
        };

        self.expect(&Token::Fi, "все")?;
        self.skip_newlines();

        Ok(Stmt::If {
            condition,
            then_branch,
            else_branch,
        })
    }

    // =========================================================================
    //         SECTION: LOOPS
    // =========================================================================

    /// Parses all loop forms starting with `нц`:
    ///
    /// - `нц пока cond ... кц`          → `LoopWhile`
    /// - `нц для i от a до b ... кц`    → `LoopFor`
    /// - `нц для x в coll ... кц`       → `LoopForEach`
    /// - `нц ... кц`                    → `LoopInfinite`
    /// - `нц ... кц при cond`           → `LoopDoWhile`
    fn parse_loop(&mut self) -> ParseResult<Stmt> {
        self.expect(&Token::Loop, "нц")?;

        // нц пока condition
        if self.match_token(&Token::While) {
            let condition = self.parse_expr()?;
            self.skip_newlines();
            let body = self.parse_stmts_until(&[Token::EndLoop])?;
            self.expect(&Token::EndLoop, "кц")?;
            self.skip_newlines();
            return Ok(Stmt::LoopWhile { condition, body });
        }

        // нц для ...
        if self.match_token(&Token::For) {
            return self.parse_loop_for_body();
        }

        // Infinite loop (possibly do-while with trailing condition)
        self.skip_newlines();
        let body = self.parse_stmts_until(&[Token::EndLoop])?;
        self.expect(&Token::EndLoop, "кц")?;

        // кц при condition → do-while
        if self.match_token(&Token::Case) {
            let condition = self.parse_expr()?;
            self.skip_newlines();
            return Ok(Stmt::LoopDoWhile { body, condition });
        }

        self.skip_newlines();
        Ok(Stmt::LoopInfinite { body })
    }

    /// Parses `для i от a до b [шаг c] ... кц`.
    fn parse_for(&mut self) -> ParseResult<Stmt> {
        self.expect(&Token::For, "для")?;
        self.parse_loop_for_body()
    }

    /// Shared body for `нц для ...` and standalone `для ...`.
    ///
    /// Distinguishes between:
    /// - Counter loop:  `для i от a до b [шаг c]`
    /// - ForEach loop:  `для x в collection`
    fn parse_loop_for_body(&mut self) -> ParseResult<Stmt> {
        let variable = self.expect_ident("loop variable")?;

        // ForEach: для x в коллекция
        if self.check_keyword("в") {
            self.advance();
            let iterable = self.parse_expr()?;
            self.skip_newlines();
            let body = self.parse_stmts_until(&[Token::EndLoop])?;
            self.expect(&Token::EndLoop, "кц")?;
            self.skip_newlines();
            return Ok(Stmt::LoopForEach {
                variable,
                var_type: None,
                iterable,
                body,
            });
        }

        // Counter loop: для i от a до b [шаг c]
        self.expect(&Token::From, "от")?;
        let from = self.parse_expr()?;
        self.expect(&Token::To, "до")?;
        let to = self.parse_expr()?;

        let step = if self.match_token(&Token::Step) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        self.skip_newlines();
        let body = self.parse_stmts_until(&[Token::EndLoop])?;
        self.expect(&Token::EndLoop, "кц")?;
        self.skip_newlines();

        Ok(Stmt::LoopFor {
            variable,
            from,
            to,
            step,
            body,
        })
    }

    /// Parses `пока condition ... кц`.
    fn parse_while(&mut self) -> ParseResult<Stmt> {
        self.expect(&Token::While, "пока")?;
        let condition = self.parse_expr()?;
        self.skip_newlines();
        let body = self.parse_stmts_until(&[Token::EndLoop])?;
        self.expect(&Token::EndLoop, "кц")?;
        self.skip_newlines();
        Ok(Stmt::LoopWhile { condition, body })
    }

    // =========================================================================
    //         SECTION: SWITCH
    // =========================================================================

    /// Parses a switch statement, desugared into `Stmt::Match`:
    /// ```text
    /// выбор
    ///   при condition1: stmts
    ///   при condition2: stmts
    ///   иначе: stmts
    /// все
    /// ```
    fn parse_switch(&mut self) -> ParseResult<Stmt> {
        self.expect(&Token::Switch, "выбор")?;
        self.skip_newlines();

        let mut arms = Vec::new();

        while self.match_token(&Token::Case) {
            let condition = self.parse_expr()?;

            // Optional colon separator (Kumir style)
            let _ = self.match_token(&Token::Colon);
            self.skip_newlines();

            let body = self.parse_stmts_until(&[Token::Case, Token::Else, Token::Fi])?;

            // Desugar: switch arm → MatchArm with a guard
            arms.push(MatchArm {
                pattern: crate::types::Pattern::Wildcard,
                guard: Some(condition),
                body,
            });
        }

        // Optional else branch
        if self.match_token(&Token::Else) {
            let _ = self.match_token(&Token::Colon);
            self.skip_newlines();
            let body = self.parse_stmts_until(&[Token::Fi])?;
            arms.push(MatchArm {
                pattern: crate::types::Pattern::Wildcard,
                guard: None,
                body,
            });
        }

        self.expect(&Token::Fi, "все")?;
        self.skip_newlines();

        // Use Expr::Literal(Boolean(true)) as a dummy scrutinee for switch
        Ok(Stmt::Match {
            expr: Expr::Literal(crate::types::Value::Boolean(true)),
            arms,
            exhaustive: false,
        })
    }

    // =========================================================================
    //         SECTION: I/O
    // =========================================================================

    /// Parses `ввод x, y, z`.
    fn parse_input(&mut self) -> ParseResult<Stmt> {
        self.expect(&Token::Input, "ввод")?;
        let mut vars = Vec::new();
        loop {
            vars.push(self.expect_ident("variable")?);
            if !self.match_token(&Token::Comma) {
                break;
            }
        }
        self.expect_eol()?;
        Ok(Stmt::Input(vars))
    }

    /// Parses `вывод expr, expr, ...` or empty `вывод` (newline).
    fn parse_output(&mut self) -> ParseResult<Stmt> {
        self.expect(&Token::Output, "вывод")?;

        // Empty output (just newline)
        if matches!(self.peek(), Token::Newline | Token::EOF | Token::Comment(_)) {
            self.expect_eol()?;
            return Ok(Stmt::Output(Vec::new()));
        }

        let exprs = self.comma_sep_until_eol(|p| p.parse_expr())?;
        self.expect_eol()?;
        Ok(Stmt::Output(exprs))
    }

    // =========================================================================
    //         SECTION: MATCH
    // =========================================================================

    /// Parses pattern-matching statement:
    /// ```text
    /// совпадение expr
    ///   при pattern [если guard] => stmts
    /// все
    /// ```
    fn parse_match(&mut self) -> ParseResult<Stmt> {
        self.expect(&Token::Match, "совпадение")?;
        let expr = self.parse_expr()?;
        self.skip_newlines();

        let mut arms = Vec::new();

        while self.match_token(&Token::Case) {
            let pattern = self.parse_pattern_with_or()?;

            let guard = if self.match_token(&Token::If) {
                Some(self.parse_expr()?)
            } else {
                None
            };

            self.expect(&Token::FatArrow, "=>")?;
            self.skip_newlines();

            let body = self.parse_stmts_until(&[Token::Case, Token::Fi])?;
            arms.push(MatchArm {
                pattern,
                guard,
                body,
            });
        }

        self.expect(&Token::Fi, "все")?;
        self.skip_newlines();

        Ok(Stmt::Match {
            expr,
            arms,
            exhaustive: false,
        })
    }

    // =========================================================================
    //         SECTION: ERROR HANDLING
    // =========================================================================

    /// Parses try-catch-finally:
    /// ```text
    /// попытка
    ///   stmts
    /// перехват [var]
    ///   stmts
    /// [наконец
    ///   stmts]
    /// кон
    /// ```
    fn parse_try_catch(&mut self) -> ParseResult<Stmt> {
        self.expect(&Token::Try, "попытка")?;
        self.skip_newlines();

        let try_block = self.parse_stmts_until(&[Token::Catch])?;
        self.expect(&Token::Catch, "перехват")?;

        // Optional catch variable and type
        let catch_var = if self.is_ident() {
            Some(self.expect_ident("catch variable")?)
        } else {
            None
        };

        let catch_type = if self.match_token(&Token::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };

        self.skip_newlines();
        let catch_block = self.parse_stmts_until(&[Token::Finally, Token::End])?;

        let finally_block = if self.match_token(&Token::Finally) {
            self.skip_newlines();
            Some(self.parse_stmts_until(&[Token::End])?)
        } else {
            None
        };

        self.expect(&Token::End, "кон")?;
        self.skip_newlines();

        Ok(Stmt::TryCatch {
            try_block,
            catch_var,
            catch_type,
            catch_block,
            finally_block,
        })
    }

    // =========================================================================
    //         SECTION: RUST EMBEDS
    // =========================================================================

    /// Parses a Rust code block in one of two syntaxes:
    ///
    /// 1. `РастВставкаНЦ ... РастВставкаКЦ`
    /// 2. `ржавчина нач ... кон`
    fn parse_rust_block(&mut self) -> ParseResult<Stmt> {
        let code = if self.match_token(&Token::RustBlockStart) {
            let code = if let Token::RustCode = self.peek().clone() {
                // RustCode is a marker — the actual code is in the next token
                // Actually, based on original parser, it can be RustCode(String)
                // Let's handle both forms:
                self.advance();
                String::new()
            } else if let Token::StringLiteral(c) = self.peek().clone() {
                self.advance();
                c
            } else {
                String::new()
            };
            self.expect(&Token::RustBlockEnd, "РастВставкаКЦ")?;
            code
        } else if self.match_token(&Token::Rust) {
            self.expect(&Token::Begin, "нач")?;
            let code = if let Token::StringLiteral(c) = self.peek().clone() {
                self.advance();
                c
            } else {
                String::new()
            };
            self.expect(&Token::End, "кон")?;
            code
        } else {
            return Err(ParseError::unexpected("rust block", self.peek(), self.span()).into());
        };

        let captured_vars = extract_captured_vars(&code);
        self.skip_newlines();

        Ok(Stmt::RustBlock {
            code,
            captured_vars,
            return_type: None,
        })
    }

    // =========================================================================
    //         SECTION: ASYNC / GENERATORS
    // =========================================================================

    /// Parses `yield expr` or `yield* expr` (delegation).
    fn parse_yield(&mut self) -> ParseResult<Stmt> {
        self.expect(&Token::Yield, "yield")?;

        let delegate = self.match_token(&Token::Star);
        let value = self.parse_expr()?;
        self.expect_eol()?;

        Ok(Stmt::Yield(YieldParam { value, delegate }))
    }

    /// Parses `отложить нач ... кон` or `отложить stmt`.
    fn parse_defer(&mut self) -> ParseResult<Stmt> {
        self.advance(); // consume `defer`

        if self.match_token(&Token::Begin) {
            self.skip_newlines();
            let stmts = self.parse_stmts_until(&[Token::End])?;
            self.expect(&Token::End, "кон")?;
            self.skip_newlines();
            Ok(Stmt::Block(stmts))
        } else {
            // Single deferred statement
            let stmt = self.parse_stmt()?;
            Ok(Stmt::Block(vec![stmt]))
        }
    }

    // =========================================================================
    //         SECTION: OWNERSHIP STATEMENTS
    // =========================================================================

    /// Parses `перемещение x в y`.
    fn parse_move_stmt(&mut self) -> ParseResult<Stmt> {
        self.advance(); // consume `move`
        let from = self.expect_ident("source variable")?;
        self.expect_keyword("в")?;
        let to = self.expect_ident("target variable")?;
        self.expect_eol()?;
        Ok(Stmt::Move { from, to })
    }

    /// Parses `заимствовать [измен] x как y`.
    fn parse_borrow_stmt(&mut self) -> ParseResult<Stmt> {
        self.advance(); // consume `borrow`
        let mutable = self.match_token(&Token::Mut);
        let source = self.expect_ident("source variable")?;
        self.expect_keyword("как")?;
        let target = self.expect_ident("target variable")?;
        self.expect_eol()?;
        Ok(Stmt::Borrow {
            source,
            target,
            mutable,
        })
    }

    /// Parses `клонировать x в y`.
    fn parse_clone_stmt(&mut self) -> ParseResult<Stmt> {
        self.advance(); // consume `clone`
        let source = self.expect_ident("source variable")?;
        self.expect_keyword("в")?;
        let target = self.expect_ident("target variable")?;
        self.expect_eol()?;
        Ok(Stmt::Clone { source, target })
    }

    // =========================================================================
    //         SECTION: EXPORT
    // =========================================================================

    /// Parses `экспорт name1, name2`.
    fn parse_export(&mut self) -> ParseResult<Stmt> {
        self.advance(); // consume `export`
        let mut names = Vec::new();
        loop {
            names.push(self.expect_ident("export name")?);
            if !self.match_token(&Token::Comma) {
                break;
            }
        }
        self.expect_eol()?;
        Ok(Stmt::Export { names })
    }

    // =========================================================================
    //         SECTION: ASSIGNMENT / CALL (identifier-led)
    // =========================================================================

    /// Parses statements that start with an identifier:
    /// - Simple assignment: `x := expr`
    /// - Compound assignment: `x += expr`
    /// - Array assignment: `x[i] := expr`
    /// - Field assignment: `x.field := expr`
    /// - Method call as statement: `x.method(args)`
    /// - Module function call: `Mod::func(args)`
    /// - Procedure call: `proc(args)` or bare `proc`
    fn parse_assignment_or_call(&mut self) -> ParseResult<Stmt> {
        // Parse the left-hand side as an expression first — this gives
        // us field access chains, index chains, module access, etc.
        let lhs = self.parse_lhs_expr()?;

        // ── Compound assignment: x += expr ──────────────────────────
        if let Some(op) = self.try_compound_assign() {
            let right = self.parse_expr()?;
            self.expect_eol()?;
            return match lhs {
                Expr::Variable(name) => {
                    let binary =
                        Expr::BinaryOp(Box::new(Expr::Variable(name.clone())), op, Box::new(right));
                    Ok(Stmt::Assignment(name, binary))
                }
                _ => Err(ParseError::custom(
                    "compound assignment requires a simple variable",
                    self.span(),
                )
                .into()),
            };
        }

        // ── Simple assignment: x := expr ────────────────────────────
        if self.match_token(&Token::Assign) {
            let value = self.parse_expr()?;
            self.expect_eol()?;
            return match lhs {
                Expr::Variable(name) => Ok(Stmt::Assignment(name, value)),
                Expr::ArrayAccess(name, indices) => Ok(Stmt::ArrayAssignment(name, indices, value)),
                Expr::FieldAccess(obj, field) => Ok(Stmt::FieldAssignment {
                    object: *obj,
                    field,
                    value,
                }),
                _ => Err(ParseError::custom("invalid assignment target", self.span()).into()),
            };
        }

        // ── Already a call / method call from LHS parse ─────────────
        // If parse_lhs_expr produced a Call or MethodCall, wrap as ExprStmt
        match &lhs {
            Expr::Call(..) | Expr::MethodCall { .. } => {
                self.expect_eol()?;
                return Ok(Stmt::ExprStmt(lhs));
            }
            _ => {}
        }

        // ── Bare procedure call: proc arg1, arg2  (no parens) ───────
        if let Expr::Variable(name) = lhs {
            // Check if arguments follow (without parens — Kumir style)
            if self.match_token(&Token::LParen) {
                let args = self.parse_args()?;
                self.expect(&Token::RParen, ")")?;
                self.expect_eol()?;
                return Ok(Stmt::ExprStmt(Expr::Call(name, args)));
            }
            self.expect_eol()?;
            return Ok(Stmt::ExprStmt(Expr::Call(name, Vec::new())));
        }

        self.expect_eol()?;
        Ok(Stmt::ExprStmt(lhs))
    }

    /// Parses a left-hand side expression: identifier with optional
    /// field/index/module chains and trailing call.
    ///
    /// This is a limited version of `parse_postfix_expr` that doesn't
    /// enter full expression parsing — only structural access.
    fn parse_lhs_expr(&mut self) -> ParseResult<Expr> {
        let name = self.expect_ident("identifier")?;
        let mut expr: Expr = Expr::Variable(name);

        loop {
            match self.peek() {
                // Field: obj.field
                Token::Dot => {
                    self.advance();
                    let field = self.expect_ident("field name")?;
                    expr = Expr::FieldAccess(Box::new(expr), field);
                }
                // Index: arr[i]
                Token::LBracket => {
                    self.advance();
                    let indices = self.comma_sep(&Token::RBracket, |p| p.parse_expr())?;
                    self.expect(&Token::RBracket, "]")?;
                    match expr {
                        Expr::Variable(name) => {
                            expr = Expr::ArrayAccess(name, indices);
                        }
                        _ => {
                            expr = Expr::MethodCall {
                                object: Box::new(expr),
                                method: "__index__".to_string(),
                                args: indices,
                            };
                        }
                    }
                }
                // Module: Mod::member
                Token::DoubleColon => {
                    self.advance();
                    let member = self.expect_ident("member")?;
                    expr = match expr {
                        Expr::Variable(mod_name) => Expr::ModuleAccess(mod_name, member),
                        Expr::ModuleAccess(m, s) => {
                            Expr::ModuleAccess(format!("{}::{}", m, s), member)
                        }
                        _ => {
                            return Err(ParseError::custom(
                                ":: requires a module name",
                                self.span(),
                            )
                            .into());
                        }
                    };
                }
                // Call: f(args) — terminal in LHS context
                Token::LParen => {
                    self.advance();
                    let args = self.parse_args()?;
                    self.expect(&Token::RParen, ")")?;
                    expr = match expr {
                        Expr::Variable(name) => Expr::Call(name, args),
                        Expr::ModuleAccess(m, f) => Expr::Call(format!("{}::{}", m, f), args),
                        Expr::FieldAccess(obj, method) => Expr::MethodCall {
                            object: obj,
                            method,
                            args,
                        },
                        _ => Expr::MethodCall {
                            object: Box::new(expr),
                            method: "__call__".to_string(),
                            args,
                        },
                    };
                    // After a call in LHS, only field/method chains continue
                    if !matches!(self.peek(), Token::Dot) {
                        break;
                    }
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    /// Parses `this.field := expr` or `self.method(args)`.
    fn parse_field_assignment(&mut self) -> ParseResult<Stmt> {
        self.advance(); // consume this/self

        self.expect(&Token::Dot, ".")?;
        let field = self.expect_ident("field name")?;

        // Method call: self.method(args)
        if self.match_token(&Token::LParen) {
            let args = self.parse_args()?;
            self.expect(&Token::RParen, ")")?;
            self.expect_eol()?;
            return Ok(Stmt::ExprStmt(Expr::MethodCall {
                object: Box::new(Expr::SelfRef),
                method: field,
                args,
            }));
        }

        // Field assignment: self.field := expr
        self.expect(&Token::Assign, ":=")?;
        let value = self.parse_expr()?;
        self.expect_eol()?;

        Ok(Stmt::FieldAssignment {
            object: Expr::SelfRef,
            field,
            value,
        })
    }

    // =========================================================================
    //         SECTION: COMPOUND ASSIGNMENT
    // =========================================================================

    /// Tries to match and consume a compound assignment operator.
    /// Returns the corresponding binary operator token on success.
    fn try_compound_assign(&mut self) -> Option<Token> {
        match self.peek() {
            Token::PlusAssign => {
                self.advance();
                Some(Token::Plus)
            }
            Token::MinusAssign => {
                self.advance();
                Some(Token::Minus)
            }
            Token::StarAssign => {
                self.advance();
                Some(Token::Star)
            }
            Token::SlashAssign => {
                self.advance();
                Some(Token::Slash)
            }
            _ => None,
        }
    }

    // =========================================================================
    //         SECTION: HELPER COMBINATORS
    // =========================================================================

    /// Parses comma-separated items until end-of-line.
    fn comma_sep_until_eol<T>(
        &mut self,
        mut element: impl FnMut(&mut Self) -> ParseResult<T>,
    ) -> ParseResult<Vec<T>> {
        let mut items = Vec::new();
        loop {
            if matches!(self.peek(), Token::Newline | Token::EOF | Token::Comment(_)) {
                break;
            }
            items.push(element(self)?);
            if !self.match_token(&Token::Comma) {
                break;
            }
        }
        Ok(items)
    }
}

// =============================================================================
//         SECTION: UTILITY FUNCTIONS
// =============================================================================

/// Extracts variable names from Rust code by looking for `{name}` patterns.
///
/// Used to determine which Kumir variables are captured by a Rust embed block.
fn extract_captured_vars(code: &str) -> Vec<String> {
    let mut vars = Vec::new();
    let mut chars = code.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '{' {
            let mut name = String::new();
            for ch in chars.by_ref() {
                if ch == '}' || ch == ':' {
                    break;
                }
                name.push(ch);
            }
            let name = name.trim();
            if !name.is_empty()
                && !name.starts_with(|c: char| c.is_ascii_digit())
                && name
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c > '\x7F')
                && !vars.contains(&name.to_string())
            {
                vars.push(name.to_string());
            }
        }
    }

    vars
}
