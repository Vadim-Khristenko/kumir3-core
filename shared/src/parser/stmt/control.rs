//! Kumir 3 Parser — Control-Flow Statements
//!
//! [STABLE] Parses conditionals (`если`), every loop form (`нц`, `для`,
//! `пока`), the `выбор` switch (desugared into a match) and the pattern
//! matching `совпадение` statement.

use crate::parser::core::Parser;
use crate::parser::error::ParseResult;
use crate::types::{Expr, MatchArm, Stmt, Token};

impl Parser {
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
    pub(super) fn parse_if(&mut self) -> ParseResult<Stmt> {
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
    pub(super) fn parse_loop(&mut self) -> ParseResult<Stmt> {
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
    pub(super) fn parse_for(&mut self) -> ParseResult<Stmt> {
        self.expect(&Token::For, "для")?;
        self.parse_loop_for_body()
    }

    /// Shared body for `нц для ...` and standalone `для ...`.
    ///
    /// Distinguishes between:
    /// - Counter loop:  `для i от a до b [шаг c]`
    /// - ForEach loop:  `для x в collection`
    fn parse_loop_for_body(&mut self) -> ParseResult<Stmt> {
        let variable = self.expect_ident("имя переменной цикла")?;

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
    pub(super) fn parse_while(&mut self) -> ParseResult<Stmt> {
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
    pub(super) fn parse_switch(&mut self) -> ParseResult<Stmt> {
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
    //         SECTION: MATCH
    // =========================================================================

    /// Parses pattern-matching statement:
    /// ```text
    /// совпадение expr
    ///   при pattern [если guard] => stmts
    /// все
    /// ```
    pub(super) fn parse_match(&mut self) -> ParseResult<Stmt> {
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
}
