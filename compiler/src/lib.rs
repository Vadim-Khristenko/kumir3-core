// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

//! Библиотека компилятора Kumir 3

pub mod ast_to_ir;
pub mod backend;
pub mod compiler;
pub mod optimizer;
pub mod typecheck;

pub use ast_to_ir::AstToIr;
pub use backend::{Backend, InterpreterBackend, RustBackend};
pub use compiler::Compiler;
pub use optimizer::IrOptimizer;
pub use typecheck::TypeChecker;
