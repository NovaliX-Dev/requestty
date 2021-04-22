use crossterm::style::Colorize;
use ui::{widgets, Validation, Widget};

use crate::{error, Answer, Answers};

use super::{Filter, Options, Transformer, Validate};

#[derive(Debug, Default)]
pub struct Input<'f, 'v, 't> {
    default: Option<String>,
    filter: Filter<'f, String>,
    validate: Validate<'v, str>,
    transformer: Transformer<'t, str>,
}

struct InputPrompt<'f, 'v, 't, 'a> {
    message: String,
    input_opts: Input<'f, 'v, 't>,
    input: widgets::StringInput,
    answers: &'a Answers,
}

impl Widget for InputPrompt<'_, '_, '_, '_> {
    fn render<W: std::io::Write>(&mut self, max_width: usize, w: &mut W) -> crossterm::Result<()> {
        self.input.render(max_width, w)
    }

    fn height(&self) -> usize {
        self.input.height()
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> bool {
        self.input.handle_key(key)
    }

    fn cursor_pos(&self, prompt_len: u16) -> (u16, u16) {
        self.input.cursor_pos(prompt_len)
    }
}

impl ui::Prompt for InputPrompt<'_, '_, '_, '_> {
    type ValidateErr = String;
    type Output = String;

    fn prompt(&self) -> &str {
        &self.message
    }

    fn hint(&self) -> Option<&str> {
        self.input_opts.default.as_ref().map(String::as_ref)
    }

    fn finish(self) -> Self::Output {
        let hint = self.input_opts.default;
        let mut ans = self
            .input
            .finish()
            .unwrap_or_else(|| remove_brackets(hint.unwrap()));

        if let Filter::Sync(filter) = self.input_opts.filter {
            ans = filter(ans, self.answers);
        }

        ans
    }
    fn validate(&mut self) -> Result<Validation, Self::ValidateErr> {
        if !self.input.has_value() {
            if self.has_default() {
                return Ok(Validation::Finish);
            } else {
                return Err("Please enter a string".into());
            }
        }

        if let Validate::Sync(ref validate) = self.input_opts.validate {
            validate(self.input.value(), self.answers)?;
        }

        Ok(Validation::Finish)
    }
    fn has_default(&self) -> bool {
        self.input_opts.default.is_some()
    }
    fn finish_default(self) -> <Self as ui::Prompt>::Output {
        remove_brackets(self.input_opts.default.unwrap())
    }
}

impl Input<'_, '_, '_> {
    pub fn ask<W: std::io::Write>(
        mut self,
        message: String,
        answers: &Answers,
        w: &mut W,
    ) -> error::Result<Answer> {
        if let Some(ref mut default) = self.default {
            default.insert(0, '(');
            default.push(')');
        }

        let transformer = self.transformer.take();

        let ans = ui::Input::new(InputPrompt {
            message,
            input_opts: self,
            input: widgets::StringInput::default(),
            answers,
        })
        .run(w)?;

        match transformer {
            Transformer::Sync(transformer) => transformer(&ans, answers, w)?,
            _ => writeln!(w, "{}", ans.as_str().dark_cyan())?,
        }

        Ok(Answer::String(ans))
    }
}

pub struct InputBuilder<'m, 'w, 'f, 'v, 't> {
    opts: Options<'m, 'w>,
    input: Input<'f, 'v, 't>,
}

impl super::Question<'static, 'static, 'static, 'static, 'static> {
    pub fn input<N: Into<String>>(
        name: N,
    ) -> InputBuilder<'static, 'static, 'static, 'static, 'static> {
        InputBuilder {
            opts: Options::new(name.into()),
            input: Default::default(),
        }
    }
}

impl<'m, 'w, 'f, 'v, 't> InputBuilder<'m, 'w, 'f, 'v, 't> {
    pub fn default<I: Into<String>>(mut self, default: I) -> Self {
        self.input.default = Some(default.into());
        self
    }

    pub fn build(self) -> super::Question<'m, 'w, 'f, 'v, 't> {
        super::Question::new(self.opts, super::QuestionKind::Input(self.input))
    }
}

crate::impl_filter_builder!(InputBuilder<'m, 'w, f, 'v, 't> String; (this, filter) => {
    InputBuilder {
        opts: this.opts,
        input: Input {
            filter,
            default: this.input.default,
            validate: this.input.validate,
            transformer: this.input.transformer,
        }
    }
});
crate::impl_validate_builder!(InputBuilder<'m, 'w, 'f, v, 't> str; (this, validate) => {
    InputBuilder {
        opts: this.opts,
        input: Input {
            validate,
            default: this.input.default,
            filter: this.input.filter,
            transformer: this.input.transformer,
        }
    }
});
crate::impl_transformer_builder!(InputBuilder<'m, 'w, 'f, 'v, t> str; (this, transformer) => {
    InputBuilder {
        opts: this.opts,
        input: Input {
            transformer,
            validate: this.input.validate,
            default: this.input.default,
            filter: this.input.filter,
        }
    }
});

impl<'m, 'w, 'f, 'v, 't> From<InputBuilder<'m, 'w, 'f, 'v, 't>>
    for super::Question<'m, 'w, 'f, 'v, 't>
{
    fn from(builder: InputBuilder<'m, 'w, 'f, 'v, 't>) -> Self {
        builder.build()
    }
}

crate::impl_options_builder!(InputBuilder<'f, 'v, 't>; (this, opts) => {
    InputBuilder {
        opts,
        input: this.input,
    }
});

fn remove_brackets(mut s: String) -> String {
    s.remove(0);
    s.pop();
    s
}
