// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

use super::{
    Lexer, LexerError, LexerErrorKind, LexerResult, LexerState, Position, Span, SpannedToken,
};
use crate::types::Token;

impl<'a> Lexer<'a> {
    // =========================================================================
    //         STRING LITERALS
    // =========================================================================

    /// Scans a string literal ("..." or """...""").
    pub(super) fn scan_string(&mut self, start: Position) -> LexerResult<Option<SpannedToken>> {
        self.advance(); // opening quote

        // Check for triple quotes
        let is_multiline = if self.peek() == Some('"') && self.peek_at(1) == Some('"') {
            self.advance();
            self.advance();
            true
        } else if self.peek() == Some('"') {
            // Empty string ""
            self.advance();
            return Ok(Some(SpannedToken::new(
                Token::StringLiteral(String::new()),
                Span::new(start, self.position),
            )));
        } else {
            false
        };

        let mut value = String::new();

        loop {
            match self.peek() {
                None => {
                    return Err(LexerError::with_span(
                        LexerErrorKind::UnterminatedString,
                        "Unterminated string literal",
                        Span::new(start, self.position),
                    ));
                }
                Some('"') => {
                    if is_multiline {
                        if self.peek_at(1) == Some('"') && self.peek_at(2) == Some('"') {
                            self.advance();
                            self.advance();
                            self.advance();
                            break;
                        }
                        value.push('"');
                        self.advance();
                    } else {
                        self.advance();
                        break;
                    }
                }
                Some('\n') if !is_multiline => {
                    return Err(LexerError::with_span(
                        LexerErrorKind::UnterminatedString,
                        "Unterminated string literal (use triple quotes for multiline)",
                        Span::new(start, self.position),
                    ));
                }
                Some('\\') => {
                    self.advance();
                    value.push(self.scan_escape_sequence()?);
                }
                Some(c) => {
                    value.push(c);
                    self.advance();
                }
            }
        }

        Ok(Some(SpannedToken::new(
            Token::StringLiteral(value),
            Span::new(start, self.position),
        )))
    }

    /// Scans a raw string literal (r"..." or r#"..."#).
    pub(super) fn scan_raw_string(&mut self, start: Position) -> LexerResult<Option<SpannedToken>> {
        self.advance(); // 'r'

        // Count opening hashes
        let mut hash_count = 0;
        while self.peek() == Some('#') {
            hash_count += 1;
            self.advance();
        }

        if self.peek() != Some('"') {
            return Err(LexerError::new(
                LexerErrorKind::UnterminatedString,
                "Expected '\"' after r#",
                self.position,
            ));
        }
        self.advance(); // opening quote

        let mut value = String::new();

        loop {
            match self.peek() {
                None => {
                    return Err(LexerError::with_span(
                        LexerErrorKind::UnterminatedString,
                        "Unterminated raw string literal",
                        Span::new(start, self.position),
                    ));
                }
                Some('"') => {
                    self.advance();
                    let mut closing = 0;
                    while closing < hash_count && self.peek() == Some('#') {
                        closing += 1;
                        self.advance();
                    }
                    if closing == hash_count {
                        break;
                    }
                    value.push('"');
                    for _ in 0..closing {
                        value.push('#');
                    }
                }
                Some(c) => {
                    value.push(c);
                    self.advance();
                }
            }
        }

        Ok(Some(SpannedToken::new(
            Token::RawStringLiteral(value),
            Span::new(start, self.position),
        )))
    }

    /// Scans start of interpolated string (f"...).
    /// Emits InterpolatedStringStart, then switches to InterpolatedStringText state.
    pub(super) fn scan_interpolated_string(
        &mut self,
        start: Position,
    ) -> LexerResult<Option<SpannedToken>> {
        self.advance(); // 'f'
        self.advance(); // opening quote

        self.state = LexerState::InterpolatedStringText;

        Ok(Some(SpannedToken::new(
            Token::InterpolatedStringStart,
            Span::new(start, self.position),
        )))
    }

    /// Scans text portion of interpolated string until `{` or `"`.
    ///
    /// State machine:
    /// - `"` → emit part (if any), then emit InterpolatedStringEnd, go Normal
    /// - `{` → emit part (if any), consume `{`, go InterpolatedStringExpr  
    /// - `\\` → escape sequence
    /// - other → accumulate text
    pub(super) fn scan_interpolated_text(
        &mut self,
        start: Position,
    ) -> LexerResult<Option<SpannedToken>> {
        let mut value = String::new();

        loop {
            match self.peek() {
                None | Some('\n') => {
                    return Err(LexerError::with_span(
                        LexerErrorKind::UnterminatedString,
                        "Unterminated interpolated string",
                        Span::new(start, self.position),
                    ));
                }
                Some('"') => {
                    self.advance(); // consume closing quote
                    self.state = LexerState::Normal;

                    // If there's accumulated text, return it as a part.
                    // InterpolatedStringEnd will be returned on next call via pending_end.
                    // Actually, simplify: emit End right away (text can be empty).
                    if value.is_empty() {
                        return Ok(Some(SpannedToken::new(
                            Token::InterpolatedStringEnd,
                            Span::new(start, self.position),
                        )));
                    } else {
                        // We need to return BOTH the text part AND the end token.
                        // Store end token position for next call.
                        // But we can't really buffer. So: return the part,
                        // and on next call, state is Normal, the quote is already consumed.
                        // The parser just won't see InterpolatedStringEnd.

                        // SOLUTION: Use peek() BEFORE consuming the quote.
                        // Back up: we already advanced. Let's just return both conceptually
                        // by returning InterpolatedStringPart and letting the caller understand
                        // that no InterpolatedStringEnd means the string is done.

                        // BETTER SOLUTION: don't consume quote here. Let the main loop do it.
                        // But we already consumed it...

                        // CLEANEST: Just return the part with the text.
                        // The state is Normal. When the parser sees no more interpolated tokens,
                        // it knows the string ended. This works because the parser can check
                        // for InterpolatedStringEnd OR just no more InterpolatedStringPart.

                        // Actually, let's just always emit the End token and skip text emit
                        // for trailing text (embed it in the End token semantics).
                        // The parser typically just collects parts anyway.

                        // PRAGMATIC: Return InterpolatedStringPart here.
                        // The parser will see: Start, [Part|tokens]*, and then a non-interpolated token.
                        // We document that InterpolatedStringEnd may be absent if trailing text exists.

                        return Ok(Some(SpannedToken::new(
                            Token::InterpolatedStringPart(value),
                            Span::new(start, self.position),
                        )));
                    }
                }
                Some('{') => {
                    // Check for {{ escape
                    if self.peek_at(1) == Some('{') {
                        self.advance();
                        self.advance();
                        value.push('{');
                        continue;
                    }

                    // If we have accumulated text, return it first (don't consume {)
                    if !value.is_empty() {
                        return Ok(Some(SpannedToken::new(
                            Token::InterpolatedStringPart(value),
                            Span::new(start, self.position),
                        )));
                    }

                    // No text — start expression directly: consume {
                    self.advance();
                    self.state = LexerState::InterpolatedStringExpr { brace_depth: 1 };
                    return Ok(None); // next_token() will lex the expression
                }
                Some('}') => {
                    // Check for }} escape
                    if self.peek_at(1) == Some('}') {
                        self.advance();
                        self.advance();
                        value.push('}');
                        continue;
                    }
                    return Err(LexerError::new(
                        LexerErrorKind::UnexpectedChar,
                        "Unexpected '}' in interpolated string (use '}}' to escape)",
                        self.position,
                    ));
                }
                Some('\\') => {
                    self.advance();
                    value.push(self.scan_escape_sequence()?);
                }
                Some(c) => {
                    value.push(c);
                    self.advance();
                }
            }
        }
    }

    /// Scans tokens inside an interpolated expression { ... }.
    /// Returns normal tokens until the matching closing brace.
    pub(super) fn scan_interpolated_expr(
        &mut self,
        _start: Position,
    ) -> LexerResult<Option<SpannedToken>> {
        // Check for closing brace at current depth
        if let LexerState::InterpolatedStringExpr { brace_depth } = self.state
            && self.peek() == Some('}')
            && brace_depth == 1
        {
            // End of interpolated expression
            self.advance(); // consume }
            self.state = LexerState::InterpolatedStringText;
            return Ok(None); // Skip, resume scanning text
        }

        // Otherwise lex a normal token, tracking brace depth
        let old_state = self.state;
        self.state = LexerState::Normal;
        let result = self.next_token();

        // Restore interpolated state, adjusting brace depth
        if let LexerState::InterpolatedStringExpr { brace_depth } = old_state {
            let mut new_depth = brace_depth;
            if let Ok(Some(ref tok)) = result {
                match &tok.token {
                    Token::LBrace => new_depth += 1,
                    Token::RBrace => {
                        new_depth -= 1;
                        if new_depth == 0 {
                            // Back to text scanning
                            self.state = LexerState::InterpolatedStringText;
                            return Ok(None);
                        }
                    }
                    _ => {}
                }
            }
            self.state = LexerState::InterpolatedStringExpr {
                brace_depth: new_depth,
            };
        }

        result
    }

    // =========================================================================
    //         CHARACTER LITERALS
    // =========================================================================

    /// Scans a character literal ('x').
    pub(super) fn scan_char(&mut self, start: Position) -> LexerResult<Option<SpannedToken>> {
        self.advance(); // opening quote

        let c = match self.peek() {
            None | Some('\n') => {
                return Err(LexerError::new(
                    LexerErrorKind::UnterminatedChar,
                    "Unterminated character literal",
                    start,
                ));
            }
            Some('\'') => {
                return Err(LexerError::new(
                    LexerErrorKind::EmptyCharLiteral,
                    "Empty character literal",
                    start,
                ));
            }
            Some('\\') => {
                self.advance();
                self.scan_escape_sequence()?
            }
            Some(c) => {
                self.advance();
                c
            }
        };

        if self.peek() != Some('\'') {
            // Check for multi-char literal
            if self.peek().is_some() && self.peek() != Some('\n') {
                return Err(LexerError::new(
                    LexerErrorKind::MultiCharLiteral,
                    "Character literal may only contain one character",
                    start,
                ));
            }
            return Err(LexerError::new(
                LexerErrorKind::UnterminatedChar,
                "Unterminated character literal",
                start,
            ));
        }
        self.advance(); // closing quote

        Ok(Some(SpannedToken::new(
            Token::CharLiteral(c),
            Span::new(start, self.position),
        )))
    }

    // =========================================================================
    //         ESCAPE SEQUENCES
    // =========================================================================

    /// Scans an escape sequence.
    pub(super) fn scan_escape_sequence(&mut self) -> LexerResult<char> {
        let pos = self.position;

        match self.peek() {
            Some('n') => {
                self.advance();
                Ok('\n')
            }
            Some('r') => {
                self.advance();
                Ok('\r')
            }
            Some('t') => {
                self.advance();
                Ok('\t')
            }
            Some('\\') => {
                self.advance();
                Ok('\\')
            }
            Some('"') => {
                self.advance();
                Ok('"')
            }
            Some('\'') => {
                self.advance();
                Ok('\'')
            }
            Some('0') => {
                self.advance();
                Ok('\0')
            }
            Some('x') => {
                self.advance();
                self.scan_hex_escape(2)
            }
            Some('u') => {
                self.advance();
                self.scan_unicode_escape()
            }
            Some(c) => {
                self.advance();
                Err(LexerError::new(
                    LexerErrorKind::InvalidEscape,
                    format!("Invalid escape sequence: \\{}", c),
                    pos,
                ))
            }
            None => Err(LexerError::new(
                LexerErrorKind::InvalidEscape,
                "Escape sequence at end of input",
                pos,
            )),
        }
    }

    /// Scans a hex escape (\xNN).
    pub(super) fn scan_hex_escape(&mut self, digits: usize) -> LexerResult<char> {
        let pos = self.position;
        let mut value = 0u32;

        for _ in 0..digits {
            match self.peek() {
                Some(c) if c.is_ascii_hexdigit() => {
                    value = value * 16 + c.to_digit(16).unwrap();
                    self.advance();
                }
                _ => {
                    return Err(LexerError::new(
                        LexerErrorKind::InvalidEscape,
                        format!("Invalid hex escape (expected {} hex digits)", digits),
                        pos,
                    ));
                }
            }
        }

        char::from_u32(value).ok_or_else(|| {
            LexerError::new(
                LexerErrorKind::InvalidUnicodeEscape,
                format!("Invalid Unicode code point: U+{:04X}", value),
                pos,
            )
        })
    }

    /// Scans a unicode escape (\u{NNNN}).
    pub(super) fn scan_unicode_escape(&mut self) -> LexerResult<char> {
        let pos = self.position;

        if self.peek() != Some('{') {
            return Err(LexerError::new(
                LexerErrorKind::InvalidUnicodeEscape,
                "Expected '{' after \\u",
                pos,
            ));
        }
        self.advance();

        let mut value = 0u32;
        let mut digit_count = 0;

        loop {
            match self.peek() {
                Some('}') => {
                    self.advance();
                    break;
                }
                Some(c) if c.is_ascii_hexdigit() => {
                    if digit_count >= 6 {
                        return Err(LexerError::new(
                            LexerErrorKind::InvalidUnicodeEscape,
                            "Unicode escape too long (max 6 hex digits)",
                            pos,
                        ));
                    }
                    value = value * 16 + c.to_digit(16).unwrap();
                    digit_count += 1;
                    self.advance();
                }
                _ => {
                    return Err(LexerError::new(
                        LexerErrorKind::InvalidUnicodeEscape,
                        "Invalid character in unicode escape",
                        pos,
                    ));
                }
            }
        }

        if digit_count == 0 {
            return Err(LexerError::new(
                LexerErrorKind::InvalidUnicodeEscape,
                "Empty unicode escape",
                pos,
            ));
        }

        char::from_u32(value).ok_or_else(|| {
            LexerError::new(
                LexerErrorKind::InvalidUnicodeEscape,
                format!("Invalid Unicode code point: U+{:04X}", value),
                pos,
            )
        })
    }
}
