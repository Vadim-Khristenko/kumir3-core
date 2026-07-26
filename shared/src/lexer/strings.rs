// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

//! String, character, and escape sequence scanning.

use super::{
    Lexer, LexerError, LexerErrorKind, LexerResult, LexerState, Position, Span, SpannedToken,
};
use crate::types::Token;

impl<'a> Lexer<'a> {
    // =============================================================================
    //         SECTION: STRING LITERALS
    // =============================================================================

    /// Scans a string literal ("..." or """...""").
    pub(super) fn scan_string(&mut self, start: Position) -> LexerResult<Option<SpannedToken>> {
        self.advance();

        let is_multiline = if self.peek() == Some('"') && self.peek_at(1) == Some('"') {
            self.advance();
            self.advance();
            true
        } else if self.peek() == Some('"') {
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
                        "Незакрытая строка",
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
                        "Незакрытая строка: перевод строки внутри кавычек \
                         допускается только в тройных кавычках",
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
        self.advance();

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
        self.advance();

        let mut value = String::new();

        loop {
            match self.peek() {
                None => {
                    return Err(LexerError::with_span(
                        LexerErrorKind::UnterminatedString,
                        "Незакрытая «сырая» строка",
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

    /// Scans start of interpolated string (f"...").
    /// Emits InterpolatedStringStart, then switches to InterpolatedStringText state.
    pub(super) fn scan_interpolated_string(
        &mut self,
        start: Position,
    ) -> LexerResult<Option<SpannedToken>> {
        self.advance();
        self.advance();

        self.state = LexerState::InterpolatedStringText;

        Ok(Some(SpannedToken::new(
            Token::InterpolatedStringStart,
            Span::new(start, self.position),
        )))
    }

    /// Scans text portion of interpolated string.
    ///
    /// Transitions:
    /// - `"` → emit part, emit InterpolatedStringEnd, return to Normal
    /// - `{` → emit part, switch to InterpolatedStringExpr
    /// - `\\` → parse escape sequence
    /// - `}}` → escaped closing brace (emit single `}`)
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
                        "Незакрытая строка со вставками",
                        Span::new(start, self.position),
                    ));
                }
                Some('"') => {
                    self.advance();
                    self.state = LexerState::Normal;

                    if value.is_empty() {
                        return Ok(Some(SpannedToken::new(
                            Token::InterpolatedStringEnd,
                            Span::new(start, self.position),
                        )));
                    } else {
                        return Ok(Some(SpannedToken::new(
                            Token::InterpolatedStringPart(value),
                            Span::new(start, self.position),
                        )));
                    }
                }
                Some('{') => {
                    if self.peek_at(1) == Some('{') {
                        self.advance();
                        self.advance();
                        value.push('{');
                        continue;
                    }

                    if !value.is_empty() {
                        return Ok(Some(SpannedToken::new(
                            Token::InterpolatedStringPart(value),
                            Span::new(start, self.position),
                        )));
                    }

                    self.advance();
                    self.state = LexerState::InterpolatedStringExpr { brace_depth: 1 };
                    return Ok(None);
                }
                Some('}') => {
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

    /// Scans tokens inside an interpolated expression `{...}`.
    /// Handles brace nesting to find the expression boundary.
    pub(super) fn scan_interpolated_expr(
        &mut self,
        _start: Position,
    ) -> LexerResult<Option<SpannedToken>> {
        if let LexerState::InterpolatedStringExpr { brace_depth } = self.state
            && self.peek() == Some('}')
            && brace_depth == 1
        {
            self.advance();
            self.state = LexerState::InterpolatedStringText;
            return Ok(None);
        }

        let old_state = self.state;
        self.state = LexerState::Normal;
        let result = self.next_token();

        if let LexerState::InterpolatedStringExpr { brace_depth } = old_state {
            let mut new_depth = brace_depth;
            if let Ok(Some(ref tok)) = result {
                match &tok.token {
                    Token::LBrace => new_depth += 1,
                    Token::RBrace => {
                        new_depth -= 1;
                        if new_depth == 0 {
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

    // =============================================================================
    //         SECTION: CHARACTER LITERALS
    // =============================================================================

    /// Scans a character literal ('x').
    pub(super) fn scan_char(&mut self, start: Position) -> LexerResult<Option<SpannedToken>> {
        self.advance();

        let c = match self.peek() {
            None | Some('\n') => {
                return Err(LexerError::new(
                    LexerErrorKind::UnterminatedChar,
                    "Незакрытый символьный литерал",
                    start,
                ));
            }
            Some('\'') => {
                return Err(LexerError::new(
                    LexerErrorKind::EmptyCharLiteral,
                    "Пустой символьный литерал",
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
                    "Символьный литерал содержит больше одного символа",
                    start,
                ));
            }
            return Err(LexerError::new(
                LexerErrorKind::UnterminatedChar,
                "Незакрытый символьный литерал",
                start,
            ));
        }
        self.advance(); // closing quote

        Ok(Some(SpannedToken::new(
            Token::CharLiteral(c),
            Span::new(start, self.position),
        )))
    }

    // =============================================================================
    //         SECTION: ESCAPE SEQUENCES
    // =============================================================================

    /// Scans an escape sequence (starting after backslash).
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
                    format!("Неизвестная запись после обратной косой черты: \\{}", c),
                    pos,
                ))
            }
            None => Err(LexerError::new(
                LexerErrorKind::InvalidEscape,
                "Обратная косая черта в конце текста программы",
                pos,
            )),
        }
    }

    /// Scans a hex escape sequence (\xNN).
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
                        format!(
                            "В записи кода символа ожидалось {} шестнадцатеричных цифр",
                            digits
                        ),
                        pos,
                    ));
                }
            }
        }

        char::from_u32(value).ok_or_else(|| {
            LexerError::new(
                LexerErrorKind::InvalidUnicodeEscape,
                format!("Такого символа Юникода не существует: U+{:04X}", value),
                pos,
            )
        })
    }

    /// Scans a Unicode escape sequence (\u{...}).
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
                        "Недопустимый знак в записи кода символа",
                        pos,
                    ));
                }
            }
        }

        if digit_count == 0 {
            return Err(LexerError::new(
                LexerErrorKind::InvalidUnicodeEscape,
                "Пустая запись кода символа",
                pos,
            ));
        }

        char::from_u32(value).ok_or_else(|| {
            LexerError::new(
                LexerErrorKind::InvalidUnicodeEscape,
                format!("Такого символа Юникода не существует: U+{:04X}", value),
                pos,
            )
        })
    }
}
