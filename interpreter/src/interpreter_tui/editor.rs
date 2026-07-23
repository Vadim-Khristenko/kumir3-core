//! Input line editor with character-based cursor tracking.
//!
//! Separated out for one critical reason: cursor position is counted **in characters**,
//! not bytes. The previous byte-index version crashed on the second Cyrillic letter
//! (two UTF-8 bytes), inserting into the middle of a character and making the
//! interpreter panic. For a language used with Russian text, this meant you could
//! not even type `цел` in the REPL.

/// Editable input line with cursor.
#[derive(Default)]
pub(crate) struct InputLine {
    chars: Vec<char>,
    cursor: usize,
}

impl InputLine {
    /// Returns the line text.
    pub(crate) fn text(&self) -> String {
        self.chars.iter().collect()
    }

    /// Returns cursor position in characters (affects indentation when drawing).
    pub(crate) fn cursor(&self) -> usize {
        self.cursor
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    /// Extracts the text, leaving the line empty.
    pub(crate) fn take(&mut self) -> String {
        self.cursor = 0;
        std::mem::take(&mut self.chars).into_iter().collect()
    }

    /// Replaces content and moves cursor to end (for history, autocomplete).
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

    /// Deletes the character to the left of the cursor.
    pub(crate) fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.chars.remove(self.cursor);
        }
    }

    /// Deletes the character under the cursor.
    pub(crate) fn delete(&mut self) {
        if self.cursor < self.chars.len() {
            self.chars.remove(self.cursor);
        }
    }

    /// Deletes the word to the left of the cursor (Ctrl+W).
    pub(crate) fn delete_word_left(&mut self) {
        let target = self.word_start();
        self.chars.drain(target..self.cursor);
        self.cursor = target;
    }

    /// Deletes everything to the left of the cursor (Ctrl+U).
    pub(crate) fn delete_to_start(&mut self) {
        self.chars.drain(..self.cursor);
        self.cursor = 0;
    }

    /// Deletes everything to the right of the cursor (Ctrl+K).
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

    /// Returns the word under the cursor (basis for autocomplete).
    pub(crate) fn word_at_cursor(&self) -> String {
        self.chars[self.word_start()..self.cursor].iter().collect()
    }

    /// Replaces the word under the cursor entirely.
    pub(crate) fn replace_word_at_cursor(&mut self, replacement: &str) {
        let start = self.word_start();
        self.chars.drain(start..self.cursor);
        self.cursor = start;
        self.insert_str(replacement);
    }

    /// Finds the start of the word to the left of cursor: skip non-word chars, then word chars.
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

    /// This module exists for this test: before, on the second Cyrillic letter,
    /// the interpreter crashed because cursor position was counted in bytes.
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
        assert_eq!(line.cursor(), 12, "cursor at start of word 'строка'");
        line.word_left();
        assert_eq!(line.cursor(), 6, "and then at start of word 'длина'");
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
