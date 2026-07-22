//! Слой операций над значениями: единая точка типовых операций интерпретатора.
//! Извлечён из ExprEvaluator (strangler). Консультируется с shared::typesys:
//! `default_value` (defaults), типовой вердикт бинарных операций (binary),
//! план приведения и подтипирование (cast), вердикт упорядочения (predicates).
//! Все точки подключены и помечены [typesys-seam: подключён]; неподключённых
//! швов не осталось.
pub struct TypeOps;

mod binary;
mod cast;
mod defaults;
mod predicates;
mod unary;
