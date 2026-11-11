//! `sheaf`: import data, make figures and export them without opening a window.
//!
//! Examples:
//!
//! ```text
//! sheaf info run1.txt
//! sheaf plot run1.txt -o cv.png --dpi 300
//! sheaf plot run1.txt run2.txt -o overlay.png      (several files in one figure)
//! sheaf plot data.csv -o fig.pdf --template line --y B,C
//! sheaf import run1.txt run2.txt -o lab.sheaf
//! sheaf export lab.sheaf --graph Graph1 -o fig.svg
//! ```

mod analyze;
mod args;
mod commands;

use clap::Parser;
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = args::Cli::parse();
    match commands::run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {}", error_text(&e));
            ExitCode::FAILURE
        }
    }
}

/// The error and its causes joined with ": ". A cause whose text is already part of the message
/// is left out: many errors (file errors, for one) include their cause's text in their own
/// message, and printing it again only doubles the line.
fn error_text(e: &anyhow::Error) -> String {
    let mut text = String::new();
    for cause in e.chain() {
        let c = cause.to_string();
        if !text.contains(&c) {
            if !text.is_empty() {
                text.push_str(": ");
            }
            text.push_str(&c);
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn causes_already_in_the_message_are_not_repeated() {
        let os = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        #[derive(Debug)]
        struct Wrapped(std::io::Error);
        impl std::fmt::Display for Wrapped {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "cannot read 'a.csv': {}", self.0)
            }
        }
        impl std::error::Error for Wrapped {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                Some(&self.0)
            }
        }
        let e = anyhow::Error::new(Wrapped(os)).context("could not import 'a.csv'");
        assert_eq!(error_text(&e), "could not import 'a.csv': cannot read 'a.csv': file not found");
        let plain = anyhow::anyhow!("inner").context("outer");
        assert_eq!(error_text(&plain), "outer: inner");
    }
}
