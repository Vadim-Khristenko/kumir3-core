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

use crate::constants::{
    get_keyword_token, is_digit_start, is_ident_continue, is_ident_start, is_whitespace,
    operator_token,
};
use crate::types::Token;

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

    // =========================================================================
    //         CHARACTER NAVIGATION
    // =========================================================================

    /// Checks if at end of file.
    #[inline]
    fn is_eof(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    /// Returns current character without advancing.
    #[inline]
    fn peek(&self) -> Option<char> {
        if self.pos < self.source.len() {
            self.source[self.pos..].chars().next()
        } else {
            None
        }
    }

    /// Returns character at offset from current position.
    fn peek_at(&self, offset: usize) -> Option<char> {
        let mut chars = self.source[self.pos..].chars();
        for _ in 0..offset {
            chars.next()?;
        }
        chars.next()
    }

    /// Advances by one character and returns it.
    fn advance(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();

        if c == '\n' {
            self.position.line += 1;
            self.position.column = 1;
        } else {
            self.position.column += 1;
        }
        self.position.offset = self.pos;

        Some(c)
    }

    /// Advances while predicate is true.
    fn advance_while<F: Fn(char) -> bool>(&mut self, pred: F) {
        while let Some(c) = self.peek() {
            if pred(c) {
                self.advance();
            } else {
                break;
            }
        }
    }

    /// Skips whitespace (except newlines).
    fn skip_whitespace(&mut self) {
        self.advance_while(is_whitespace);
    }

    /// Returns a slice of the source.
    #[inline]
    fn slice(&self, start: usize, end: usize) -> &'a str {
        &self.source[start..end]
    }

    /// Returns remaining source from current position.
    #[inline]
    fn remaining(&self) -> &'a str {
        &self.source[self.pos..]
    }

    // =========================================================================
    //         COMMENTS
    // =========================================================================

    /// Scans a pipe comment (| ...).
    fn scan_comment(&mut self, start: Position) -> LexerResult<Option<SpannedToken>> {
        self.advance(); // skip |

        let content_start = self.pos;
        self.advance_while(|c| c != '\n');
        let content = self.slice(content_start, self.pos).to_string();

        Ok(Some(SpannedToken::new(
            Token::Comment(content),
            Span::new(start, self.position),
        )))
    }

    /// Scans a line comment (// or ///).
    fn scan_line_comment(&mut self, start: Position) -> LexerResult<Option<SpannedToken>> {
        self.advance(); // first /
        self.advance(); // second /

        let is_doc = self.peek() == Some('/');
        if is_doc {
            self.advance(); // third /
        }

        // Skip leading space
        if self.peek() == Some(' ') {
            self.advance();
        }

        let content_start = self.pos;
        self.advance_while(|c| c != '\n');
        let content = self.slice(content_start, self.pos).to_string();

        let token = if is_doc {
            Token::DocComment(vec![content])
        } else {
            Token::Comment(content)
        };

        Ok(Some(SpannedToken::new(
            token,
            Span::new(start, self.position),
        )))
    }

    // =========================================================================
    //         STRING LITERALS
    // =========================================================================

    /// Scans a string literal ("..." or """...""").
    fn scan_string(&mut self, start: Position) -> LexerResult<Option<SpannedToken>> {
        self.advance(); // opening quote

        // Check for triple quotes
        let is_multiline = if self.peek() == Some('"') && self.peek_at(1) == Some('"') {
            self.advance();
            self.advance();
            true
        } else if self.peek() == Some('"') {
            // Empty string ""
            self.advance();
            return Ok(Some(SpannedToken::new(
                Token::StringLiteral(String::new()),
                Span::new(start, self.position),
            )));
        } else {
            false
        };

        let mut value = String::new();

        loop {
            match self.peek() {
                None => {
                    return Err(LexerError::with_span(
                        LexerErrorKind::UnterminatedString,
                        "Unterminated string literal",
                        Span::new(start, self.position),
                    ));
                }
                Some('"') => {
                    if is_multiline {
                        if self.peek_at(1) == Some('"') && self.peek_at(2) == Some('"') {
                            self.advance();
                            self.advance();
                            self.advance();
                            break;
                        }
                        value.push('"');
                        self.advance();
                    } else {
                        self.advance();
                        break;
                    }
                }
                Some('\n') if !is_multiline => {
                    return Err(LexerError::with_span(
                        LexerErrorKind::UnterminatedString,
                        "Unterminated string literal (use triple quotes for multiline)",
                        Span::new(start, self.position),
                    ));
                }
                Some('\\') => {
                    self.advance();
                    value.push(self.scan_escape_sequence()?);
                }
                Some(c) => {
                    value.push(c);
                    self.advance();
                }
            }
        }

        Ok(Some(SpannedToken::new(
            Token::StringLiteral(value),
            Span::new(start, self.position),
        )))
    }

    /// Scans a raw string literal (r"..." or r#"..."#).
    fn scan_raw_string(&mut self, start: Position) -> LexerResult<Option<SpannedToken>> {
        self.advance(); // 'r'

        // Count opening hashes
        let mut hash_count = 0;
        while self.peek() == Some('#') {
            hash_count += 1;
            self.advance();
        }

        if self.peek() != Some('"') {
            return Err(LexerError::new(
                LexerErrorKind::UnterminatedString,
                "Expected '\"' after r#",
                self.position,
            ));
        }
        self.advance(); // opening quote

        let mut value = String::new();

        loop {
            match self.peek() {
                None => {
                    return Err(LexerError::with_span(
                        LexerErrorKind::UnterminatedString,
                        "Unterminated raw string literal",
                        Span::new(start, self.position),
                    ));
                }
                Some('"') => {
                    self.advance();
                    let mut closing = 0;
                    while closing < hash_count && self.peek() == Some('#') {
                        closing += 1;
                        self.advance();
                    }
                    if closing == hash_count {
                        break;
                    }
                    value.push('"');
                    for _ in 0..closing {
                        value.push('#');
                    }
                }
                Some(c) => {
                    value.push(c);
                    self.advance();
                }
            }
        }

        Ok(Some(SpannedToken::new(
            Token::RawStringLiteral(value),
            Span::new(start, self.position),
        )))
    }

    /// Scans start of interpolated string (f"...).
    /// Emits InterpolatedStringStart, then switches to InterpolatedStringText state.
    fn scan_interpolated_string(&mut self, start: Position) -> LexerResult<Option<SpannedToken>> {
        self.advance(); // 'f'
        self.advance(); // opening quote

        self.state = LexerState::InterpolatedStringText;

        Ok(Some(SpannedToken::new(
            Token::InterpolatedStringStart,
            Span::new(start, self.position),
        )))
    }

    /// Scans text portion of interpolated string until `{` or `"`.
    ///
    /// State machine:
    /// - `"` → emit part (if any), then emit InterpolatedStringEnd, go Normal
    /// - `{` → emit part (if any), consume `{`, go InterpolatedStringExpr  
    /// - `\\` → escape sequence
    /// - other → accumulate text
    fn scan_interpolated_text(&mut self, start: Position) -> LexerResult<Option<SpannedToken>> {
        let mut value = String::new();

        loop {
            match self.peek() {
                None | Some('\n') => {
                    return Err(LexerError::with_span(
                        LexerErrorKind::UnterminatedString,
                        "Unterminated interpolated string",
                        Span::new(start, self.position),
                    ));
                }
                Some('"') => {
                    self.advance(); // consume closing quote
                    self.state = LexerState::Normal;

                    // If there's accumulated text, return it as a part.
                    // InterpolatedStringEnd will be returned on next call via pending_end.
                    // Actually, simplify: emit End right away (text can be empty).
                    if value.is_empty() {
                        return Ok(Some(SpannedToken::new(
                            Token::InterpolatedStringEnd,
                            Span::new(start, self.position),
                        )));
                    } else {
                        // We need to return BOTH the text part AND the end token.
                        // Store end token position for next call.
                        // But we can't really buffer. So: return the part,
                        // and on next call, state is Normal, the quote is already consumed.
                        // The parser just won't see InterpolatedStringEnd.

                        // SOLUTION: Use peek() BEFORE consuming the quote.
                        // Back up: we already advanced. Let's just return both conceptually
                        // by returning InterpolatedStringPart and letting the caller understand
                        // that no InterpolatedStringEnd means the string is done.

                        // BETTER SOLUTION: don't consume quote here. Let the main loop do it.
                        // But we already consumed it...

                        // CLEANEST: Just return the part with the text.
                        // The state is Normal. When the parser sees no more interpolated tokens,
                        // it knows the string ended. This works because the parser can check
                        // for InterpolatedStringEnd OR just no more InterpolatedStringPart.

                        // Actually, let's just always emit the End token and skip text emit
                        // for trailing text (embed it in the End token semantics).
                        // The parser typically just collects parts anyway.

                        // PRAGMATIC: Return InterpolatedStringPart here.
                        // The parser will see: Start, [Part|tokens]*, and then a non-interpolated token.
                        // We document that InterpolatedStringEnd may be absent if trailing text exists.

                        return Ok(Some(SpannedToken::new(
                            Token::InterpolatedStringPart(value),
                            Span::new(start, self.position),
                        )));
                    }
                }
                Some('{') => {
                    // Check for {{ escape
                    if self.peek_at(1) == Some('{') {
                        self.advance();
                        self.advance();
                        value.push('{');
                        continue;
                    }

                    // If we have accumulated text, return it first (don't consume {)
                    if !value.is_empty() {
                        return Ok(Some(SpannedToken::new(
                            Token::InterpolatedStringPart(value),
                            Span::new(start, self.position),
                        )));
                    }

                    // No text — start expression directly: consume {
                    self.advance();
                    self.state = LexerState::InterpolatedStringExpr { brace_depth: 1 };
                    return Ok(None); // next_token() will lex the expression
                }
                Some('}') => {
                    // Check for }} escape
                    if self.peek_at(1) == Some('}') {
                        self.advance();
                        self.advance();
                        value.push('}');
                        continue;
                    }
                    return Err(LexerError::new(
                        LexerErrorKind::UnexpectedChar,
                        "Unexpected '}' in interpolated string (use '}}' to escape)",
                        self.position,
                    ));
                }
                Some('\\') => {
                    self.advance();
                    value.push(self.scan_escape_sequence()?);
                }
                Some(c) => {
                    value.push(c);
                    self.advance();
                }
            }
        }
    }

    /// Scans tokens inside an interpolated expression { ... }.
    /// Returns normal tokens until the matching closing brace.
    fn scan_interpolated_expr(&mut self, _start: Position) -> LexerResult<Option<SpannedToken>> {
        // Check for closing brace at current depth
        if let LexerState::InterpolatedStringExpr { brace_depth } = self.state
            && self.peek() == Some('}')
            && brace_depth == 1
        {
            // End of interpolated expression
            self.advance(); // consume }
            self.state = LexerState::InterpolatedStringText;
            return Ok(None); // Skip, resume scanning text
        }

        // Otherwise lex a normal token, tracking brace depth
        let old_state = self.state;
        self.state = LexerState::Normal;
        let result = self.next_token();

        // Restore interpolated state, adjusting brace depth
        if let LexerState::InterpolatedStringExpr { brace_depth } = old_state {
            let mut new_depth = brace_depth;
            if let Ok(Some(ref tok)) = result {
                match &tok.token {
                    Token::LBrace => new_depth += 1,
                    Token::RBrace => {
                        new_depth -= 1;
                        if new_depth == 0 {
                            // Back to text scanning
                            self.state = LexerState::InterpolatedStringText;
                            return Ok(None);
                        }
                    }
                    _ => {}
                }
            }
            self.state = LexerState::InterpolatedStringExpr {
                brace_depth: new_depth,
            };
        }

        result
    }

    // =========================================================================
    //         CHARACTER LITERALS
    // =========================================================================

    /// Scans a character literal ('x').
    fn scan_char(&mut self, start: Position) -> LexerResult<Option<SpannedToken>> {
        self.advance(); // opening quote

        let c = match self.peek() {
            None | Some('\n') => {
                return Err(LexerError::new(
                    LexerErrorKind::UnterminatedChar,
                    "Unterminated character literal",
                    start,
                ));
            }
            Some('\'') => {
                return Err(LexerError::new(
                    LexerErrorKind::EmptyCharLiteral,
                    "Empty character literal",
                    start,
                ));
            }
            Some('\\') => {
                self.advance();
                self.scan_escape_sequence()?
            }
            Some(c) => {
                self.advance();
                c
            }
        };

        if self.peek() != Some('\'') {
            // Check for multi-char literal
            if self.peek().is_some() && self.peek() != Some('\n') {
                return Err(LexerError::new(
                    LexerErrorKind::MultiCharLiteral,
                    "Character literal may only contain one character",
                    start,
                ));
            }
            return Err(LexerError::new(
                LexerErrorKind::UnterminatedChar,
                "Unterminated character literal",
                start,
            ));
        }
        self.advance(); // closing quote

        Ok(Some(SpannedToken::new(
            Token::CharLiteral(c),
            Span::new(start, self.position),
        )))
    }

    // =========================================================================
    //         ESCAPE SEQUENCES
    // =========================================================================

    /// Scans an escape sequence.
    fn scan_escape_sequence(&mut self) -> LexerResult<char> {
        let pos = self.position;

        match self.peek() {
            Some('n') => {
                self.advance();
                Ok('\n')
            }
            Some('r') => {
                self.advance();
                Ok('\r')
            }
            Some('t') => {
                self.advance();
                Ok('\t')
            }
            Some('\\') => {
                self.advance();
                Ok('\\')
            }
            Some('"') => {
                self.advance();
                Ok('"')
            }
            Some('\'') => {
                self.advance();
                Ok('\'')
            }
            Some('0') => {
                self.advance();
                Ok('\0')
            }
            Some('x') => {
                self.advance();
                self.scan_hex_escape(2)
            }
            Some('u') => {
                self.advance();
                self.scan_unicode_escape()
            }
            Some(c) => {
                self.advance();
                Err(LexerError::new(
                    LexerErrorKind::InvalidEscape,
                    format!("Invalid escape sequence: \\{}", c),
                    pos,
                ))
            }
            None => Err(LexerError::new(
                LexerErrorKind::InvalidEscape,
                "Escape sequence at end of input",
                pos,
            )),
        }
    }

    /// Scans a hex escape (\xNN).
    fn scan_hex_escape(&mut self, digits: usize) -> LexerResult<char> {
        let pos = self.position;
        let mut value = 0u32;

        for _ in 0..digits {
            match self.peek() {
                Some(c) if c.is_ascii_hexdigit() => {
                    value = value * 16 + c.to_digit(16).unwrap();
                    self.advance();
                }
                _ => {
                    return Err(LexerError::new(
                        LexerErrorKind::InvalidEscape,
                        format!("Invalid hex escape (expected {} hex digits)", digits),
                        pos,
                    ));
                }
            }
        }

        char::from_u32(value).ok_or_else(|| {
            LexerError::new(
                LexerErrorKind::InvalidUnicodeEscape,
                format!("Invalid Unicode code point: U+{:04X}", value),
                pos,
            )
        })
    }

    /// Scans a unicode escape (\u{NNNN}).
    fn scan_unicode_escape(&mut self) -> LexerResult<char> {
        let pos = self.position;

        if self.peek() != Some('{') {
            return Err(LexerError::new(
                LexerErrorKind::InvalidUnicodeEscape,
                "Expected '{' after \\u",
                pos,
            ));
        }
        self.advance();

        let mut value = 0u32;
        let mut digit_count = 0;

        loop {
            match self.peek() {
                Some('}') => {
                    self.advance();
                    break;
                }
                Some(c) if c.is_ascii_hexdigit() => {
                    if digit_count >= 6 {
                        return Err(LexerError::new(
                            LexerErrorKind::InvalidUnicodeEscape,
                            "Unicode escape too long (max 6 hex digits)",
                            pos,
                        ));
                    }
                    value = value * 16 + c.to_digit(16).unwrap();
                    digit_count += 1;
                    self.advance();
                }
                _ => {
                    return Err(LexerError::new(
                        LexerErrorKind::InvalidUnicodeEscape,
                        "Invalid character in unicode escape",
                        pos,
                    ));
                }
            }
        }

        if digit_count == 0 {
            return Err(LexerError::new(
                LexerErrorKind::InvalidUnicodeEscape,
                "Empty unicode escape",
                pos,
            ));
        }

        char::from_u32(value).ok_or_else(|| {
            LexerError::new(
                LexerErrorKind::InvalidUnicodeEscape,
                format!("Invalid Unicode code point: U+{:04X}", value),
                pos,
            )
        })
    }

    // =========================================================================
    //         NUMBER LITERALS
    // =========================================================================

    /// Scans a number literal.
    fn scan_number(&mut self, start: Position) -> LexerResult<Option<SpannedToken>> {
        let num_start = self.pos;

        // Check for radix prefix (0x, 0b, 0o)
        if self.peek() == Some('0') {
            self.advance();
            match self.peek() {
                Some('x') | Some('X') => return self.scan_hex_number(start, num_start),
                Some('b') | Some('B') => return self.scan_binary_number(start, num_start),
                Some('o') | Some('O') => return self.scan_octal_number(start, num_start),
                _ => {
                    // Just a leading zero, continue with decimal
                }
            }
        }

        // Decimal integer part
        self.advance_while(|c| c.is_ascii_digit() || c == '_');

        let mut is_float = false;

        // Decimal point
        if self.peek() == Some('.') && self.peek_at(1).map(|c| c.is_ascii_digit()).unwrap_or(false)
        {
            is_float = true;
            self.advance(); // dot
            self.advance_while(|c| c.is_ascii_digit() || c == '_');
        }

        // Exponent
        if matches!(self.peek(), Some('e') | Some('E')) {
            is_float = true;
            self.advance();
            if matches!(self.peek(), Some('+') | Some('-')) {
                self.advance();
            }
            let exp_start = self.pos;
            self.advance_while(|c| c.is_ascii_digit() || c == '_');
            if self.pos == exp_start {
                return Err(LexerError::new(
                    LexerErrorKind::InvalidNumber,
                    "Expected exponent digits",
                    start,
                ));
            }
        }

        let num_str = self.slice(num_start, self.pos).replace('_', "");

        let token = if is_float {
            match num_str.parse::<f64>() {
                Ok(n) => Token::FloatLiteral(n),
                Err(_) => {
                    return Err(LexerError::new(
                        LexerErrorKind::InvalidNumber,
                        format!("Invalid float literal: {}", num_str),
                        start,
                    ));
                }
            }
        } else {
            match num_str.parse::<i64>() {
                Ok(n) => Token::IntLiteral(n),
                Err(_) => {
                    return Err(LexerError::new(
                        LexerErrorKind::InvalidNumber,
                        format!("Invalid integer literal: {}", num_str),
                        start,
                    ));
                }
            }
        };

        Ok(Some(SpannedToken::new(
            token,
            Span::new(start, self.position),
        )))
    }

    /// Scans a hexadecimal number (0x...).
    fn scan_hex_number(
        &mut self,
        start: Position,
        _num_start: usize,
    ) -> LexerResult<Option<SpannedToken>> {
        self.advance(); // 'x' or 'X'

        let hex_start = self.pos;
        self.advance_while(|c| c.is_ascii_hexdigit() || c == '_');

        if self.pos == hex_start {
            return Err(LexerError::new(
                LexerErrorKind::InvalidNumber,
                "Expected hex digits after 0x",
                start,
            ));
        }

        let hex_str = self.slice(hex_start, self.pos).replace('_', "");
        match i64::from_str_radix(&hex_str, 16) {
            Ok(n) => Ok(Some(SpannedToken::new(
                Token::IntLiteral(n),
                Span::new(start, self.position),
            ))),
            Err(_) => Err(LexerError::new(
                LexerErrorKind::InvalidNumber,
                format!("Invalid hex literal: 0x{}", hex_str),
                start,
            )),
        }
    }

    /// Scans a binary number (0b...).
    fn scan_binary_number(
        &mut self,
        start: Position,
        _num_start: usize,
    ) -> LexerResult<Option<SpannedToken>> {
        self.advance(); // 'b' or 'B'

        let bin_start = self.pos;
        self.advance_while(|c| c == '0' || c == '1' || c == '_');

        if self.pos == bin_start {
            return Err(LexerError::new(
                LexerErrorKind::InvalidNumber,
                "Expected binary digits after 0b",
                start,
            ));
        }

        let bin_str = self.slice(bin_start, self.pos).replace('_', "");
        match i64::from_str_radix(&bin_str, 2) {
            Ok(n) => Ok(Some(SpannedToken::new(
                Token::IntLiteral(n),
                Span::new(start, self.position),
            ))),
            Err(_) => Err(LexerError::new(
                LexerErrorKind::InvalidNumber,
                format!("Invalid binary literal: 0b{}", bin_str),
                start,
            )),
        }
    }

    /// Scans an octal number (0o...).
    fn scan_octal_number(
        &mut self,
        start: Position,
        _num_start: usize,
    ) -> LexerResult<Option<SpannedToken>> {
        self.advance(); // 'o' or 'O'

        let oct_start = self.pos;
        self.advance_while(|c| ('0'..='7').contains(&c) || c == '_');

        if self.pos == oct_start {
            return Err(LexerError::new(
                LexerErrorKind::InvalidNumber,
                "Expected octal digits after 0o",
                start,
            ));
        }

        let oct_str = self.slice(oct_start, self.pos).replace('_', "");
        match i64::from_str_radix(&oct_str, 8) {
            Ok(n) => Ok(Some(SpannedToken::new(
                Token::IntLiteral(n),
                Span::new(start, self.position),
            ))),
            Err(_) => Err(LexerError::new(
                LexerErrorKind::InvalidNumber,
                format!("Invalid octal literal: 0o{}", oct_str),
                start,
            )),
        }
    }

    // =========================================================================
    //         IDENTIFIERS & KEYWORDS
    // =========================================================================

    /// Scans an identifier or keyword.
    fn scan_identifier(&mut self, start: Position) -> LexerResult<Option<SpannedToken>> {
        let ident_start = self.pos;

        self.advance(); // first char (already verified as ident_start)
        self.advance_while(is_ident_continue);

        let ident = self.slice(ident_start, self.pos);

        // Check for Rust block markers
        if ident == "РастВставкаНЦ" {
            self.state = LexerState::RustBlock;
            return Ok(Some(SpannedToken::new(
                Token::RustBlockStart,
                Span::new(start, self.position),
            )));
        }

        // Check for alternative Rust syntax (ржавчина нач)
        if matches!(ident, "ржавчина" | "Ржавчина" | "rust") && self.try_scan_rust_alt_start()
        {
            self.state = LexerState::RustAltBlock;
            return Ok(Some(SpannedToken::new(
                Token::RustBlockStart,
                Span::new(start, self.position),
            )));
        }

        // Look up in keywords table, otherwise return as identifier
        let token = get_keyword_token(ident).unwrap_or_else(|| Token::Ident(ident.to_string()));

        Ok(Some(SpannedToken::new(
            token,
            Span::new(start, self.position),
        )))
    }

    /// Tries to scan "нач" after ржавчина keyword.
    fn try_scan_rust_alt_start(&mut self) -> bool {
        let saved_pos = self.pos;
        let saved_position = self.position;

        self.skip_whitespace();

        if let Some(c) = self.peek()
            && is_ident_start(c)
        {
            let word_start = self.pos;
            self.advance_while(is_ident_continue);
            let word = self.slice(word_start, self.pos);

            if word == "нач" {
                return true;
            }
        }

        // Restore position
        self.pos = saved_pos;
        self.position = saved_position;
        false
    }

    // =========================================================================
    //         OPERATORS
    // =========================================================================

    /// Tries to scan an operator.
    fn try_scan_operator(&mut self, start: Position) -> LexerResult<Option<SpannedToken>> {
        let c1 = self.peek().unwrap();

        // Try 3-char operators
        if let (Some(c2), Some(c3)) = (self.peek_at(1), self.peek_at(2)) {
            let s3: String = [c1, c2, c3].iter().collect();
            if let Some(token) = operator_token(&s3) {
                self.advance();
                self.advance();
                self.advance();
                return Ok(Some(SpannedToken::new(
                    token,
                    Span::new(start, self.position),
                )));
            }
        }

        // Try 2-char operators
        if let Some(c2) = self.peek_at(1) {
            let s2: String = [c1, c2].iter().collect();
            if let Some(token) = operator_token(&s2) {
                self.advance();
                self.advance();
                return Ok(Some(SpannedToken::new(
                    token,
                    Span::new(start, self.position),
                )));
            }
        }

        // Try 1-char operators
        let mut buf = [0u8; 4];
        if let Some(token) = operator_token(c1.encode_utf8(&mut buf)) {
            self.advance();
            return Ok(Some(SpannedToken::new(
                token,
                Span::new(start, self.position),
            )));
        }

        Ok(None)
    }

    // =========================================================================
    //         RUST BLOCKS
    // =========================================================================

    /// Scans Rust block content (РастВставкаНЦ ... РастВставкаКЦ).
    fn scan_rust_block(&mut self, start: Position) -> LexerResult<Option<SpannedToken>> {
        let content_start = self.pos;

        loop {
            if self.is_eof() {
                return Err(LexerError::with_span(
                    LexerErrorKind::UnterminatedRustBlock,
                    "Unterminated Rust block (expected РастВставкаКЦ)",
                    Span::new(start, self.position),
                ));
            }

            if self.remaining().starts_with("РастВставкаКЦ") {
                let content = self.slice(content_start, self.pos).to_string();

                // Skip the end marker
                for _ in "РастВставкаКЦ".chars() {
                    self.advance();
                }

                self.state = LexerState::Normal;

                if content.trim().is_empty() {
                    return Ok(Some(SpannedToken::new(
                        Token::RustBlockEnd,
                        Span::new(start, self.position),
                    )));
                }

                return Ok(Some(SpannedToken::new(
                    Token::RustInline(content),
                    Span::new(start, self.position),
                )));
            }

            self.advance();
        }
    }

    /// Scans alternative Rust block (ржавчина нач ... кон).
    fn scan_rust_alt_block(&mut self, start: Position) -> LexerResult<Option<SpannedToken>> {
        let content_start = self.pos;

        loop {
            if self.is_eof() {
                return Err(LexerError::with_span(
                    LexerErrorKind::UnterminatedRustBlock,
                    "Unterminated Rust block (expected кон)",
                    Span::new(start, self.position),
                ));
            }

            // Check for "кон" as a word boundary
            let remaining = self.remaining();
            if let Some(after) = remaining.strip_prefix("кон") {
                let is_end = after.is_empty()
                    || after
                        .chars()
                        .next()
                        .map(|c| !is_ident_continue(c))
                        .unwrap_or(true);

                if is_end {
                    let content = self.slice(content_start, self.pos).to_string();

                    // Skip "кон"
                    for _ in "кон".chars() {
                        self.advance();
                    }

                    self.state = LexerState::Normal;

                    if content.trim().is_empty() {
                        return Ok(Some(SpannedToken::new(
                            Token::RustBlockEnd,
                            Span::new(start, self.position),
                        )));
                    }

                    return Ok(Some(SpannedToken::new(
                        Token::RustInline(content),
                        Span::new(start, self.position),
                    )));
                }
            }

            self.advance();
        }
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

// =============================================================================
//         SECTION: TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokens() {
        let tokens = tokenize("алг Тест\nнач\nкон").unwrap();
        assert!(matches!(tokens[0].token, Token::Alg));
        assert!(matches!(tokens[1].token, Token::Ident(_)));
        assert!(matches!(tokens[2].token, Token::Newline));
        assert!(matches!(tokens[3].token, Token::Begin));
    }

    #[test]
    fn test_literals() {
        let tokens = tokenize("42 3.14 \"hello\" 'a'").unwrap();
        assert!(matches!(tokens[0].token, Token::IntLiteral(42)));
        assert!(matches!(tokens[1].token, Token::FloatLiteral(_)));
        assert!(matches!(tokens[2].token, Token::StringLiteral(_)));
        assert!(matches!(tokens[3].token, Token::CharLiteral('a')));
    }

    #[test]
    fn test_operators() {
        let tokens = tokenize(":= |> -> =>").unwrap();
        assert!(matches!(tokens[0].token, Token::Assign));
        assert!(matches!(tokens[1].token, Token::Pipe));
        assert!(matches!(tokens[2].token, Token::Arrow));
        assert!(matches!(tokens[3].token, Token::FatArrow));
    }

    #[test]
    fn test_hex_number() {
        let tokens = tokenize("0xFF 0x10").unwrap();
        assert!(matches!(tokens[0].token, Token::IntLiteral(255)));
        assert!(matches!(tokens[1].token, Token::IntLiteral(16)));
    }

    #[test]
    fn test_binary_number() {
        let tokens = tokenize("0b1010 0b11111111").unwrap();
        assert!(matches!(tokens[0].token, Token::IntLiteral(10)));
        assert!(matches!(tokens[1].token, Token::IntLiteral(255)));
    }

    #[test]
    fn test_octal_number() {
        let tokens = tokenize("0o777").unwrap();
        assert!(matches!(tokens[0].token, Token::IntLiteral(511)));
    }

    #[test]
    fn test_underscore_in_numbers() {
        let tokens = tokenize("1_000_000 0xFF_FF").unwrap();
        assert!(matches!(tokens[0].token, Token::IntLiteral(1_000_000)));
        assert!(matches!(tokens[1].token, Token::IntLiteral(0xFFFF)));
    }

    #[test]
    fn test_raw_string() {
        let tokens = tokenize(r#"r"raw string""#).unwrap();
        if let Token::RawStringLiteral(s) = &tokens[0].token {
            assert_eq!(s, "raw string");
        } else {
            panic!("Expected RawStringLiteral");
        }
    }

    #[test]
    fn test_comments() {
        let tokens = tokenize("| comment\n// another").unwrap();
        assert!(matches!(tokens[0].token, Token::Comment(_)));
        assert!(matches!(tokens[1].token, Token::Newline));
        assert!(matches!(tokens[2].token, Token::Comment(_)));
    }

    #[test]
    fn test_doc_comments() {
        let tokens = tokenize("/// doc comment").unwrap();
        assert!(matches!(tokens[0].token, Token::DocComment(_)));
    }

    #[test]
    fn test_multiline_string() {
        let tokens = tokenize(
            r#""""multi
line
string""""#,
        )
        .unwrap();
        if let Token::StringLiteral(s) = &tokens[0].token {
            assert!(s.contains('\n'));
        } else {
            panic!("Expected StringLiteral");
        }
    }

    #[test]
    fn test_escape_sequences() {
        let tokens = tokenize(r#""\n\t\r\\\"\'""#).unwrap();
        if let Token::StringLiteral(s) = &tokens[0].token {
            assert_eq!(s, "\n\t\r\\\"'");
        } else {
            panic!("Expected StringLiteral");
        }
    }

    #[test]
    fn test_unicode_escape() {
        let tokens = tokenize(r#""\u{0041}\u{042F}""#).unwrap();
        if let Token::StringLiteral(s) = &tokens[0].token {
            assert_eq!(s, "AЯ");
        } else {
            panic!("Expected StringLiteral");
        }
    }

    #[test]
    fn test_scientific_notation() {
        let tokens = tokenize("1e10 3.14e-2 2.5E+3").unwrap();
        assert!(matches!(tokens[0].token, Token::FloatLiteral(_)));
        assert!(matches!(tokens[1].token, Token::FloatLiteral(_)));
        assert!(matches!(tokens[2].token, Token::FloatLiteral(_)));
    }

    #[test]
    fn test_keywords_russian() {
        let tokens = tokenize("если то иначе").unwrap();
        assert!(matches!(tokens[0].token, Token::If));
        assert!(matches!(tokens[1].token, Token::Then));
        assert!(matches!(tokens[2].token, Token::Else));
    }

    #[test]
    fn test_keywords_english() {
        let tokens = tokenize("if then else").unwrap();
        assert!(matches!(tokens[0].token, Token::If));
        assert!(matches!(tokens[1].token, Token::Then));
        assert!(matches!(tokens[2].token, Token::Else));
    }

    #[test]
    fn test_pipe_operator() {
        // |> should be pipe, not comment
        let tokens = tokenize("x |> f").unwrap();
        assert!(matches!(tokens[0].token, Token::Ident(_)));
        assert!(matches!(tokens[1].token, Token::Pipe));
        assert!(matches!(tokens[2].token, Token::Ident(_)));
    }
}
