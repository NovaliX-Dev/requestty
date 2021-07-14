use std::io;

use ui::{
    backend::Backend,
    events::{EventIterator, KeyCode, KeyEvent},
    style::Color,
    widgets::{self, Text},
    Prompt, Validation, Widget,
};

use super::{Choice, Filter, Options, Transform, Validate};
use crate::{Answer, Answers, ListItem};

#[cfg(test)]
mod tests;

#[derive(Debug, Default)]
pub(super) struct MultiSelect<'a> {
    choices: super::ChoiceList<Text<String>>,
    selected: Vec<bool>,
    filter: Filter<'a, Vec<bool>>,
    validate: Validate<'a, [bool]>,
    transform: Transform<'a, [ListItem]>,
}

fn set_seperators_false(selected: &mut [bool], choices: &[Choice<Text<String>>]) {
    for (i, choice) in choices.iter().enumerate() {
        selected[i] &= !choice.is_separator();
    }
}

struct MultiSelectPrompt<'a, 'c> {
    prompt: widgets::Prompt<&'a str>,
    select: widgets::Select<MultiSelect<'c>>,
    answers: &'a Answers,
}

fn create_list_items(
    selected: Vec<bool>,
    choices: super::ChoiceList<Text<String>>,
) -> Vec<ListItem> {
    selected
        .into_iter()
        .enumerate()
        .zip(choices.choices.into_iter())
        .filter_map(|((index, is_selected), name)| match (is_selected, name) {
            (true, Choice::Choice(name)) => Some(ListItem {
                index,
                name: name.text,
            }),
            _ => None,
        })
        .collect()
}

impl Prompt for MultiSelectPrompt<'_, '_> {
    type ValidateErr = widgets::Text<String>;
    type Output = Vec<ListItem>;

    fn validate(&mut self) -> Result<Validation, Self::ValidateErr> {
        if let Validate::Sync(ref mut validate) = self.select.list.validate {
            set_seperators_false(
                &mut self.select.list.selected,
                &self.select.list.choices.choices,
            );
            validate(&self.select.list.selected, self.answers)?;
        }
        Ok(Validation::Finish)
    }

    fn finish(self) -> Self::Output {
        let MultiSelect {
            mut selected,
            choices,
            filter,
            ..
        } = self.select.into_inner();

        if let Filter::Sync(filter) = filter {
            set_seperators_false(&mut selected, &choices.choices);

            selected = filter(selected, self.answers);
        }

        create_list_items(selected, choices)
    }
}

impl Widget for MultiSelectPrompt<'_, '_> {
    fn render<B: Backend>(&mut self, layout: &mut ui::layout::Layout, b: &mut B) -> io::Result<()> {
        self.prompt.render(layout, b)?;
        self.select.render(layout, b)
    }

    fn height(&mut self, layout: &mut ui::layout::Layout) -> u16 {
        self.prompt.height(layout) + self.select.height(layout) - 1
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char(' ') => {
                let index = self.select.get_at();
                self.select.list.selected[index] = !self.select.list.selected[index];
            }
            KeyCode::Char('i') => {
                self.select.list.selected.iter_mut().for_each(|s| *s = !*s);
            }
            KeyCode::Char('a') => {
                let select_state = self.select.list.selected.iter().any(|s| !s);
                self.select
                    .list
                    .selected
                    .iter_mut()
                    .for_each(|s| *s = select_state);
            }
            _ => return self.select.handle_key(key),
        }

        true
    }

    fn cursor_pos(&mut self, layout: ui::layout::Layout) -> (u16, u16) {
        self.select.cursor_pos(layout)
    }
}

impl widgets::List for MultiSelect<'_> {
    fn render_item<B: Backend>(
        &mut self,
        index: usize,
        hovered: bool,
        mut layout: ui::layout::Layout,
        b: &mut B,
    ) -> io::Result<()> {
        if hovered {
            b.set_fg(Color::Cyan)?;
            write!(b, "{} ", ui::symbols::ARROW)?;
        } else {
            b.write_all(b"  ")?;
        }

        if self.is_selectable(index) {
            if self.selected[index] {
                b.set_fg(Color::LightGreen)?;
            } else {
                b.set_fg(Color::DarkGrey)?;
            }

            write!(b, "{} ", ui::symbols::TICK)?;

            if hovered {
                b.set_fg(Color::Cyan)?;
            } else {
                b.set_fg(Color::Reset)?;
            }
        } else {
            b.set_fg(Color::DarkGrey)?;
        }

        layout.offset_x += 4;

        self.choices[index].render(&mut layout, b)?;

        b.set_fg(Color::Reset)
    }

    fn is_selectable(&self, index: usize) -> bool {
        !self.choices[index].is_separator()
    }

    fn height_at(&mut self, index: usize, mut layout: ui::layout::Layout) -> u16 {
        layout.offset_x += 4;
        self.choices[index].height(&mut layout)
    }

    fn len(&self) -> usize {
        self.choices.len()
    }

    fn page_size(&self) -> usize {
        self.choices.page_size()
    }

    fn should_loop(&self) -> bool {
        self.choices.should_loop()
    }
}

impl<'c> MultiSelect<'c> {
    fn into_multi_select_prompt<'a>(
        self,
        message: &'a str,
        answers: &'a Answers,
    ) -> MultiSelectPrompt<'a, 'c> {
        MultiSelectPrompt {
            prompt: widgets::Prompt::new(message)
                .with_hint("Press <space> to select, <a> to toggle all, <i> to invert selection"),
            select: widgets::Select::new(self),
            answers,
        }
    }

    pub(crate) fn ask<B: Backend, E: EventIterator>(
        mut self,
        message: String,
        answers: &Answers,
        b: &mut B,
        events: &mut E,
    ) -> ui::Result<Answer> {
        let transform = self.transform.take();

        let ans = ui::Input::new(self.into_multi_select_prompt(&message, answers), b)
            .hide_cursor()
            .run(events)?;

        crate::write_final!(transform, message, &ans, answers, b, {
            b.set_fg(Color::Cyan)?;
            print_comma_separated(
                ans.iter().map(|item| {
                    item.name
                        .lines()
                        .next()
                        .expect("There must be at least one line in a `str`")
                }),
                b,
            )?;
            b.set_fg(Color::Reset)?;
        });

        Ok(Answer::ListItems(ans))
    }
}

/// The builder for a [`multi_select`] prompt.
///
/// Unlike the other list based prompts, this has a per choice boolean default.
///
/// See the various methods for more details on each available option.
///
/// # Examples
///
/// ```
/// use discourse::{Question, DefaultSeparator};
///
/// let multi_select = Question::multi_select("cheese")
///     .message("What cheese do you want?")
///     .choice_with_default("Mozzarella", true)
///     .choices(vec![
///         "Cheddar",
///         "Parmesan",
///     ])
///     .build();
/// ```
///
/// [`multi_select`]: crate::question::Question::multi_select
#[derive(Debug)]
pub struct MultiSelectBuilder<'a> {
    opts: Options<'a>,
    multi_select: MultiSelect<'a>,
}

impl<'a> MultiSelectBuilder<'a> {
    pub(crate) fn new(name: String) -> Self {
        MultiSelectBuilder {
            opts: Options::new(name),
            multi_select: Default::default(),
        }
    }

    crate::impl_options_builder! {
    message
    /// # Examples
    ///
    /// ```
    /// use discourse::Question;
    ///
    /// let multi_select = Question::multi_select("cheese")
    ///     .message("What cheese do you want?")
    ///     .build();
    /// ```

    when
    /// # Examples
    ///
    /// ```
    /// use discourse::{Answers, Question};
    ///
    /// let multi_select = Question::multi_select("cheese")
    ///     .when(|previous_answers: &Answers| match previous_answers.get("vegan") {
    ///         Some(ans) => ans.as_bool().unwrap(),
    ///         None => true,
    ///     })
    ///     .build();
    /// ```

    ask_if_answered
    /// # Examples
    ///
    /// ```
    /// use discourse::{Answers, Question};
    ///
    /// let multi_select = Question::multi_select("cheese")
    ///     .ask_if_answered(true)
    ///     .build();
    /// ```
    }

    /// The maximum height that can be taken by the list
    ///
    /// If the total height exceeds the page size, the list will be scrollable.
    ///
    /// The `page_size` must be a minimum of 5. If `page_size` is not set, it will default to 15.
    ///
    /// # Panics
    ///
    /// It will panic if the `page_size` is less than 5.
    ///
    /// # Examples
    ///
    /// ```
    /// use discourse::Question;
    ///
    /// let multi_select = Question::multi_select("cheese")
    ///     .page_size(10)
    ///     .build();
    /// ```
    pub fn page_size(mut self, page_size: usize) -> Self {
        assert!(page_size >= 5, "page size can be a minimum of 5");

        self.multi_select.choices.set_page_size(page_size);
        self
    }

    /// Whether to wrap around when user gets to the last element.
    ///
    /// This only applies when the list is scrollable, i.e. page size > total height.
    ///
    /// If `should_loop` is not set, it will default to `true`.
    ///
    /// # Examples
    ///
    /// ```
    /// use discourse::Question;
    ///
    /// let multi_select = Question::multi_select("cheese")
    ///     .should_loop(false)
    ///     .build();
    /// ```
    pub fn should_loop(mut self, should_loop: bool) -> Self {
        self.multi_select.choices.set_should_loop(should_loop);
        self
    }

    /// Inserts a [`Choice`] with its default checked state as `false`.
    ///
    /// If you want to set the default checked state, use [`choice_with_default`].
    ///
    /// See [`multi_select`] for more information.
    ///
    /// [`Choice`]: super::Choice::Choice
    /// [`choice_with_default`]: Self::choice_with_default
    /// [`multi_select`]: super::Question::multi_select
    ///
    /// # Examples
    ///
    /// ```
    /// use discourse::Question;
    ///
    /// let multi_select = Question::multi_select("cheese")
    ///     .choice("Cheddar")
    ///     .build();
    /// ```
    pub fn choice<I: Into<String>>(self, choice: I) -> Self {
        self.choice_with_default(choice.into(), false)
    }

    /// Inserts a [`Choice`] with a given default checked state.
    ///
    /// See [`multi_select`] for more information.
    ///
    /// [`Choice`]: super::Choice::Choice
    /// [`multi_select`]: super::Question::multi_select
    ///
    /// # Examples
    ///
    /// ```
    /// use discourse::Question;
    ///
    /// let multi_select = Question::multi_select("cheese")
    ///     .choice_with_default("Mozzarella", true)
    ///     .build();
    /// ```
    pub fn choice_with_default<I: Into<String>>(mut self, choice: I, default: bool) -> Self {
        self.multi_select
            .choices
            .choices
            .push(Choice::Choice(Text::new(choice.into())));
        self.multi_select.selected.push(default);
        self
    }

    /// Inserts a [`Separator`] with the given text
    ///
    /// See [`multi_select`] for more information.
    ///
    /// [`Separator`]: super::Choice::Separator
    /// [`multi_select`]: super::Question::multi_select
    ///
    /// # Examples
    ///
    /// ```
    /// use discourse::Question;
    ///
    /// let multi_select = Question::multi_select("cheese")
    ///     .separator("-- custom separator text --")
    ///     .build();
    /// ```
    pub fn separator<I: Into<String>>(mut self, text: I) -> Self {
        self.multi_select
            .choices
            .choices
            .push(Choice::Separator(text.into()));
        self.multi_select.selected.push(false);
        self
    }

    /// Inserts a [`DefaultSeparator`]
    ///
    /// See [`multi_select`] for more information.
    ///
    /// [`DefaultSeparator`]: super::Choice::DefaultSeparator
    /// [`multi_select`]: super::Question::multi_select
    ///
    /// # Examples
    ///
    /// ```
    /// use discourse::Question;
    ///
    /// let multi_select = Question::multi_select("cheese")
    ///     .default_separator()
    ///     .build();
    /// ```
    pub fn default_separator(mut self) -> Self {
        self.multi_select
            .choices
            .choices
            .push(Choice::DefaultSeparator);
        self.multi_select.selected.push(false);
        self
    }

    /// Extends the given iterator of [`Choice`]s
    ///
    /// Every [`Choice::Choice`] within will have a default checked value of `false`. If you want to
    /// set the default checked value, use [`choices_with_default`].
    ///
    /// See [`multi_select`] for more information.
    ///
    /// [`Choice`]: super::Choice
    /// [`choices_with_default`]: Self::choices_with_default
    /// [`multi_select`]: super::Question::multi_select
    ///
    /// # Examples
    ///
    /// ```
    /// use discourse::Question;
    ///
    /// let multi_select = Question::multi_select("cheese")
    ///     .choices(vec![
    ///         "Mozzarella",
    ///         "Cheddar",
    ///         "Parmesan",
    ///     ])
    ///     .build();
    /// ```
    pub fn choices<I, T>(mut self, choices: I) -> Self
    where
        T: Into<Choice<String>>,
        I: IntoIterator<Item = T>,
    {
        self.multi_select
            .choices
            .choices
            .extend(choices.into_iter().map(|c| c.into().map(Text::new)));
        self.multi_select
            .selected
            .resize(self.multi_select.choices.len(), false);
        self
    }

    /// Extends the given iterator of [`Choice`]s with the given default checked value.
    ///
    /// See [`multi_select`] for more information.
    ///
    /// [`Choice`]: super::Choice
    /// [`multi_select`]: super::Question::multi_select
    ///
    /// # Examples
    ///
    /// ```
    /// use discourse::Question;
    ///
    /// let multi_select = Question::multi_select("cheese")
    ///     .choices_with_default(vec![
    ///         ("Mozzarella", true),
    ///         ("Cheddar", false),
    ///         ("Parmesan", false),
    ///     ])
    ///     .build();
    /// ```
    pub fn choices_with_default<I, T>(mut self, choices: I) -> Self
    where
        T: Into<Choice<(String, bool)>>,
        I: IntoIterator<Item = T>,
    {
        let iter = choices.into_iter();
        self.multi_select
            .selected
            .reserve(iter.size_hint().0.saturating_add(1));
        self.multi_select
            .choices
            .choices
            .reserve(iter.size_hint().0.saturating_add(1));

        for choice in iter {
            match choice.into() {
                Choice::Choice((choice, selected)) => {
                    self.multi_select
                        .choices
                        .choices
                        .push(Choice::Choice(Text::new(choice)));
                    self.multi_select.selected.push(selected);
                }
                Choice::Separator(s) => {
                    self.multi_select.choices.choices.push(Choice::Separator(s));
                    self.multi_select.selected.push(false);
                }
                Choice::DefaultSeparator => {
                    self.multi_select
                        .choices
                        .choices
                        .push(Choice::DefaultSeparator);
                    self.multi_select.selected.push(false);
                }
            }
        }
        self
    }

    crate::impl_filter_builder! {
    /// NOTE: The boolean [`Vec`] contains a boolean value for each index even if it is a separator.
    /// However it is guaranteed that all the separator indices will be false.
    ///
    /// # Examples
    ///
    /// ```
    /// use discourse::Question;
    ///
    /// let multi_select = Question::multi_select("evil-cheese")
    ///     .filter(|mut cheeses, previous_answers| {
    ///         cheeses.iter_mut().for_each(|checked| *checked = !*checked);
    ///         cheeses
    ///     })
    ///     .build();
    /// ```
    Vec<bool>; multi_select
    }
    crate::impl_validate_builder! {
    /// NOTE: The boolean [`slice`] contains a boolean value for each index even if it is a
    /// separator. However it is guaranteed that all the separator indices will be false.
    ///
    /// # Examples
    ///
    /// ```
    /// use discourse::Question;
    ///
    /// let multi_select = Question::multi_select("cheese")
    ///     .validate(|cheeses, previous_answers| {
    ///         if cheeses.iter().filter(|&&a| a).count() < 1 {
    ///             Err("You must choose at least one cheese.".into())
    ///         } else {
    ///             Ok(())
    ///         }
    ///     })
    ///     .build();
    /// ```
    [bool]; multi_select
    }

    crate::impl_transform_builder! {
    /// # Examples
    ///
    /// ```
    /// use discourse::Question;
    ///
    /// let multi_select = Question::multi_select("cheese")
    ///     .transform(|cheeses, previous_answers, backend| {
    ///         for cheese in cheeses {
    ///             write!(backend, "({}) {}, ", cheese.index, cheese.name)?;
    ///         }
    ///         Ok(())
    ///     })
    ///     .build();
    /// ```
    [ListItem]; multi_select
    }

    /// Consumes the builder returning a [`Question`]
    ///
    /// [`Question`]: crate::question::Question
    pub fn build(self) -> super::Question<'a> {
        super::Question::new(
            self.opts,
            super::QuestionKind::MultiSelect(self.multi_select),
        )
    }
}

impl<'a> From<MultiSelectBuilder<'a>> for super::Question<'a> {
    /// Consumes the builder returning a [`Question`]
    ///
    /// [`Question`]: crate::question::Question
    fn from(builder: MultiSelectBuilder<'a>) -> Self {
        builder.build()
    }
}

fn print_comma_separated<'a, B: Backend>(
    iter: impl Iterator<Item = &'a str>,
    b: &mut B,
) -> io::Result<()> {
    let mut iter = iter.peekable();

    while let Some(item) = iter.next() {
        b.write_all(item.as_bytes())?;
        if iter.peek().is_some() {
            b.write_all(b", ")?;
        }
    }

    Ok(())
}