//! Parser error types and diagnostics.
//!
//! [STABLE] Production-quality error reporting with:
//! - Structured error kinds (for programmatic handling)
//! - Rich diagnostics with notes and help messages
//! - Span-based error locations
//! - Accumulation mode (continue parsing after errors)

use crate::lexer::{LexerError, Span};
use crate::types::Token;
use std::fmt;

// =============================================================================
//         SECTION: DIAGNOSTIC LEVEL
// =============================================================================

/// Severity level for diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiagnosticLevel {
    /// Informational hint
    Hint,
    /// Warning (compilation succeeds but something is suspicious)
    Warning,
    /// Error (prevents compilation)
    Error,
}

impl fmt::Display for DiagnosticLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hint => write!(f, "подсказка"),
            Self::Warning => write!(f, "предупреждение"),
            Self::Error => write!(f, "ошибка"),
        }
    }
}

// =============================================================================
//         SECTION: PARSE ERROR KIND
// =============================================================================

/// Categorized parser error kinds.
///
/// Each variant maps to a stable error code (P001–P099).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParseErrorKind {
    // --- Token-level (P001–P009) ---
    UnexpectedToken,
    UnexpectedEof,
    ExpectedNewline,

    // --- Expressions (P010–P019) ---
    ExpectedExpression,
    ExpectedPrimary,
    InvalidOperator,

    // --- Types (P020–P029) ---
    ExpectedType,
    UnknownType,

    // --- Declarations (P030–P039) ---
    ExpectedIdent,
    ExpectedAlgBody,
    DuplicateDecl,
    InvalidParam,

    // --- Statements (P040–P049) ---
    ExpectedStatement,
    UnclosedBlock,
    InvalidAssignTarget,

    // --- Patterns (P050–P059) ---
    ExpectedPattern,
    InvalidPattern,

    // --- OOP (P060–P069) ---
    InvalidClassMember,
    ExpectedMethodBody,

    // --- Lexer passthrough (P090) ---
    LexerError,

    // --- Generic (P099) ---
    Custom,
}

impl ParseErrorKind {
    /// Returns the stable error code string.
    pub fn code(self) -> &'static str {
        match self {
            Self::UnexpectedToken => "P001",
            Self::UnexpectedEof => "P002",
            Self::ExpectedNewline => "P003",
            Self::ExpectedExpression => "P010",
            Self::ExpectedPrimary => "P011",
            Self::InvalidOperator => "P012",
            Self::ExpectedType => "P020",
            Self::UnknownType => "P021",
            Self::ExpectedIdent => "P030",
            Self::ExpectedAlgBody => "P031",
            Self::DuplicateDecl => "P032",
            Self::InvalidParam => "P033",
            Self::ExpectedStatement => "P040",
            Self::UnclosedBlock => "P041",
            Self::InvalidAssignTarget => "P042",
            Self::ExpectedPattern => "P050",
            Self::InvalidPattern => "P051",
            Self::InvalidClassMember => "P060",
            Self::ExpectedMethodBody => "P061",
            Self::LexerError => "P090",
            Self::Custom => "P099",
        }
    }
}

// =============================================================================
//         SECTION: DIAGNOSTIC NOTE
// =============================================================================

/// Additional context note attached to a diagnostic.
#[derive(Debug, Clone, PartialEq)]
pub struct DiagnosticNote {
    pub message: String,
    pub span: Option<Span>,
}

impl DiagnosticNote {
    /// Note pointing to a specific span.
    pub fn at(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span: Some(span),
        }
    }

    /// Freeform note without location.
    pub fn text(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            span: None,
        }
    }
}

// =============================================================================
//         SECTION: PARSE ERROR
// =============================================================================

/// A parser error with full diagnostic information.
///
/// # Example
/// ```ignore
/// let err = ParseError::new(ParseErrorKind::UnexpectedToken, "ожидался \":=\"", span)
///     .with_note(DiagnosticNote::text("Используйте := для присваивания"))
///     .with_help("Возможно, вы имели в виду: x := 42");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    /// Error category
    pub kind: ParseErrorKind,
    /// Human-readable message
    pub message: String,
    /// Primary source span
    pub span: Span,
    /// Additional notes
    pub notes: Vec<DiagnosticNote>,
    /// Help suggestion
    pub help: Option<String>,
}

impl ParseError {
    /// Creates a new parse error.
    pub fn new(kind: ParseErrorKind, message: impl Into<String>, span: Span) -> Self {
        Self {
            kind,
            message: message.into(),
            span,
            notes: Vec::new(),
            help: None,
        }
    }

    /// Adds a note.
    pub fn with_note(mut self, note: DiagnosticNote) -> Self {
        self.notes.push(note);
        self
    }

    /// Adds a help suggestion.
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    // =========================================================================
    //         CONVENIENCE CONSTRUCTORS
    // =========================================================================

    /// "Unexpected token" error.
    pub fn unexpected(expected: &str, found: &Token, span: Span) -> Self {
        Self::new(
            ParseErrorKind::UnexpectedToken,
            format!("Ожидалось {}, найдено {:?}", expected, found),
            span,
        )
    }

    /// "Unexpected EOF" error.
    pub fn unexpected_eof(expected: &str, span: Span) -> Self {
        Self::new(
            ParseErrorKind::UnexpectedEof,
            format!("Неожиданный конец файла, ожидалось {}", expected),
            span,
        )
    }

    /// "Expected type" error.
    pub fn expected_type(span: Span) -> Self {
        Self::new(ParseErrorKind::ExpectedType, "Ожидался тип", span)
    }

    /// "Expected expression" error.
    pub fn expected_expr(span: Span) -> Self {
        Self::new(
            ParseErrorKind::ExpectedExpression,
            "Ожидалось выражение",
            span,
        )
    }

    /// "Expected identifier" error.
    pub fn expected_ident(span: Span) -> Self {
        Self::new(
            ParseErrorKind::ExpectedIdent,
            "Ожидался идентификатор",
            span,
        )
    }

    /// "Expected newline" error.
    pub fn expected_newline(span: Span) -> Self {
        Self::new(
            ParseErrorKind::ExpectedNewline,
            "Ожидался конец строки",
            span,
        )
    }

    /// "Expected statement" error.
    pub fn expected_stmt(span: Span) -> Self {
        Self::new(
            ParseErrorKind::ExpectedStatement,
            "Ожидалась инструкция",
            span,
        )
    }

    /// "Unclosed block" error with opening location.
    pub fn unclosed_block(what: &str, opened_at: Span, span: Span) -> Self {
        Self::new(
            ParseErrorKind::UnclosedBlock,
            format!("Незакрытый блок {}", what),
            span,
        )
        .with_note(DiagnosticNote::at(
            format!("{} начинается здесь", what),
            opened_at,
        ))
    }

    /// Generic custom error.
    pub fn custom(message: impl Into<String>, span: Span) -> Self {
        Self::new(ParseErrorKind::Custom, message, span)
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {} в {}", self.kind.code(), self.message, self.span)?;
        for note in &self.notes {
            if let Some(span) = note.span {
                write!(f, "\n  заметка: {} в {}", note.message, span)?;
            } else {
                write!(f, "\n  заметка: {}", note.message)?;
            }
        }
        if let Some(help) = &self.help {
            write!(f, "\n  помощь: {}", help)?;
        }
        Ok(())
    }
}

impl std::error::Error for ParseError {}

impl From<LexerError> for ParseError {
    fn from(err: LexerError) -> Self {
        let span = err.span.unwrap_or_else(|| Span::point(err.position));
        ParseError::new(ParseErrorKind::LexerError, err.message, span)
    }
}

// =============================================================================
//         SECTION: DIAGNOSTICS COLLECTOR
// =============================================================================

/// Accumulates diagnostics during parsing.
///
/// Used for error recovery mode — parser can continue after errors
/// and report all issues at once.
#[derive(Debug, Clone, Default)]
pub struct Diagnostics {
    entries: Vec<Diagnostic>,
    max_errors: usize,
}

/// Single diagnostic entry.
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub error: ParseError,
}

impl Diagnostics {
    /// Creates a new collector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets maximum errors before aborting.
    pub fn with_max_errors(mut self, max: usize) -> Self {
        self.max_errors = max;
        self
    }

    /// Reports an error.
    pub fn error(&mut self, err: ParseError) {
        self.entries.push(Diagnostic {
            level: DiagnosticLevel::Error,
            error: err,
        });
    }

    /// Reports a warning.
    pub fn warning(&mut self, err: ParseError) {
        self.entries.push(Diagnostic {
            level: DiagnosticLevel::Warning,
            error: err,
        });
    }

    /// Reports a hint.
    pub fn hint(&mut self, err: ParseError) {
        self.entries.push(Diagnostic {
            level: DiagnosticLevel::Hint,
            error: err,
        });
    }

    /// Has the error limit been reached?
    pub fn is_over_limit(&self) -> bool {
        self.max_errors > 0 && self.error_count() >= self.max_errors
    }

    /// Number of errors.
    pub fn error_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|d| d.level == DiagnosticLevel::Error)
            .count()
    }

    /// Number of warnings.
    pub fn warning_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|d| d.level == DiagnosticLevel::Warning)
            .count()
    }

    /// Has any errors?
    pub fn has_errors(&self) -> bool {
        self.error_count() > 0
    }

    /// All entries.
    pub fn entries(&self) -> &[Diagnostic] {
        &self.entries
    }

    /// Only errors.
    pub fn errors(&self) -> Vec<&ParseError> {
        self.entries
            .iter()
            .filter(|d| d.level == DiagnosticLevel::Error)
            .map(|d| &d.error)
            .collect()
    }

    /// Only warnings.
    pub fn warnings(&self) -> Vec<&ParseError> {
        self.entries
            .iter()
            .filter(|d| d.level == DiagnosticLevel::Warning)
            .map(|d| &d.error)
            .collect()
    }

    /// Takes the first error (for fail-fast compat).
    pub fn into_first_error(self) -> Option<ParseError> {
        self.entries
            .into_iter()
            .find(|d| d.level == DiagnosticLevel::Error)
            .map(|d| d.error)
    }

    /// Drains all entries.
    pub fn drain(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.entries)
    }
}

// =============================================================================
//         SECTION: PARSE RESULT
// =============================================================================

/// Parser result type.
pub type ParseResult<T> = Result<T, Box<ParseError>>;

impl From<LexerError> for Box<ParseError> {
    fn from(err: LexerError) -> Self {
        Box::new(ParseError::from(err))
    }
}
