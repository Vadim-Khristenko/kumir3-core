// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

//! Operator and delimiter scanning.

use super::{Lexer, LexerResult, Position, Span, SpannedToken};
use crate::constants::operator_token;

impl<'a> Lexer<'a> {
    // =============================================================================
    //         SECTION: OPERATORS
    // =============================================================================

    /// Scans an operator or delimiter (tries 3-char, 2-char, then 1-char).
    pub(super) fn try_scan_operator(
        &mut self,
        start: Position,
    ) -> LexerResult<Option<SpannedToken>> {
        let c1 = self.peek().unwrap();

        if let (Some(c2), Some(c3)) = (self.peek_at(1), self.peek_at(2)) {
            let s3: String = [c1, c2, c3].iter().collect();
            if let Some(token) = operator_token(&s3) {
                self.advance();
                self.advance();
                self.advance();
                return Ok(Some(SpannedToken::new(
                    token,
                    Span::new(start, self.position),
                )));
            }
        }

        if let Some(c2) = self.peek_at(1) {
            let s2: String = [c1, c2].iter().collect();
            if let Some(token) = operator_token(&s2) {
                self.advance();
                self.advance();
                return Ok(Some(SpannedToken::new(
                    token,
                    Span::new(start, self.position),
                )));
            }
        }

        let mut buf = [0u8; 4];
        if let Some(token) = operator_token(c1.encode_utf8(&mut buf)) {
            self.advance();
            return Ok(Some(SpannedToken::new(
                token,
                Span::new(start, self.position),
            )));
        }

        Ok(None)
    }
}
