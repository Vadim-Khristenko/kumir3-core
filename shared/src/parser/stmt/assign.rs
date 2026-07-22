//! Kumir 3 Parser — Assignment & Call Statements
//!
//! [STABLE] Parses identifier-led statements: plain and compound
//! assignments, array/field assignment targets, procedure and method
//! calls, and `это.поле := ...` self-field assignments.

use crate::parser::core::Parser;
use crate::parser::error::{ParseError, ParseResult};
use crate::types::{Expr, Stmt, Token};

impl Parser {
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
    pub(super) fn parse_assignment_or_call(&mut self) -> ParseResult<Stmt> {
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
    pub(super) fn parse_field_assignment(&mut self) -> ParseResult<Stmt> {
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
}
