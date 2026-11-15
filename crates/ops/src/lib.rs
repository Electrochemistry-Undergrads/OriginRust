//! The work behind `sheaf` (the command line) and `sheaf-mcp`: import, plot, export, analyze and
//! fit as functions that take a request and return data. Nothing here prints; the callers say
//! what to show.

pub mod analyze;
pub mod args;
pub mod cli_analyze;
pub mod cli_fit;
pub mod cli_plot;
pub mod fit;
pub mod import;
pub mod info;
pub mod pick;
pub mod plot;
pub mod read;
pub mod render;
pub mod session;
pub mod tabular;
pub mod themes;

pub use read::{DataFormat, NoFlagOptions, ReadSettings, SharedOptions, ThousandsSetting};
pub use session::{Loaded, SaveReport};
