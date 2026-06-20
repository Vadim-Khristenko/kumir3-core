//! Слой операций над значениями: единая точка типовых операций интерпретатора.
//! Извлечён из ExprEvaluator (strangler). Консультируется с shared::typesys там,
//! где это безопасно (default_value); прочие typesys-точки помечены [typesys-seam]
//! для будущего среза 2b.
pub struct TypeOps;

mod binary;
mod cast;
mod defaults;
mod predicates;
mod unary;
