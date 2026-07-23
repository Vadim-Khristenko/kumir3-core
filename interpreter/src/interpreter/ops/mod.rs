//! Operations layer for values: unified entry point for type operations in the interpreter.
//! Extracted from ExprEvaluator (strangler pattern). Consults with shared::typesys:
//! `default_value` (defaults), type verdict for binary operations (binary),
//! coercion plan and subtyping (cast), ordering verdict (predicates).
//! All connection points are marked [typesys-seam: подключён]; no unconnected seams remain.

pub struct TypeOps;

mod binary;
mod cast;
mod defaults;
mod predicates;
mod unary;
