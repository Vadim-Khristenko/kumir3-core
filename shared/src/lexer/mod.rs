// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

//! Kumir 3 Lexer
//!
//! [STABLE] Lexical analyzer for the Kumir language (v2 and v3 dialects).
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │ Source Code (UTF-8)                                             │
//! └─────────────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │ Lexer                                                           │
//! │   - Character iteration with lookahead                          │
//! │   - Position tracking (line, column, offset)                    │
//! │   - State machines for strings, comments, Rust blocks           │
//! └─────────────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │ Vec<SpannedToken>                                               │
//! │   - Token variant (keyword, literal, operator, etc.)            │
//! │   - Source span for error reporting                             │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Features
//!
//! - Full Unicode support (Cyrillic identifiers)
//! - Kumir 2/3 keyword recognition  
//! - String literals: `"..."`, `"""..."""`, `r"..."`, `r#"..."#`, `f"...{expr}..."`
//! - Number literals: decimal, hex (0x), binary (0b), octal (0o), float, scientific
//! - Rust code embedding: `РастВставкаНЦ...РастВставкаКЦ`, `ржавчина нач...кон`
//! - Comments: `| ...`, `// ...`, `/// ...` (doc)
//! - All Kumir 3 operators and delimiters

use crate::constants::{is_digit_start, is_ident_start, is_whitespace};
use crate::types::Token;

mod comments;
mod identifiers;
mod numbers;
mod operators;
mod rust_blocks;
mod scanner;
mod strings;

// =============================================================================
//         SECTION: POSITION
// =============================================================================

/// Source code position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Position {
    /// Line number (1-based)
    pub line: usize,
    /// Column number (1-based, in characters)
    pub column: usize,
    /// Byte offset from source start
    pub offset: usize,
}

impl Position {
    /// Creates a new position.
    #[inline]
    pub const fn new(line: usize, column: usize, offset: usize) -> Self {
        Self {
            line,
            column,
            offset,
        }
    }

    /// Creates position at start of file.
    #[inline]
    pub const fn start() -> Self {
        Self {
            line: 1,
            column: 1,
            offset: 0,
        }
    }
}

impl std::fmt::Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

// =============================================================================
//         SECTION: SPAN
// =============================================================================

/// Source code span (range of positions).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    /// Start position (inclusive)
    pub start: Position,
    /// End position (exclusive)
    pub end: Position,
}

impl Span {
    /// Creates a new span.
    #[inline]
    pub const fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    /// Creates a zero-length span at position.
    #[inline]
    pub const fn point(pos: Position) -> Self {
        Self {
            start: pos,
            end: pos,
        }
    }

    /// Merges two spans into one covering both.
    pub fn merge(self, other: Span) -> Span {
        Span {
            start: if self.start.offset <= other.start.offset {
                self.start
            } else {
                other.start
            },
            end: if self.end.offset >= other.end.offset {
                self.end
            } else {
                other.end
            },
        }
    }

    /// Returns the length in bytes.
    #[inline]
    pub fn len(&self) -> usize {
        self.end.offset.saturating_sub(self.start.offset)
    }

    /// Checks if span is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.start.offset >= self.end.offset
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.start.line == self.end.line {
            write!(
                f,
                "{}:{}-{}",
                self.start.line, self.start.column, self.end.column
            )
        } else {
            write!(f, "{}-{}", self.start, self.end)
        }
    }
}

// =============================================================================
//         SECTION: SPANNED TOKEN
// =============================================================================

/// Token with source location information.
#[derive(Debug, Clone, PartialEq)]
pub struct SpannedToken {
    /// The token variant
    pub token: Token,
    /// Source span
    pub span: Span,
}

impl SpannedToken {
    /// Creates a new spanned token.
    #[inline]
    pub const fn new(token: Token, span: Span) -> Self {
        Self { token, span }
    }

    /// Checks if this is a specific token variant.
    #[inline]
    pub fn is(&self, token: &Token) -> bool {
        std::mem::discriminant(&self.token) == std::mem::discriminant(token)
    }

    /// Checks if this is EOF.
    #[inline]
    pub fn is_eof(&self) -> bool {
        matches!(self.token, Token::EOF)
    }

    /// Checks if this is a newline.
    #[inline]
    pub fn is_newline(&self) -> bool {
        matches!(self.token, Token::Newline)
    }

    /// Checks if this is a comment.
    #[inline]
    pub fn is_comment(&self) -> bool {
        matches!(self.token, Token::Comment(_) | Token::DocComment(_))
    }
}

// =============================================================================
//         SECTION: LEXER ERROR
// =============================================================================

/// Lexical analysis error.
#[derive(Debug, Clone, PartialEq)]
pub struct LexerError {
    /// Error kind
    pub kind: LexerErrorKind,
    /// Error message
    pub message: String,
    /// Error position
    pub position: Position,
    /// Optional span (for errors covering multiple characters)
    pub span: Option<Span>,
}

/// Kinds of lexer errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LexerErrorKind {
    /// Unexpected character
    UnexpectedChar,
    /// Unterminated string literal
    UnterminatedString,
    /// Unterminated character literal
    UnterminatedChar,
    /// Invalid escape sequence
    InvalidEscape,
    /// Invalid number format
    InvalidNumber,
    /// Unterminated Rust block
    UnterminatedRustBlock,
    /// Empty character literal
    EmptyCharLiteral,
    /// Multi-character in char literal
    MultiCharLiteral,
    /// Invalid Unicode escape
    InvalidUnicodeEscape,
    /// Unexpected EOF
    UnexpectedEof,
}

impl LexerError {
    /// Creates a new lexer error.
    pub fn new(kind: LexerErrorKind, message: impl Into<String>, position: Position) -> Self {
        Self {
            kind,
            message: message.into(),
            position,
            span: None,
        }
    }

    /// Creates error with span.
    pub fn with_span(kind: LexerErrorKind, message: impl Into<String>, span: Span) -> Self {
        Self {
            kind,
            message: message.into(),
            position: span.start,
            span: Some(span),
        }
    }
}

impl std::fmt::Display for LexerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}] {} at {}",
            match self.kind {
                LexerErrorKind::UnexpectedChar => "E001",
                LexerErrorKind::UnterminatedString => "E002",
                LexerErrorKind::UnterminatedChar => "E003",
                LexerErrorKind::InvalidEscape => "E004",
                LexerErrorKind::InvalidNumber => "E005",
                LexerErrorKind::UnterminatedRustBlock => "E006",
                LexerErrorKind::EmptyCharLiteral => "E007",
                LexerErrorKind::MultiCharLiteral => "E008",
                LexerErrorKind::InvalidUnicodeEscape => "E009",
                LexerErrorKind::UnexpectedEof => "E010",
            },
            self.message,
            self.position
        )
    }
}

impl std::error::Error for LexerError {}

/// Lexer result type.
pub type LexerResult<T> = Result<T, LexerError>;

// =============================================================================
//         SECTION: LEXER STATE
// =============================================================================

/// Internal lexer state for special contexts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum LexerState {
    /// Normal tokenization
    #[default]
    Normal,
    /// Inside Rust block (РастВставкаНЦ...РастВставкаКЦ)
    RustBlock,
    /// Inside alternative Rust block (ржавчина нач...кон)
    RustAltBlock,
    /// Inside interpolated string text portion
    InterpolatedStringText,
    /// Inside interpolated string expression (after {)
    InterpolatedStringExpr {
        /// Brace nesting depth (starts at 1 after opening {)
        brace_depth: usize,
    },
}

// =============================================================================
//         SECTION: LEXER
// =============================================================================

/// Kumir language lexer.
///
/// Transforms source code into a stream of tokens.
///
/// # Example
///
/// ```ignore
/// use kumir3_shared::lexer::Lexer;
///
/// let source = "алг Привет\nнач\n  вывод \"Hello\"\nкон";
/// let mut lexer = Lexer::new(source);
/// let tokens = lexer.tokenize()?;
///
/// for tok in &tokens {
///     println!("{:?} at {}", tok.token, tok.span);
/// }
/// ```
pub struct Lexer<'a> {
    /// Source code
    source: &'a str,
    /// Source as bytes for fast indexing
    bytes: &'a [u8],
    /// Current byte position
    pos: usize,
    /// Current position (line, column)
    position: Position,
    /// Lexer state
    state: LexerState,
    /// Accumulated errors (for error recovery mode)
    errors: Vec<LexerError>,
    /// Enable error recovery (continue after errors)
    error_recovery: bool,
}

impl<'a> Lexer<'a> {
    /// Creates a new lexer for the given source code.
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
            pos: 0,
            position: Position::start(),
            state: LexerState::Normal,
            errors: Vec::new(),
            error_recovery: false,
        }
    }

    /// Enables error recovery mode.
    pub fn with_error_recovery(mut self) -> Self {
        self.error_recovery = true;
        self
    }

    /// Returns accumulated errors.
    pub fn errors(&self) -> &[LexerError] {
        &self.errors
    }

    /// Tokenizes the entire source code.
    pub fn tokenize(&mut self) -> LexerResult<Vec<SpannedToken>> {
        let mut tokens = Vec::new();

        loop {
            match self.next_token() {
                Ok(Some(tok)) => {
                    let is_eof = tok.is_eof();
                    tokens.push(tok);
                    if is_eof {
                        break;
                    }
                }
                Ok(None) => continue,
                Err(e) => {
                    if self.error_recovery {
                        self.errors.push(e);
                        self.advance(); // Skip problematic character
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        Ok(tokens)
    }

    /// Returns an iterator over tokens.
    pub fn tokens(self) -> TokenIterator<'a> {
        TokenIterator {
            lexer: self,
            done: false,
        }
    }

    // =========================================================================
    //         CORE SCANNING
    // =========================================================================

    /// Returns the next token or None for skipped elements.
    pub fn next_token(&mut self) -> LexerResult<Option<SpannedToken>> {
        self.skip_whitespace();

        if self.is_eof() {
            return Ok(Some(SpannedToken::new(
                Token::EOF,
                Span::point(self.position),
            )));
        }

        let start = self.position;

        // Handle special states
        match self.state {
            LexerState::RustBlock => return self.scan_rust_block(start),
            LexerState::RustAltBlock => return self.scan_rust_alt_block(start),
            LexerState::InterpolatedStringText => return self.scan_interpolated_text(start),
            LexerState::InterpolatedStringExpr { .. } => return self.scan_interpolated_expr(start),
            LexerState::Normal => {}
        }

        let c = self.peek().unwrap();

        // Newline (significant in Kumir)
        if c == '\n' {
            self.advance();
            return Ok(Some(SpannedToken::new(
                Token::Newline,
                Span::new(start, self.position),
            )));
        }

        // Comments
        if c == '|' && self.peek_at(1) != Some('>') {
            return self.scan_comment(start);
        }
        if c == '/' && self.peek_at(1) == Some('/') {
            return self.scan_line_comment(start);
        }

        // String literals
        if c == '"' {
            return self.scan_string(start);
        }
        if c == 'r' && matches!(self.peek_at(1), Some('"') | Some('#')) {
            return self.scan_raw_string(start);
        }
        if c == 'f' && self.peek_at(1) == Some('"') {
            return self.scan_interpolated_string(start);
        }

        // Character literal
        if c == '\'' {
            return self.scan_char(start);
        }

        // Number literal
        if is_digit_start(c)
            || (c == '.' && self.peek_at(1).map(|c| c.is_ascii_digit()).unwrap_or(false))
        {
            return self.scan_number(start);
        }

        // Identifier or keyword
        if is_ident_start(c) {
            return self.scan_identifier(start);
        }

        // Operators and delimiters
        if let Some(tok) = self.try_scan_operator(start)? {
            return Ok(Some(tok));
        }

        // Unknown character
        self.advance();
        Err(LexerError::new(
            LexerErrorKind::UnexpectedChar,
            format!("Unexpected character: '{}'", c),
            start,
        ))
    }
}

// =============================================================================
//         SECTION: TOKEN ITERATOR
// =============================================================================

/// Iterator over tokens.
pub struct TokenIterator<'a> {
    lexer: Lexer<'a>,
    done: bool,
}

impl<'a> Iterator for TokenIterator<'a> {
    type Item = LexerResult<SpannedToken>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        loop {
            match self.lexer.next_token() {
                Ok(Some(tok)) => {
                    if tok.is_eof() {
                        self.done = true;
                    }
                    return Some(Ok(tok));
                }
                Ok(None) => continue,
                Err(e) => {
                    self.done = true;
                    return Some(Err(e));
                }
            }
        }
    }
}

// =============================================================================
//         SECTION: CONVENIENCE FUNCTIONS
// =============================================================================

/// Tokenizes source code.
///
/// # Example
///
/// ```ignore
/// let tokens = tokenize("алг Тест\nнач\nкон")?;
/// ```
pub fn tokenize(source: &str) -> LexerResult<Vec<SpannedToken>> {
    Lexer::new(source).tokenize()
}

/// Tokenizes source code with error recovery.
///
/// Returns tokens and accumulated errors.
pub fn tokenize_with_recovery(source: &str) -> (Vec<SpannedToken>, Vec<LexerError>) {
    let mut lexer = Lexer::new(source).with_error_recovery();
    let tokens = lexer.tokenize().unwrap_or_default();
    let errors = lexer.errors.clone();
    (tokens, errors)
}

#[cfg(test)]
mod tests;
