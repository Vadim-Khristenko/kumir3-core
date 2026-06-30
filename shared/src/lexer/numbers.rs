// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

use super::{Lexer, LexerError, LexerErrorKind, LexerResult, Position, Span, SpannedToken};
use crate::types::Token;

impl<'a> Lexer<'a> {
    // =========================================================================
    //         NUMBER LITERALS
    // =========================================================================

    /// Scans a number literal.
    pub(super) fn scan_number(&mut self, start: Position) -> LexerResult<Option<SpannedToken>> {
        let num_start = self.pos;

        // Check for radix prefix (0x, 0b, 0o)
        if self.peek() == Some('0') {
            self.advance();
            match self.peek() {
                Some('x') | Some('X') => return self.scan_hex_number(start, num_start),
                Some('b') | Some('B') => return self.scan_binary_number(start, num_start),
                Some('o') | Some('O') => return self.scan_octal_number(start, num_start),
                _ => {
                    // Just a leading zero, continue with decimal
                }
            }
        }

        // Decimal integer part
        self.advance_while(|c| c.is_ascii_digit() || c == '_');

        let mut is_float = false;

        // Decimal point
        if self.peek() == Some('.') && self.peek_at(1).map(|c| c.is_ascii_digit()).unwrap_or(false)
        {
            is_float = true;
            self.advance(); // dot
            self.advance_while(|c| c.is_ascii_digit() || c == '_');
        }

        // Exponent
        if matches!(self.peek(), Some('e') | Some('E')) {
            is_float = true;
            self.advance();
            if matches!(self.peek(), Some('+') | Some('-')) {
                self.advance();
            }
            let exp_start = self.pos;
            self.advance_while(|c| c.is_ascii_digit() || c == '_');
            if self.pos == exp_start {
                return Err(LexerError::new(
                    LexerErrorKind::InvalidNumber,
                    "Expected exponent digits",
                    start,
                ));
            }
        }

        let num_str = self.slice(num_start, self.pos).replace('_', "");

        let token = if is_float {
            match num_str.parse::<f64>() {
                Ok(n) => Token::FloatLiteral(n),
                Err(_) => {
                    return Err(LexerError::new(
                        LexerErrorKind::InvalidNumber,
                        format!("Invalid float literal: {}", num_str),
                        start,
                    ));
                }
            }
        } else {
            match num_str.parse::<i64>() {
                Ok(n) => Token::IntLiteral(n),
                Err(_) => {
                    return Err(LexerError::new(
                        LexerErrorKind::InvalidNumber,
                        format!("Invalid integer literal: {}", num_str),
                        start,
                    ));
                }
            }
        };

        Ok(Some(SpannedToken::new(
            token,
            Span::new(start, self.position),
        )))
    }

    /// Scans a hexadecimal number (0x...).
    pub(super) fn scan_hex_number(
        &mut self,
        start: Position,
        _num_start: usize,
    ) -> LexerResult<Option<SpannedToken>> {
        self.advance(); // 'x' or 'X'

        let hex_start = self.pos;
        self.advance_while(|c| c.is_ascii_hexdigit() || c == '_');

        if self.pos == hex_start {
            return Err(LexerError::new(
                LexerErrorKind::InvalidNumber,
                "Expected hex digits after 0x",
                start,
            ));
        }

        let hex_str = self.slice(hex_start, self.pos).replace('_', "");
        match i64::from_str_radix(&hex_str, 16) {
            Ok(n) => Ok(Some(SpannedToken::new(
                Token::IntLiteral(n),
                Span::new(start, self.position),
            ))),
            Err(_) => Err(LexerError::new(
                LexerErrorKind::InvalidNumber,
                format!("Invalid hex literal: 0x{}", hex_str),
                start,
            )),
        }
    }

    /// Scans a binary number (0b...).
    pub(super) fn scan_binary_number(
        &mut self,
        start: Position,
        _num_start: usize,
    ) -> LexerResult<Option<SpannedToken>> {
        self.advance(); // 'b' or 'B'

        let bin_start = self.pos;
        self.advance_while(|c| c == '0' || c == '1' || c == '_');

        if self.pos == bin_start {
            return Err(LexerError::new(
                LexerErrorKind::InvalidNumber,
                "Expected binary digits after 0b",
                start,
            ));
        }

        let bin_str = self.slice(bin_start, self.pos).replace('_', "");
        match i64::from_str_radix(&bin_str, 2) {
            Ok(n) => Ok(Some(SpannedToken::new(
                Token::IntLiteral(n),
                Span::new(start, self.position),
            ))),
            Err(_) => Err(LexerError::new(
                LexerErrorKind::InvalidNumber,
                format!("Invalid binary literal: 0b{}", bin_str),
                start,
            )),
        }
    }

    /// Scans an octal number (0o...).
    pub(super) fn scan_octal_number(
        &mut self,
        start: Position,
        _num_start: usize,
    ) -> LexerResult<Option<SpannedToken>> {
        self.advance(); // 'o' or 'O'

        let oct_start = self.pos;
        self.advance_while(|c| ('0'..='7').contains(&c) || c == '_');

        if self.pos == oct_start {
            return Err(LexerError::new(
                LexerErrorKind::InvalidNumber,
                "Expected octal digits after 0o",
                start,
            ));
        }

        let oct_str = self.slice(oct_start, self.pos).replace('_', "");
        match i64::from_str_radix(&oct_str, 8) {
            Ok(n) => Ok(Some(SpannedToken::new(
                Token::IntLiteral(n),
                Span::new(start, self.position),
            ))),
            Err(_) => Err(LexerError::new(
                LexerErrorKind::InvalidNumber,
                format!("Invalid octal literal: 0o{}", oct_str),
                start,
            )),
        }
    }
}
