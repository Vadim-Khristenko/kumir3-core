use shared::types::TypeKind;

use super::TypeOps;

impl TypeOps {
    /// Returns default value for a type.
    ///
    /// Delegates to type system engine (shared::typesys) so variable initialization
    /// is uniform across interpreter and compiler. Types without natural default
    /// value give `Undefined`.
    pub fn default_value(ty: &TypeKind) -> shared::types::Value {
        shared::typesys::default_engine()
            .default_value(ty)
            .unwrap_or(shared::types::Value::Undefined)
    }
}
