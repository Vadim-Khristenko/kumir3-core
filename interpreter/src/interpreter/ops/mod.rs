//! Слой операций над значениями: единая точка типовых операций интерпретатора.
//! Извлечён из ExprEvaluator (strangler). Консультируется с shared::typesys:
//! `default_value` (defaults) и типовой вердикт бинарных операций (binary);
//! оставшиеся typesys-точки помечены [typesys-seam].
pub struct TypeOps;

mod binary;
mod cast;
mod defaults;
mod predicates;
mod unary;
