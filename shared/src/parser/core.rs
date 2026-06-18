//! Kumir 3 Parser — Core Engine
//!
//! [STABLE] Token-stream navigation, speculative parsing, span tracking
//! and error recovery infrastructure for the Kumir 3 parser.
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────┐
//! │  Source Code (UTF-8)                                             │
//! └──────────────┬───────────────────────────────────────────────────┘
//!                │  tokenize() / tokenize_with_recovery()
//!                ▼
//! ┌──────────────────────────────────────────────────────────────────┐
//! │  Vec<SpannedToken>     (lexer output)                            │
//! └──────────────┬───────────────────────────────────────────────────┘
//!                │
//!                ▼
//! ┌──────────────────────────────────────────────────────────────────┐
//! │  Parser (this module)                                            │
//! │                                                                  │
//! │  ┌────────────────┐  ┌────────────────┐  ┌───────────────────┐   │
//! │  │ Token Stream   │  │ Checkpoint /   │  │ Diagnostics       │   │
//! │  │ Navigation     │  │ Backtrack      │  │ Collector         │   │
//! │  │                │  │                │  │                   │   │
//! │  │ peek/advance   │  │ checkpoint()   │  │ report_error()    │   │
//! │  │ check/expect   │  │ backtrack()    │  │ report_warning()  │   │
//! │  │ match_token    │  │ try_parse()    │  │ take_diagnostics()│   │
//! │  └────────────────┘  └────────────────┘  └───────────────────┘   │
//! │                                                                  │
//! │  ┌────────────────┐  ┌────────────────┐  ┌───────────────────┐   │
//! │  │ Span Helpers   │  │ Error Recovery │  │ Convenience API   │   │
//! │  │                │  │                │  │                   │   │
//! │  │ mark()/since() │  │ recover_to()   │  │ expect_ident()    │   │
//! │  │ span_from()    │  │ recover_past() │  │ expect_string()   │   │
//! │  │                │  │ skip_until()   │  │ at_expr_start()   │   │
//! │  └────────────────┘  └────────────────┘  └───────────────────┘   │
//! └──────────────────────────────────────────────────────────────────┘
//!                │
//!                ▼  (consumed by parse modules)
//! ┌──────────────────────────────────────────────────────────────────┐
//! │  types.rs · expr.rs · pattern.rs · stmt.rs · decl.rs · oop.rs    │
//! └──────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Design Principles
//!
//! 1. **Zero-copy where possible** — tokens are borrowed via index, only
//!    cloned when data extraction is required (identifiers, literals).
//! 2. **Speculative parsing** — `try_parse()` saves/restores position and
//!    diagnostics atomically; no side effects on failure.
//! 3. **Span tracking** — `mark()` + `since()` pair automatically builds
//!    source spans covering any parsed construct.
//! 4. **Error recovery** — in recovery mode errors are accumulated into
//!    `Diagnostics`; parsing continues until sync tokens are found.
//! 5. **Composable API** — every method returns `ParseResult<T>` or
//!    `Option<T>`, enabling clean `?` chains in parse functions.

use super::error::{Diagnostics, ParseError, ParseResult};
use crate::lexer::{Position, Span, SpannedToken, tokenize, tokenize_with_recovery};
use crate::types::Token;

// =============================================================================
//         SECTION: CHECKPOINT & MARK
// =============================================================================

/// Saved parser position for speculative (backtracking) parsing.
///
/// Created by [`Parser::checkpoint()`], restored by [`Parser::backtrack()`].
#[derive(Debug, Clone, Copy)]
pub struct Checkpoint {
    pos: usize,
}

/// Start position for building a [`Span`] via [`Parser::since()`].
///
/// Created by [`Parser::mark()`] before parsing a construct,
/// then passed to `since()` after the construct is fully parsed.
#[derive(Debug, Clone, Copy)]
pub struct Mark {
    pos: usize,
}

// =============================================================================
//         SECTION: PARSER STRUCT
// =============================================================================

/// Kumir 3 recursive-descent parser core.
///
/// Holds the token stream, cursor position, accumulated diagnostics
/// and the error-recovery flag. All domain-specific parse methods
/// (expressions, statements, declarations, etc.) are implemented as
/// `impl Parser` in sibling modules.
pub struct Parser {
    /// Lexed token stream (always ends with `Token::EOF`).
    tokens: Vec<SpannedToken>,
    /// Current cursor position in `tokens`.
    pos: usize,
    /// Accumulated errors, warnings, hints.
    diagnostics: Diagnostics,
    /// When `true`, errors are collected instead of returned immediately.
    error_recovery: bool,
}

// =============================================================================
//         SECTION: CONSTRUCTORS
// =============================================================================

impl Parser {
    /// Creates a parser from source code (fail-fast mode).
    ///
    /// Tokenises the input; returns `Err` on the first lexer error.
    pub fn new(source: &str) -> ParseResult<Self> {
        let tokens = tokenize(source)?;
        Ok(Self {
            tokens,
            pos: 0,
            diagnostics: Diagnostics::new(),
            error_recovery: false,
        })
    }

    /// Creates a parser in **recovery mode**.
    ///
    /// Lexer errors are collected into diagnostics — the constructor
    /// never fails.  Subsequent parse methods will also accumulate
    /// errors instead of short-circuiting.
    pub fn with_recovery(source: &str) -> Self {
        let (tokens, lex_errors) = tokenize_with_recovery(source);
        let mut diagnostics = Diagnostics::new();
        for err in lex_errors {
            diagnostics.error(ParseError::from(err));
        }
        Self {
            tokens,
            pos: 0,
            diagnostics,
            error_recovery: true,
        }
    }

    /// Creates a parser from pre-lexed tokens (useful for tests).
    pub fn from_tokens(tokens: Vec<SpannedToken>) -> Self {
        Self {
            tokens,
            pos: 0,
            diagnostics: Diagnostics::new(),
            error_recovery: false,
        }
    }

    // =========================================================================
    //         SECTION: TOKEN STREAM NAVIGATION
    // =========================================================================

    /// Returns the current token without advancing.
    #[inline]
    pub fn peek(&self) -> &Token {
        &self.tokens[self.pos].token
    }

    /// Returns the current [`SpannedToken`] without advancing.
    #[inline]
    pub fn current(&self) -> &SpannedToken {
        &self.tokens[self.pos]
    }

    /// Lookahead `n` tokens (0 = current). Clamped to EOF.
    #[inline]
    pub fn peek_n(&self, n: usize) -> &Token {
        &self.tokens[(self.pos + n).min(self.tokens.len() - 1)].token
    }

    /// [`Span`] of the current token.
    #[inline]
    pub fn span(&self) -> Span {
        self.tokens[self.pos].span
    }

    /// `true` when the cursor points to `Token::EOF`.
    #[inline]
    pub fn is_eof(&self) -> bool {
        matches!(self.peek(), Token::EOF)
    }

    /// Advances by one token and returns the **consumed** token.
    ///
    /// At EOF the cursor stays put (idempotent).
    #[inline]
    pub fn advance(&mut self) -> &SpannedToken {
        if !self.is_eof() {
            self.pos += 1;
        }
        &self.tokens[self.pos - 1]
    }

    /// Checks discriminant equality with the current token (no advance).
    #[inline]
    pub fn check(&self, token: &Token) -> bool {
        std::mem::discriminant(self.peek()) == std::mem::discriminant(token)
    }

    /// If the current token matches `token` (by discriminant), consumes it
    /// and returns `true`; otherwise returns `false`.
    #[inline]
    pub fn match_token(&mut self, token: &Token) -> bool {
        if self.check(token) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Requires a specific token; returns an error on mismatch.
    ///
    /// `msg` is the human-readable description used in the error
    /// (e.g. `")"`, `"нач"`, `"идентификатор"`).
    #[inline]
    pub fn expect(&mut self, token: &Token, msg: &str) -> ParseResult<&SpannedToken> {
        if self.check(token) {
            Ok(self.advance())
        } else {
            Err(ParseError::unexpected(msg, self.peek(), self.span()))
        }
    }

    /// Skips newlines and line comments (keeps doc-comments).
    #[inline]
    pub fn skip_newlines(&mut self) {
        while matches!(self.peek(), Token::Newline | Token::Comment(_)) {
            self.advance();
        }
    }

    /// Requires end-of-line (Newline / Comment / EOF).
    pub fn expect_eol(&mut self) -> ParseResult<()> {
        if matches!(self.peek(), Token::Newline | Token::EOF | Token::Comment(_)) {
            if !self.is_eof() {
                self.advance();
            }
            Ok(())
        } else {
            Err(ParseError::expected_newline(self.span()))
        }
    }

    /// `true` if the current token matches **any** token in `tokens`.
    #[inline]
    pub fn check_any(&self, tokens: &[Token]) -> bool {
        tokens.iter().any(|t| self.check(t))
    }

    // =========================================================================
    //         SECTION: IDENTIFIER HELPERS
    // =========================================================================

    /// `true` if the current token is any identifier variant.
    #[inline]
    pub fn is_ident(&self) -> bool {
        matches!(
            self.peek(),
            Token::Ident(_)
                | Token::VarIdent(_)
                | Token::FuncIdent(_)
                | Token::TypeIdent(_)
                | Token::ClassIdent(_)
                | Token::NamespaceIdent(_)
        )
    }

    /// Consumes and returns an identifier name, or errors.
    ///
    /// Accepts all identifier variants: `Ident`, `VarIdent`, `FuncIdent`,
    /// `TypeIdent`, `ClassIdent`, `NamespaceIdent`.
    pub fn expect_ident(&mut self, msg: &str) -> ParseResult<String> {
        let name = match self.peek().clone() {
            Token::Ident(s)
            | Token::VarIdent(s)
            | Token::FuncIdent(s)
            | Token::TypeIdent(s)
            | Token::ClassIdent(s)
            | Token::NamespaceIdent(s) => s,
            _ => return Err(ParseError::unexpected(msg, self.peek(), self.span())),
        };
        self.advance();
        Ok(name)
    }

    /// Consumes and returns a string literal, or errors.
    pub fn expect_string(&mut self, msg: &str) -> ParseResult<String> {
        if let Token::StringLiteral(s) = self.peek().clone() {
            self.advance();
            Ok(s)
        } else {
            Err(ParseError::unexpected(msg, self.peek(), self.span()))
        }
    }

    /// Consumes an import path — either a string literal or an identifier.
    pub fn expect_import_path(&mut self) -> ParseResult<String> {
        match self.peek().clone() {
            Token::StringLiteral(s) => {
                self.advance();
                Ok(s)
            }
            Token::Ident(s)
            | Token::VarIdent(s)
            | Token::FuncIdent(s)
            | Token::TypeIdent(s)
            | Token::ClassIdent(s)
            | Token::NamespaceIdent(s) => {
                self.advance();
                Ok(s)
            }
            _ => Err(ParseError::unexpected(
                "import path",
                self.peek(),
                self.span(),
            )),
        }
    }

    /// Consumes an integer literal, or errors.
    pub fn expect_int(&mut self, msg: &str) -> ParseResult<i64> {
        if let Token::IntLiteral(n) = *self.peek() {
            self.advance();
            Ok(n)
        } else {
            Err(ParseError::unexpected(msg, self.peek(), self.span()))
        }
    }

    /// Scans forward (without moving cursor) for any token matching `token`.
    pub fn has_token_ahead(&self, token: &Token) -> bool {
        let disc = std::mem::discriminant(token);
        self.tokens[self.pos..]
            .iter()
            .any(|t| std::mem::discriminant(&t.token) == disc)
    }

    // =========================================================================
    //         SECTION: KEYWORD HELPERS
    // =========================================================================

    /// `true` if the current token is `Ident(name)` (case-sensitive).
    #[inline]
    pub fn check_keyword(&self, name: &str) -> bool {
        matches!(self.peek(), Token::Ident(s) if s == name)
    }

    /// Consumes `Ident(name)` if matched; returns `true` on success.
    #[inline]
    pub fn match_keyword(&mut self, name: &str) -> bool {
        if self.check_keyword(name) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Requires `Ident(name)`, or returns an error.
    pub fn expect_keyword(&mut self, name: &str) -> ParseResult<()> {
        if self.check_keyword(name) {
            self.advance();
            Ok(())
        } else {
            Err(ParseError::unexpected(
                &format!("keyword «{}»", name),
                self.peek(),
                self.span(),
            ))
        }
    }

    // =========================================================================
    //         SECTION: CHECKPOINT / BACKTRACK
    // =========================================================================

    /// Saves the current cursor position.
    #[inline]
    pub fn checkpoint(&self) -> Checkpoint {
        Checkpoint { pos: self.pos }
    }

    /// Restores the cursor to a previously saved checkpoint.
    #[inline]
    pub fn backtrack(&mut self, cp: Checkpoint) {
        self.pos = cp.pos;
    }

    /// Attempts speculative parsing: runs `f`, on failure
    /// rolls back both position and diagnostics.
    ///
    /// Diagnostics produced inside `f` are **discarded** on failure.
    pub fn try_parse<T>(&mut self, f: impl FnOnce(&mut Self) -> ParseResult<T>) -> Option<T> {
        let cp = self.checkpoint();
        let saved = std::mem::replace(&mut self.diagnostics, Diagnostics::new());
        let saved_recovery = self.error_recovery;
        self.error_recovery = false;

        let result = f(self);

        self.error_recovery = saved_recovery;
        match result {
            Ok(val) => {
                self.diagnostics = saved;
                Some(val)
            }
            Err(_) => {
                self.backtrack(cp);
                self.diagnostics = saved;
                None
            }
        }
    }

    /// Like `try_parse`, but returns `Result` instead of `Option`,
    /// preserving the error for the caller to inspect.
    pub fn try_parse_result<T>(
        &mut self,
        f: impl FnOnce(&mut Self) -> ParseResult<T>,
    ) -> ParseResult<T> {
        let cp = self.checkpoint();
        let saved = std::mem::replace(&mut self.diagnostics, Diagnostics::new());
        let saved_recovery = self.error_recovery;
        self.error_recovery = false;

        let result = f(self);

        self.error_recovery = saved_recovery;
        match result {
            Ok(val) => {
                self.diagnostics = saved;
                Ok(val)
            }
            Err(e) => {
                self.backtrack(cp);
                self.diagnostics = saved;
                Err(e)
            }
        }
    }

    // =========================================================================
    //         SECTION: SPAN HELPERS
    // =========================================================================

    /// Records the current position as a span start.
    ///
    /// Usage pattern:
    /// ```ignore
    /// let m = parser.mark();
    /// // ... parse some construct ...
    /// let span = parser.since(m);
    /// ```
    #[inline]
    pub fn mark(&self) -> Mark {
        Mark { pos: self.pos }
    }

    /// Builds a [`Span`] from `mark` to the **last consumed** token.
    ///
    /// If nothing was consumed since the mark, returns the span of
    /// the token at the mark position.
    pub fn since(&self, mark: Mark) -> Span {
        let start = self.tokens[mark.pos].span.start;
        let end_idx = if self.pos > 0 { self.pos - 1 } else { 0 };
        let end = self.tokens[end_idx.max(mark.pos)].span.end;
        Span::new(start, end)
    }

    /// Builds a span from a raw [`Position`] to the last consumed token.
    pub fn span_from(&self, start: Position) -> Span {
        let end_idx = if self.pos > 0 { self.pos - 1 } else { 0 };
        Span::new(start, self.tokens[end_idx].span.end)
    }

    // =========================================================================
    //         SECTION: ERROR RECOVERY
    // =========================================================================

    /// Skips tokens until the cursor lands on one of `sync_tokens`
    /// (the sync token is **not** consumed).
    pub fn recover_to(&mut self, sync_tokens: &[Token]) {
        while !self.is_eof() && !self.check_any(sync_tokens) {
            self.advance();
        }
    }

    /// Skips tokens until `token` is found, then **consumes** that token.
    pub fn recover_past(&mut self, token: &Token) {
        while !self.is_eof() {
            if self.check(token) {
                self.advance();
                return;
            }
            self.advance();
        }
    }

    /// Reports an error.
    ///
    /// - **Recovery mode**: pushes into diagnostics, returns `Ok(())`.
    /// - **Normal mode**: returns `Err(err)` immediately.
    pub fn report_error(&mut self, err: ParseError) -> ParseResult<()> {
        if self.error_recovery {
            self.diagnostics.error(err);
            Ok(())
        } else {
            Err(err)
        }
    }

    /// Reports a warning (always accumulated, never returned as `Err`).
    pub fn report_warning(&mut self, warn: ParseError) {
        self.diagnostics.warning(warn);
    }

    // =========================================================================
    //         SECTION: CONVENIENCE COMBINATORS
    // =========================================================================

    /// Parses a comma-separated list of `element` until `end` token.
    ///
    /// The `end` token is **not** consumed. Useful for argument lists,
    /// parameter lists, field lists, etc.
    ///
    /// ```ignore
    /// parser.expect(&Token::LParen, "(")?;
    /// let items = parser.comma_sep(&Token::RParen, |p| p.parse_expr())?;
    /// parser.expect(&Token::RParen, ")")?;
    /// ```
    pub fn comma_sep<T>(
        &mut self,
        end: &Token,
        mut element: impl FnMut(&mut Self) -> ParseResult<T>,
    ) -> ParseResult<Vec<T>> {
        let mut items = Vec::new();
        if !self.check(end) {
            loop {
                items.push(element(self)?);
                if !self.match_token(&Token::Comma) {
                    break;
                }
            }
        }
        Ok(items)
    }

    /// Parses zero-or-more elements separated by newlines until a
    /// stop-token is reached (not consumed).
    ///
    /// Automatically skips blank lines and comments between elements.
    pub fn many_until<T>(
        &mut self,
        stop: &[Token],
        mut element: impl FnMut(&mut Self) -> ParseResult<T>,
    ) -> ParseResult<Vec<T>> {
        let mut items = Vec::new();
        while !self.is_eof() {
            if stop.iter().any(|t| self.check(t)) {
                break;
            }
            if matches!(self.peek(), Token::Newline | Token::Comment(_)) {
                self.advance();
                continue;
            }
            items.push(element(self)?);
        }
        Ok(items)
    }

    // =========================================================================
    //         SECTION: TOKEN CLASSIFICATION
    // =========================================================================

    /// `true` if the current token can start an expression.
    #[inline]
    pub fn at_expr_start(&self) -> bool {
        matches!(
            self.peek(),
            Token::IntLiteral(_)
                | Token::FloatLiteral(_)
                | Token::StringLiteral(_)
                | Token::CharLiteral(_)
                | Token::RawStringLiteral(_)
                | Token::InterpolatedStringStart
                | Token::True
                | Token::False
                | Token::None
                | Token::NotImplemented
                | Token::Ident(_)
                | Token::VarIdent(_)
                | Token::FuncIdent(_)
                | Token::TypeIdent(_)
                | Token::ClassIdent(_)
                | Token::NamespaceIdent(_)
                | Token::LParen
                | Token::LBracket
                | Token::Minus
                | Token::Not
                | Token::Ampersand
                | Token::Caret
                | Token::New
                | Token::Lambda
                | Token::If
                | Token::Self_
                | Token::This
                | Token::Super
        )
    }

    /// `true` if the current token can start a statement.
    #[inline]
    pub fn at_stmt_start(&self) -> bool {
        matches!(
            self.peek(),
            Token::If
                | Token::Loop
                | Token::For
                | Token::While
                | Token::Switch
                | Token::Input
                | Token::Output
                | Token::Return
                | Token::ResultValue
                | Token::Assert
                | Token::Halt
                | Token::Pause
                | Token::Try
                | Token::Throw
                | Token::Delete
                | Token::Defer
                | Token::Await
                | Token::Match
                | Token::IntType
                | Token::FloatType
                | Token::BoolType
                | Token::CharType
                | Token::StringType
                | Token::ArrayType
                | Token::AutoType
                | Token::PointerType
                | Token::Alg
                | Token::Import
                | Token::Use
                | Token::RustBlockStart
                | Token::Rust
                | Token::Ident(_)
                | Token::VarIdent(_)
                | Token::FuncIdent(_)
                | Token::TypeIdent(_)
                | Token::ClassIdent(_)
                | Token::NamespaceIdent(_)
                | Token::Self_
                | Token::This
        )
    }

    /// `true` if the current token can start a top-level declaration.
    #[inline]
    pub fn at_decl_start(&self) -> bool {
        matches!(
            self.peek(),
            Token::Alg
                | Token::Import
                | Token::Use
                | Token::Class
                | Token::Struct
                | Token::Interface
                | Token::EnumType
                | Token::Module
                | Token::Abstract
                | Token::Final
                | Token::IntType
                | Token::FloatType
                | Token::BoolType
                | Token::CharType
                | Token::StringType
                | Token::ArrayType
                | Token::AutoType
                | Token::PointerType
        )
    }

    /// Skips tokens until `pred` returns `true` for the current token.
    /// Stops at EOF.
    pub fn skip_until(&mut self, pred: impl Fn(&Token) -> bool) {
        while !self.is_eof() && !pred(self.peek()) {
            self.advance();
        }
    }

    // =========================================================================
    //         SECTION: DIAGNOSTICS ACCESS
    // =========================================================================

    /// Returns `true` if error-recovery mode is active.
    #[inline]
    pub fn is_recovery_mode(&self) -> bool {
        self.error_recovery
    }

    /// Returns `true` if any errors have been accumulated.
    #[inline]
    pub fn has_errors(&self) -> bool {
        self.diagnostics.has_errors()
    }

    /// Takes accumulated diagnostics, replacing with an empty collector.
    pub fn take_diagnostics(&mut self) -> Diagnostics {
        std::mem::replace(&mut self.diagnostics, Diagnostics::new())
    }

    /// Immutable reference to diagnostics.
    #[inline]
    pub fn diagnostics(&self) -> &Diagnostics {
        &self.diagnostics
    }
}
