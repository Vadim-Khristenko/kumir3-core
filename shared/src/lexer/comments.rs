// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

use super::{Lexer, LexerResult, Position, Span, SpannedToken};
use crate::types::Token;

impl<'a> Lexer<'a> {
    // =========================================================================
    //         COMMENTS
    // =========================================================================

    /// Scans a pipe comment (| ...).
    pub(super) fn scan_comment(&mut self, start: Position) -> LexerResult<Option<SpannedToken>> {
        self.advance(); // skip |

        let content_start = self.pos;
        self.advance_while(|c| c != '\n');
        let content = self.slice(content_start, self.pos).to_string();

        Ok(Some(SpannedToken::new(
            Token::Comment(content),
            Span::new(start, self.position),
        )))
    }

    /// Scans a line comment (// or ///).
    pub(super) fn scan_line_comment(
        &mut self,
        start: Position,
    ) -> LexerResult<Option<SpannedToken>> {
        self.advance(); // first /
        self.advance(); // second /

        let is_doc = self.peek() == Some('/');
        if is_doc {
            self.advance(); // third /
        }

        // Skip leading space
        if self.peek() == Some(' ') {
            self.advance();
        }

        let content_start = self.pos;
        self.advance_while(|c| c != '\n');
        let content = self.slice(content_start, self.pos).to_string();

        let token = if is_doc {
            Token::DocComment(vec![content])
        } else {
            Token::Comment(content)
        };

        Ok(Some(SpannedToken::new(
            token,
            Span::new(start, self.position),
        )))
    }
}
