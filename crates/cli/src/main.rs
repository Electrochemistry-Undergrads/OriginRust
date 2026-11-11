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

mod args;
mod commands;

use clap::Parser;
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = args::Cli::parse();
    match commands::run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}
