use std::fmt::Display;
use std::time::Duration;

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
        let is_penultimate = i == items.len() - 2;
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
