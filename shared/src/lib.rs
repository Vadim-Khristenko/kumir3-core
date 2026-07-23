// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

//! Shared core library for Kumir 3 — lexer, parser, type system, and runtime.

pub mod codegen;
pub mod constants;
pub mod f128;
pub mod iostream;
pub mod lexer;
pub mod libraries;
pub mod math;
pub mod parser;
pub mod runtime;
pub mod strings;
pub mod types;
pub mod typesys;

pub use types::*;
