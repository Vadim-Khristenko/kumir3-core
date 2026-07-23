// ============================================================================
//                    CODE GENERATION MODULE
// ============================================================================
//
// This module contains common infrastructure for:
// - Executing Rust embeddings (interpreter and compiler)
// - Intermediate representation (IR) for compiler
// - Code generation for various backends
//
// ============================================================================

pub mod rust_block;
pub mod ir;

pub use rust_block::*;
pub use ir::*;
