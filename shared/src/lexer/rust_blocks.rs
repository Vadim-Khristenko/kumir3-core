// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

use super::{
    Lexer, LexerError, LexerErrorKind, LexerResult, LexerState, Position, Span, SpannedToken,
};
use crate::constants::is_ident_continue;
use crate::types::Token;

impl<'a> Lexer<'a> {
    // =========================================================================
    //         RUST BLOCKS
    // =========================================================================

    /// Scans Rust block content (РастВставкаНЦ ... РастВставкаКЦ).
    pub(super) fn scan_rust_block(&mut self, start: Position) -> LexerResult<Option<SpannedToken>> {
        let content_start = self.pos;

        loop {
            if self.is_eof() {
                return Err(LexerError::with_span(
                    LexerErrorKind::UnterminatedRustBlock,
                    "Unterminated Rust block (expected РастВставкаКЦ)",
                    Span::new(start, self.position),
                ));
            }

            if self.remaining().starts_with("РастВставкаКЦ") {
                let content = self.slice(content_start, self.pos).to_string();

                // Skip the end marker
                for _ in "РастВставкаКЦ".chars() {
                    self.advance();
                }

                self.state = LexerState::Normal;

                if content.trim().is_empty() {
                    return Ok(Some(SpannedToken::new(
                        Token::RustBlockEnd,
                        Span::new(start, self.position),
                    )));
                }

                return Ok(Some(SpannedToken::new(
                    Token::RustInline(content),
                    Span::new(start, self.position),
                )));
            }

            self.advance();
        }
    }

    /// Scans alternative Rust block (ржавчина нач ... кон).
    pub(super) fn scan_rust_alt_block(
        &mut self,
        start: Position,
    ) -> LexerResult<Option<SpannedToken>> {
        let content_start = self.pos;

        loop {
            if self.is_eof() {
                return Err(LexerError::with_span(
                    LexerErrorKind::UnterminatedRustBlock,
                    "Unterminated Rust block (expected кон)",
                    Span::new(start, self.position),
                ));
            }

            // Check for "кон" as a word boundary
            let remaining = self.remaining();
            if let Some(after) = remaining.strip_prefix("кон") {
                let is_end = after.is_empty()
                    || after
                        .chars()
                        .next()
                        .map(|c| !is_ident_continue(c))
                        .unwrap_or(true);

                if is_end {
                    let content = self.slice(content_start, self.pos).to_string();

                    // Skip "кон"
                    for _ in "кон".chars() {
                        self.advance();
                    }

                    self.state = LexerState::Normal;

                    if content.trim().is_empty() {
                        return Ok(Some(SpannedToken::new(
                            Token::RustBlockEnd,
                            Span::new(start, self.position),
                        )));
                    }

                    return Ok(Some(SpannedToken::new(
                        Token::RustInline(content),
                        Span::new(start, self.position),
                    )));
                }
            }

            self.advance();
        }
    }
}
