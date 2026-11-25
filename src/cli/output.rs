//! Shared formatting for command output.

use std::cmp;

/// The width of the separator lines dividing output sections.
const SEPARATOR_WIDTH: usize = 60;

/// Prints the banner that opens a command's output.
pub(crate) fn print_header(title: &str) {
    println!();
    print_separator();
    println!("YAPZM - {title}");
    print_separator();
    println!();
}

/// Prints the separator and title that open an output section.
pub(crate) fn print_section(title: &str) {
    print_separator();
    println!();
    println!("{title}:");
    println!();
}

/// Prints a horizontal separator line dividing output sections.
pub(crate) fn print_separator() {
    println!("{}", "-".repeat(SEPARATOR_WIDTH));
}

/// Picks the singular or plural form of a word for a count.
pub(crate) fn plural<'a>(count: usize, singular: &'a str, plural: &'a str) -> &'a str {
    match count.cmp(&1) {
        cmp::Ordering::Equal => singular,
        _ => plural,
    }
}
