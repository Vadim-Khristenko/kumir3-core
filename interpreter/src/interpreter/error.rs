//! Runtime errors and error types for the Kumir 3 interpreter.

use shared::codegen::RustCodeBlockError;
use shared::math::MathErr;
use shared::types::Value;
use std::fmt;

/// A runtime error.
#[derive(Debug, Clone)]
pub struct RuntimeError {
    /// Error message.
    pub message: String,
    /// Line number (if known).
    pub line: Option<usize>,
    /// Context (algorithm name, class, etc.).
    pub context: Option<String>,
    /// Error kind.
    pub kind: RuntimeErrorKind,
    /// [KITE-0002] Carrier for early return via the `?` operator.
    ///
    /// When `expr?` is applied to an "erroneous"/"absent" value, the evaluator
    /// returns `Err` with `propagate` set, carrying the original value. At the
    /// boundary of the enclosing algorithm (see `call.rs`, `method.rs`,
    /// `instance.rs`, `run.rs`), this signal is intercepted and converted to an
    /// early return of that value — as if `return` were executed.
    /// This is NOT a true runtime error; `message`/`kind` fields serve only for
    /// diagnostics if the signal accidentally reaches the program's top level.
    pub propagate: Option<Box<Value>>,
}

/// Kind of runtime error.
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeErrorKind {
    /// Division by zero.
    DivisionByZero,
    /// Number overflow.
    Overflow,
    /// Undefined variable.
    UndefinedVariable,
    /// Undefined algorithm.
    UndefinedAlgorithm,
    /// Undefined type.
    UndefinedType,
    /// Type mismatch.
    TypeMismatch,
    /// Array index out of bounds.
    IndexOutOfBounds,
    /// Incorrect number of arguments.
    ArgumentCount,
    /// Assertion failed.
    AssertionFailed,
    /// I/O error.
    IOError,
    /// User exception.
    UserException,
    /// Not implemented.
    NotImplemented,
    /// Other error.
    Other,
}

impl RuntimeError {
    /// Creates a new error.
    pub fn new(message: impl Into<String>, kind: RuntimeErrorKind) -> Self {
        Self {
            message: message.into(),
            line: None,
            context: None,
            kind,
            propagate: None,
        }
    }

    /// [KITE-0002] Creates an early-return signal for the `?` operator.
    ///
    /// Carries the value that will become the result of the enclosing algorithm.
    pub fn propagation(value: Value) -> Self {
        Self {
            message: "распространение ошибки оператором '?'".to_string(),
            line: None,
            context: None,
            kind: RuntimeErrorKind::Other,
            propagate: Some(Box::new(value)),
        }
    }

    /// Checks if this `Err` is an early-return signal from the `?` operator.
    #[inline]
    pub fn is_propagation(&self) -> bool {
        self.propagate.is_some()
    }

    /// Adds a line number.
    pub fn with_line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    /// Adds context.
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    // =============================================================================
    //                         COMMON ERROR CONSTRUCTORS
    // =============================================================================

    pub fn division_by_zero() -> Self {
        Self::new("Деление на ноль", RuntimeErrorKind::DivisionByZero)
    }

    pub fn undefined_variable(name: &str) -> Self {
        Self::new(
            format!("Переменная не определена: '{}'", name),
            RuntimeErrorKind::UndefinedVariable,
        )
    }

    pub fn undefined_algorithm(name: &str) -> Self {
        Self::new(
            format!("Алгоритм не определён: '{}'", name),
            RuntimeErrorKind::UndefinedAlgorithm,
        )
    }

    pub fn undefined_type(name: &str) -> Self {
        Self::new(
            format!("Тип '{}' не определён", name),
            RuntimeErrorKind::UndefinedType,
        )
    }

    pub fn type_mismatch(expected: &str, got: &str) -> Self {
        Self::new(
            format!(
                "Несоответствие типов: ожидался {}, получен {}",
                expected, got
            ),
            RuntimeErrorKind::TypeMismatch,
        )
    }

    pub fn index_out_of_bounds(index: i64, length: usize) -> Self {
        Self::new(
            format!("Индекс вне границ: индекс {} при размере {}", index, length),
            RuntimeErrorKind::IndexOutOfBounds,
        )
    }

    pub fn argument_count(name: &str, expected: usize, got: usize) -> Self {
        Self::new(
            format!(
                "Неверное количество аргументов для '{}': ожидалось {}, получено {}",
                name, expected, got
            ),
            RuntimeErrorKind::ArgumentCount,
        )
    }

    pub fn assertion_failed(condition: &str) -> Self {
        Self::new(
            format!("Утверждение не выполнено: {}", condition),
            RuntimeErrorKind::AssertionFailed,
        )
    }

    pub fn not_implemented(feature: &str) -> Self {
        Self::new(
            format!("Не реализовано: {}", feature),
            RuntimeErrorKind::NotImplemented,
        )
    }

    pub fn user_exception(message: impl Into<String>) -> Self {
        Self::new(message, RuntimeErrorKind::UserException)
    }

    pub fn io_error(message: impl Into<String>) -> Self {
        Self::new(message, RuntimeErrorKind::IOError)
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[Ошибка выполнения] {}", self.message)?;
        if let Some(line) = self.line {
            write!(f, " (строка {})", line)?;
        }
        if let Some(ctx) = &self.context {
            write!(f, " в {}", ctx)?;
        }
        Ok(())
    }
}

// =============================================================================
//                  EXTENSION: RUST CODE BLOCK ERRORS
// =============================================================================
impl From<RustCodeBlockError> for RuntimeError {
    fn from(value: RustCodeBlockError) -> Self {
        Self::new(
            format!("[Rust-вставка] {}", value.message()),
            RuntimeErrorKind::Other,
        )
    }
}
impl std::error::Error for RuntimeError {}

// =============================================================================
//                      MATH KERNEL ERRORS (shared::math)
// =============================================================================

/// Maps math kernel error kinds to runtime error kinds.
///
/// The kernel reports what went wrong ([`MathErr`]); this maps to the runtime
/// error kind from KITE-0014 § 3.2. Previously, any arithmetic error collapsed
/// into [`RuntimeErrorKind::Other`], making `DivisionByZero` and `Overflow`
/// unreachable in practice.
impl From<MathErr> for RuntimeErrorKind {
    fn from(err: MathErr) -> Self {
        match err {
            MathErr::DivisionByZero => RuntimeErrorKind::DivisionByZero,
            MathErr::Overflow | MathErr::FloatOverflow => RuntimeErrorKind::Overflow,
            MathErr::TypeMismatch(_) => RuntimeErrorKind::TypeMismatch,
            // Domain violations (sqrt of negative, negative base with fractional exponent, ...)
            // are neither overflow nor type mismatch; KITE-0014 has no dedicated kind for them,
            // so they remain `Other`.
            MathErr::NegativeSqrt
            | MathErr::NegativeRoot
            | MathErr::NotRealOneSqrt
            | MathErr::NegativePowNonInteger
            | MathErr::DomainError(_) => RuntimeErrorKind::Other,
        }
    }
}

impl From<MathErr> for RuntimeError {
    fn from(err: MathErr) -> Self {
        Self::new(err.msg(), RuntimeErrorKind::from(err))
    }
}

/// Result type for interpreter operations.
pub type RuntimeResult<T> = Result<T, RuntimeError>;

/// Control flow signal (break, continue, return).
#[derive(Debug, Clone)]
pub enum ControlFlow {
    /// Normal continuation.
    None,
    /// Loop break.
    Break,
    /// Loop continue.
    Continue,
    /// Return from algorithm.
    Return(Option<shared::types::Value>),
}
