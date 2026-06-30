// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

use super::Lexer;
use crate::constants::is_whitespace;

impl<'a> Lexer<'a> {
    // =========================================================================
    //         CHARACTER NAVIGATION
    // =========================================================================

    /// Checks if at end of file.
    #[inline]
    pub(super) fn is_eof(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    /// Returns current character without advancing.
    #[inline]
    pub(super) fn peek(&self) -> Option<char> {
        if self.pos < self.source.len() {
            self.source[self.pos..].chars().next()
        } else {
            None
        }
    }

    /// Returns character at offset from current position.
    pub(super) fn peek_at(&self, offset: usize) -> Option<char> {
        let mut chars = self.source[self.pos..].chars();
        for _ in 0..offset {
            chars.next()?;
        }
        chars.next()
    }

    /// Advances by one character and returns it.
    pub(super) fn advance(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();

        if c == '\n' {
            self.position.line += 1;
            self.position.column = 1;
        } else {
            self.position.column += 1;
        }
        self.position.offset = self.pos;

        Some(c)
    }

    /// Advances while predicate is true.
    pub(super) fn advance_while<F: Fn(char) -> bool>(&mut self, pred: F) {
        while let Some(c) = self.peek() {
            if pred(c) {
                self.advance();
            } else {
                break;
            }
        }
    }

    /// Skips whitespace (except newlines).
    pub(super) fn skip_whitespace(&mut self) {
        self.advance_while(is_whitespace);
    }

    /// Returns a slice of the source.
    #[inline]
    pub(super) fn slice(&self, start: usize, end: usize) -> &'a str {
        &self.source[start..end]
    }

    /// Returns remaining source from current position.
    #[inline]
    pub(super) fn remaining(&self) -> &'a str {
        &self.source[self.pos..]
    }
}
