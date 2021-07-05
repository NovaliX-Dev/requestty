use crate::{
    backend::Backend,
    error,
    events::{KeyCode, KeyEvent},
    layout::Layout,
};

/// A widget that inputs a single character.
///
/// A `filter_map` function can optionally be provided to limit and change the characters allowed,
/// similar to [`Iterator::filter_map`].
///
/// If multiple characters are received, they will overwrite the previous character. If a
/// multi-character string is required, use [`StringInput`].
///
/// [`StringInput`]: crate::widgets::StringInput
#[derive(Debug, Clone)]
pub struct CharInput<F = super::widgets::FilterMapChar> {
    value: Option<char>,
    filter_map: F,
}

impl CharInput {
    /// Creates a new [`CharInput`] which accepts all characters.
    pub fn new() -> Self {
        Self::with_filter_map(super::widgets::no_filter)
    }
}

impl<F> CharInput<F>
where
    F: Fn(char) -> Option<char>,
{
    /// Creates a new [`CharInput`] which only accepts characters as per the `filter_map` function.
    pub fn with_filter_map(filter_map: F) -> Self {
        Self {
            value: None,
            filter_map,
        }
    }

    /// The last inputted char (if any).
    pub fn value(&self) -> Option<char> {
        self.value
    }

    /// Sets the value to the given character.
    pub fn set_value(&mut self, value: char) {
        self.value = Some(value);
    }

    /// Clears the value.
    pub fn clear_value(&mut self) {
        self.value = None;
    }
}

impl<F> super::Widget for CharInput<F>
where
    F: Fn(char) -> Option<char>,
{
    fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char(c) => {
                if let Some(c) = (self.filter_map)(c) {
                    self.value = Some(c);

                    return true;
                }

                false
            }

            KeyCode::Backspace | KeyCode::Delete if self.value.is_some() => {
                self.value = None;
                true
            }

            _ => false,
        }
    }

    fn render<B: Backend>(&mut self, layout: &mut Layout, backend: &mut B) -> error::Result<()> {
        if let Some(value) = self.value {
            layout.line_offset += 1;

            write!(backend, "{}", value)?;
        }
        Ok(())
    }

    fn height(&mut self, layout: &mut Layout) -> u16 {
        layout.line_offset += self.value.is_some() as u16;
        1
    }

    /// Returns the position right after the character if any.
    fn cursor_pos(&mut self, layout: Layout) -> (u16, u16) {
        (layout.line_offset + self.value.is_some() as u16, 0)
    }
}

impl Default for CharInput {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{backend::TestBackend, events::KeyModifiers, Widget};

    #[test]
    fn test_cursor_pos() {
        let layout = Layout::new(0, (100, 20).into());
        let mut input = CharInput::default();

        assert_eq!(input.cursor_pos(layout), (0, 0));
        assert_eq!(input.cursor_pos(layout.with_line_offset(5)), (5, 0));

        input.set_value('c');

        assert_eq!(input.cursor_pos(layout), (1, 0));
        assert_eq!(input.cursor_pos(layout.with_line_offset(5)), (6, 0));
    }

    #[test]
    fn test_handle_key() {
        let modifiers = KeyModifiers::empty();

        let mut input = CharInput::default();
        assert!(input.handle_key(KeyEvent::new(KeyCode::Char('c'), modifiers)));
        assert_eq!(input.value(), Some('c'));
        assert!(!input.handle_key(KeyEvent::new(KeyCode::Tab, modifiers)));
        assert!(input.handle_key(KeyEvent::new(KeyCode::Char('d'), modifiers)));
        assert_eq!(input.value(), Some('d'));
        assert!(input.handle_key(KeyEvent::new(KeyCode::Backspace, modifiers)));
        assert_eq!(input.value(), None);
        assert!(input.handle_key(KeyEvent::new(KeyCode::Char('c'), modifiers)));
        assert_eq!(input.value(), Some('c'));
        assert!(input.handle_key(KeyEvent::new(KeyCode::Delete, modifiers)));
        assert_eq!(input.value(), None);
        assert!(!input.handle_key(KeyEvent::new(KeyCode::Delete, modifiers)));
        assert!(!input.handle_key(KeyEvent::new(KeyCode::Backspace, modifiers)));

        let mut input =
            CharInput::with_filter_map(|c| if c.is_uppercase() { None } else { Some(c) });
        assert!(!input.handle_key(KeyEvent::new(KeyCode::Char('C'), modifiers)));
        assert_eq!(input.value(), None);
        assert!(input.handle_key(KeyEvent::new(KeyCode::Char('c'), modifiers)));
        assert_eq!(input.value(), Some('c'));
        assert!(!input.handle_key(KeyEvent::new(KeyCode::Char('C'), modifiers)));
        assert_eq!(input.value(), Some('c'));
    }

    #[test]
    fn test_render() {
        let size = (30, 10).into();
        let mut layout = Layout::new(0, size);
        let mut input = CharInput::default();

        let mut backend = TestBackend::new(size);
        input.render(&mut layout, &mut backend).unwrap();
        assert_eq!(backend, TestBackend::new(size));

        assert_eq!(layout, Layout::new(0, size));

        input.set_value('c');

        let mut backend = TestBackend::new(size);
        input.render(&mut layout, &mut backend).unwrap();

        crate::assert_backend_snapshot!(backend);

        assert_eq!(layout, Layout::new(0, size).with_line_offset(1));
    }
}
