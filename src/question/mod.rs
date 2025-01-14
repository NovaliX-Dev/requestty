//! A module that contains things related to [`Question`]s.

mod choice;
mod confirm;
mod editor;
mod expand;
mod handler;
#[macro_use]
mod impl_macros;
mod input;
mod multi_select;
mod number;
mod order_select;
#[macro_use]
mod options;
mod custom_prompt;
mod password;
mod raw_select;
mod select;

pub use choice::Choice;
pub use confirm::ConfirmBuilder;
pub use custom_prompt::{CustomPromptBuilder, Prompt};
pub use editor::EditorBuilder;
pub use expand::ExpandBuilder;
pub use input::InputBuilder;
pub use multi_select::MultiSelectBuilder;
pub use number::{FloatBuilder, IntBuilder};
pub use order_select::{OrderSelectBuilder, OrderSelectItem};
pub use password::PasswordBuilder;
pub use raw_select::RawSelectBuilder;
pub use select::SelectBuilder;

use ui::{backend::Backend, events::EventIterator};

use crate::{Answer, Answers};
use choice::{get_sep_str, ChoiceList};
use custom_prompt::CustomPromptInteral;
use handler::{
    AutoComplete, Filter, Transform, TransformByVal, Validate, ValidateByVal, ValidateOnKey,
    ValidateOnKeyByVal,
};
use options::Options;

/// A `Question` that can be asked.
///
/// There are 11 variants.
///
/// - [`input`](Question::input)
/// - [`password`](Question::password)
/// - [`editor`](Question::editor)
/// - [`confirm`](Question::confirm)
/// - [`int`](Question::int)
/// - [`float`](Question::float)
/// - [`expand`](Question::expand)
/// - [`select`](Question::select)
/// - [`raw_select`](Question::raw_select)
/// - [`multi_select`](Question::multi_select)
/// - [`order_select`](Question::order_select)
/// - [`custom`](Question::custom)
///
/// Every [`Question`] has 4 common options.
///
/// - `name` (required): This is used as the key in [`Answers`].
///   It is not shown to the user unless `message` is unspecified.
///
/// - `message`: The message to display when the prompt is rendered in the terminal.
///   If it is not given, the `message` defaults to "\<name\>: ". It is recommended to set this as
///   `name` is meant to be a programmatic `id`.
///
/// - `when`: Whether to ask the question or not.
///   This can be used to have context based questions. If it is not given, it defaults to `true`.
///
/// - `ask_if_answered`: Prompt the question even if it is answered.
///   By default if an answer with the given `name` already exists, the question will be skipped.
///   This can be override by setting `ask_if_answered` is set to `true`.
///
/// A `Question` can be asked by creating a [`PromptModule`] or using [`prompt_one`] or
/// [`prompt_one_with`].
///
/// # Examples
///
/// ```
/// use requestty::Question;
///
/// let question = Question::input("name")
///     .message("What is your name?")
///     .default("John Doe")
///     .transform(|name, previous_answers, backend| {
///         write!(backend, "Hello, {}!", name)
///     })
///     .build();
/// ```
///
/// [`PromptModule`]: crate::PromptModule
/// [`prompt_one`]: crate::prompt_one
/// [`prompt_one_with`]: crate::prompt_one_with
#[derive(Debug)]
pub struct Question<'a> {
    kind: QuestionKind<'a>,
    opts: Options<'a>,
}

impl<'a> Question<'a> {
    fn new(opts: Options<'a>, kind: QuestionKind<'a>) -> Self {
        Self { kind, opts }
    }
}

impl Question<'static> {
    /// Prompt that takes user input and returns a [`String`]
    ///
    /// <img
    ///   src="https://raw.githubusercontent.com/lutetium-vanadium/requestty/master/assets/input.gif"
    ///   style="max-height: 11rem"
    /// />
    ///
    /// See the various methods on the [`builder`] for more details on each available option.
    ///
    /// # Examples
    ///
    /// ```
    /// use requestty::Question;
    ///
    /// let input = Question::input("name")
    ///     .message("What is your name?")
    ///     .default("John Doe")
    ///     .transform(|name, previous_answers, backend| {
    ///         write!(backend, "Hello, {}!", name)
    ///     })
    ///     .build();
    /// ```
    ///
    /// [`builder`]: InputBuilder
    pub fn input<N: Into<String>>(name: N) -> InputBuilder<'static> {
        InputBuilder::new(name.into())
    }

    /// Prompt that takes user input and hides it.
    ///
    /// How it looks if you set a mask:
    ///
    /// <img
    ///   src="https://raw.githubusercontent.com/lutetium-vanadium/requestty/master/assets/password-mask.gif"
    ///   style="max-height: 11rem"
    /// />
    ///
    /// How it looks if you do not set a mask:
    ///
    /// <img
    ///   src="https://raw.githubusercontent.com/lutetium-vanadium/requestty/master/assets/password-hidden.gif"
    ///   style="max-height: 11rem"
    /// />
    ///
    /// See the various methods on the [`builder`] for more details on each available option.
    ///
    /// # Examples
    ///
    /// ```
    /// use requestty::Question;
    ///
    /// let password = Question::password("password")
    ///     .message("What is your password?")
    ///     .mask('*')
    ///     .build();
    /// ```
    ///
    /// [`builder`]: PasswordBuilder
    pub fn password<N: Into<String>>(name: N) -> PasswordBuilder<'static> {
        PasswordBuilder::new(name.into())
    }

    /// Prompt that takes launches the users preferred editor on a temporary file
    ///
    /// Once the user exits their editor, the contents of the temporary file are read in as the
    /// result. The editor to use can be specified by the [`editor`] method. If unspecified, the
    /// editor is determined by the `$VISUAL` or `$EDITOR` environment variables. If neither of
    /// those are present, `vim` (for unix) or `notepad` (for windows) is used.
    ///
    /// <img
    ///   src="https://raw.githubusercontent.com/lutetium-vanadium/requestty/master/assets/editor.gif"
    ///   style="max-height: 30rem"
    /// />
    ///
    /// See the various methods on the [`builder`] for more details on each available option.
    ///
    /// # Examples
    ///
    /// ```
    /// use requestty::Question;
    ///
    /// let editor = Question::editor("description")
    ///     .message("Please enter a short description about yourself")
    ///     .extension(".md")
    ///     .build();
    /// ```
    ///
    /// [`builder`]: EditorBuilder
    /// [`editor`]: EditorBuilder::editor
    pub fn editor<N: Into<String>>(name: N) -> EditorBuilder<'static> {
        EditorBuilder::new(name.into())
    }

    /// Prompt that returns `true` or `false`.
    ///
    /// <img
    ///   src="https://raw.githubusercontent.com/lutetium-vanadium/requestty/master/assets/confirm.gif"
    ///   style="max-height: 11rem"
    /// />
    ///
    /// See the various methods on the [`builder`] for more details on each available option.
    ///
    /// # Examples
    ///
    /// ```
    /// use requestty::Question;
    ///
    /// let confirm = Question::confirm("anonymous")
    ///     .message("Do you want to remain anonymous?")
    ///     .build();
    /// ```
    ///
    /// [`builder`]: ConfirmBuilder
    pub fn confirm<N: Into<String>>(name: N) -> ConfirmBuilder<'static> {
        ConfirmBuilder::new(name.into())
    }

    /// Prompt that takes a [`i64`] as input.
    ///
    /// The number is parsed using [`from_str`].
    ///
    /// <img
    ///   src="https://raw.githubusercontent.com/lutetium-vanadium/requestty/master/assets/int.gif"
    ///   style="max-height: 11rem"
    /// />
    ///
    /// See the various methods on the [`builder`] for more details on each available option.
    ///
    /// # Examples
    ///
    /// ```
    /// use requestty::Question;
    ///
    /// let int = Question::int("age")
    ///     .message("What is your age?")
    ///     .validate(|age, previous_answers| {
    ///         if age > 0 && age < 130 {
    ///             Ok(())
    ///         } else {
    ///             Err(format!("You cannot be {} years old!", age))
    ///         }
    ///     })
    ///     .build();
    /// ```
    ///
    /// [`builder`]: IntBuilder
    /// [`from_str`]: https://doc.rust-lang.org/std/primitive.i64.html#method.from_str
    pub fn int<N: Into<String>>(name: N) -> IntBuilder<'static> {
        IntBuilder::new(name.into())
    }

    /// Prompt that takes a [`f64`] as input.
    ///
    /// The number is parsed using [`from_str`], but cannot be `NaN`.
    ///
    /// <img
    ///   src="https://raw.githubusercontent.com/lutetium-vanadium/requestty/master/assets/float.gif"
    ///   style="max-height: 11rem"
    /// />
    ///
    /// See the various methods on the [`builder`] for more details on each available option.
    ///
    /// # Examples
    ///
    /// ```
    /// use requestty::Question;
    ///
    /// let float = Question::float("number")
    ///     .message("What is your favourite number?")
    ///     .validate(|num, previous_answers| {
    ///         if num.is_finite() {
    ///             Ok(())
    ///         } else {
    ///             Err("Please enter a finite number".to_owned())
    ///         }
    ///     })
    ///     .build();
    /// ```
    ///
    /// [`builder`]: FloatBuilder
    /// [`from_str`]: https://doc.rust-lang.org/std/primitive.f64.html#method.from_str
    pub fn float<N: Into<String>>(name: N) -> FloatBuilder<'static> {
        FloatBuilder::new(name.into())
    }

    /// Prompt that allows the user to select from a list of options by key
    ///
    /// The keys are ascii case-insensitive characters. The 'h' option is added by the prompt and
    /// shouldn't be defined.
    ///
    /// The choices are represented with the [`Choice`] enum. [`Choice::Choice`] can be multi-line,
    /// but [`Choice::Separator`]s can only be single line.
    ///
    /// <img
    ///   src="https://raw.githubusercontent.com/lutetium-vanadium/requestty/master/assets/expand.gif"
    ///   style="max-height: 15rem"
    /// />
    ///
    /// See the various methods on the [`builder`] for more details on each available option.
    ///
    /// # Examples
    ///
    /// ```
    /// use requestty::Question;
    ///
    /// let expand = Question::expand("overwrite")
    ///     .message("Conflict on `file.rs`")
    ///     .choices(vec![
    ///         ('y', "Overwrite"),
    ///         ('a', "Overwrite this one and all next"),
    ///         ('d', "Show diff"),
    ///     ])
    ///     .default_separator()
    ///     .choice('x', "Abort")
    ///     .build();
    /// ```
    ///
    /// [`builder`]: ExpandBuilder
    pub fn expand<N: Into<String>>(name: N) -> ExpandBuilder<'static> {
        ExpandBuilder::new(name.into())
    }

    /// Prompt that allows the user to select from a list of options
    ///
    /// The choices are represented with the [`Choice`] enum. [`Choice::Choice`] can be multi-line,
    /// but [`Choice::Separator`]s can only be single line.
    ///
    /// <img
    ///   src="https://raw.githubusercontent.com/lutetium-vanadium/requestty/master/assets/select.gif"
    ///   style="max-height: 15rem"
    /// />
    ///
    /// See the various methods on the [`builder`] for more details on each available option.
    ///
    /// # Examples
    ///
    /// ```
    /// use requestty::{Question, DefaultSeparator};
    ///
    /// let select = Question::select("theme")
    ///     .message("What do you want to do?")
    ///     .choices(vec![
    ///         "Order a pizza".into(),
    ///         "Make a reservation".into(),
    ///         DefaultSeparator,
    ///         "Ask for opening hours".into(),
    ///         "Contact support".into(),
    ///         "Talk to the receptionist".into(),
    ///     ])
    ///     .build();
    /// ```
    ///
    /// [`builder`]: SelectBuilder
    pub fn select<N: Into<String>>(name: N) -> SelectBuilder<'static> {
        SelectBuilder::new(name.into())
    }

    /// Prompt that allows the user to select from a list of options with indices
    ///
    /// The choices are represented with the [`Choice`] enum. [`Choice::Choice`] can be multi-line,
    /// but [`Choice::Separator`]s can only be single line.
    ///
    /// <img
    ///   src="https://raw.githubusercontent.com/lutetium-vanadium/requestty/master/assets/raw-select.gif"
    ///   style="max-height: 15rem"
    /// />
    ///
    /// See the various methods on the [`builder`] for more details on each available option.
    ///
    /// # Examples
    ///
    /// ```
    /// use requestty::{Question, DefaultSeparator};
    ///
    /// let raw_select = Question::raw_select("theme")
    ///     .message("What do you want to do?")
    ///     .choices(vec![
    ///         "Order a pizza".into(),
    ///         "Make a reservation".into(),
    ///         DefaultSeparator,
    ///         "Ask for opening hours".into(),
    ///         "Contact support".into(),
    ///         "Talk to the receptionist".into(),
    ///     ])
    ///     .build();
    /// ```
    ///
    /// [`builder`]: RawSelectBuilder
    pub fn raw_select<N: Into<String>>(name: N) -> RawSelectBuilder<'static> {
        RawSelectBuilder::new(name.into())
    }

    /// Prompt that allows the user to select multiple items from a list of options
    ///
    /// Unlike the other list based prompts, this has a per choice boolean default.
    ///
    /// The choices are represented with the [`Choice`] enum. [`Choice::Choice`] can be multi-line,
    /// but [`Choice::Separator`]s can only be single line.
    ///
    /// <img
    ///   src="https://raw.githubusercontent.com/lutetium-vanadium/requestty/master/assets/multi-select.gif"
    ///   style="max-height: 20rem"
    /// />
    ///
    /// See the various methods on the [`builder`] for more details on each available option.
    ///
    /// # Examples
    ///
    /// ```
    /// use requestty::{Question, DefaultSeparator};
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
    /// [`builder`]: MultiSelectBuilder
    pub fn multi_select<N: Into<String>>(name: N) -> MultiSelectBuilder<'static> {
        MultiSelectBuilder::new(name.into())
    }

    /// Prompt that allows the user to organize a list of options.
    ///
    /// The choices are [`String`]s and can be multiline.
    ///
    /// // TODO : add a gif for OrderSelect
    /// <img
    ///   src="https://raw.githubusercontent.com/lutetium-vanadium/requestty/master/assets/multi-select.gif"
    ///   style="max-height: 20rem"
    /// />
    ///
    /// See the various methods on the [`builder`] for more details on each available option.
    ///
    /// # Examples
    ///
    /// ```
    /// use requestty::{Question, DefaultSeparator};
    ///
    /// let multi_select = Question::order_select("tasks")
    ///     .message("Please organize the tasks to be done at home")
    ///     .choices(vec![
    ///         "Make the bed",
    ///         "Clean the dishes",
    ///         "Mow the lawn",
    ///     ])
    ///     .build();
    /// ```
    ///
    /// [`builder`]: OrderSelectBuilder
    pub fn order_select<N: Into<String>>(name: N) -> OrderSelectBuilder<'static> {
        OrderSelectBuilder::new(name.into())
    }

    /// Create a [`Question`] from a custom prompt.
    ///
    /// See [`Prompt`] for more information on writing custom prompts and the various methods on the
    /// [`builder`] for more details on each available option.
    ///
    /// # Examples
    ///
    /// ```
    /// use requestty::{prompt, Question};
    ///
    /// #[derive(Debug)]
    /// struct MyPrompt { /* ... */ }
    ///
    /// # impl MyPrompt {
    /// #     fn new() -> MyPrompt {
    /// #         MyPrompt {}
    /// #     }
    /// # }
    ///
    /// impl prompt::Prompt for MyPrompt {
    ///     fn ask(
    ///         self,
    ///         message: String,
    ///         answers: &prompt::Answers,
    ///         backend: &mut dyn prompt::Backend,
    ///         events: &mut dyn prompt::EventIterator,
    ///     ) -> requestty::Result<Option<prompt::Answer>> {
    /// #       todo!()
    ///         /* ... */
    ///     }
    /// }
    ///
    /// let prompt = Question::custom("my-prompt", MyPrompt::new())
    ///     .message("Hello from MyPrompt!")
    ///     .build();
    /// ```
    ///
    /// [`builder`]: CustomPromptBuilder
    pub fn custom<'a, N, P>(name: N, prompt: P) -> CustomPromptBuilder<'a>
    where
        N: Into<String>,
        P: Prompt + 'a,
    {
        CustomPromptBuilder::new(name.into(), Box::new(Some(prompt)))
    }
}

#[derive(Debug)]
enum QuestionKind<'a> {
    Input(input::Input<'a>),
    Int(number::Int<'a>),
    Float(number::Float<'a>),
    Confirm(confirm::Confirm<'a>),
    Select(select::Select<'a>),
    RawSelect(raw_select::RawSelect<'a>),
    Expand(expand::Expand<'a>),
    MultiSelect(multi_select::MultiSelect<'a>),
    OrderSelect(order_select::OrderSelect<'a>),
    Password(password::Password<'a>),
    Editor(editor::Editor<'a>),
    Custom(Box<dyn CustomPromptInteral + 'a>),
}

impl Question<'_> {
    pub(crate) fn ask<B: Backend, I: EventIterator>(
        self,
        answers: &Answers,
        b: &mut B,
        events: &mut I,
    ) -> ui::Result<Option<(String, Answer)>> {
        // Already asked
        if !self.opts.ask_if_answered && answers.contains_key(&self.opts.name) {
            return Ok(None);
        }

        // Shouldn't be asked
        if !self.opts.when.get(answers) {
            return Ok(None);
        }

        let name = self.opts.name;
        let message = self
            .opts
            .message
            .map(|message| message.get(answers))
            .unwrap_or_else(|| name.clone() + ":");
        let on_esc = self.opts.on_esc.get(answers);

        let res = match self.kind {
            QuestionKind::Input(i) => i.ask(message, on_esc, answers, b, events)?,
            QuestionKind::Int(i) => i.ask(message, on_esc, answers, b, events)?,
            QuestionKind::Float(f) => f.ask(message, on_esc, answers, b, events)?,
            QuestionKind::Confirm(c) => c.ask(message, on_esc, answers, b, events)?,
            QuestionKind::Select(l) => l.ask(message, on_esc, answers, b, events)?,
            QuestionKind::RawSelect(r) => r.ask(message, on_esc, answers, b, events)?,
            QuestionKind::Expand(e) => e.ask(message, on_esc, answers, b, events)?,
            QuestionKind::MultiSelect(c) => c.ask(message, on_esc, answers, b, events)?,
            QuestionKind::OrderSelect(c) => c.ask(message, on_esc, answers, b, events)?,
            QuestionKind::Password(p) => p.ask(message, on_esc, answers, b, events)?,
            QuestionKind::Editor(e) => e.ask(message, on_esc, answers, b, events)?,
            QuestionKind::Custom(mut o) => o.ask(message, answers, b, events)?,
        };

        Ok(res.map(|res| (name, res)))
    }
}

/// The type which needs to be returned by the [`auto_complete`] function.
///
/// [`auto_complete`]: InputBuilder::auto_complete
#[cfg(feature = "smallvec")]
pub type Completions<T> = smallvec::SmallVec<[T; 1]>;

/// The type which needs to be returned by the [`auto_complete`] function.
///
/// [`auto_complete`]: InputBuilder::auto_complete
#[cfg(not(feature = "smallvec"))]
pub type Completions<T> = Vec<T>;

#[cfg(feature = "smallvec")]
pub use smallvec::smallvec as completions;

#[cfg(not(feature = "smallvec"))]
pub use std::vec as completions;
