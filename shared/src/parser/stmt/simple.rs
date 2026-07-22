//! Kumir 3 Parser — Simple Statements
//!
//! [STABLE] Parses the single-line statement forms that need a dedicated
//! sub-parser: I/O (`ввод` / `вывод`), generator/async statements
//! (`yield`, `отложить`), ownership statements (`перемещение`,
//! `заимствовать`, `клонировать`) and `экспорт`.

use crate::parser::core::Parser;
use crate::parser::error::ParseResult;
use crate::types::{Stmt, Token, YieldParam};

impl Parser {
    // =========================================================================
    //         SECTION: I/O
    // =========================================================================

    /// Parses `ввод x, y, z`.
    pub(super) fn parse_input(&mut self) -> ParseResult<Stmt> {
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
    pub(super) fn parse_output(&mut self) -> ParseResult<Stmt> {
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
    //         SECTION: ASYNC / GENERATORS
    // =========================================================================

    /// Parses `yield expr` or `yield* expr` (delegation).
    pub(super) fn parse_yield(&mut self) -> ParseResult<Stmt> {
        self.expect(&Token::Yield, "yield")?;

        let delegate = self.match_token(&Token::Star);
        let value = self.parse_expr()?;
        self.expect_eol()?;

        Ok(Stmt::Yield(YieldParam { value, delegate }))
    }

    /// Parses `отложить нач ... кон` or `отложить stmt`.
    pub(super) fn parse_defer(&mut self) -> ParseResult<Stmt> {
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
    pub(super) fn parse_move_stmt(&mut self) -> ParseResult<Stmt> {
        self.advance(); // consume `move`
        let from = self.expect_ident("source variable")?;
        self.expect_keyword("в")?;
        let to = self.expect_ident("target variable")?;
        self.expect_eol()?;
        Ok(Stmt::Move { from, to })
    }

    /// Parses `заимствовать [измен] x как y`.
    pub(super) fn parse_borrow_stmt(&mut self) -> ParseResult<Stmt> {
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
    pub(super) fn parse_clone_stmt(&mut self) -> ParseResult<Stmt> {
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
    pub(super) fn parse_export(&mut self) -> ParseResult<Stmt> {
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
}
