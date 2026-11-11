//! Command-line arguments.

use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "sheaf", version, about = "Import lab data, make figures and export them without a window")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Describe a data file or a project: windows, sheets, columns, row counts.
    Info(InfoArgs),
    /// Import data files into a project file.
    Import(ImportArgs),
    /// Make a figure from a data file or a project and write it as PNG, SVG or PDF.
    Plot(PlotArgs),
    /// Write a graph of a project as PNG, SVG or PDF.
    Export(ExportArgs),
    /// Write a worksheet of a project as delimited text.
    ExportData(ExportDataArgs),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum Format {
    /// Decide from the file content.
    Auto,
    /// CH Instruments text export.
    Chi,
    /// Delimited text (CSV, tab-separated, ...).
    Ascii,
}

/// Options that control how data files are read.
#[derive(Args, Debug, Clone)]
pub struct ReadOptions {
    /// File format of the input.
    #[arg(long, value_enum, default_value_t = Format::Auto)]
    pub format: Format,
    /// CH Instruments: keep a cyclic voltammetry file as one curve instead of splitting it into scans.
    #[arg(long)]
    pub no_split_scans: bool,
    /// CH Instruments: do not add a current column in mA.
    #[arg(long)]
    pub no_milliamp: bool,
    /// CH Instruments: do not add a charge column to current-vs-time files.
    #[arg(long)]
    pub no_charge: bool,
    /// CH Instruments: multiply the current by -1.
    #[arg(long)]
    pub invert_current: bool,
    /// Delimited text: the separator. One of comma, tab, semicolon, space, or a single character.
    #[arg(long, value_name = "SEP")]
    pub delimiter: Option<String>,
    /// Delimited text: number of lines before the first data row (default: detect).
    #[arg(long, value_name = "N")]
    pub header_lines: Option<usize>,
    /// Delimited text: numbers use a comma as the decimal separator.
    #[arg(long)]
    pub decimal_comma: bool,
}

#[derive(Args, Debug)]
pub struct InfoArgs {
    /// A data file or a .sheaf project.
    pub path: PathBuf,
    #[command(flatten)]
    pub read: ReadOptions,
}

#[derive(Args, Debug)]
pub struct ImportArgs {
    /// Data files to import. Each becomes one workbook.
    #[arg(required = true)]
    pub files: Vec<PathBuf>,
    /// The project file to write.
    #[arg(short, long)]
    pub output: PathBuf,
    /// Add the workbooks to the existing project at --output instead of starting a new one.
    #[arg(long)]
    pub append: bool,
    /// Also add one graph per imported workbook, in the template the importer suggests.
    #[arg(long)]
    pub plot: bool,
    #[command(flatten)]
    pub read: ReadOptions,
}

/// Changes applied to a graph before it is rendered.
#[derive(Args, Debug, Clone, Default)]
pub struct StyleArgs {
    /// X axis title (default: from the column's long name and units).
    #[arg(long, value_name = "TEXT")]
    pub x_title: Option<String>,
    /// Y axis title.
    #[arg(long, value_name = "TEXT")]
    pub y_title: Option<String>,
    /// Lower X limit.
    #[arg(long, allow_hyphen_values = true)]
    pub x_min: Option<f64>,
    /// Upper X limit.
    #[arg(long, allow_hyphen_values = true)]
    pub x_max: Option<f64>,
    /// Lower Y limit.
    #[arg(long, allow_hyphen_values = true)]
    pub y_min: Option<f64>,
    /// Upper Y limit.
    #[arg(long, allow_hyphen_values = true)]
    pub y_max: Option<f64>,
    /// Distance between major X ticks.
    #[arg(long)]
    pub x_step: Option<f64>,
    /// Distance between major Y ticks.
    #[arg(long)]
    pub y_step: Option<f64>,
    /// Page width in inches.
    #[arg(long, value_name = "INCHES")]
    pub width: Option<f64>,
    /// Page height in inches.
    #[arg(long, value_name = "INCHES")]
    pub height: Option<f64>,
    /// Line width of the data curves in points.
    #[arg(long, value_name = "PT")]
    pub line_width: Option<f64>,
    /// Curve colors in plot order, comma-separated, as #RRGGBB or basic names.
    #[arg(long, value_delimiter = ',', value_name = "COLORS")]
    pub colors: Vec<String>,
    /// Legend labels in plot order, comma-separated.
    #[arg(long, value_delimiter = ',', value_name = "LABELS")]
    pub labels: Vec<String>,
    /// Hide the legend.
    #[arg(long)]
    pub no_legend: bool,
}

#[derive(Args, Debug)]
pub struct PlotArgs {
    /// A data file or a .sheaf project.
    pub input: PathBuf,
    /// The figure to write: .png, .svg or .pdf.
    #[arg(short, long)]
    pub output: PathBuf,
    /// Resolution of PNG output in dots per inch.
    #[arg(long, default_value_t = 300.0)]
    pub dpi: f64,
    /// Figure template: line, scatter, line-symbol or cv (default: what the importer suggests).
    #[arg(short, long)]
    pub template: Option<String>,
    /// Y columns to plot, comma-separated short or long names (default: all Y columns).
    #[arg(short, long, value_delimiter = ',')]
    pub y: Vec<String>,
    /// For a project input: the workbook to plot from (default: the first one).
    #[arg(long)]
    pub book: Option<String>,
    /// The sheet to plot from, by name (default: the workbook's active sheet).
    #[arg(long)]
    pub sheet: Option<String>,
    /// For a project input: render this existing graph instead of making a new one.
    #[arg(long)]
    pub graph: Option<String>,
    /// Also save the resulting project (data plus the new graph) to this .sheaf file.
    #[arg(long, value_name = "PROJECT")]
    pub save: Option<PathBuf>,
    #[command(flatten)]
    pub style: StyleArgs,
    #[command(flatten)]
    pub read: ReadOptions,
}

#[derive(Args, Debug)]
pub struct ExportArgs {
    /// A .sheaf project.
    pub project: PathBuf,
    /// Name of the graph (default: the only graph of the project).
    #[arg(long)]
    pub graph: Option<String>,
    /// The figure to write: .png, .svg or .pdf.
    #[arg(short, long)]
    pub output: PathBuf,
    /// Resolution of PNG output in dots per inch.
    #[arg(long, default_value_t = 300.0)]
    pub dpi: f64,
    #[command(flatten)]
    pub style: StyleArgs,
}

#[derive(Args, Debug)]
pub struct ExportDataArgs {
    /// A .sheaf project.
    pub project: PathBuf,
    /// Name of the workbook (default: the first one).
    #[arg(long)]
    pub book: Option<String>,
    /// Name of the sheet (default: the workbook's active sheet).
    #[arg(long)]
    pub sheet: Option<String>,
    /// The text file to write. A .tsv or .txt extension gives tab-separated output, anything else commas.
    #[arg(short, long)]
    pub output: PathBuf,
}
