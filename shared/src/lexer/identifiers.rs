// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

use super::{Lexer, LexerResult, LexerState, Position, Span, SpannedToken};
use crate::constants::{get_keyword_token, is_ident_continue, is_ident_start};
use crate::types::Token;

impl<'a> Lexer<'a> {
    // =========================================================================
    //         IDENTIFIERS & KEYWORDS
    // =========================================================================

    /// Scans an identifier or keyword.
    pub(super) fn scan_identifier(&mut self, start: Position) -> LexerResult<Option<SpannedToken>> {
        let ident_start = self.pos;

        self.advance(); // first char (already verified as ident_start)
        self.advance_while(is_ident_continue);

        let ident = self.slice(ident_start, self.pos);

        // Check for Rust block markers
        if ident == "РастВставкаНЦ" {
            self.state = LexerState::RustBlock;
            return Ok(Some(SpannedToken::new(
                Token::RustBlockStart,
                Span::new(start, self.position),
            )));
        }

        // Check for alternative Rust syntax (ржавчина нач)
        if matches!(ident, "ржавчина" | "Ржавчина" | "rust") && self.try_scan_rust_alt_start()
        {
            self.state = LexerState::RustAltBlock;
            return Ok(Some(SpannedToken::new(
                Token::RustBlockStart,
                Span::new(start, self.position),
            )));
        }

        // Look up in keywords table, otherwise return as identifier
        let token = get_keyword_token(ident).unwrap_or_else(|| Token::Ident(ident.to_string()));

        Ok(Some(SpannedToken::new(
            token,
            Span::new(start, self.position),
        )))
    }

    /// Tries to scan "нач" after ржавчина keyword.
    pub(super) fn try_scan_rust_alt_start(&mut self) -> bool {
        let saved_pos = self.pos;
        let saved_position = self.position;

        self.skip_whitespace();

        if let Some(c) = self.peek()
            && is_ident_start(c)
        {
            let word_start = self.pos;
            self.advance_while(is_ident_continue);
            let word = self.slice(word_start, self.pos);

            if word == "нач" {
                return true;
            }
        }

        // Restore position
        self.pos = saved_pos;
        self.position = saved_position;
        false
    }
}
