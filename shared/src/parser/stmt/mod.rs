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
//!
//! ## Module Layout
//!
//! | Submodule        | Responsibility                                  |
//! |------------------|-------------------------------------------------|
//! | `assign`         | identifier-led assignment / call statements     |
//! | `control`        | if, loops, switch, match                        |
//! | `error_handling` | try / catch / finally                           |
//! | `rust_embed`     | Rust embed blocks + capture extraction          |
//! | `simple`         | I/O, ownership, export, yield, defer            |
//! | `var_decl`       | variable / auto / const declaration statements  |
//! | `mod` (here)     | statement list + single-statement dispatch      |

mod assign;
mod control;
mod error_handling;
mod rust_embed;
mod simple;
mod var_decl;

use super::core::Parser;
use super::error::{ParseError, ParseResult};
use crate::types::{Stmt, Token};

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
            | Token::AnyType
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
                Ok(Stmt::Pause)
            }

            // ── Type alias: type Name = Type ─────────────────────────
            Token::TypeAlias => {
                self.advance();
                let name = self.expect_ident("имя псевдонима типа")?;
                self.expect(&Token::Equal, "=")?;
                let target = self.parse_type()?;
                self.expect_eol()?;
                Ok(Stmt::TypeAlias { name, target })
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
