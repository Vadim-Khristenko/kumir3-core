//! Kumir 3 Parser — Expression Parsing
//!
//! [STABLE] Pratt-based recursive-descent parser for all Kumir 3 expressions.
//! Handles arithmetic, logic, comparison, field/index access, method calls,
//! lambdas, interpolated strings, pipe/compose chains, new/ref/deref,
//! conditional (ternary), range, tuple, array literals, casts, type checks,
//! async (await/spawn/yield), and ownership (move/borrow/clone/copy).
//!
//! ## Architecture
//!
//! ```text
//!   parse_expr()
//!     └── parse_binary_expr(min_prec)     ← Pratt loop
//!           └── parse_unary_expr()
//!                 └── parse_postfix_expr()
//!                       └── parse_primary_expr()
//!
//! Postfix chain:  call · index · field · method · :: · cast(как) · is(это) · ?
//! Primary atoms:  literal · ident · (group/tuple) · [array] · lambda
//!                 if-expr · interpolated-string · new · self/super
//!                 await · spawn · yield · match-expr · typeof
//!                 not-implemented · not-available · deprecated
//! ```
//!
//! ## Token → Expr Mapping (primary)
//!
//! | Token                    | Expr variant              |
//! |--------------------------|---------------------------|
//! | `IntLiteral(n)`          | `Literal(Number::I64)`    |
//! | `FloatLiteral(n)`        | `Literal(Number::F64)`    |
//! | `StringLiteral(s)`       | `Literal(String)`         |
//! | `CharLiteral(c)`         | `Literal(Char)`           |
//! | `RawStringLiteral(s)`    | `Literal(String)`         |
//! | `InterpolatedStringStart`| concat chain via `+`      |
//! | `True` / `False`         | `Literal(Boolean)`        |
//! | `None`                   | `Expr::None`              |
//! | `NotImplemented`         | `NotImplemented(msg?)`    |
//! | `NotAvailable`           | `NotAvailable(msg?)`      |
//! | `Deprecated`             | `Deprecated(msg?)`        |
//! | `Ident(s)` / typed ids   | `Variable(s)`             |
//! | `LParen`                 | grouped expr / `TupleExpr`|
//! | `LBracket`               | array literal             |
//! | `Lambda`                 | `Lambda { .. }`           |
//! | `If`                     | `IfExpr { .. }`           |
//! | `Match`                  | `MatchExpr { .. }`        |
//! | `New`                    | `NewInstance` / `New`      |
//! | `Self_` / `This`         | `SelfRef`                 |
//! | `Super`                  | `SuperRef`                |
//! | `Await`                  | `Await(expr)`             |
//! | `Spawn`                  | `Spawn(expr)`             |
//! | `Yield`                  | `YieldExpr(expr)`         |
//! | `Move`                   | `Move(expr)`              |
//! | `Clone`                  | `Clone(expr)`             |
//! | `Copy`                   | `Copy(expr)`              |
//! | `Borrow`                 | `Borrow { expr, mut }`    |

use super::core::Parser;
use super::error::{ParseError, ParseResult};
use super::precedence::{binary_precedence, is_right_associative};
use crate::types::{Expr, Number, Token, TypeKind, Value};

impl Parser {
    // =========================================================================
    //         SECTION: PUBLIC ENTRY POINT
    // =========================================================================

    /// Parses one full expression (top-level entry point).
    ///
    /// This is the main entry used by statement parsers, declaration
    /// parsers, and all other callers that need an expression.
    pub fn parse_expr(&mut self) -> ParseResult<Expr> {
        self.parse_binary_expr(0)
    }

    // =========================================================================
    //         SECTION: PRATT BINARY PARSING
    // =========================================================================

    /// Pratt parser for binary operators.
    ///
    /// `min_prec` is the minimum binding power required to consume the
    /// next operator.  Right-associative operators (`**`) recurse with
    /// the same `prec` for the right-hand side so they group correctly.
    ///
    /// Special operators:
    /// - `|>` (pipe)     → `Expr::Pipe`
    /// - `>>` (compose)  → `Expr::Compose`
    /// - `..` (range)    → `Expr::Range { inclusive: false }`
    fn parse_binary_expr(&mut self, min_prec: u8) -> ParseResult<Expr> {
        let mut left = self.parse_unary_expr()?;

        loop {
            let op = self.peek().clone();

            let prec = match binary_precedence(&op) {
                Some(p) if p >= min_prec => p,
                _ => break,
            };

            self.advance();

            let next_min = if is_right_associative(&op) {
                prec
            } else {
                prec + 1
            };
            let right = self.parse_binary_expr(next_min)?;

            left = match op {
                Token::Pipe => Expr::Pipe(Box::new(left), Box::new(right)),
                Token::Compose => Expr::Compose(Box::new(left), Box::new(right)),
                Token::DoubleDot => Expr::Range {
                    start: Some(Box::new(left)),
                    end: Some(Box::new(right)),
                    inclusive: false,
                },
                Token::DoubleDotEq => Expr::Range {
                    start: Some(Box::new(left)),
                    end: Some(Box::new(right)),
                    inclusive: true,
                },
                _ => Expr::BinaryOp(Box::new(left), op, Box::new(right)),
            };
        }

        Ok(left)
    }

    // =========================================================================
    //         SECTION: UNARY EXPRESSIONS
    // =========================================================================

    /// Parses prefix unary operators.
    ///
    /// | Prefix      | Expr variant              |
    /// |-------------|---------------------------|
    /// | `-`         | `UnaryOp(Minus, e)`       |
    /// | `не`        | `UnaryOp(Not, e)`         |
    /// | `&`         | `Ref(e)`                  |
    /// | `&измен`    | `Borrow { e, mut: true }` |
    /// | `^`         | `Deref(e)`                |
    /// | `новый`     | `NewInstance` / `New`      |
    /// | `перемещение` | `Move(e)`               |
    /// | `клонировать` | `Clone(e)`              |
    /// | `копировать`  | `Copy(e)`               |
    /// | `заимствовать`| `Borrow { e, mut }`     |
    fn parse_unary_expr(&mut self) -> ParseResult<Expr> {
        match self.peek().clone() {
            // Arithmetic / logic negation
            Token::Minus | Token::Not => {
                let op = self.advance().token.clone();
                let expr = self.parse_unary_expr()?;
                Ok(Expr::UnaryOp(op, Box::new(expr)))
            }

            // Reference: &expr  or  &измен expr
            Token::Ampersand => {
                self.advance();
                let mutable = self.match_token(&Token::Mut);
                let inner = self.parse_unary_expr()?;
                if mutable {
                    Ok(Expr::Borrow {
                        expr: Box::new(inner),
                        mutable: true,
                    })
                } else {
                    Ok(Expr::Ref(Box::new(inner)))
                }
            }

            // Dereference: ^expr
            Token::Caret => {
                self.advance();
                Ok(Expr::Deref(Box::new(self.parse_unary_expr()?)))
            }

            // new ClassName(args) | new expr
            Token::New => self.parse_new_expr(),

            // Ownership: move expr
            Token::Move => {
                self.advance();
                Ok(Expr::Move(Box::new(self.parse_unary_expr()?)))
            }

            // Ownership: clone expr
            Token::Clone => {
                self.advance();
                Ok(Expr::Clone(Box::new(self.parse_unary_expr()?)))
            }

            // Ownership: copy expr
            Token::Copy => {
                self.advance();
                Ok(Expr::Copy(Box::new(self.parse_unary_expr()?)))
            }

            // Ownership: borrow [mut] expr
            Token::Borrow => {
                self.advance();
                let mutable = self.match_token(&Token::Mut);
                let inner = self.parse_unary_expr()?;
                Ok(Expr::Borrow {
                    expr: Box::new(inner),
                    mutable,
                })
            }

            _ => self.parse_postfix_expr(),
        }
    }

    // =========================================================================
    //         SECTION: NEW EXPRESSION
    // =========================================================================

    /// Parses `новый ClassName(args)` or `новый expr`.
    ///
    /// - `новый Класс(a, b)` → `Expr::NewInstance { class_name, args }`
    /// - `новый Класс`       → `Expr::New(Variable("Класс"))`
    /// - `новый <expr>`      → `Expr::New(expr)`
    fn parse_new_expr(&mut self) -> ParseResult<Expr> {
        self.advance(); // consume `new`

        // Try `new Ident(args)` — class instantiation
        if self.is_ident() {
            let name = self.expect_ident("class name")?;

            if self.match_token(&Token::LParen) {
                let args = self.parse_args()?;
                self.expect(&Token::RParen, ")")?;
                return Ok(Expr::NewInstance {
                    class_name: name,
                    args,
                });
            }

            // `new Ident` without parens — pointer allocation
            return Ok(Expr::New(Box::new(Expr::Variable(name))));
        }

        // `new <expr>` — generic heap allocation
        Ok(Expr::New(Box::new(self.parse_unary_expr()?)))
    }

    // =========================================================================
    //         SECTION: POSTFIX EXPRESSIONS
    // =========================================================================

    /// Parses postfix chains.
    ///
    /// Each iteration extends `expr` with one postfix operation:
    ///
    /// | Token    | Operation                          |
    /// |----------|------------------------------------|
    /// | `(`      | function / method call              |
    /// | `[`      | array index (1-d or multi-d)        |
    /// | `::`     | module / enum access                |
    /// | `.`      | field access (→ MethodCall if `(`)  |
    /// | `как`    | type cast                           |
    /// | `это`    | type check                          |
    /// | `?`      | error propagation (sugar)           |
    fn parse_postfix_expr(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_primary_expr()?;

        loop {
            match self.peek() {
                // ── Call: f(args) ────────────────────────────────────
                Token::LParen => {
                    self.advance();
                    let args = self.parse_args()?;
                    self.expect(&Token::RParen, ")")?;

                    expr = match expr {
                        Expr::Variable(name) => Expr::Call(name, args),
                        Expr::ModuleAccess(module, func) => {
                            Expr::Call(format!("{}::{}", module, func), args)
                        }
                        Expr::FieldAccess(obj, method) => Expr::MethodCall {
                            object: obj,
                            method,
                            args,
                        },
                        other => {
                            // Indirect call: (expr)(args) — wrap as method
                            Expr::MethodCall {
                                object: Box::new(other),
                                method: "__call__".to_string(),
                                args,
                            }
                        }
                    };
                }

                // ── Index: arr[i] / arr[i, j] ──────────────────────
                Token::LBracket => {
                    self.advance();
                    let indices = self.comma_sep(&Token::RBracket, |p| p.parse_expr())?;
                    self.expect(&Token::RBracket, "]")?;

                    expr = match expr {
                        Expr::Variable(name) => Expr::ArrayAccess(name, indices),
                        other => Expr::MethodCall {
                            object: Box::new(other),
                            method: "__index__".to_string(),
                            args: indices,
                        },
                    };
                }

                // ── Module / enum access: Mod::member ───────────────
                Token::DoubleColon => {
                    self.advance();
                    let member = self.expect_ident("member name")?;

                    expr = match expr {
                        Expr::Variable(module) => Expr::ModuleAccess(module, member),
                        Expr::ModuleAccess(m, sub) => {
                            Expr::ModuleAccess(format!("{}::{}", m, sub), member)
                        }
                        _ => {
                            return Err(ParseError::custom(
                                ":: requires a module or enum on the left",
                                self.span(),
                            ));
                        }
                    };
                }

                // ── Field access: obj.field ─────────────────────────
                Token::Dot => {
                    self.advance();
                    let field = self.expect_ident("field name")?;
                    expr = Expr::FieldAccess(Box::new(expr), field);
                }

                // ── Type cast: expr как Тип ─────────────────────────
                Token::Ident(s) if s == "как" => {
                    self.advance();
                    let target_type = self.parse_type()?;
                    expr = Expr::Cast {
                        expr: Box::new(expr),
                        target_type,
                    };
                }

                // ── Type check: expr это Тип ────────────────────────
                Token::Ident(s) if s == "это" => {
                    self.advance();
                    let check_type = self.parse_type()?;
                    expr = Expr::TypeCheck {
                        expr: Box::new(expr),
                        check_type,
                    };
                }

                // ── Error propagation: expr? ────────────────────────
                Token::Question => {
                    self.advance();
                    expr = Expr::MethodCall {
                        object: Box::new(expr),
                        method: "__propagate__".to_string(),
                        args: Vec::new(),
                    };
                }

                _ => break,
            }
        }

        Ok(expr)
    }

    // =========================================================================
    //         SECTION: PRIMARY EXPRESSIONS
    // =========================================================================

    /// Parses atomic / primary expressions — the leaves of the
    /// expression tree or constructs that start with a unique token.
    fn parse_primary_expr(&mut self) -> ParseResult<Expr> {
        match self.peek().clone() {
            // ── Integer literal ─────────────────────────────────────
            Token::IntLiteral(n) => {
                self.advance();
                Ok(Expr::Literal(Value::Number(Number::I64(n))))
            }

            // ── Float literal ───────────────────────────────────────
            Token::FloatLiteral(n) => {
                self.advance();
                Ok(Expr::Literal(Value::Number(Number::F64(n))))
            }

            // ── String literal ──────────────────────────────────────
            Token::StringLiteral(s) => {
                self.advance();
                Ok(Expr::Literal(Value::String(s)))
            }

            // ── Raw string literal ──────────────────────────────────
            Token::RawStringLiteral(s) => {
                self.advance();
                Ok(Expr::Literal(Value::String(s)))
            }

            // ── Char literal ────────────────────────────────────────
            Token::CharLiteral(c) => {
                self.advance();
                Ok(Expr::Literal(Value::Char(c)))
            }

            // ── Boolean literals ────────────────────────────────────
            Token::True => {
                self.advance();
                Ok(Expr::Literal(Value::Boolean(true)))
            }
            Token::False => {
                self.advance();
                Ok(Expr::Literal(Value::Boolean(false)))
            }

            // ── None / null ─────────────────────────────────────────
            Token::None => {
                self.advance();
                Ok(Expr::None)
            }

            // ── NotImplemented (optional message) ───────────────────
            Token::NotImplemented => {
                self.advance();
                Ok(Expr::NotImplemented(self.parse_optional_paren_string()))
            }

            // ── NotAvailable ────────────────────────────────────────
            Token::NotAvailable => {
                self.advance();
                Ok(Expr::NotAvailable(self.parse_optional_paren_string()))
            }

            // ── Deprecated ──────────────────────────────────────────
            Token::Deprecated => {
                self.advance();
                Ok(Expr::Deprecated(self.parse_optional_paren_string()))
            }

            // ── Self reference: я / self / это ──────────────────────
            Token::This | Token::Self_ => {
                self.advance();
                Ok(Expr::SelfRef)
            }

            // ── Super reference: предок / super ─────────────────────
            Token::Super => {
                self.advance();
                Ok(Expr::SuperRef)
            }

            // ── Identifier (variable, function name, etc.) ──────────
            Token::Ident(_)
            | Token::VarIdent(_)
            | Token::FuncIdent(_)
            | Token::TypeIdent(_)
            | Token::ClassIdent(_)
            | Token::NamespaceIdent(_) => {
                let name = self.expect_ident("identifier")?;
                Ok(Expr::Variable(name))
            }

            // ── Parenthesised expression / tuple ────────────────────
            Token::LParen => self.parse_paren_or_tuple(),

            // ── Array literal: [a, b, c] ────────────────────────────
            Token::LBracket => self.parse_array_literal(),

            // ── Lambda: лямбда(x, y) -> expr ────────────────────────
            Token::Lambda => {
                self.advance();
                self.parse_lambda()
            }

            // ── Conditional expression ───────────────────────────────
            Token::If => self.parse_if_expr(),

            // ── Interpolated string: f"text {expr} text" ────────────
            Token::InterpolatedStringStart => self.parse_interpolated_string(),

            // ── Await expression: ждать expr ────────────────────────
            Token::Await => {
                self.advance();
                Ok(Expr::Await(Box::new(self.parse_unary_expr()?)))
            }

            // ── Spawn: создать { expr } ─────────────────────────────
            Token::Spawn => {
                self.advance();
                Ok(Expr::Spawn(Box::new(self.parse_unary_expr()?)))
            }

            // ── Yield: yield expr ───────────────────────────────────
            Token::Yield => {
                self.advance();
                Ok(Expr::YieldExpr(Box::new(self.parse_expr()?)))
            }

            // ── Match expression (value-producing) ──────────────────
            Token::Match => self.parse_match_expr(),

            // ── Typeof: тип_зн(expr) / typeof(expr) ────────────────
            Token::Ident(ref s) if s == "тип_зн" || s == "typeof" => {
                self.advance();
                self.expect(&Token::LParen, "(")?;
                let inner = self.parse_expr()?;
                self.expect(&Token::RParen, ")")?;
                Ok(Expr::TypeOf(Box::new(inner)))
            }

            _ => Err(ParseError::expected_expr(self.span())),
        }
    }

    // =========================================================================
    //         SECTION: PARENTHESISED / TUPLE
    // =========================================================================

    /// Parses `(expr)` for grouping  or `(a, b, c)` for tuples.
    ///
    /// Empty parens `()` produce an empty tuple: `TupleExpr([])`.
    fn parse_paren_or_tuple(&mut self) -> ParseResult<Expr> {
        self.advance(); // consume `(`

        // Empty tuple: ()
        if self.check(&Token::RParen) {
            self.advance();
            return Ok(Expr::TupleExpr(Vec::new()));
        }

        let first = self.parse_expr()?;

        // Tuple: (a, b, ...)
        if self.match_token(&Token::Comma) {
            let mut elems = vec![first];
            loop {
                if self.check(&Token::RParen) {
                    break;
                }
                elems.push(self.parse_expr()?);
                if !self.match_token(&Token::Comma) {
                    break;
                }
            }
            self.expect(&Token::RParen, ")")?;
            return Ok(Expr::TupleExpr(elems));
        }

        // Simple grouping
        self.expect(&Token::RParen, ")")?;
        Ok(first)
    }

    // =========================================================================
    //         SECTION: ARRAY LITERAL
    // =========================================================================

    /// Parses `[a, b, c]` — array literal.
    ///
    /// When all elements are compile-time literals, produces
    /// `Expr::Literal(Value::Array(...))`.  Otherwise produces
    /// a `Value::Array` with `Value::Undefined` placeholders
    /// for runtime-computed elements.
    fn parse_array_literal(&mut self) -> ParseResult<Expr> {
        self.advance(); // consume `[`
        let elements = self.comma_sep(&Token::RBracket, |p| p.parse_expr())?;
        self.expect(&Token::RBracket, "]")?;

        let values: Vec<Value> = elements.into_iter().map(expr_to_value).collect();
        Ok(Expr::Literal(Value::Array(values)))
    }

    // =========================================================================
    //         SECTION: LAMBDA
    // =========================================================================

    /// Parses lambda body after the `лямбда` keyword has been consumed.
    ///
    /// Supported forms:
    /// ```text
    /// лямбда(x, y) -> expr             — positional params
    /// лямбда x -> expr                 — single param shorthand
    /// лямбда(цел x, вещ y) -> expr     — typed params
    /// лямбда(x, y): Тип -> expr        — with return type
    /// ```
    fn parse_lambda(&mut self) -> ParseResult<Expr> {
        let mut params = Vec::new();
        let mut param_types: Option<Vec<TypeKind>> = None;
        let mut return_type: Option<TypeKind> = None;

        if self.match_token(&Token::LParen) {
            // Try typed parameters: (Type name, Type name, ...)
            if let Some((names, types)) = self.try_parse(|p| p.parse_typed_lambda_params()) {
                params = names;
                if types.iter().any(|t| *t != TypeKind::Auto) {
                    param_types = Some(types);
                }
            } else {
                // Simple identifier parameters
                if !self.check(&Token::RParen) {
                    loop {
                        params.push(self.expect_ident("parameter")?);
                        if !self.match_token(&Token::Comma) {
                            break;
                        }
                    }
                }
            }
            self.expect(&Token::RParen, ")")?;
        } else if self.is_ident() {
            // Single-param shorthand: лямбда x -> expr
            params.push(self.expect_ident("parameter")?);
        }

        // Optional return type annotation
        if self.match_token(&Token::Colon) {
            return_type = Some(self.parse_type()?);
        }

        self.expect(&Token::Arrow, "->")?;
        let body = self.parse_expr()?;

        Ok(Expr::Lambda {
            params,
            param_types,
            return_type,
            body: Box::new(body),
        })
    }

    /// Parses typed lambda parameters: `(цел x, вещ y)`.
    ///
    /// Returns `(names, types)` on success; fails if the syntax
    /// doesn't match (used speculatively via `try_parse`).
    fn parse_typed_lambda_params(&mut self) -> ParseResult<(Vec<String>, Vec<TypeKind>)> {
        let mut names = Vec::new();
        let mut types = Vec::new();

        if !self.check(&Token::RParen) {
            loop {
                let tk = self.parse_type()?;
                let name = self.expect_ident("parameter name")?;
                types.push(tk);
                names.push(name);
                if !self.match_token(&Token::Comma) {
                    break;
                }
            }
        }

        Ok((names, types))
    }

    // =========================================================================
    //         SECTION: IF EXPRESSION
    // =========================================================================

    /// Parses conditional expression:
    /// `если <cond> то <then> иначе <else> все`
    fn parse_if_expr(&mut self) -> ParseResult<Expr> {
        self.advance(); // consume `если`
        let condition = self.parse_expr()?;
        self.expect(&Token::Then, "то")?;
        let then_expr = self.parse_expr()?;
        self.expect(&Token::Else, "иначе")?;
        let else_expr = self.parse_expr()?;
        self.expect(&Token::Fi, "все")?;

        Ok(Expr::IfExpr {
            condition: Box::new(condition),
            then_expr: Box::new(then_expr),
            else_expr: Box::new(else_expr),
        })
    }

    // =========================================================================
    //         SECTION: INTERPOLATED STRING
    // =========================================================================

    /// Parses an interpolated string: `f"text {expr} more text"`.
    ///
    /// The lexer emits the sequence:
    /// ```text
    /// InterpolatedStringStart
    ///   (InterpolatedStringPart(text) · expression)*
    /// InterpolatedStringEnd
    /// ```
    ///
    /// Desugared into a chain of binary `+` (concatenation) operations
    /// so `f"Hello {name}!"` becomes `"Hello " + name + "!"`.
    fn parse_interpolated_string(&mut self) -> ParseResult<Expr> {
        self.advance(); // consume InterpolatedStringStart

        let mut parts: Vec<Expr> = Vec::new();

        loop {
            match self.peek().clone() {
                Token::InterpolatedStringPart(text) => {
                    self.advance();
                    if !text.is_empty() {
                        parts.push(Expr::Literal(Value::String(text)));
                    }
                }
                Token::InterpolatedStringEnd => {
                    self.advance();
                    break;
                }
                Token::EOF => {
                    return Err(ParseError::custom(
                        "unterminated interpolated string",
                        self.span(),
                    ));
                }
                _ => {
                    // Embedded expression inside {…}
                    parts.push(self.parse_expr()?);
                }
            }
        }

        // Fold parts via binary `+`
        if parts.is_empty() {
            return Ok(Expr::Literal(Value::String(String::new())));
        }

        let mut result = parts.remove(0);
        for part in parts {
            result = Expr::BinaryOp(Box::new(result), Token::Plus, Box::new(part));
        }
        Ok(result)
    }

    // =========================================================================
    //         SECTION: MATCH EXPRESSION (value-producing)
    // =========================================================================

    /// Parses a value-producing match expression:
    /// ```text
    /// совпадение expr
    ///   при pattern => expr
    ///   при pattern => expr
    /// все
    /// ```
    fn parse_match_expr(&mut self) -> ParseResult<Expr> {
        self.advance(); // consume `совпадение`
        let scrutinee = self.parse_expr()?;
        self.skip_newlines();

        let mut arms: Vec<(crate::types::Pattern, Expr)> = Vec::new();

        while self.match_token(&Token::Case) {
            let pattern = self.parse_pattern_with_or()?;
            self.expect(&Token::FatArrow, "=>")?;
            let value = self.parse_expr()?;
            self.skip_newlines();
            arms.push((pattern, value));
        }

        self.expect(&Token::Fi, "все")?;

        Ok(Expr::MatchExpr {
            expr: Box::new(scrutinee),
            arms,
        })
    }

    // =========================================================================
    //         SECTION: ARGUMENT LIST
    // =========================================================================

    /// Parses a comma-separated argument list (between already-consumed
    /// `(` and not-yet-consumed `)`).
    ///
    /// Uses the [`comma_sep`](Parser::comma_sep) combinator from core.
    pub fn parse_args(&mut self) -> ParseResult<Vec<Expr>> {
        self.comma_sep(&Token::RParen, |p| p.parse_expr())
    }

    // =========================================================================
    //         SECTION: STANDALONE CAST
    // =========================================================================

    /// Parses a type cast when the `как` keyword has already been consumed.
    pub fn parse_cast(&mut self, expr: Expr) -> ParseResult<Expr> {
        let target_type = self.parse_type()?;
        Ok(Expr::Cast {
            expr: Box::new(expr),
            target_type,
        })
    }

    // =========================================================================
    //         SECTION: HELPERS
    // =========================================================================

    /// Tries to parse an optional parenthesised string: `("message")`.
    ///
    /// Returns `None` if the next token is not `(` or there is no
    /// string literal inside the parens.
    fn parse_optional_paren_string(&mut self) -> Option<String> {
        if !self.match_token(&Token::LParen) {
            return None;
        }
        let msg = if let Token::StringLiteral(s) = self.peek().clone() {
            self.advance();
            Some(s)
        } else {
            None
        };
        let _ = self.match_token(&Token::RParen);
        msg
    }
}

// =============================================================================
//         SECTION: UTILITY FUNCTIONS
// =============================================================================

/// Best-effort conversion of `Expr` → `Value` for compile-time array literals.
///
/// Returns `Value::Undefined` for non-literal expressions (these will
/// be evaluated at runtime by the interpreter).
fn expr_to_value(expr: Expr) -> Value {
    match expr {
        Expr::Literal(v) => v,
        Expr::None => Value::Null,
        _ => Value::Undefined,
    }
}
