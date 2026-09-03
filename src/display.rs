use std::fmt::Display;
use std::time::Duration;

use indicatif::{ProgressBar, ProgressStyle};

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
