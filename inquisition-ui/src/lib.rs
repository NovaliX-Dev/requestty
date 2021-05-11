//! A widget based cli ui rendering library
use std::sync::Mutex;

pub use sync_input::{Input, Prompt};
pub use widget::Widget;

/// In build widgets
pub mod widgets {
    pub use super::char_input::CharInput;
    pub use super::list::{List, ListPicker};
    pub use super::string_input::StringInput;

    /// The default type for filter_map_char in [`StringInput`] and [`CharInput`]
    pub type FilterMapChar = fn(char) -> Option<char>;

    /// Character filter that lets every character through
    pub fn no_filter(c: char) -> Option<char> {
        Some(c)
    }
}

cfg_async! {
pub use async_input::AsyncPrompt;
mod async_input;
}

pub mod backend;
mod char_input;
pub mod error;
pub mod events;
mod list;
mod string_input;
mod sync_input;
mod widget;

/// Returned by [`Prompt::validate`]
pub enum Validation {
    /// If the prompt is ready to finish.
    Finish,
    /// If the state is valid, but the prompt should still persist.
    /// Unlike returning an Err, this will not print anything unique, and is a way for the prompt to
    /// say that it internally has processed the `Enter` key, but is not complete.
    Continue,
}

lazy_static::lazy_static! {
    static ref EXIT_HANDLER: Mutex<fn()> = Mutex::new(default_exit);
}

/// Sets the exit handler to call when `CTRL+C` or EOF is received
///
/// By default, it exits the program, however it can be overridden to not exit. If it doesn't exit,
/// [`Input::run`] will return an `Err`
pub fn set_exit_handler(handler: fn()) {
    *EXIT_HANDLER.lock().unwrap() = handler;
}

fn default_exit() {
    std::process::exit(130);
}

fn exit() {
    match EXIT_HANDLER.lock() {
        Ok(exit) => exit(),
        Err(_) => default_exit(),
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! cfg_async {
    ($($item:item)*) => {
        $(
            #[cfg(any(feature = "tokio", feature = "async-std", feature = "smol"))]
            $item
        )*
    };
}