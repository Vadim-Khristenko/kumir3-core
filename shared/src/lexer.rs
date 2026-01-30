// ============================================================================
//                         ЛЕКСЕР ЯЗЫКА КУМИР 3
// ============================================================================
//
// Модуль токенизации исходного кода Кумир.
//
// Лексер преобразует исходный текст программы в поток токенов,
// которые затем обрабатываются парсером для построения AST.
//
// ============================================================================

use crate::types::Token;
use crate::constants::{
    KEYWORDS, OPERATORS_1, OPERATORS_2, OPERATORS_3,
    is_ident_start, is_ident_continue, is_whitespace, is_digit_start,
};
use crate::constants::errors::errors;

// ============================================================================
//                         ПОЗИЦИЯ В ИСХОДНОМ КОДЕ
// ============================================================================

/// Позиция токена в исходном коде.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Position {
    /// Номер строки (начиная с 1)
    pub line: usize,
    /// Номер столбца (начиная с 1)
    pub column: usize,
    /// Абсолютное смещение от начала файла
    pub offset: usize,
}

impl Position {
    /// Создаёт новую позицию.
    pub fn new(line: usize, column: usize, offset: usize) -> Self {
        Self { line, column, offset }
    }
}

impl std::fmt::Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

// ============================================================================
//                         ДИАПАЗОН В ИСХОДНОМ КОДЕ
// ============================================================================

/// Диапазон (span) токена в исходном коде.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    /// Начальная позиция
    pub start: Position,
    /// Конечная позиция
    pub end: Position,
}

impl Span {
    /// Создаёт новый диапазон.
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }
    
    /// Объединяет два диапазона.
    pub fn merge(self, other: Span) -> Span {
        Span {
            start: if self.start.offset < other.start.offset { self.start } else { other.start },
            end: if self.end.offset > other.end.offset { self.end } else { other.end },
        }
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.start, self.end)
    }
}

// ============================================================================
//                         ТОКЕН С ПОЗИЦИЕЙ
// ============================================================================

/// Токен с информацией о позиции в исходном коде.
#[derive(Debug, Clone, PartialEq)]
pub struct SpannedToken {
    /// Тип токена
    pub token: Token,
    /// Диапазон в исходном коде
    pub span: Span,
}

impl SpannedToken {
    /// Создаёт новый токен с позицией.
    pub fn new(token: Token, span: Span) -> Self {
        Self { token, span }
    }
}

// ============================================================================
//                         ОШИБКА ЛЕКСЕРА
// ============================================================================

/// Ошибка лексического анализа.
#[derive(Debug, Clone, PartialEq)]
pub struct LexerError {
    /// Сообщение об ошибке
    pub message: String,
    /// Позиция ошибки
    pub position: Position,
}

impl LexerError {
    /// Создаёт новую ошибку лексера.
    pub fn new(message: impl Into<String>, position: Position) -> Self {
        Self {
            message: message.into(),
            position,
        }
    }
}

impl std::fmt::Display for LexerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} в позиции {}", self.message, self.position)
    }
}

impl std::error::Error for LexerError {}

// ============================================================================
//                         РЕЗУЛЬТАТ ЛЕКСЕРА
// ============================================================================

/// Результат работы лексера.
pub type LexerResult<T> = Result<T, LexerError>;

// ============================================================================
//                         ЛЕКСЕР
// ============================================================================

/// Лексер языка Кумир.
///
/// Преобразует исходный код в поток токенов.
///
/// # Пример
///
/// ```
/// use crate::shared::lexer::Lexer;
///
/// let source = "алг Привет\nнач\n  вывод \"Hello\"\nкон";
/// let mut lexer = Lexer::new(source);
/// let tokens = lexer.tokenize().unwrap();
/// ```
pub struct Lexer<'a> {
    /// Исходный код как срез байтов
    source: &'a str,
    /// Итератор по символам
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
    /// Текущая позиция
    position: Position,
    /// Текущий символ (если есть)
    current: Option<(usize, char)>,
    /// Флаг: находимся внутри Rust-вставки (РастВставкаНЦ...РастВставкаКЦ)
    in_rust_block: bool,
    /// Флаг: находимся внутри альтернативной Rust-вставки (ржавчина нач...кон)
    in_rust_alt_block: bool,
}

impl<'a> Lexer<'a> {
    /// Создаёт новый лексер для заданного исходного кода.
    pub fn new(source: &'a str) -> Self {
        let mut chars = source.char_indices().peekable();
        let current = chars.next();
        
        Self {
            source,
            chars,
            position: Position::new(1, 1, 0),
            current,
            in_rust_block: false,
            in_rust_alt_block: false,
        }
    }
    
    /// Выполняет полную токенизацию исходного кода.
    pub fn tokenize(&mut self) -> LexerResult<Vec<SpannedToken>> {
        let mut tokens = Vec::new();
        
        while !self.is_eof() {
            if let Some(token) = self.next_token()? {
                tokens.push(token);
            }
        }
        
        // Добавляем токен конца файла
        tokens.push(SpannedToken::new(
            Token::EOF,
            Span::new(self.position, self.position),
        ));
        
        Ok(tokens)
    }
    
    /// Возвращает следующий токен или None для пропускаемых элементов.
    pub fn next_token(&mut self) -> LexerResult<Option<SpannedToken>> {
        // Пропускаем пробелы
        self.skip_whitespace();
        
        if self.is_eof() {
            return Ok(None);
        }
        
        let start = self.position;
        let (_, c) = self.current.unwrap();
        
        // Обработка Rust-вставок (РастВставкаНЦ...РастВставкаКЦ)
        if self.in_rust_block {
            return self.scan_rust_block(start);
        }
        
        // Обработка альтернативных Rust-вставок (ржавчина нач...кон)
        if self.in_rust_alt_block {
            return self.scan_rust_alt_block(start);
        }
        
        // Новая строка
        if c == '\n' {
            self.advance();
            return Ok(Some(SpannedToken::new(
                Token::Newline,
                Span::new(start, self.position),
            )));
        }
        
        // Комментарий
        // Комментарий (но не |>)
        if c == '|' {
            // Проверяем, не является ли это оператором |>
            if self.peek_next() == Some('>') {
                // Это |> - обрабатываем как оператор
            } else {
                return self.scan_comment(start);
            }
        }
        
        // Строковый литерал
        if c == '"' {
            return self.scan_string(start);
        }
        
        // Символьный литерал
        if c == '\'' {
            return self.scan_char(start);
        }
        
        // Число
        if is_digit_start(c) {
            return self.scan_number(start);
        }
        
        // Идентификатор или ключевое слово
        if is_ident_start(c) {
            return self.scan_identifier(start);
        }
        
        // Операторы (сначала проверяем более длинные)
        if let Some(token) = self.try_scan_operator()? {
            return Ok(Some(token));
        }
        
        // Неизвестный символ
        self.advance();
        Err(LexerError::new(
            format!("{}: '{}'", errors::UNEXPECTED_CHAR, c),
            start,
        ))
    }
    
    // -------------------------------------------------------------------------
    //                         ВСПОМОГАТЕЛЬНЫЕ МЕТОДЫ
    // -------------------------------------------------------------------------
    
    /// Проверяет, достигнут ли конец файла.
    #[inline]
    fn is_eof(&self) -> bool {
        self.current.is_none()
    }
    
    /// Возвращает текущий символ без продвижения.
    #[inline]
    fn peek(&self) -> Option<char> {
        self.current.map(|(_, c)| c)
    }
    
    /// Возвращает следующий символ (заглядывание вперёд).
    #[inline]
    fn peek_next(&mut self) -> Option<char> {
        self.chars.peek().map(|(_, c)| *c)
    }
    
    /// Продвигает позицию на один символ.
    fn advance(&mut self) -> Option<char> {
        let result = self.current.map(|(_, c)| c);
        
        if let Some((_, c)) = self.current {
            if c == '\n' {
                self.position.line += 1;
                self.position.column = 1;
            } else {
                self.position.column += 1;
            }
            self.position.offset += c.len_utf8();
        }
        
        self.current = self.chars.next();
        result
    }
    
    /// Пропускает пробельные символы (кроме новой строки).
    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if is_whitespace(c) {
                self.advance();
            } else {
                break;
            }
        }
    }
    
    /// Извлекает срез исходного кода.
    fn slice(&self, start: usize, end: usize) -> &'a str {
        &self.source[start..end]
    }
    
    // -------------------------------------------------------------------------
    //                         СКАНИРОВАНИЕ ТОКЕНОВ
    // -------------------------------------------------------------------------
    
    /// Сканирует комментарий (начинается с |).
    fn scan_comment(&mut self, start: Position) -> LexerResult<Option<SpannedToken>> {
        self.advance(); // пропускаем |
        
        let content_start = self.position.offset;
        
        while let Some(c) = self.peek() {
            if c == '\n' {
                break;
            }
            self.advance();
        }
        
        let content = self.slice(content_start, self.position.offset).to_string();
        
        Ok(Some(SpannedToken::new(
            Token::Comment(content),
            Span::new(start, self.position),
        )))
    }
    
    /// Сканирует строковый литерал.
    /// 
    /// Поддерживает:
    /// - Однострочные строки: "hello"
    /// - Многострочные строки (тройные кавычки): """multi
    ///   line
    ///   string"""
    fn scan_string(&mut self, start: Position) -> LexerResult<Option<SpannedToken>> {
        self.advance(); // пропускаем первую кавычку
        
        // Проверяем на тройные кавычки (многострочная строка)
        let is_multiline = if self.peek() == Some('"') {
            self.advance(); // вторая кавычка
            if self.peek() == Some('"') {
                self.advance(); // третья кавычка
                true
            } else {
                // Пустая строка ""
                return Ok(Some(SpannedToken::new(
                    Token::String(String::new()),
                    Span::new(start, self.position),
                )));
            }
        } else {
            false
        };
        
        let mut value = String::new();
        
        if is_multiline {
            // Многострочная строка - ищем закрывающие """
            loop {
                match self.peek() {
                    None => {
                        return Err(LexerError::new(errors::UNTERMINATED_STRING, start));
                    }
                    Some('"') => {
                        // Проверяем на """
                        if self.peek_next() == Some('"') {
                            // Сохраняем позицию для проверки третьей кавычки
                            self.advance(); // первая "
                            if self.peek_next() == Some('"') {
                                self.advance(); // вторая "
                                self.advance(); // третья "
                                break;
                            } else {
                                // Только две кавычки - добавляем их в строку
                                value.push('"');
                                // Вторая кавычка уже считана, добавим её тоже
                                value.push('"');
                            }
                        } else {
                            // Одна кавычка - добавляем в строку
                            value.push('"');
                            self.advance();
                        }
                    }
                    Some('\\') => {
                        self.advance();
                        let escaped = self.scan_escape_sequence()?;
                        value.push(escaped);
                    }
                    Some(c) => {
                        value.push(c);
                        self.advance();
                    }
                }
            }
        } else {
            // Однострочная строка
            loop {
                match self.peek() {
                    None | Some('\n') => {
                        return Err(LexerError::new(errors::UNTERMINATED_STRING, start));
                    }
                    Some('"') => {
                        self.advance(); // пропускаем закрывающую кавычку
                        break;
                    }
                    Some('\\') => {
                        self.advance();
                        let escaped = self.scan_escape_sequence()?;
                        value.push(escaped);
                    }
                    Some(c) => {
                        value.push(c);
                        self.advance();
                    }
                }
            }
        }
        
        Ok(Some(SpannedToken::new(
            Token::String(value),
            Span::new(start, self.position),
        )))
    }
    
    /// Сканирует символьный литерал.
    fn scan_char(&mut self, start: Position) -> LexerResult<Option<SpannedToken>> {
        self.advance(); // пропускаем открывающую кавычку
        
        let c = match self.peek() {
            None | Some('\n') | Some('\'') => {
                return Err(LexerError::new(errors::UNTERMINATED_CHAR, start));
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
            return Err(LexerError::new(errors::UNTERMINATED_CHAR, start));
        }
        self.advance(); // пропускаем закрывающую кавычку
        
        Ok(Some(SpannedToken::new(
            Token::Char(c),
            Span::new(start, self.position),
        )))
    }
    
    /// Сканирует escape-последовательность.
    fn scan_escape_sequence(&mut self) -> LexerResult<char> {
        match self.peek() {
            Some('n') => { self.advance(); Ok('\n') }
            Some('r') => { self.advance(); Ok('\r') }
            Some('t') => { self.advance(); Ok('\t') }
            Some('\\') => { self.advance(); Ok('\\') }
            Some('"') => { self.advance(); Ok('"') }
            Some('\'') => { self.advance(); Ok('\'') }
            Some('0') => { self.advance(); Ok('\0') }
            Some(c) => {
                let pos = self.position;
                self.advance();
                Err(LexerError::new(
                    format!("{}: \\{}", errors::INVALID_ESCAPE, c),
                    pos,
                ))
            }
            None => {
                Err(LexerError::new(errors::INVALID_ESCAPE, self.position))
            }
        }
    }
    
    /// Сканирует числовой литерал.
    fn scan_number(&mut self, start: Position) -> LexerResult<Option<SpannedToken>> {
        let num_start = self.position.offset;
        let mut is_float = false;
        let mut has_exponent = false;
        
        // Целая часть
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.advance();
            } else {
                break;
            }
        }
        
        // Проверка шестнадцатеричного числа (0x...)
        if self.slice(num_start, self.position.offset) == "0" {
            if let Some('x') | Some('X') = self.peek() {
                self.advance();
                while let Some(c) = self.peek() {
                    if c.is_ascii_hexdigit() {
                        self.advance();
                    } else {
                        break;
                    }
                }
                let hex_str = self.slice(num_start + 2, self.position.offset);
                return match i64::from_str_radix(hex_str, 16) {
                    Ok(n) => Ok(Some(SpannedToken::new(
                        Token::Integer(n),
                        Span::new(start, self.position),
                    ))),
                    Err(_) => Err(LexerError::new(errors::INVALID_NUMBER, start)),
                };
            }
        }
        
        // Десятичная точка
        if self.peek() == Some('.') {
            if let Some(next) = self.peek_next() {
                if next.is_ascii_digit() {
                    is_float = true;
                    self.advance(); // точка
                    while let Some(c) = self.peek() {
                        if c.is_ascii_digit() {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
            }
        }
        
        // Экспонента (e или E)
        if let Some('e') | Some('E') = self.peek() {
            has_exponent = true;
            is_float = true;
            self.advance();
            
            // Знак экспоненты
            if let Some('+') | Some('-') = self.peek() {
                self.advance();
            }
            
            // Цифры экспоненты
            let exp_start = self.position.offset;
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    self.advance();
                } else {
                    break;
                }
            }
            
            if self.position.offset == exp_start {
                return Err(LexerError::new(errors::INVALID_NUMBER, start));
            }
        }
        
        let num_str = self.slice(num_start, self.position.offset);
        
        if is_float || has_exponent {
            match num_str.parse::<f64>() {
                Ok(n) => Ok(Some(SpannedToken::new(
                    Token::Float(n),
                    Span::new(start, self.position),
                ))),
                Err(_) => Err(LexerError::new(errors::INVALID_NUMBER, start)),
            }
        } else {
            match num_str.parse::<i64>() {
                Ok(n) => Ok(Some(SpannedToken::new(
                    Token::Integer(n),
                    Span::new(start, self.position),
                ))),
                Err(_) => Err(LexerError::new(errors::INVALID_NUMBER, start)),
            }
        }
    }
    
    /// Сканирует идентификатор или ключевое слово.
    fn scan_identifier(&mut self, start: Position) -> LexerResult<Option<SpannedToken>> {
        let ident_start = self.position.offset;
        
        while let Some(c) = self.peek() {
            if is_ident_continue(c) {
                self.advance();
            } else {
                break;
            }
        }
        
        let ident = self.slice(ident_start, self.position.offset);
        
        // Проверка на начало/конец Rust-вставки (РастВставкаНЦ)
        if ident == "РастВставкаНЦ" {
            self.in_rust_block = true;
            return Ok(Some(SpannedToken::new(
                Token::RustBlockStart,
                Span::new(start, self.position),
            )));
        }
        
        // Проверка на альтернативный синтаксис Rust-вставки (ржавчина нач ... кон)
        if ident == "ржавчина" || ident == "Ржавчина" || ident == "rust" {
            // Смотрим вперёд: пропускаем пробелы и проверяем на "нач"
            let saved_position = self.position;
            let saved_current = self.current;
            let saved_chars = self.chars.clone();
            
            self.skip_whitespace();
            
            // Проверяем следующий идентификатор
            if let Some(c) = self.peek() {
                if is_ident_start(c) {
                    let next_start = self.position.offset;
                    while let Some(c) = self.peek() {
                        if is_ident_continue(c) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    let next_ident = self.slice(next_start, self.position.offset);
                    
                    if next_ident == "нач" {
                        // Это "ржавчина нач" - входим в альтернативный Rust-блок
                        self.in_rust_alt_block = true;
                        return Ok(Some(SpannedToken::new(
                            Token::RustBlockStart,
                            Span::new(start, self.position),
                        )));
                    }
                }
            }
            
            // Не "нач" - восстанавливаем позицию и возвращаем как обычный токен
            self.position = saved_position;
            self.current = saved_current;
            self.chars = saved_chars;
        }
        
        // Поиск в таблице ключевых слов
        let token = if let Some(kw_token) = KEYWORDS.get(ident) {
            kw_token.clone()
        } else {
            Token::Identifier(ident.to_string())
        };
        
        Ok(Some(SpannedToken::new(
            token,
            Span::new(start, self.position),
        )))
    }
    
    /// Пытается сканировать оператор.
    fn try_scan_operator(&mut self) -> LexerResult<Option<SpannedToken>> {
        let start = self.position;
        let _start_offset = self.position.offset;
        
        // Получаем текущий и следующие символы для проверки
        let c1 = self.peek();
        
        if c1.is_none() {
            return Ok(None);
        }
        
        let c1 = c1.unwrap();
        
        // Пробуем трёхсимвольные операторы
        if let Some(c2) = self.peek_next() {
            let two_chars = format!("{}{}", c1, c2);
            
            // Сохраняем состояние для проверки трёхсимвольных
            // peek_next() возвращает c2, поэтому после клонирования нужно пропустить c2 чтобы получить c3
            let mut temp_chars = self.chars.clone();
            temp_chars.next(); // пропускаем c2
            if let Some((_, c3)) = temp_chars.next() {
                let three_chars = format!("{}{}{}", c1, c2, c3);
                if let Some(token) = OPERATORS_3.get(three_chars.as_str()) {
                    self.advance();
                    self.advance();
                    self.advance();
                    return Ok(Some(SpannedToken::new(
                        token.clone(),
                        Span::new(start, self.position),
                    )));
                }
            }
            
            // Пробуем двухсимвольные операторы
            if let Some(token) = OPERATORS_2.get(two_chars.as_str()) {
                self.advance();
                self.advance();
                return Ok(Some(SpannedToken::new(
                    token.clone(),
                    Span::new(start, self.position),
                )));
            }
        }
        
        // Пробуем односимвольные операторы
        if let Some(token) = OPERATORS_1.get(&c1) {
            self.advance();
            return Ok(Some(SpannedToken::new(
                token.clone(),
                Span::new(start, self.position),
            )));
        }
        
        Ok(None)
    }
    
    /// Сканирует содержимое Rust-вставки.
    fn scan_rust_block(&mut self, start: Position) -> LexerResult<Option<SpannedToken>> {
        let mut content = String::new();
        let _content_start = self.position.offset;
        
        loop {
            // Проверяем на конец Rust-вставки
            if let Some(c) = self.peek() {
                // Ищем "РастВставкаКЦ"
                let remaining = &self.source[self.position.offset..];
                if remaining.starts_with("РастВставкаКЦ") {
                    // Сохраняем накопленный контент
                    if !content.is_empty() {
                        // Возвращаем контент как RustCode
                        return Ok(Some(SpannedToken::new(
                            Token::RustCode(content),
                            Span::new(start, self.position),
                        )));
                    }
                    
                    // Пропускаем "РастВставкаКЦ"
                    for _ in "РастВставкаКЦ".chars() {
                        self.advance();
                    }
                    
                    self.in_rust_block = false;
                    return Ok(Some(SpannedToken::new(
                        Token::RustBlockEnd,
                        Span::new(start, self.position),
                    )));
                }
                
                content.push(c);
                self.advance();
            } else {
                // EOF внутри Rust-вставки
                if !content.is_empty() {
                    return Ok(Some(SpannedToken::new(
                        Token::RustCode(content),
                        Span::new(start, self.position),
                    )));
                }
                return Ok(None);
            }
        }
    }
    
    /// Сканирует содержимое альтернативной Rust-вставки (ржавчина нач ... кон).
    fn scan_rust_alt_block(&mut self, start: Position) -> LexerResult<Option<SpannedToken>> {
        let mut content = String::new();
        
        loop {
            if let Some(c) = self.peek() {
                // Проверяем на "кон" как отдельное слово
                let remaining = &self.source[self.position.offset..];
                
                // Проверяем начинается ли с "кон" и после него нет буквы/цифры
                if remaining.starts_with("кон") {
                    let after_kon = remaining.get(6..); // "кон" в UTF-8 занимает 6 байт (3 символа по 2 байта)
                    let is_word_end = match after_kon {
                        None => true,
                        Some(s) => s.chars().next().map(|c| !is_ident_continue(c)).unwrap_or(true),
                    };
                    
                    if is_word_end {
                        // Возвращаем накопленный контент
                        if !content.is_empty() {
                            return Ok(Some(SpannedToken::new(
                                Token::RustCode(content),
                                Span::new(start, self.position),
                            )));
                        }
                        
                        // Пропускаем "кон"
                        for _ in "кон".chars() {
                            self.advance();
                        }
                        
                        self.in_rust_alt_block = false;
                        return Ok(Some(SpannedToken::new(
                            Token::RustBlockEnd,
                            Span::new(start, self.position),
                        )));
                    }
                }
                
                content.push(c);
                self.advance();
            } else {
                // EOF внутри Rust-вставки
                if !content.is_empty() {
                    return Ok(Some(SpannedToken::new(
                        Token::RustCode(content),
                        Span::new(start, self.position),
                    )));
                }
                return Ok(None);
            }
        }
    }
}

// ============================================================================
//                         УДОБНАЯ ФУНКЦИЯ ТОКЕНИЗАЦИИ
// ============================================================================

/// Выполняет токенизацию исходного кода.
///
/// # Аргументы
///
/// * `source` - Исходный код программы на Кумире
///
/// # Возвращает
///
/// Вектор токенов с информацией о позиции или ошибку лексера.
///
/// # Пример
///
/// ```ignore
/// let tokens = tokenize("алг Тест\nнач\nкон")?;
/// ```
pub fn tokenize(source: &str) -> LexerResult<Vec<SpannedToken>> {
    Lexer::new(source).tokenize()
}
