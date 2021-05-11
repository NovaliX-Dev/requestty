use std::{
    fmt,
    io::{self, Write},
    ops::Range,
};

use unicode_segmentation::UnicodeSegmentation;

use crate::{
    backend::Backend,
    error,
    events::{KeyCode, KeyEvent, KeyModifiers, Movement},
};

/// A widget that inputs a line of text
pub struct StringInput<F = super::widgets::FilterMapChar> {
    value: String,
    mask: Option<char>,
    hide_output: bool,
    /// The character length of the string
    value_len: usize,
    /// The position of the 'cursor' in characters
    at: usize,
    filter_map_char: F,
}

impl<F> StringInput<F> {
    /// Creates a new [`StringInput`]. The filter_map_char is used in [`StringInput::handle_key`] to
    /// avoid some characters to limit and filter characters.
    pub fn new(filter_map_char: F) -> Self {
        Self {
            value: String::new(),
            value_len: 0,
            at: 0,
            filter_map_char,
            mask: None,
            hide_output: false,
        }
    }

    /// A mask to print in render instead of the actual characters
    pub fn mask(mut self, mask: char) -> Self {
        self.mask = Some(mask);
        self
    }

    /// Whether to render nothing, but still keep track of all the characters
    pub fn hide_output(mut self) -> Self {
        self.hide_output = true;
        self
    }

    /// A helper that sets mask if mask is some, otherwise hides the output
    pub fn password(self, mask: Option<char>) -> Self {
        match mask {
            Some(mask) => self.mask(mask),
            None => self.hide_output(),
        }
    }

    /// The currently inputted value
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Sets the value
    pub fn set_value(&mut self, value: String) {
        self.value_len = value.chars().count();
        self.at = self.value_len;
        self.value = value;
    }

    /// Check whether any character has come to the input
    pub fn has_value(&self) -> bool {
        self.value.capacity() > 0
    }

    /// Returns None if no characters have been inputted, otherwise returns Some
    ///
    /// note: it can return Some(""), if a character was added and then deleted. It will only return
    /// None when no character was ever received
    pub fn finish(self) -> Option<String> {
        self.has_value().then(|| self.value)
    }

    /// Gets the byte index of a given char index
    fn get_byte_i(&self, index: usize) -> usize {
        self.value
            .char_indices()
            .nth(index)
            .map(|(i, _)| i)
            .unwrap_or_else(|| self.value.len())
    }

    /// Gets the char index of a given byte index
    fn get_char_i(&self, byte_i: usize) -> usize {
        self.value
            .char_indices()
            .position(|(i, _)| i == byte_i)
            .unwrap_or_else(|| self.value.char_indices().count())
    }

    /// Get the word bound iterator for a given range
    fn word_iter(
        &self,
        r: Range<usize>,
    ) -> impl DoubleEndedIterator<Item = (usize, &str)> {
        self.value[r].split_word_bound_indices().filter(|(_, s)| {
            !s.chars().next().map(char::is_whitespace).unwrap_or(true)
        })
    }

    /// Returns the byte index of the start of the first word to the left (< byte_i)
    fn find_word_left(&self, byte_i: usize) -> usize {
        self.word_iter(0..byte_i)
            .next_back()
            .map(|(new_byte_i, _)| new_byte_i)
            .unwrap_or(0)
    }

    /// Returns the byte index of the start of the first word to the right (> byte_i)
    fn find_word_right(&self, byte_i: usize) -> usize {
        self.word_iter(byte_i..self.value.len())
            .nth(1)
            .map(|(new_byte_i, _)| new_byte_i + byte_i)
            .unwrap_or_else(|| self.value.len())
    }

    fn is_delete_movement(&self, key: KeyEvent) -> Option<Movement> {
        let mov = match key.code {
            KeyCode::Backspace if self.at == 0 => return None,
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Movement::Home
            }
            KeyCode::Backspace if key.modifiers.contains(KeyModifiers::ALT) => {
                Movement::PrevWord
            }
            KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::ALT) => {
                Movement::PrevWord
            }
            KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Movement::Left
            }
            KeyCode::Backspace => Movement::Left,

            KeyCode::Delete if self.at == self.value_len => return None,
            KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Movement::End
            }

            KeyCode::Delete if key.modifiers.contains(KeyModifiers::ALT) => {
                Movement::NextWord
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::ALT) => {
                Movement::NextWord
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Movement::Right
            }
            KeyCode::Delete => Movement::Right,

            _ => return None,
        };

        Some(mov)
    }
}

impl<F> super::Widget for StringInput<F>
where
    F: Fn(char) -> Option<char>,
{
    /// Handles characters, backspace, delete, left arrow, right arrow, home and end.
    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if let Some(movement) = self.is_delete_movement(key) {
            match movement {
                Movement::Home => {
                    let byte_i = self.get_byte_i(self.at);
                    self.value_len -= self.at;
                    self.at = 0;
                    self.value.replace_range(..byte_i, "");
                    return true;
                }
                Movement::PrevWord => {
                    let was_at = self.at;
                    let byte_i = self.get_byte_i(self.at);
                    let prev_word = self.find_word_left(byte_i);
                    self.at = self.get_char_i(prev_word);
                    self.value_len -= was_at - self.at;
                    self.value.replace_range(prev_word..byte_i, "");
                    return true;
                }
                Movement::Left if self.at == self.value_len => {
                    self.at -= 1;
                    self.value_len -= 1;
                    self.value.pop();
                    return true;
                }
                Movement::Left => {
                    self.at -= 1;
                    let byte_i = self.get_byte_i(self.at);
                    self.value_len -= 1;
                    self.value.remove(byte_i);
                    return true;
                }

                Movement::End => {
                    let byte_i = self.get_byte_i(self.at);
                    self.value_len = self.at;
                    self.value.truncate(byte_i);
                    return true;
                }
                Movement::NextWord => {
                    let byte_i = self.get_byte_i(self.at);
                    let next_word = self.find_word_right(byte_i);
                    self.value_len -= self.get_char_i(next_word) - self.at;
                    self.value.replace_range(byte_i..next_word, "");
                    return true;
                }
                Movement::Right if self.at == self.value_len - 1 => {
                    self.value_len -= 1;
                    self.value.pop();
                    return true;
                }
                Movement::Right => {
                    let byte_i = self.get_byte_i(self.at);
                    self.value_len -= 1;
                    self.value.remove(byte_i);
                    return true;
                }

                _ => {}
            }
        }

        match key.code {
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                if let Some(c) = (self.filter_map_char)(c) {
                    if self.at == self.value_len {
                        self.value.push(c);
                    } else {
                        let byte_i = self.get_byte_i(self.at);
                        self.value.insert(byte_i, c);
                    };

                    self.at += 1;
                    self.value_len += 1;
                    return true;
                }
            }

            _ => {}
        }

        match Movement::try_from_key(key) {
            Some(Movement::PrevWord) if self.at != 0 => {
                self.at =
                    self.get_char_i(self.find_word_left(self.get_byte_i(self.at)));
            }
            Some(Movement::Left) if self.at != 0 => {
                self.at -= 1;
            }

            Some(Movement::NextWord) if self.at != self.value_len => {
                self.at =
                    self.get_char_i(self.find_word_right(self.get_byte_i(self.at)));
            }
            Some(Movement::Right) if self.at != self.value_len => {
                self.at += 1;
            }

            Some(Movement::Home) if self.at != 0 => {
                self.at = 0;
            }
            Some(Movement::End) if self.at != self.value_len => {
                self.at = self.value_len;
            }
            _ => return false,
        }

        true
    }

    fn render<B: Backend>(
        &mut self,
        max_width: usize,
        backend: &mut B,
    ) -> error::Result<()> {
        if self.hide_output {
            return Ok(());
        }

        if max_width <= 3 {
            return Err(fmt::Error.into());
        }

        if self.value_len > max_width {
            unimplemented!(
                "Big strings {} {} {}",
                self.value_len,
                self.value().chars().count(),
                max_width
            );
        } else if let Some(mask) = self.mask {
            print_mask(self.value_len, mask, backend)?;
        } else {
            backend.write_all(self.value.as_bytes())?;
        }

        Ok(())
    }

    fn cursor_pos(&self, prompt_len: u16) -> (u16, u16) {
        if self.hide_output {
            (prompt_len, 0)
        } else {
            (prompt_len + self.at as u16, 0)
        }
    }

    fn height(&self) -> usize {
        0
    }
}

impl Default for StringInput {
    fn default() -> Self {
        Self::new(super::widgets::no_filter)
    }
}

fn print_mask<W: Write>(len: usize, mask: char, w: &mut W) -> io::Result<()> {
    let mut buf = [0; 4];
    let mask = mask.encode_utf8(&mut buf[..]);

    for _ in 0..len {
        w.write_all(mask.as_bytes())?;
    }

    Ok(())
}