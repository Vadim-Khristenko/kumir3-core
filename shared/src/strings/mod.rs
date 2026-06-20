// ============================================================================
//                    МОДУЛЬ РАБОТЫ СО СТРОКАМИ (Кумир 3)
// ============================================================================
//
// Реализация стандартных строковых функций языка Кумир.
// Поддерживает как CP-1251 (для совместимости), так и Unicode.
//
// ============================================================================

use crate::types::Value;

// ============================================================================
//                         ОШИБКИ СТРОКОВЫХ ОПЕРАЦИЙ
// ============================================================================

/// Ошибки при работе со строками.
#[derive(Debug, Clone, PartialEq)]
pub enum StringErr {
    /// Индекс выходит за границы строки
    IndexOutOfBounds { index: i64, length: usize },
    /// Некорректный диапазон
    InvalidRange { start: i64, end: i64 },
    /// Некорректный код символа
    InvalidCharCode(i64),
    /// Пустой разделитель
    EmptyDelimiter,
    /// Некорректное количество
    InvalidCount(i64),
    /// Несовместимые типы
    TypeMismatch(&'static str),
    /// Некорректная кодировка
    InvalidEncoding,
}

impl StringErr {
    /// Возвращает сообщение об ошибке на русском языке.
    pub fn msg(&self) -> String {
        match self {
            StringErr::IndexOutOfBounds { index, length } => format!(
                "[StringErr] Индекс {} выходит за границы строки (длина {})",
                index, length
            ),
            StringErr::InvalidRange { start, end } => {
                format!("[StringErr] Некорректный диапазон [{}, {}]", start, end)
            }
            StringErr::InvalidCharCode(code) => {
                format!("[StringErr] Некорректный код символа: {}", code)
            }
            StringErr::EmptyDelimiter => "[StringErr] Разделитель не может быть пустым".to_string(),
            StringErr::InvalidCount(count) => {
                format!("[StringErr] Некорректное количество: {}", count)
            }
            StringErr::TypeMismatch(op) => {
                format!("[StringErr] Несовместимые типы для операции: {}", op)
            }
            StringErr::InvalidEncoding => "[StringErr] Некорректная кодировка".to_string(),
        }
    }
}

impl std::fmt::Display for StringErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg())
    }
}

impl std::error::Error for StringErr {}

/// Результат строковой операции.
pub type StringResult<T> = Result<T, StringErr>;

// ============================================================================
//                         ТАБЛИЦА CP-1251
// ============================================================================

/// Таблица преобразования CP-1251 → Unicode для кодов 128-255.
/// Коды 0-127 совпадают с ASCII.
static CP1251_TO_UNICODE: [u32; 128] = [
    // 0x80-0x8F
    0x0402, 0x0403, 0x201A, 0x0453, 0x201E, 0x2026, 0x2020, 0x2021, 0x20AC, 0x2030, 0x0409, 0x2039,
    0x040A, 0x040C, 0x040B, 0x040F, // 0x90-0x9F
    0x0452, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014, 0x0098, 0x2122, 0x0459, 0x203A,
    0x045A, 0x045C, 0x045B, 0x045F, // 0xA0-0xAF
    0x00A0, 0x040E, 0x045E, 0x0408, 0x00A4, 0x0490, 0x00A6, 0x00A7, 0x0401, 0x00A9, 0x0404, 0x00AB,
    0x00AC, 0x00AD, 0x00AE, 0x0407, // 0xB0-0xBF
    0x00B0, 0x00B1, 0x0406, 0x0456, 0x0491, 0x00B5, 0x00B6, 0x00B7, 0x0451, 0x2116, 0x0454, 0x00BB,
    0x0458, 0x0405, 0x0455, 0x0457, // 0xC0-0xCF (А-П)
    0x0410, 0x0411, 0x0412, 0x0413, 0x0414, 0x0415, 0x0416, 0x0417, 0x0418, 0x0419, 0x041A, 0x041B,
    0x041C, 0x041D, 0x041E, 0x041F, // 0xD0-0xDF (Р-Я)
    0x0420, 0x0421, 0x0422, 0x0423, 0x0424, 0x0425, 0x0426, 0x0427, 0x0428, 0x0429, 0x042A, 0x042B,
    0x042C, 0x042D, 0x042E, 0x042F, // 0xE0-0xEF (а-п)
    0x0430, 0x0431, 0x0432, 0x0433, 0x0434, 0x0435, 0x0436, 0x0437, 0x0438, 0x0439, 0x043A, 0x043B,
    0x043C, 0x043D, 0x043E, 0x043F, // 0xF0-0xFF (р-я)
    0x0440, 0x0441, 0x0442, 0x0443, 0x0444, 0x0445, 0x0446, 0x0447, 0x0448, 0x0449, 0x044A, 0x044B,
    0x044C, 0x044D, 0x044E, 0x044F,
];

/// Строит обратную таблицу Unicode → CP-1251.
fn build_unicode_to_cp1251() -> std::collections::HashMap<u32, u8> {
    let mut map = std::collections::HashMap::new();
    // ASCII часть (0-127)
    for i in 0u8..128 {
        map.insert(i as u32, i);
    }
    // Расширенная часть (128-255)
    for (i, &unicode) in CP1251_TO_UNICODE.iter().enumerate() {
        map.insert(unicode, (i + 128) as u8);
    }
    map
}

use once_cell::sync::Lazy;
static UNICODE_TO_CP1251: Lazy<std::collections::HashMap<u32, u8>> =
    Lazy::new(build_unicode_to_cp1251);

// ============================================================================
//                         СТРУКТУРА СТРОКОВЫХ ОПЕРАЦИЙ
// ============================================================================

/// Структура для работы со строками в Кумире.
pub struct StringOperations;

impl StringOperations {
    // ========================================================================
    //                    ФУНКЦИИ КОДИРОВАНИЯ СИМВОЛОВ
    // ========================================================================

    /// код(сим c) — возвращает код символа в CP-1251.
    ///
    /// Если символ не представим в CP-1251, возвращает -1.
    ///
    /// # Пример
    /// ```
    /// use shared::strings::StringOperations;
    /// assert_eq!(StringOperations::code_cp1251('А'), 192);
    /// assert_eq!(StringOperations::code_cp1251('a'), 97);
    /// ```
    pub fn code_cp1251(c: char) -> i64 {
        let unicode = c as u32;
        if let Some(&cp1251) = UNICODE_TO_CP1251.get(&unicode) {
            cp1251 as i64
        } else {
            -1 // Символ не представим в CP-1251
        }
    }

    /// юникод(сим c) — возвращает Unicode code point символа.
    ///
    /// # Пример
    /// ```
    /// use shared::strings::StringOperations;
    /// assert_eq!(StringOperations::code_unicode('А'), 0x0410);
    /// assert_eq!(StringOperations::code_unicode('a'), 97);
    /// ```
    pub fn code_unicode(c: char) -> i64 {
        c as u32 as i64
    }

    /// символ(цел n) — возвращает символ по коду CP-1251.
    ///
    /// # Пример
    /// ```
    /// use shared::strings::StringOperations;
    /// assert_eq!(StringOperations::char_from_cp1251(192), Ok('А'));
    /// assert_eq!(StringOperations::char_from_cp1251(97), Ok('a'));
    /// ```
    pub fn char_from_cp1251(code: i64) -> StringResult<char> {
        if !(0..=255).contains(&code) {
            return Err(StringErr::InvalidCharCode(code));
        }

        let code = code as u8;
        if code < 128 {
            Ok(code as char)
        } else {
            let unicode = CP1251_TO_UNICODE[(code - 128) as usize];
            char::from_u32(unicode).ok_or(StringErr::InvalidCharCode(code as i64))
        }
    }

    /// юнисимвол(цел n) — возвращает символ по Unicode code point.
    ///
    /// # Пример
    /// ```
    /// use shared::strings::StringOperations;
    /// assert_eq!(StringOperations::char_from_unicode(0x0410), Ok('А'));
    /// assert_eq!(StringOperations::char_from_unicode(97), Ok('a'));
    /// ```
    pub fn char_from_unicode(code: i64) -> StringResult<char> {
        if !(0..=0x10FFFF).contains(&code) {
            return Err(StringErr::InvalidCharCode(code));
        }
        char::from_u32(code as u32).ok_or(StringErr::InvalidCharCode(code))
    }

    // ========================================================================
    //                    ФУНКЦИИ ДЛИНЫ СТРОКИ
    // ========================================================================

    /// длин(лит s) / длина(лит s) — возвращает длину строки в символах (Unicode).
    ///
    /// # Пример
    /// ```
    /// use shared::strings::StringOperations;
    /// assert_eq!(StringOperations::length("Привет"), 6);
    /// assert_eq!(StringOperations::length("Hello"), 5);
    /// ```
    pub fn length(s: &str) -> usize {
        s.chars().count()
    }

    /// длин_байт(лит s) — возвращает длину строки в байтах (UTF-8).
    pub fn length_bytes(s: &str) -> usize {
        s.len()
    }

    // ========================================================================
    //                    ФУНКЦИИ РЕГИСТРА
    // ========================================================================

    /// верхний регистр(лит s) — преобразует строку в верхний регистр.
    ///
    /// # Пример
    /// ```
    /// use shared::strings::StringOperations;
    /// assert_eq!(StringOperations::to_upper("Привет Мир"), "ПРИВЕТ МИР");
    /// ```
    pub fn to_upper(s: &str) -> String {
        s.to_uppercase()
    }

    /// нижний регистр(лит s) — преобразует строку в нижний регистр.
    ///
    /// # Пример
    /// ```
    /// use shared::strings::StringOperations;
    /// assert_eq!(StringOperations::to_lower("Привет Мир"), "привет мир");
    /// ```
    pub fn to_lower(s: &str) -> String {
        s.to_lowercase()
    }

    /// заглавная(лит s) — первая буква заглавная, остальные строчные.
    ///
    /// # Пример
    /// ```
    /// use shared::strings::StringOperations;
    /// assert_eq!(StringOperations::capitalize("привет"), "Привет");
    /// ```
    pub fn capitalize(s: &str) -> String {
        let mut chars = s.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => first
                .to_uppercase()
                .chain(chars.flat_map(|c| c.to_lowercase()))
                .collect(),
        }
    }

    /// каждое слово заглавное(лит s) — каждое слово с заглавной буквы.
    pub fn title_case(s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        let mut capitalize_next = true;

        for c in s.chars() {
            if c.is_whitespace() {
                result.push(c);
                capitalize_next = true;
            } else if capitalize_next {
                result.extend(c.to_uppercase());
                capitalize_next = false;
            } else {
                result.extend(c.to_lowercase());
            }
        }
        result
    }

    /// поменять регистр(лит s) — меняет регистр каждого символа на противоположный.
    pub fn swap_case(s: &str) -> String {
        s.chars()
            .map(|c| {
                if c.is_uppercase() {
                    c.to_lowercase().collect::<String>()
                } else {
                    c.to_uppercase().collect::<String>()
                }
            })
            .collect()
    }

    // ========================================================================
    //                    ФУНКЦИИ ПОИСКА
    // ========================================================================

    /// позиция(лит s, лит frag) / поз(лит s, лит frag) — позиция первого вхождения.
    ///
    /// Возвращает 0, если подстрока не найдена. Индексация с 1.
    ///
    /// # Пример
    /// ```
    /// use shared::strings::StringOperations;
    /// assert_eq!(StringOperations::position("Hello World", "World"), 7);
    /// assert_eq!(StringOperations::position("Hello", "xyz"), 0);
    /// ```
    pub fn position(s: &str, frag: &str) -> i64 {
        if frag.is_empty() {
            return if s.is_empty() { 0 } else { 1 };
        }

        let chars: Vec<char> = s.chars().collect();
        let frag_chars: Vec<char> = frag.chars().collect();

        if frag_chars.len() > chars.len() {
            return 0;
        }

        for i in 0..=(chars.len() - frag_chars.len()) {
            if chars[i..i + frag_chars.len()] == frag_chars[..] {
                return (i + 1) as i64; // Индексация с 1
            }
        }
        0
    }

    /// позиция после(цел start, лит s, лит frag) — позиция вхождения после позиции start.
    ///
    /// # Пример
    /// ```
    /// use shared::strings::StringOperations;
    /// assert_eq!(StringOperations::position_after(1, "abcabc", "bc"), 2);
    /// assert_eq!(StringOperations::position_after(3, "abcabc", "bc"), 5);
    /// ```
    pub fn position_after(start: i64, s: &str, frag: &str) -> i64 {
        if start < 1 {
            return Self::position(s, frag);
        }

        let chars: Vec<char> = s.chars().collect();
        let frag_chars: Vec<char> = frag.chars().collect();
        let start_idx = (start - 1) as usize; // Преобразуем в 0-индексацию

        if start_idx >= chars.len() || frag_chars.is_empty() {
            return 0;
        }

        if frag_chars.len() > chars.len() - start_idx {
            return 0;
        }

        for i in start_idx..=(chars.len() - frag_chars.len()) {
            if chars[i..i + frag_chars.len()] == frag_chars[..] {
                return (i + 1) as i64;
            }
        }
        0
    }

    /// позиция справа(лит s, лит frag) — позиция последнего вхождения.
    ///
    /// # Пример
    /// ```
    /// use shared::strings::StringOperations;
    /// assert_eq!(StringOperations::position_last("abcabc", "bc"), 5);
    /// ```
    pub fn position_last(s: &str, frag: &str) -> i64 {
        if frag.is_empty() {
            return Self::length(s) as i64;
        }

        let chars: Vec<char> = s.chars().collect();
        let frag_chars: Vec<char> = frag.chars().collect();

        if frag_chars.len() > chars.len() {
            return 0;
        }

        for i in (0..=(chars.len() - frag_chars.len())).rev() {
            if chars[i..i + frag_chars.len()] == frag_chars[..] {
                return (i + 1) as i64;
            }
        }
        0
    }

    /// содержит(лит s, лит frag) — проверяет, содержит ли строка подстроку.
    pub fn contains(s: &str, frag: &str) -> bool {
        s.contains(frag)
    }

    /// начинается с(лит s, лит prefix) — проверяет, начинается ли строка с префикса.
    pub fn starts_with(s: &str, prefix: &str) -> bool {
        s.starts_with(prefix)
    }

    /// заканчивается на(лит s, лит suffix) — проверяет, заканчивается ли строка суффиксом.
    pub fn ends_with(s: &str, suffix: &str) -> bool {
        s.ends_with(suffix)
    }

    /// количество вхождений(лит s, лит frag) — подсчитывает количество вхождений.
    pub fn count_occurrences(s: &str, frag: &str) -> i64 {
        if frag.is_empty() {
            return 0;
        }
        s.matches(frag).count() as i64
    }

    // ========================================================================
    //                    ФУНКЦИИ ИЗВЛЕЧЕНИЯ ПОДСТРОК
    // ========================================================================

    /// вырезка(лит s, цел start, цел count) — извлекает подстроку.
    ///
    /// start — начальная позиция (с 1), count — количество символов.
    ///
    /// # Пример
    /// ```
    /// use shared::strings::StringOperations;
    /// assert_eq!(StringOperations::substring("Привет Мир", 1, 6), Ok("Привет".to_string()));
    /// assert_eq!(StringOperations::substring("Hello", 2, 3), Ok("ell".to_string()));
    /// ```
    pub fn substring(s: &str, start: i64, count: i64) -> StringResult<String> {
        if count < 0 {
            return Err(StringErr::InvalidCount(count));
        }

        let chars: Vec<char> = s.chars().collect();
        let len = chars.len() as i64;

        if start < 1 || start > len + 1 {
            return Err(StringErr::IndexOutOfBounds {
                index: start,
                length: chars.len(),
            });
        }

        let start_idx = (start - 1) as usize;
        let count = count as usize;
        let end_idx = (start_idx + count).min(chars.len());

        Ok(chars[start_idx..end_idx].iter().collect())
    }

    /// слева(лит s, цел count) — извлекает count символов слева.
    ///
    /// # Пример
    /// ```
    /// use shared::strings::StringOperations;
    /// assert_eq!(StringOperations::left("Привет", 3), Ok("При".to_string()));
    /// ```
    pub fn left(s: &str, count: i64) -> StringResult<String> {
        if count < 0 {
            return Err(StringErr::InvalidCount(count));
        }
        let chars: Vec<char> = s.chars().collect();
        let count = (count as usize).min(chars.len());
        Ok(chars[..count].iter().collect())
    }

    /// справа(лит s, цел count) — извлекает count символов справа.
    ///
    /// # Пример
    /// ```
    /// use shared::strings::StringOperations;
    /// assert_eq!(StringOperations::right("Привет", 3), Ok("вет".to_string()));
    /// ```
    pub fn right(s: &str, count: i64) -> StringResult<String> {
        if count < 0 {
            return Err(StringErr::InvalidCount(count));
        }
        let chars: Vec<char> = s.chars().collect();
        let count = (count as usize).min(chars.len());
        let start = chars.len() - count;
        Ok(chars[start..].iter().collect())
    }

    /// символ по позиции(лит s, цел pos) — возвращает символ на позиции pos (с 1).
    pub fn char_at(s: &str, pos: i64) -> StringResult<char> {
        let chars: Vec<char> = s.chars().collect();
        if pos < 1 || pos > chars.len() as i64 {
            return Err(StringErr::IndexOutOfBounds {
                index: pos,
                length: chars.len(),
            });
        }
        Ok(chars[(pos - 1) as usize])
    }

    // ========================================================================
    //                    ФУНКЦИИ МОДИФИКАЦИИ (мутирующие)
    // ========================================================================

    /// вставить(лит frag, аргрез лит s, арг цел pos) — вставляет frag в s на позицию pos.
    ///
    /// # Пример
    /// ```
    /// use shared::strings::StringOperations;
    /// let mut s = "Привет".to_string();
    /// StringOperations::insert("123", &mut s, 4).unwrap();
    /// assert_eq!(s, "При123вет");
    /// ```
    pub fn insert(frag: &str, s: &mut String, pos: i64) -> StringResult<()> {
        let chars: Vec<char> = s.chars().collect();
        let len = chars.len();

        if pos < 1 || pos > len as i64 + 1 {
            return Err(StringErr::IndexOutOfBounds {
                index: pos,
                length: len,
            });
        }

        let pos_idx = (pos - 1) as usize;
        let mut result = String::with_capacity(s.len() + frag.len());

        result.extend(chars[..pos_idx].iter());
        result.push_str(frag);
        result.extend(chars[pos_idx..].iter());

        *s = result;
        Ok(())
    }

    /// удалить(аргрез лит s, арг цел pos, арг цел count) — удаляет count символов начиная с pos.
    ///
    /// # Пример
    /// ```
    /// use shared::strings::StringOperations;
    /// let mut s = "Привет Мир".to_string();
    /// StringOperations::delete(&mut s, 7, 4).unwrap();
    /// assert_eq!(s, "Привет");
    /// ```
    pub fn delete(s: &mut String, pos: i64, count: i64) -> StringResult<()> {
        if count < 0 {
            return Err(StringErr::InvalidCount(count));
        }

        let chars: Vec<char> = s.chars().collect();
        let len = chars.len();

        if pos < 1 || pos > len as i64 {
            return Err(StringErr::IndexOutOfBounds {
                index: pos,
                length: len,
            });
        }

        let pos_idx = (pos - 1) as usize;
        let count = count as usize;
        let end_idx = (pos_idx + count).min(len);

        let mut result = String::with_capacity(s.len());
        result.extend(chars[..pos_idx].iter());
        result.extend(chars[end_idx..].iter());

        *s = result;
        Ok(())
    }

    /// заменить(аргрез лит s, арг лит old, арг лит new, арг лог каждый) — заменяет вхождения.
    ///
    /// Если `each` = true, заменяет все вхождения, иначе только первое.
    ///
    /// # Пример
    /// ```
    /// use shared::strings::StringOperations;
    /// let mut s = "abcabc".to_string();
    /// StringOperations::replace(&mut s, "bc", "XY", true).unwrap();
    /// assert_eq!(s, "aXYaXY");
    /// ```
    pub fn replace(s: &mut String, old: &str, new: &str, each: bool) -> StringResult<()> {
        if old.is_empty() {
            return Err(StringErr::EmptyDelimiter);
        }

        *s = if each {
            s.replace(old, new)
        } else {
            s.replacen(old, new, 1)
        };
        Ok(())
    }

    /// заменить_один_раз — алиас для replace с each=false.
    pub fn replace_first(s: &mut String, old: &str, new: &str) -> StringResult<()> {
        Self::replace(s, old, new, false)
    }

    /// заменить_все — алиас для replace с each=true.
    pub fn replace_all(s: &mut String, old: &str, new: &str) -> StringResult<()> {
        Self::replace(s, old, new, true)
    }

    // ========================================================================
    //                    ФУНКЦИИ ОБРЕЗКИ
    // ========================================================================

    /// обрезать(лит s) / trim — удаляет пробелы с обоих концов.
    ///
    /// # Пример
    /// ```
    /// use shared::strings::StringOperations;
    /// assert_eq!(StringOperations::trim("  Привет  "), "Привет");
    /// ```
    pub fn trim(s: &str) -> String {
        s.trim().to_string()
    }

    /// обрезать слева(лит s) — удаляет пробелы слева.
    pub fn trim_left(s: &str) -> String {
        s.trim_start().to_string()
    }

    /// обрезать справа(лит s) — удаляет пробелы справа.
    pub fn trim_right(s: &str) -> String {
        s.trim_end().to_string()
    }

    /// обрезать символы(лит s, лит chars) — удаляет указанные символы с концов.
    pub fn trim_chars(s: &str, chars: &str) -> String {
        let char_set: Vec<char> = chars.chars().collect();
        s.trim_matches(|c| char_set.contains(&c)).to_string()
    }

    // ========================================================================
    //                    ФУНКЦИИ РАЗБИЕНИЯ И ОБЪЕДИНЕНИЯ
    // ========================================================================

    /// разбить(лит s, лит delim) — разбивает строку по разделителю.
    ///
    /// # Пример
    /// ```
    /// use shared::strings::StringOperations;
    /// assert_eq!(StringOperations::split("a,b,c", ","), vec!["a", "b", "c"]);
    /// ```
    pub fn split(s: &str, delim: &str) -> Vec<String> {
        if delim.is_empty() {
            // Разбить на отдельные символы
            return s.chars().map(|c| c.to_string()).collect();
        }
        s.split(delim).map(|x| x.to_string()).collect()
    }

    /// разбить на строки(лит s) — разбивает строку по переводам строк.
    pub fn split_lines(s: &str) -> Vec<String> {
        s.lines().map(|x| x.to_string()).collect()
    }

    /// разбить на слова(лит s) — разбивает строку на слова (по пробелам).
    pub fn split_words(s: &str) -> Vec<String> {
        s.split_whitespace().map(|x| x.to_string()).collect()
    }

    /// соединить(таб arr, лит delim) — объединяет массив строк через разделитель.
    ///
    /// # Пример
    /// ```
    /// use shared::strings::StringOperations;
    /// assert_eq!(StringOperations::join(&["a", "b", "c"], ","), "a,b,c");
    /// ```
    pub fn join(parts: &[&str], delim: &str) -> String {
        parts.join(delim)
    }

    /// соединить значения(таб arr, лит delim) — объединяет массив Value через разделитель.
    pub fn join_values(parts: &[Value], delim: &str) -> String {
        parts
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(delim)
    }

    // ========================================================================
    //                    ФУНКЦИИ ПОВТОРЕНИЯ И ЗАПОЛНЕНИЯ
    // ========================================================================

    /// повторить(лит s, цел count) — повторяет строку count раз.
    ///
    /// # Пример
    /// ```
    /// use shared::strings::StringOperations;
    /// assert_eq!(StringOperations::repeat("ab", 3), Ok("ababab".to_string()));
    /// ```
    pub fn repeat(s: &str, count: i64) -> StringResult<String> {
        if count < 0 {
            return Err(StringErr::InvalidCount(count));
        }
        Ok(s.repeat(count as usize))
    }

    /// заполнить слева(лит s, цел width, сим fill) — дополняет строку слева до ширины.
    ///
    /// # Пример
    /// ```
    /// use shared::strings::StringOperations;
    /// assert_eq!(StringOperations::pad_left("42", 5, '0'), "00042");
    /// ```
    pub fn pad_left(s: &str, width: i64, fill: char) -> String {
        let len = Self::length(s);
        if width <= 0 || len >= width as usize {
            return s.to_string();
        }
        let padding = (width as usize) - len;
        format!("{}{}", fill.to_string().repeat(padding), s)
    }

    /// заполнить справа(лит s, цел width, сим fill) — дополняет строку справа до ширины.
    pub fn pad_right(s: &str, width: i64, fill: char) -> String {
        let len = Self::length(s);
        if width <= 0 || len >= width as usize {
            return s.to_string();
        }
        let padding = (width as usize) - len;
        format!("{}{}", s, fill.to_string().repeat(padding))
    }

    /// центрировать(лит s, цел width, сим fill) — центрирует строку.
    pub fn center(s: &str, width: i64, fill: char) -> String {
        let len = Self::length(s);
        if width <= 0 || len >= width as usize {
            return s.to_string();
        }
        let total_padding = (width as usize) - len;
        let left_padding = total_padding / 2;
        let right_padding = total_padding - left_padding;
        format!(
            "{}{}{}",
            fill.to_string().repeat(left_padding),
            s,
            fill.to_string().repeat(right_padding)
        )
    }

    // ========================================================================
    //                    ФУНКЦИИ РЕВЕРСА
    // ========================================================================

    /// обратить(лит s) — переворачивает строку.
    ///
    /// # Пример
    /// ```
    /// use shared::strings::StringOperations;
    /// assert_eq!(StringOperations::reverse("Привет"), "тевирП");
    /// ```
    pub fn reverse(s: &str) -> String {
        s.chars().rev().collect()
    }

    // ========================================================================
    //                    ФУНКЦИИ ПРОВЕРКИ
    // ========================================================================

    /// пусто(лит s) — проверяет, пуста ли строка.
    pub fn is_empty(s: &str) -> bool {
        s.is_empty()
    }

    /// пробелы(лит s) — проверяет, состоит ли строка только из пробелов.
    pub fn is_whitespace(s: &str) -> bool {
        !s.is_empty() && s.chars().all(|c| c.is_whitespace())
    }

    /// цифры(лит s) — проверяет, состоит ли строка только из цифр.
    pub fn is_digits(s: &str) -> bool {
        !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
    }

    /// буквы(лит s) — проверяет, состоит ли строка только из букв.
    pub fn is_alpha(s: &str) -> bool {
        !s.is_empty() && s.chars().all(|c| c.is_alphabetic())
    }

    /// буквы и цифры(лит s) — проверяет, состоит ли строка из букв и цифр.
    pub fn is_alphanumeric(s: &str) -> bool {
        !s.is_empty() && s.chars().all(|c| c.is_alphanumeric())
    }

    /// верхний регистр?(лит s) — проверяет, вся ли строка в верхнем регистре.
    pub fn is_uppercase(s: &str) -> bool {
        !s.is_empty() && s.chars().all(|c| !c.is_alphabetic() || c.is_uppercase())
    }

    /// нижний регистр?(лит s) — проверяет, вся ли строка в нижнем регистре.
    pub fn is_lowercase(s: &str) -> bool {
        !s.is_empty() && s.chars().all(|c| !c.is_alphabetic() || c.is_lowercase())
    }

    /// число?(лит s) — проверяет, является ли строка числом.
    pub fn is_numeric(s: &str) -> bool {
        if s.is_empty() {
            return false;
        }
        s.parse::<f64>().is_ok()
    }

    // ========================================================================
    //                    ФУНКЦИИ ПРЕОБРАЗОВАНИЯ
    // ========================================================================

    /// в число(лит s) — преобразует строку в число.
    pub fn to_number(s: &str) -> StringResult<f64> {
        s.trim()
            .parse::<f64>()
            .map_err(|_| StringErr::TypeMismatch("строка не является числом"))
    }

    /// в целое(лит s) — преобразует строку в целое число.
    pub fn to_integer(s: &str) -> StringResult<i64> {
        s.trim()
            .parse::<i64>()
            .map_err(|_| StringErr::TypeMismatch("строка не является целым числом"))
    }

    /// строка(знач x) — преобразует значение в строку.
    pub fn to_string(v: &Value) -> String {
        v.to_string()
    }

    // ========================================================================
    //                    ФУНКЦИИ ФОРМАТИРОВАНИЯ
    // ========================================================================

    /// формат(лит шаблон, ...) — форматирует строку по шаблону.
    ///
    /// Поддерживает {} как placeholder для подстановки значений.
    pub fn format(template: &str, args: &[Value]) -> String {
        let mut result = template.to_string();
        for arg in args {
            if let Some(pos) = result.find("{}") {
                result = format!("{}{}{}", &result[..pos], arg, &result[pos + 2..]);
            } else {
                break;
            }
        }
        result
    }

    // ========================================================================
    //                    УТИЛИТЫ
    // ========================================================================

    /// сравнить(лит a, лит b) — лексикографическое сравнение строк.
    ///
    /// Возвращает: -1 если a < b, 0 если a == b, 1 если a > b.
    pub fn compare(a: &str, b: &str) -> i64 {
        match a.cmp(b) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        }
    }

    /// сравнить без регистра(лит a, лит b) — сравнение без учёта регистра.
    pub fn compare_ignore_case(a: &str, b: &str) -> i64 {
        Self::compare(&a.to_lowercase(), &b.to_lowercase())
    }

    /// равны без регистра(лит a, лит b) — проверка равенства без учёта регистра.
    pub fn equals_ignore_case(a: &str, b: &str) -> bool {
        a.to_lowercase() == b.to_lowercase()
    }
}

#[cfg(test)]
mod tests;
