//! Kumir 3 Parser — Generic Type Parameters
//!
//! [STABLE] Parses generic parameter lists (`<T, U: Trait1 + Trait2>`)
//! shared by classes, interfaces, traits, impl blocks and methods.

use std::sync::Arc;

use crate::parser::core::Parser;
use crate::types::{Token, TypeConstraint, TypeParam};

impl Parser {
    // =========================================================================
    //         SECTION: TYPE PARAMETERS (Generics)
    // =========================================================================

    /// Tries to parse generic type parameters: `<T, U: Constraint>`.
    ///
    /// Returns empty vec if no `<` follows.
    pub(super) fn try_parse_type_params(&mut self) -> Vec<TypeParam> {
        if !self.match_token(&Token::Less) {
            return Vec::new();
        }

        let mut params = Vec::new();

        loop {
            if self.check(&Token::Greater) {
                break;
            }

            let name: Arc<str> = match self.expect_ident("type parameter") {
                Ok(n) => Arc::from(n.as_str()),
                Err(_) => break,
            };

            // Constraints: T: Trait1 + Trait2
            let constraints = if self.match_token(&Token::Colon) {
                let mut c = Vec::new();
                while let Ok(n) = self.expect_ident("constraint") {
                    let cname = Arc::from(n.as_str());
                    c.push(TypeConstraint::Implements(cname));
                    if !self.match_token(&Token::Plus) {
                        break;
                    }
                }
                c
            } else {
                Vec::new()
            };

            params.push(TypeParam {
                name,
                constraints,
                span: None,
            });

            if !self.match_token(&Token::Comma) {
                break;
            }
        }

        let _ = self.match_token(&Token::Greater);
        params
    }
}
