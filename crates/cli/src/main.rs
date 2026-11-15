//! `sheaf`: import data, make figures and export them without opening a window.
//!
//! Examples:
//!
//! ```text
//! sheaf info run1.txt
//! sheaf plot run1.txt -o cv.png --dpi 300
//! sheaf plot run1.txt run2.txt -o overlay.png      (several files in one figure)
//! sheaf plot data.csv -o fig.pdf --template line --y B,C
//! sheaf fit peak.csv --model Gauss --x-range 0.4:0.9 --plot fit.png
//! sheaf fit --list
//! sheaf import run1.txt run2.txt -o lab.opj
//! sheaf export lab.sheaf --graph Graph1 -o fig.svg
//! ```

mod analyze;
mod commands;
mod fit;
mod simulate;

use clap::{ArgMatches, CommandFactory, FromArgMatches, parser::ValueSource};
use sheaf_ops::args;
use std::process::ExitCode;

fn main() -> ExitCode {
    let matches = args::Cli::command().get_matches();
    let cli = args::Cli::from_arg_matches(&matches).unwrap_or_else(|e| e.exit());
    let mut given = std::collections::HashSet::new();
    given_options(&matches, &mut given);
    commands::set_explicit(given);
    match commands::run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {}", error_text(&e));
            ExitCode::FAILURE
        }
    }
}

/// The ids of the options typed on the command line, at every level of subcommands. An option that
/// only has its default does not count (a saved theme gives way to typed options only).
fn given_options(matches: &ArgMatches, out: &mut std::collections::HashSet<String>) {
    for id in matches.ids() {
        if matches.value_source(id.as_str()) == Some(ValueSource::CommandLine) {
            out.insert(id.to_string());
        }
    }
    if let Some((_, sub)) = matches.subcommand() {
        given_options(sub, out);
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
