use ui::backend::Backend;

use super::{Float, Int};
use crate::question::Options;

macro_rules! builder {
    ($(#[$meta:meta])* struct $builder_name:ident : $type:ident -> $inner_ty:ty, $litral:expr;
     declare = $declare:expr;
     default = $default:expr;
     filter = $filter:expr;
     validate = $validate:expr;
     ) => {
        $(#[$meta])*
        #[derive(Debug)]
        pub struct $builder_name<'a> {
            opts: Options<'a>,
            inner: $type<'a>,
        }

        impl<'a> $builder_name<'a> {
            pub(crate) fn new(name: String) -> Self {
                $builder_name {
                    opts: Options::new(name),
                    inner: Default::default(),
                }
            }

            crate::impl_options_builder! {
            message
            /// # Examples
            ///
            /// ```
            /// use discourse::Question;
            ///
            #[doc = $declare]
            ///     .message("Please enter a number")
            ///     .build();
            /// ```

            when
            /// # Examples
            ///
            /// ```
            /// use discourse::{Question, Answers};
            ///
            #[doc = $declare]
            ///     .when(|previous_answers: &Answers| match previous_answers.get("ask_number") {
            ///         Some(ans) => ans.as_bool().unwrap(),
            ///         None => true,
            ///     })
            ///     .build();
            /// ```

            ask_if_answered
            /// # Examples
            ///
            /// ```
            /// use discourse::{Question, Answers};
            ///
            #[doc = $declare]
            ///     .ask_if_answered(true)
            ///     .build();
            /// ```
            }

            /// Set a default value
            ///
            /// If the input text is empty, the `default` is taken as the answer.
            ///
            /// If `default` is used, validation is skipped, but `filter` is still called.
            ///
            /// # Examples
            ///
            /// ```
            /// use discourse::Question;
            ///
            #[doc = $declare]
            #[doc = $default]
            ///     .build();
            /// ```
            pub fn default(mut self, default: $inner_ty) -> Self {
                self.inner.default = Some(default);
                self
            }

            crate::impl_filter_builder! {
            /// # Examples
            ///
            /// ```
            /// use discourse::Question;
            ///
            #[doc = $declare]
            #[doc = $filter]
            ///     .build();
            /// ```
            $inner_ty; inner
            }

            crate::impl_validate_builder! {
            /// # Examples
            ///
            /// ```
            /// use discourse::Question;
            ///
            #[doc = $declare]
            ///     .validate(|n, previous_answers| {
            #[doc = $validate]
            ///             Ok(())
            ///         } else {
            ///             Err("Please enter a positive number".to_owned())
            ///         }
            ///     })
            ///     .build();
            /// ```
            by val $inner_ty; inner
            }

            crate::impl_transform_builder! {
            /// # Examples
            ///
            /// ```
            /// use discourse::Question;
            ///
            #[doc = $declare]
            ///     .transform(|n, previous_answers, backend| {
            ///         write!(backend, "{:e}", n)
            ///     })
            ///     .build();
            /// ```
            by val $inner_ty; inner
            }

            /// Consumes the builder returning a [`Question`]
            ///
            /// [`Question`]: crate::question::Question
            pub fn build(self) -> crate::question::Question<'a> {
                crate::question::Question::new(self.opts, crate::question::QuestionKind::$type(self.inner))
            }
        }

        impl<'a> From<$builder_name<'a>> for crate::question::Question<'a> {
            /// Consumes the builder returning a [`Question`]
            ///
            /// [`Question`]: crate::question::Question
            fn from(builder: $builder_name<'a>) -> Self {
                builder.build()
            }
        }
    };
}

builder! {
/// The builder for an [`int`] prompt.
///
/// The number is parsed using [`from_str`].
///
/// See the various methods for more details on each available option.
///
/// # Examples
///
/// ```
/// use discourse::Question;
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
/// [`from_str`]: https://doc.rust-lang.org/std/primitive.i64.html#method.from_str
/// [`int`]: crate::question::Question::int
struct IntBuilder: Int -> i64, 10;
declare  = r#"let question = Question::int("int")"#;
default  = "    .default(10)";
filter   = "    .filter(|n, previous_answers| n + 10)";
validate = "        if n.is_positive() {";
}

builder! {
/// The builder for a [`float`] prompt.
///
/// The number is parsed using [`from_str`], but cannot be `NaN`.
///
/// See the various methods for more details on each available option.
///
/// # Examples
///
/// ```
/// use discourse::Question;
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
/// [`float`]: crate::question::Question::float
/// [`from_str`]: https://doc.rust-lang.org/std/primitive.f64.html#method.from_str
struct FloatBuilder: Float -> f64, 10.0;
declare  = r#"let question = Question::float("float")"#;
default  = "    .default(10.0)";
filter   = "    .filter(|n, previous_answers| (n * 10000.0).round() / 10000.0)";
validate = "        if n.is_sign_positive() {";
}