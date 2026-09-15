use std::fmt::Display;
use std::time::Duration;

use colored::Colorize;
use dialoguer::theme::Theme;
use dialoguer::{Input, Password, Select};
use indicatif::{ProgressBar, ProgressStyle};
use tabled::Table;
use tabled::builder::Builder;
use tabled::settings::object::Columns;
use tabled::settings::{Modify, Padding, Style};

/// Displays a spinner loading bar on the console.
///
/// Arguments:
/// * `message` - The message to display with the spinner.
///
/// Returns:
/// [`ProgressBar`] - The spinner bar already being displayed on the console.
pub fn spinner(message: impl Into<String>) -> ProgressBar {
    let bar = ProgressBar::new_spinner()
        .with_message(message.into())
        .with_style(
            ProgressStyle::default_spinner()
                .tick_strings(&["[\\]", "[|]", "[/]", "[-]", "OK "])
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
    bar.enable_steady_tick(Duration::from_millis(200));

    bar
}

/// Formats items into a human-readable list.
///
/// For example,
/// - `[1] => "1"`
/// - `[1, 2] => "1 and 2"`
/// - `[1, 2, 3] => "1, 2, and 3"`.
/// - `[1, 2, 3, 4] => "1, 2, 3, and 4"`.
///
/// Arguments:
/// * `items` - The items of the list.
///
/// Returns:
/// [`String`] -> The formatted list.
pub fn list<T>(items: &[T]) -> String
where
    T: Display,
{
    let mut string = String::new();

    for (i, item) in items.iter().enumerate() {
        let is_first = i == 0;
        let is_penultimate = items.len() >= 2 && i == items.len() - 2;
        let is_last = i == items.len() - 1;

        match (is_first, is_penultimate, is_last) {
            (false, false, false) => string.push_str(&format!("{item}, ")),
            (false, false, true) => string.push_str(&item.to_string()),
            (false, true, false) => string.push_str(&format!("{item}, and ")),
            (false, true, true) => unreachable!(),
            (true, false, false) => string.push_str(&format!("{item}, ")),
            (true, false, true) => string.push_str(&item.to_string()),
            (true, true, false) => string.push_str(&format!("{item} and ")),
            (true, true, true) => unreachable!(),
        }
    }

    string
}

/// Creates an aligned list using a [`Table`].
///
/// For example,
/// ```
/// let rows = vec![
///     ["mike", "wazowski"],
///     ["obi-wan", "kenobi"],
/// ];
///
/// let table = display::list_table(rows);
/// println!("{table}");
/// ```
///
/// Arguments:
/// * `items` - The rows to add to the table.
///
/// Returns:
/// [`Table`] - The list table. It can be rendered via `println!("{table}");`.
pub fn list_table<T, I>(items: impl IntoIterator<Item = T>) -> Table
where
    T: IntoIterator<Item = I>,
    I: Into<String>,
{
    let mut builder = Builder::new();
    for item in items.into_iter() {
        builder.push_record(item);
    }

    let mut table = builder.build();
    table.with(Style::empty());
    table.with(Modify::new(Columns::first()).with(Padding::zero()));

    table
}

struct InputTheme;

impl Theme for InputTheme {
    fn format_input_prompt(
        &self,
        f: &mut dyn std::fmt::Write,
        prompt: &str,
        _default: Option<&str>,
    ) -> std::fmt::Result {
        let prefix = ">>>".purple();

        write!(f, "{prefix} {prompt}")
    }

    fn format_input_prompt_selection(
        &self,
        f: &mut dyn std::fmt::Write,
        prompt: &str,
        sel: &str,
    ) -> std::fmt::Result {
        let prefix = ">>>".purple();

        write!(f, "{prefix} {prompt}{sel}")
    }

    fn format_select_prompt(&self, f: &mut dyn std::fmt::Write, prompt: &str) -> std::fmt::Result {
        let prefix = ">>>".purple();

        write!(f, "{prefix} {prompt}")
    }

    fn format_select_prompt_item(
        &self,
        f: &mut dyn std::fmt::Write,
        text: &str,
        active: bool,
    ) -> std::fmt::Result {
        if active {
            write!(f, "{}", format!("  * {text}").purple())
        } else {
            write!(f, "    {text}")
        }
    }

    fn format_select_prompt_selection(
        &self,
        f: &mut dyn std::fmt::Write,
        prompt: &str,
        sel: &str,
    ) -> std::fmt::Result {
        let prefix = ">>>".purple();

        write!(f, "{prefix} {prompt}{}", sel.purple())
    }
}

/// Prompts the user for input.
///
/// Arguments:
/// * `prompt` - The message to prompt the user with.
///
/// Returns:
/// * `Ok(String)` - The user's input.
/// * `Err(Error)` - If the user cancelled the operation or if the user could not be prompted.
pub fn input(prompt: impl Into<String>) -> Result<String, dialoguer::Error> {
    Input::with_theme(&InputTheme)
        .with_prompt(prompt)
        .interact_text()
}

/// Prompts the user for input.
///
/// Arguments:
/// * `prompt` - The message to prompt the user with.
///
/// Returns:
/// * `Ok(String)` - The user's input.
/// * `Err(Error)` - If the user cancelled the operation or if the user could not be prompted.
pub fn input_with_validation<'a, V>(
    prompt: impl Into<String>,
    validator: V,
) -> Result<String, dialoguer::Error>
where
    V: FnMut(&String) -> Result<(), &'a str>,
{
    Input::with_theme(&InputTheme)
        .with_prompt(prompt)
        .validate_with(validator)
        .interact_text()
}

/// Prompt the user for a password.
///
/// Arguments:
/// * `prompt` - The message to prompt the user with.
///
/// Returns:
/// * `Ok(String)` - The password they entered.
/// * `Err(Error)` - If the user cancelled the operation of or the user could not be prompted.
pub fn password(prompt: impl Into<String>) -> Result<String, dialoguer::Error> {
    Password::with_theme(&InputTheme)
        .with_prompt(prompt)
        .interact()
}

/// Prompts the user for a password.
///
/// Arguments:
/// * `prompt` - The message to prompt the user with.
/// * `repeat` - Ask the user to confirm their password with this message.
/// * `error` - The error to display if the user didn't confirm their password correctly.
///
/// Returns:
/// * `Ok(String)` - The user's input.
/// * `Err(Error)` - If the user cancelled the operation or if the user could not be prompted.
pub fn password_with_confirmation(
    prompt: impl Into<String>,
    repeat: impl Into<String>,
    error: impl Into<String>,
) -> Result<String, dialoguer::Error> {
    Password::with_theme(&InputTheme)
        .with_prompt(prompt)
        .with_confirmation(repeat, error)
        .interact()
}

/// Prompts the user to select an option.
///
/// Arguments:
/// * `prompt` - The message to prompt the user with.
/// * `options` - The options to give the user for selection.
///
/// Returns:
/// * `Ok(usize)` - The index of the option selected by the user.
/// * `Err(Error)` - If the user cancelled the operation or if the user could not be prompted.
pub fn select(
    prompt: impl Into<String>,
    options: impl IntoIterator<Item = impl ToString>,
) -> Result<usize, dialoguer::Error> {
    Select::with_theme(&InputTheme)
        .with_prompt(prompt)
        .items(options)
        .interact()
}

/// Prompts the user to select an option with an option selected by default.
///
/// Arguments:
/// * `prompt` - The message to prompt the user with.
/// * `options` - The options to give the user for selection.
/// * `default` - The index of the default option.
///
/// Returns:
/// * `Ok(usize)` - The index of the option selected by the user.
/// * `Err(Error)` - If the user cancelled the operation or if the user could not be prompted.
pub fn select_with_default(
    prompt: impl Into<String>,
    options: impl IntoIterator<Item = impl ToString>,
    default: usize,
) -> Result<usize, dialoguer::Error> {
    Select::with_theme(&InputTheme)
        .with_prompt(prompt)
        .items(options)
        .default(default)
        .interact()
}

/// Prompts the user to select an option with an option selected by default, without reporting the
/// selection.
///
/// Arguments:
/// * `prompt` - The message to prompt the user with.
/// * `options` - The options to give the user for selection.
/// * `default` - The index of the default option.
///
/// Returns:
/// * `Ok(usize)` - The index of the option selected by the user.
/// * `Err(Error)` - If the user cancelled the operation or if the user could not be prompted.
pub fn select_with_default_without_report(
    prompt: impl Into<String>,
    options: impl IntoIterator<Item = impl ToString>,
    default: usize,
) -> Result<usize, dialoguer::Error> {
    Select::with_theme(&InputTheme)
        .with_prompt(prompt)
        .items(options)
        .default(default)
        .report(false)
        .interact()
}
