//! Значения по умолчанию для типов — делегирует движку shared::typesys.

use shared::types::TypeKind;

use super::TypeOps;

impl TypeOps {
    /// Возвращает значение по умолчанию для типа.
    ///
    /// Делегирует движку системы типов (shared::typesys), чтобы инициализация
    /// переменных была единообразной в интерпретаторе и компиляторе. Типы без
    /// естественного значения по умолчанию дают `Undefined`.
    pub fn default_value(ty: &TypeKind) -> shared::types::Value {
        shared::typesys::default_engine()
            .default_value(ty)
            .unwrap_or(shared::types::Value::Undefined)
    }
}
