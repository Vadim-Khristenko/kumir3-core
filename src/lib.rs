pub mod shared;
pub mod interpreter;

// Реэкспорт основных типов для удобства
pub use shared::types::{
    Value, Number, Token, Expr, Stmt, TypeSpec,
    Algorithm, Parameter, ParamMode, Program,
    ClassDef, Field, Method, Pattern,
};
pub use shared::parser::{parse, parse_expression, ParseError, ParseResult};
pub use interpreter::{
    Interpreter, Environment, RuntimeError, RuntimeResult,
    run, eval, run_and_get_output,
};
