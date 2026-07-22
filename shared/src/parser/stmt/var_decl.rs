//! Kumir 3 Parser — Declaration Statements
//!
//! [STABLE] Parses the statement-level declaration forms: explicitly typed
//! variables, `авто` inference and `конст` constants. (Top-level program
//! declarations live in [`crate::parser`]'s `decl` module.)

use crate::parser::core::Parser;
use crate::parser::error::ParseResult;
use crate::types::{Stmt, Token, VarModifiers};

impl Parser {
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
    pub(super) fn parse_var_decl(&mut self) -> ParseResult<Stmt> {
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
    pub(super) fn parse_auto_decl(&mut self) -> ParseResult<Stmt> {
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
    pub(super) fn parse_const_decl(&mut self) -> ParseResult<Stmt> {
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
}
