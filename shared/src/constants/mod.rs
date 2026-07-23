//! Language constants and utilities for Kumir 3.
//!
//! Centralizes keywords, operators, mathematical constants, built-in functions,
//! and identifier utilities.

pub mod builtins;
pub mod ident;
pub mod keywords;
pub mod math;
pub mod operators;

// Re-export for convenience
pub use builtins::*;
pub use ident::*;
pub use keywords::*;
pub use math::*;
pub use operators::*;

#[cfg(test)]
mod tests;
