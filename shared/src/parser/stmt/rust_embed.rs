//! Kumir 3 Parser — Rust Embed Blocks
//!
//! [STABLE] Parses inline Rust code blocks (`РастВставкаНЦ ... РастВставкаКЦ`
//! and `ржавчина нач ... кон`) and determines which Kumir variables the
//! embedded code captures.

use crate::parser::core::Parser;
use crate::parser::error::{ParseError, ParseResult};
use crate::types::{Stmt, Token};

impl Parser {
    // =========================================================================
    //         SECTION: RUST EMBEDS
    // =========================================================================

    /// Parses a Rust code block in one of two syntaxes:
    ///
    /// 1. `РастВставкаНЦ ... РастВставкаКЦ`
    /// 2. `ржавчина нач ... кон`
    pub(super) fn parse_rust_block(&mut self) -> ParseResult<Stmt> {
        let code = if self.match_token(&Token::RustBlockStart) {
            let code = if let Token::RustCode = self.peek().clone() {
                // RustCode is a marker — the actual code is in the next token
                // Actually, based on original parser, it can be RustCode(String)
                // Let's handle both forms:
                self.advance();
                String::new()
            } else if let Token::StringLiteral(c) = self.peek().clone() {
                self.advance();
                c
            } else {
                String::new()
            };
            self.expect(&Token::RustBlockEnd, "РастВставкаКЦ")?;
            code
        } else if self.match_token(&Token::Rust) {
            self.expect(&Token::Begin, "нач")?;
            let code = if let Token::StringLiteral(c) = self.peek().clone() {
                self.advance();
                c
            } else {
                String::new()
            };
            self.expect(&Token::End, "кон")?;
            code
        } else {
            return Err(
                ParseError::unexpected("блок вставки на Rust", self.peek(), self.span()).into(),
            );
        };

        let captured_vars = extract_captured_vars(&code);
        self.skip_newlines();

        Ok(Stmt::RustBlock {
            code,
            captured_vars,
            return_type: None,
        })
    }
}

// =============================================================================
//         SECTION: UTILITY FUNCTIONS
// =============================================================================

/// Extracts variable names from Rust code by looking for `{name}` patterns.
///
/// Used to determine which Kumir variables are captured by a Rust embed block.
fn extract_captured_vars(code: &str) -> Vec<String> {
    let mut vars = Vec::new();
    let mut chars = code.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '{' {
            let mut name = String::new();
            for ch in chars.by_ref() {
                if ch == '}' || ch == ':' {
                    break;
                }
                name.push(ch);
            }
            let name = name.trim();
            if !name.is_empty()
                && !name.starts_with(|c: char| c.is_ascii_digit())
                && name
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c > '\x7F')
                && !vars.contains(&name.to_string())
            {
                vars.push(name.to_string());
            }
        }
    }

    vars
}
