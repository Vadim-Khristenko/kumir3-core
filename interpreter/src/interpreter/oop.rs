//! Object-oriented programming validation and library function calls.

use super::Interpreter;
use super::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};
use shared::types::Value;

impl Interpreter {
    // =============================================================================
    //                            OOP VALIDATION
    // =============================================================================

    /// [KITE 11] Validates OOP invariants across all loaded classes:
    /// final methods cannot be overridden; non-abstract classes must implement
    /// all inherited abstract methods.
    pub(crate) fn validate_classes(&self) -> RuntimeResult<()> {
        use std::collections::{HashMap, HashSet};
        type ClassDef = shared::types::ClassDef;

        // Snapshot of all classes (without holding env borrow during checks).
        let map: HashMap<String, ClassDef> = self
            .env
            .all_classes()
            .map(|(n, c)| (n.clone(), c.clone()))
            .collect();

        // Ancestor chain (names) from parent upward.
        let ancestors = |start: &str| -> Vec<String> {
            let mut chain = Vec::new();
            let mut cur = map
                .get(start)
                .and_then(|c| c.parent.as_ref().map(|p| p.to_string()));
            while let Some(name) = cur {
                if chain.contains(&name) {
                    break; // cycle protection
                }
                chain.push(name.clone());
                cur = map
                    .get(&name)
                    .and_then(|c| c.parent.as_ref().map(|p| p.to_string()));
            }
            chain
        };

        for class in map.values() {
            let chain = ancestors(&class.name);

            // 1) Forbid overriding final methods from ancestors.
            for m in &class.methods {
                for anc in &chain {
                    if let Some(am) = map.get(anc).and_then(|c| {
                        c.methods
                            .iter()
                            .find(|x| x.algorithm.name == m.algorithm.name)
                    }) && am.is_final
                    {
                        return Err(RuntimeError::new(
                            format!(
                                "Метод '{}' помечен 'финал' в классе '{}' и не может быть переопределён в '{}'",
                                m.algorithm.name, anc, class.name
                            ),
                            RuntimeErrorKind::Other,
                        ));
                    }
                }
            }

            // 2) Non-abstract class must implement all abstract methods in hierarchy.
            if !class.is_abstract {
                // Names of abstract methods in class and ancestors.
                let mut abstract_names: HashSet<String> = HashSet::new();
                for scope in std::iter::once(class.name.to_string()).chain(chain.iter().cloned()) {
                    if let Some(c) = map.get(&scope) {
                        for m in &c.methods {
                            if m.is_abstract {
                                abstract_names.insert(m.algorithm.name.to_string());
                            }
                        }
                    }
                }
                // The effective implementation (first top-down) must be concrete.
                for name in &abstract_names {
                    let mut implemented = false;
                    for scope in
                        std::iter::once(class.name.to_string()).chain(chain.iter().cloned())
                    {
                        if let Some(m) = map.get(&scope).and_then(|c| {
                            c.methods
                                .iter()
                                .find(|x| x.algorithm.name.as_ref() == name.as_str())
                        }) {
                            implemented = !m.is_abstract;
                            break; // first match determines behavior
                        }
                    }
                    if !implemented {
                        return Err(RuntimeError::new(
                            format!(
                                "Класс '{}' должен реализовать абстрактный метод '{}'",
                                class.name, name
                            ),
                            RuntimeErrorKind::Other,
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Calls a library function.
    pub fn call_library_function(
        &self,
        name: &str,
        args: &[Value],
    ) -> RuntimeResult<Option<Value>> {
        self.libraries
            .read()
            .map_err(|_| {
                RuntimeError::new(
                    "Не удалось получить доступ к библиотекам",
                    RuntimeErrorKind::Other,
                )
            })?
            .call_function(name, args)
    }

    /// Checks if a name is a library function.
    pub fn is_library_function(&self, name: &str) -> bool {
        self.libraries
            .read()
            .map(|m| m.is_library_function(name))
            .unwrap_or(false)
    }
}
