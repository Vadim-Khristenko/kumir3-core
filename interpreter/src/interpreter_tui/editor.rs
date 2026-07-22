//! Редактируемая строка ввода.
//!
//! Вынесена отдельно ради одной вещи: позиция курсора считается **в символах**,
//! а не в байтах. Прежняя реализация хранила её как байтовый индекс, но двигала
//! на единицу за символ — на русской букве (два байта в UTF-8) вставка попадала
//! в середину символа, и интерпретатор падал. Для языка, на котором пишут
//! по-русски, это означало, что набрать `цел` в интерактивном режиме нельзя.

/// Строка ввода с курсором.
#[derive(Default)]
pub(crate) struct InputLine {
    chars: Vec<char>,
    cursor: usize,
}

impl InputLine {
    /// Текст строки.
    pub(crate) fn text(&self) -> String {
        self.chars.iter().collect()
    }

    /// Положение курсора в символах — столько ячеек отступа при отрисовке.
    pub(crate) fn cursor(&self) -> usize {
        self.cursor
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    /// Забирает текст, оставляя строку пустой.
    pub(crate) fn take(&mut self) -> String {
        self.cursor = 0;
        std::mem::take(&mut self.chars).into_iter().collect()
    }

    /// Заменяет содержимое, ставя курсор в конец (история, автодополнение).
    pub(crate) fn set(&mut self, text: &str) {
        self.chars = text.chars().collect();
        self.cursor = self.chars.len();
    }

    pub(crate) fn clear(&mut self) {
        self.chars.clear();
        self.cursor = 0;
    }

    pub(crate) fn insert(&mut self, ch: char) {
        self.chars.insert(self.cursor, ch);
        self.cursor += 1;
    }

    pub(crate) fn insert_str(&mut self, text: &str) {
        for ch in text.chars() {
            self.insert(ch);
        }
    }

    /// Удаляет символ слева от курсора.
    pub(crate) fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.chars.remove(self.cursor);
        }
    }

    /// Удаляет символ под курсором.
    pub(crate) fn delete(&mut self) {
        if self.cursor < self.chars.len() {
            self.chars.remove(self.cursor);
        }
    }

    /// Удаляет слово слева от курсора (Ctrl+W).
    pub(crate) fn delete_word_left(&mut self) {
        let target = self.word_start();
        self.chars.drain(target..self.cursor);
        self.cursor = target;
    }

    /// Удаляет всё слева от курсора (Ctrl+U).
    pub(crate) fn delete_to_start(&mut self) {
        self.chars.drain(..self.cursor);
        self.cursor = 0;
    }

    /// Удаляет всё справа от курсора (Ctrl+K).
    pub(crate) fn delete_to_end(&mut self) {
        self.chars.truncate(self.cursor);
    }

    pub(crate) fn left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub(crate) fn right(&mut self) {
        if self.cursor < self.chars.len() {
            self.cursor += 1;
        }
    }

    pub(crate) fn word_left(&mut self) {
        self.cursor = self.word_start();
    }

    pub(crate) fn word_right(&mut self) {
        let mut i = self.cursor;
        while i < self.chars.len() && !is_word(self.chars[i]) {
            i += 1;
        }
        while i < self.chars.len() && is_word(self.chars[i]) {
            i += 1;
        }
        self.cursor = i;
    }

    pub(crate) fn home(&mut self) {
        self.cursor = 0;
    }

    pub(crate) fn end(&mut self) {
        self.cursor = self.chars.len();
    }

    /// Слово, в котором стоит курсор, — основа для автодополнения.
    pub(crate) fn word_at_cursor(&self) -> String {
        self.chars[self.word_start()..self.cursor].iter().collect()
    }

    /// Заменяет слово под курсором целиком.
    pub(crate) fn replace_word_at_cursor(&mut self, replacement: &str) {
        let start = self.word_start();
        self.chars.drain(start..self.cursor);
        self.cursor = start;
        self.insert_str(replacement);
    }

    /// Начало слова слева от курсора: сначала пропускаются разделители,
    /// затем — сами буквы.
    fn word_start(&self) -> usize {
        let mut i = self.cursor;
        while i > 0 && !is_word(self.chars[i - 1]) {
            i -= 1;
        }
        while i > 0 && is_word(self.chars[i - 1]) {
            i -= 1;
        }
        i
    }
}

fn is_word(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with(text: &str) -> InputLine {
        let mut line = InputLine::default();
        line.set(text);
        line
    }

    /// Ради этого модуль и появился: раньше на второй русской букве
    /// интерпретатор падал, потому что позиция курсора считалась в байтах.
    #[test]
    fn nabor_kirillicy_ne_lomaetsya() {
        let mut line = InputLine::default();
        for ch in "цел счётчик := 5".chars() {
            line.insert(ch);
        }
        assert_eq!(line.text(), "цел счётчик := 5");
        assert_eq!(line.cursor(), 16);
    }

    #[test]
    fn vstavka_v_seredinu_kirillicheskogo_slova() {
        let mut line = with("цел");
        line.home();
        line.right();
        line.insert('X');
        assert_eq!(line.text(), "цXел");
    }

    #[test]
    fn udalenie_schitaet_simvoly_a_ne_bajty() {
        let mut line = with("абв");
        line.backspace();
        assert_eq!(line.text(), "аб");
        line.home();
        line.delete();
        assert_eq!(line.text(), "б");
    }

    #[test]
    fn dvizhenie_po_slovam() {
        let mut line = with("вывод длина(строка)");
        line.word_left();
        assert_eq!(line.cursor(), 12, "курсор встал в начало слова «строка»");
        line.word_left();
        assert_eq!(line.cursor(), 6, "и дальше — в начало слова «длина»");
        line.word_right();
        assert_eq!(line.cursor(), 11);
    }

    #[test]
    fn udalenie_slova_i_chastej_stroki() {
        let mut line = with("вывод длина");
        line.delete_word_left();
        assert_eq!(line.text(), "вывод ");

        let mut line = with("вывод длина");
        line.home();
        line.word_right();
        line.delete_to_start();
        assert_eq!(line.text(), " длина");

        let mut line = with("вывод длина");
        line.home();
        line.word_right();
        line.delete_to_end();
        assert_eq!(line.text(), "вывод");
    }

    #[test]
    fn slovo_pod_kursorom_dlya_dopolneniya() {
        let mut line = with("вывод дли");
        assert_eq!(line.word_at_cursor(), "дли");
        line.replace_word_at_cursor("длина");
        assert_eq!(line.text(), "вывод длина");
        assert_eq!(line.cursor(), 11);
    }

    #[test]
    fn kursor_ne_vyhodit_za_granicy() {
        let mut line = with("аб");
        line.home();
        line.left();
        assert_eq!(line.cursor(), 0);
        line.end();
        line.right();
        assert_eq!(line.cursor(), 2);
    }
}
