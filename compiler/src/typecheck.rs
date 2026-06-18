// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

//! [EXPERIMENTAL] Static type checking for the compiler.
//!
//! Thin compiler-facing facade over the shared type engine
//! ([`shared::typesys::TypeSystem`], see KITE 10). The compiler and the
//! interpreter share the *same* engine and rules, so assignability,
//! unification and operator typing are decided consistently everywhere.

// =============================================================================
//         SECTION: IMPORTS
// =============================================================================

use shared::types::value::TypeKind;
use shared::typesys::{TypeError, TypeOp, TypeSystem};

// =============================================================================
//         SECTION: TYPE CHECKER
// =============================================================================

/// [EXPERIMENTAL] Static type checker used during compilation.
///
/// Wraps the shared [`TypeSystem`] so compiler passes (codegen, diagnostics)
/// can ask the same questions the interpreter does.
pub struct TypeChecker {
    engine: TypeSystem,
}

impl TypeChecker {
    /// Creates a checker with the standard rule set.
    pub fn new() -> Self {
        Self {
            engine: TypeSystem::new(),
        }
    }

    /// Borrows the underlying engine (to register extra rules, attach a registry, …).
    pub fn engine(&self) -> &TypeSystem {
        &self.engine
    }

    /// Mutable access to the engine (extensibility hook).
    pub fn engine_mut(&mut self) -> &mut TypeSystem {
        &mut self.engine
    }

    /// Checks that `source` can be assigned where `target` is expected.
    pub fn check_assignment(&self, target: &TypeKind, source: &TypeKind) -> Result<(), TypeError> {
        self.engine.check_assignable(target, source).map(|_| ())
    }

    /// Computes the static result type of a binary operation.
    pub fn binop_result(
        &self,
        op: TypeOp,
        left: &TypeKind,
        right: &TypeKind,
    ) -> Result<TypeKind, TypeError> {
        self.engine.result_of_binop(op, left, right)
    }

    /// Common type of two branches (e.g. `если … то A иначе B`).
    pub fn branch_type(&self, a: &TypeKind, b: &TypeKind) -> Result<TypeKind, TypeError> {
        self.engine.common_type(a, b)
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
//         SECTION: UNIT TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// [STABLE] Widening is implicit; narrowing is compatible only via coercion.
    fn checker_assignment_widening() {
        let tc = TypeChecker::new();
        // Widening (цел_16 -> цел_64) is implicitly assignable.
        assert!(
            tc.engine()
                .is_assignable(&TypeKind::Int64, &TypeKind::Int16)
        );
        // Narrowing (цел_64 -> цел_16) is NOT implicit ...
        assert!(
            !tc.engine()
                .is_assignable(&TypeKind::Int16, &TypeKind::Int64)
        );
        // ... but it is still compatible (coercible), so a checked assignment is Ok.
        assert!(
            tc.check_assignment(&TypeKind::Int16, &TypeKind::Int64)
                .is_ok()
        );
        // A genuinely incompatible assignment is rejected.
        assert!(
            tc.check_assignment(&TypeKind::Int64, &TypeKind::String)
                .is_err()
        );
    }

    #[test]
    /// [STABLE] Operator typing matches the shared engine.
    fn checker_binop_and_branch() {
        let tc = TypeChecker::new();
        assert_eq!(
            tc.binop_result(TypeOp::Add, &TypeKind::Int16, &TypeKind::Int64)
                .unwrap(),
            TypeKind::Int64
        );
        assert_eq!(
            tc.branch_type(&TypeKind::Int16, &TypeKind::Int64).unwrap(),
            TypeKind::Int64
        );
    }
}
