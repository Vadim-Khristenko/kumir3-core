//! Kumir 3 Parser — Error-Handling Statements
//!
//! [STABLE] Parses `попытка` / `перехват` / `наконец` blocks.
//! (`бросить` is handled inline by the dispatcher in [`super`].)

use crate::parser::core::Parser;
use crate::parser::error::ParseResult;
use crate::types::{Stmt, Token};

impl Parser {
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
    pub(super) fn parse_try_catch(&mut self) -> ParseResult<Stmt> {
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
}
