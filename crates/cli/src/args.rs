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
    Plot(Box<PlotArgs>),
    /// Write a graph of a project as PNG, SVG or PDF.
    Export(ExportArgs),
    /// Write a worksheet of a project as delimited text.
    ExportData(ExportDataArgs),
    /// Run an analysis tool on worksheet columns: integrate, differentiate, smooth, normalize,
    /// baseline, interpolate, peaks, stats, sort, filter.
    Analyze(AnalyzeArgs),
    /// Fit a model to a column: straight line, polynomial, a built-in function, a formula or an
    /// Origin .FDF function. Prints the parameters, statistics and ANOVA table.
    Fit(FitArgs),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum WeightKind {
    /// Every point counts the same.
    None,
    /// 1 / error² from the Y-error column.
    Instrumental,
    /// 1 / abs(y); a y of exactly 0 is refused.
    Statistical,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum BandKind {
    /// No bands.
    None,
    /// The band that holds the true curve with the chosen probability.
    Confidence,
    /// The wider band that holds a new observation with the chosen probability.
    Prediction,
    /// Both bands.
    Both,
}

#[derive(Args, Debug)]
pub struct FitArgs {
    /// A data file, a .sheaf project or an Origin .opj project. Not needed with --list.
    #[arg(required_unless_present = "list")]
    pub input: Option<PathBuf>,
    /// List the built-in fit functions and exit.
    #[arg(long)]
    pub list: bool,
    /// The model: linear, poly:ORDER (for example poly:3), or the name of a built-in function
    /// such as Gauss, Lorentz, ExpDec1, Boltzmann (see --list).
    #[arg(short, long, default_value = "linear", conflicts_with_all = ["formula", "fdf"])]
    pub model: String,
    /// A formula to fit instead of --model, for example "y0 + A*exp(-x/t)"; give its parameters
    /// with --params.
    #[arg(long, requires = "params")]
    pub formula: Option<String>,
    /// Parameter names of --formula, comma-separated.
    #[arg(long, value_delimiter = ',', requires = "formula")]
    pub params: Vec<String>,
    /// Fit the function defined in this Origin .FDF file.
    #[arg(long, value_name = "FILE")]
    pub fdf: Option<PathBuf>,
    /// Starting values, for example --init xc=0.45,w=0.05.
    #[arg(long, value_delimiter = ',', value_name = "NAME=VALUE", allow_hyphen_values = true)]
    pub init: Vec<String>,
    /// Parameters held at a value, for example --fix Intercept=0.
    #[arg(long, value_delimiter = ',', value_name = "NAME=VALUE", allow_hyphen_values = true)]
    pub fix: Vec<String>,
    /// Bounds, for example --bounds w=0:,A=0:100 (leave a side empty for no bound).
    #[arg(long, value_delimiter = ',', value_name = "NAME=LOW:HIGH", allow_hyphen_values = true)]
    pub bounds: Vec<String>,
    /// How points are weighted. Instrumental weighting uses the Y-error column given with
    /// --yerr, or the one that belongs to the Y column in a project.
    #[arg(long, value_enum, default_value_t = WeightKind::None)]
    pub weight: WeightKind,
    /// The Y-error column(s) for --weight instrumental, by short or long name: one for all fitted
    /// columns, or one per --y column in the same order.
    #[arg(long, value_delimiter = ',', value_name = "COLUMNS")]
    pub yerr: Vec<String>,
    /// Confidence level of the parameter limits.
    #[arg(long, default_value_t = 0.95)]
    pub confidence: f64,
    /// Bands around the fitted curve, written to the FitCurve sheet and drawn in the figure.
    #[arg(long, value_enum, default_value_t = BandKind::None)]
    pub bands: BandKind,
    /// Confidence level of the bands in percent (95 means 95 %).
    #[arg(long, default_value_t = 95.0, value_name = "PERCENT")]
    pub level: f64,
    /// Most iterations of a nonlinear fit.
    #[arg(long, default_value_t = 400, value_name = "N")]
    pub max_iterations: usize,
    /// Convergence tolerance of a nonlinear fit.
    #[arg(long, default_value_t = 1e-9)]
    pub tolerance: f64,
    /// Y columns to fit, comma-separated short or long names; each is fitted on its own
    /// (default: the columns a plot of the sheet would show).
    #[arg(short, long, value_delimiter = ',')]
    pub y: Vec<String>,
    /// For a project input: the workbook (default: the first one).
    #[arg(long)]
    pub book: Option<String>,
    /// The sheet, by name (default: the workbook's active sheet).
    #[arg(long)]
    pub sheet: Option<String>,
    /// Fit only these worksheet rows, counted from 1, for example 120:480.
    #[arg(long, value_name = "FROM:TO", conflicts_with = "x_range")]
    pub rows: Option<String>,
    /// Fit only the first unbroken run of rows whose X value lies in this interval (write
    /// negative limits as --x-range=-0.5:0.1).
    #[arg(long, value_name = "FROM:TO", allow_hyphen_values = true)]
    pub x_range: Option<String>,
    /// Draw the data with the fitted curve and write the figure (.png, .svg or .pdf). With
    /// several columns, only the first one is drawn.
    #[arg(long, value_name = "FIGURE")]
    pub plot: Option<PathBuf>,
    /// Resolution of a PNG figure in dots per inch.
    #[arg(long, default_value_t = 300.0)]
    pub dpi: f64,
    /// Save the project with the results to this file: .opj (also when no extension is given, with a
    /// companion .opj.sheaf if the .opj cannot hold everything) or .sheaf.
    #[arg(long, value_name = "PROJECT")]
    pub save: Option<PathBuf>,
    /// Write the parameter sheet as delimited text (.tsv or .txt gives tabs, anything else commas).
    /// With several columns, only the first one's. The statistics and curve sheets are kept with
    /// --save and can be written with export-data.
    #[arg(long, value_name = "FILE")]
    pub export: Option<PathBuf>,
    #[command(flatten)]
    pub read: ReadOptions,
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
    /// CH Instruments: do not add the sheet with one group of columns per charge or discharge
    /// segment to a chronopotentiometry file.
    #[arg(long)]
    pub no_split_segments: bool,
    /// CH Instruments: keep the times as the file prints them (4 significant digits) instead of
    /// recomputing them from the sampling interval in the header.
    #[arg(long)]
    pub keep_file_times: bool,
    /// Delimited text: the separator. One of comma, tab, semicolon, space, or a single character.
    #[arg(long, value_name = "SEP")]
    pub delimiter: Option<String>,
    /// Delimited text: number of lines before the first data row (default: detect).
    #[arg(long, value_name = "N")]
    pub header_lines: Option<usize>,
    /// Delimited text: numbers use a comma as the decimal separator.
    #[arg(long)]
    pub decimal_comma: bool,
    /// Delimited text: keep date and time columns as text instead of reading them as date numbers.
    #[arg(long)]
    pub dates_as_text: bool,
    /// Delimited text: read 11/12/2026 as 11 December (default: 12 November) when the file does not decide.
    #[arg(long)]
    pub day_first: bool,
    /// Delimited text: the digit-grouping character of numbers.
    #[arg(long, value_enum, default_value_t = ThousandsArg::Auto)]
    pub thousands: ThousandsArg,
    /// Delimited text: line (1-based) holding the column long names (0: none; default: detect).
    #[arg(long, value_name = "N")]
    pub long_name_line: Option<usize>,
    /// Delimited text: line holding the units (0: none).
    #[arg(long, value_name = "N")]
    pub units_line: Option<usize>,
    /// Delimited text: line holding the comments (0: none).
    #[arg(long, value_name = "N")]
    pub comments_line: Option<usize>,
    /// Delimited text: header lines that become named parameter rows (for example the wavelengths
    /// of an ICP-OES export), as a comma-separated list. The first cell of the line is the name.
    #[arg(long, value_name = "N,N", value_delimiter = ',')]
    pub param_lines: Vec<usize>,
    /// Delimited text: a file with several blocks of data separated by blank lines gets one sheet per block.
    #[arg(long)]
    pub blocks_as_sheets: bool,
}

#[derive(Args, Debug)]
pub struct InfoArgs {
    /// A data file, a .sheaf project or an Origin .opj project.
    pub path: PathBuf,
    #[command(flatten)]
    pub read: ReadOptions,
}

#[derive(Args, Debug)]
pub struct ImportArgs {
    /// Data files, folders or patterns to import. A folder gives its data files (.txt, .csv, .tsv,
    /// .dat, .asc, .prn, .tab, .bin); a name with * or ? in it (for example "exports/icp_*.csv")
    /// gives the matching files, in natural order (icp_2 before icp_10). An Origin .opj project adds
    /// all its workbooks.
    #[arg(required = true)]
    pub files: Vec<PathBuf>,
    /// The project file to write: .opj (the default, also when no extension is given) or .sheaf.
    /// An .opj that cannot hold everything gets a companion .opj.sheaf.
    #[arg(short, long, visible_alias = "into")]
    pub output: PathBuf,
    /// Add the workbooks to the existing project at --output instead of starting a new one.
    #[arg(long)]
    pub append: bool,
    /// Also add one graph per imported workbook, in the template the importer suggests.
    #[arg(long)]
    pub plot: bool,
    /// Where the files go: one book each (books), one book with a sheet per file (sheets), one sheet
    /// with the columns of all files side by side (columns), or one sheet with the rows of all files
    /// one under the other (rows).
    #[arg(long, value_enum, default_value_t = ImportLayout::Books)]
    pub mode: ImportLayout,
    /// Long name of the shared book when --mode is not books (default: the name of the folder).
    #[arg(long, value_name = "TEXT")]
    pub book_name: Option<String>,
    /// Write the file name into the comment row and the SourceFile parameter row of every column
    /// (always done with --mode columns).
    #[arg(long)]
    pub file_name_comments: bool,
    #[command(flatten)]
    pub read: ReadOptions,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum AxisColors {
    /// Each Y axis (line, ticks and tick labels) in the color of its first curve.
    Plot,
    /// Every axis in its own color (black in the built-in templates).
    Fixed,
}

/// The layout of a multi-file import (`sheaf import --mode`).
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum ImportLayout {
    /// One workbook per file.
    Books,
    /// One workbook, one sheet per file, named after the file.
    Sheets,
    /// One sheet; the columns of the files side by side.
    Columns,
    /// One sheet; the rows of the files one under the other.
    Rows,
}

/// The digit-grouping character of numbers (`sheaf import --thousands`).
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum ThousandsArg {
    /// Use one when a number in the file shows it unambiguously.
    Auto,
    /// None: 1,234 is not a number.
    None,
    /// 1,234.5
    Comma,
    /// 1.234,5 (the decimal separator is then a comma)
    Dot,
    /// 1 234,5 (the decimal separator is then a comma)
    Space,
    /// 1'234.5
    Apostrophe,
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
    /// Title of the right Y axis (the graph needs one: see --y2 and the double-y template).
    #[arg(long, value_name = "TEXT")]
    pub y2_title: Option<String>,
    /// Lower limit of the right Y axis.
    #[arg(long, allow_hyphen_values = true)]
    pub y2_min: Option<f64>,
    /// Upper limit of the right Y axis.
    #[arg(long, allow_hyphen_values = true)]
    pub y2_max: Option<f64>,
    /// Distance between major ticks of the right Y axis.
    #[arg(long)]
    pub y2_step: Option<f64>,
    /// Draw each Y axis in the color of its first curve (plot) or in its own color (fixed).
    /// Without this option the template decides: double-y follows the curves.
    #[arg(long, value_enum, value_name = "MODE")]
    pub axis_colors: Option<AxisColors>,
    /// Page width in inches.
    #[arg(long, value_name = "INCHES")]
    pub width: Option<f64>,
    /// Page height in inches.
    #[arg(long, value_name = "INCHES")]
    pub height: Option<f64>,
    /// Line width of the data curves in points.
    #[arg(long, value_name = "PT")]
    pub line_width: Option<f64>,
    /// Curve colors in legend order (left axis first, then right axis), comma-separated, as
    /// #RRGGBB or basic names.
    #[arg(long, value_delimiter = ',', value_name = "COLORS")]
    pub colors: Vec<String>,
    /// Legend labels in legend order (left axis first, then right axis), comma-separated.
    #[arg(long, value_delimiter = ',', value_name = "LABELS")]
    pub labels: Vec<String>,
    /// Hide the legend.
    #[arg(long)]
    pub no_legend: bool,
    /// Move the curves apart: none; auto (each curve above the one before it with a gap of 10 %
    /// of the tallest curve); auto:GAP (gap in percent); step:STEP (curve k moved up by k * STEP
    /// axis units; write a negative step as --stack=step:-500).
    #[arg(long, value_name = "MODE", conflicts_with_all = ["offsets", "waterfall"])]
    pub stack: Option<String>,
    /// Move each curve up by its own amount in axis units, in plot order, for example
    /// --offsets=0,1000,2000 (curves after the list keep their offsets).
    #[arg(
        long,
        value_delimiter = ',',
        value_name = "Y1,Y2,...",
        allow_hyphen_values = true,
        conflicts_with = "waterfall"
    )]
    pub offsets: Vec<f64>,
    /// Draw a waterfall: curve k moved right by k * DX and up by k * DY percent of the front frame
    /// (each from -100 to 100), for example --waterfall 5,10.
    #[arg(long, value_delimiter = ',', value_name = "DX,DY", allow_hyphen_values = true)]
    pub waterfall: Vec<f64>,
    /// Hide the Y tick labels (stacked curves: the values on the axis are true only for the first
    /// curve).
    #[arg(long)]
    pub no_y_tick_labels: bool,
    /// Draw no Y ticks.
    #[arg(long)]
    pub no_y_ticks: bool,
    /// Tick labels of the X axis: auto (the cells of the first curve's X column when it holds text,
    /// such as sample names, otherwise numbers), numbers, or the short or long name of a column whose
    /// cells label the ticks (looked up in the sheet of the first curve).
    #[arg(long, value_name = "auto|numbers|COLUMN")]
    pub x_labels: Option<String>,
    /// Turn the X tick labels by this many degrees counter-clockwise, from -90 to 90 (write a
    /// negative angle as --x-label-angle=-45).
    #[arg(long, value_name = "DEG", allow_hyphen_values = true)]
    pub x_label_angle: Option<f64>,
    /// Column charts: the gap between neighbouring groups of bars, in percent of the distance
    /// between X positions, from 0 to 100 (default 20).
    #[arg(long, value_name = "PCT")]
    pub bar_gap: Option<f64>,
    /// Column charts: how much neighbouring bars of a group cover each other, in percent of a bar's
    /// width, from -100 to 100 (default 0; a negative value leaves a gap, written as
    /// --bar-overlap=-20).
    #[arg(long, value_name = "PCT", allow_hyphen_values = true)]
    pub bar_overlap: Option<f64>,
    /// Add a free text label to the first layer: "TEXT@X,Y", where X and Y are fractions of the
    /// layer frame (0,0 is its top left corner, 1,1 its bottom right; the label's text starts
    /// there). The text may use Origin's escapes, for example --label "\b(Zn-LMO)@0.85,0.1".
    /// Repeat the option for more labels.
    #[arg(long, value_name = "TEXT@X,Y")]
    pub label: Vec<String>,
}

#[derive(Args, Debug)]
pub struct PlotArgs {
    /// A data file, a .sheaf project or an Origin .opj project. Several data files are overlaid in one figure.
    #[arg(required = true)]
    pub inputs: Vec<PathBuf>,
    /// The figure to write: .png, .svg or .pdf.
    #[arg(short, long)]
    pub output: PathBuf,
    /// Resolution of PNG output in dots per inch.
    #[arg(long, default_value_t = 300.0)]
    pub dpi: f64,
    /// Width of the exported picture, with a unit: in, cm, mm, pt or px (for example 3.5in or
    /// 8.5cm; px counts at --dpi). The whole page is scaled to it, text and lines included, as
    /// Origin's export image size does; --width instead changes the page and keeps the text size.
    #[arg(long, value_name = "LENGTH")]
    pub image_width: Option<String>,
    /// Height of the exported picture, with a unit as for --image-width. With both, the picture is
    /// scaled to fit inside both.
    #[arg(long, value_name = "LENGTH")]
    pub image_height: Option<String>,
    /// Figure template: line, scatter, line-symbol, column, cv, double-y, stacked or waterfall
    /// (default: what the importer suggests). column draws one group of bars per row, the Y columns
    /// side by side, with sample names from a text X column under the bars; double-y puts the 1st,
    /// 3rd, ... Y column on the left axis and the 2nd, 4th, ... on a right axis; stacked moves each
    /// curve above the one before it.
    #[arg(short, long)]
    pub template: Option<String>,
    /// Y columns to plot, comma-separated short or long names (default: all Y columns).
    #[arg(short, long, value_delimiter = ',')]
    pub y: Vec<String>,
    /// Y columns to plot on a second Y axis on the right, comma-separated short or long names.
    /// The left axis gets the -y columns (default: the other Y columns).
    #[arg(long, value_delimiter = ',', value_name = "COLUMNS", conflicts_with = "graph")]
    pub y2: Vec<String>,
    /// For a project input: the workbook to plot from (default: the first one).
    #[arg(long)]
    pub book: Option<String>,
    /// The sheet to plot from, by name (default: the workbook's active sheet).
    #[arg(long)]
    pub sheet: Option<String>,
    /// For a project input: render this existing graph instead of making a new one.
    #[arg(long)]
    pub graph: Option<String>,
    /// Also save the resulting project (data plus the new graph) to this file: .opj (the default,
    /// with a companion .opj.sheaf if needed) or .sheaf.
    #[arg(long, value_name = "PROJECT")]
    pub save: Option<PathBuf>,
    /// Make a graph of several panels: stack (one column of panels that share the X axis, the
    /// first at the bottom, as Origin's Stack command), or ROWSxCOLUMNS such as 2x1 (Origin's
    /// Vertical 2 Panel), 1x2 (Horizontal 2 Panel) or 2x2 (4 Panel). Each Y column gets its own
    /// panel; with several files, each file does. Style options act on every panel; touching
    /// panels with the same Y title (for example from --y-title) show it once.
    #[arg(long, value_name = "stack|RxC", conflicts_with_all = ["graph", "y2"])]
    pub panels: Option<String>,
    /// The Y columns of one panel, comma-separated short or long names. Give it once per panel, in
    /// panel order, for example --panel-y B,C --panel-y D. With several files, each panel gets
    /// these columns of every file.
    #[arg(long = "panel-y", value_name = "COLUMNS", requires = "panels", conflicts_with_all = ["y", "per_panel"])]
    pub panel_y: Vec<String>,
    /// How many curves go into each panel, in order, for example --per-panel 2,1,1.
    #[arg(long, value_delimiter = ',', value_name = "N,N,...", requires = "panels")]
    pub per_panel: Vec<usize>,
    /// The gap between panels in percent of the page, from 0 to 50: one number for both
    /// directions, or H,V (default: 0 for a stack; for a grid, the room the template leaves around
    /// its frame).
    #[arg(long, value_delimiter = ',', value_name = "PCT|H,V", allow_hyphen_values = true, requires = "panels")]
    pub panel_gap: Vec<f64>,
    /// The margins around the panels in percent of the page, from 0 to 90: L,R,T,B (default: 15,
    /// 10, 10, 10 for a stack; for a grid, the template's margins).
    #[arg(long, value_delimiter = ',', value_name = "L,R,T,B", allow_hyphen_values = true, requires = "panels")]
    pub panel_margins: Vec<f64>,
    /// Where the first panel goes (default: at the bottom for a stack and a single column, at the
    /// top left otherwise).
    #[arg(long, value_enum, requires = "panels")]
    pub panel_order: Option<PanelOrderArg>,
    /// The axes the panels share (default: x for a stack, none for a grid).
    #[arg(long, value_enum, requires = "panels")]
    pub panel_link: Option<PanelLinkArg>,
    #[command(flatten)]
    pub style: StyleArgs,
    #[command(flatten)]
    pub read: ReadOptions,
}

/// Where the first panel goes (`sheaf plot --panel-order`).
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum PanelOrderArg {
    /// Row by row from the bottom left: the first panel is at the bottom.
    BottomUp,
    /// Row by row from the top left.
    TopDown,
}

/// The axes the panels share (`sheaf plot --panel-link`).
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum PanelLinkArg {
    /// One X axis for every panel.
    X,
    /// One Y axis for every panel.
    Y,
    /// Both.
    Xy,
    /// Every panel has its own axes.
    None,
}

#[derive(Args, Debug)]
pub struct ExportArgs {
    /// A .sheaf project or an Origin .opj project (its graphs are read). For an output ending in
    /// .opj a data file (CH Instruments text, CSV or other delimited text) is also accepted.
    pub project: PathBuf,
    /// Name of the graph (default: the only graph of the project). Not used for .opj output.
    #[arg(long)]
    pub graph: Option<String>,
    /// What to write: a figure (.png, .svg or .pdf) or, with .opj, the whole project as an Origin
    /// 2016 project file (workbooks and graphs; what the file cannot hold is listed as warnings).
    #[arg(short, long)]
    pub output: PathBuf,
    /// Resolution of PNG output in dots per inch.
    #[arg(long, default_value_t = 300.0)]
    pub dpi: f64,
    /// Width of the exported picture, with a unit: in, cm, mm, pt or px (for example 3.5in or
    /// 8.5cm; px counts at --dpi). The whole page is scaled to it, text and lines included, as
    /// Origin's export image size does; --width instead changes the page and keeps the text size.
    #[arg(long, value_name = "LENGTH")]
    pub image_width: Option<String>,
    /// Height of the exported picture, with a unit as for --image-width. With both, the picture is
    /// scaled to fit inside both.
    #[arg(long, value_name = "LENGTH")]
    pub image_height: Option<String>,
    #[command(flatten)]
    pub style: StyleArgs,
    #[command(flatten)]
    pub read: ReadOptions,
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

#[derive(Args, Debug)]
pub struct AnalyzeArgs {
    #[command(subcommand)]
    pub tool: AnalyzeTool,
}

/// Which data an analysis tool works on and where the results go.
#[derive(Args, Debug, Clone)]
pub struct TargetArgs {
    /// A data file, a .sheaf project or an Origin .opj project.
    pub input: PathBuf,
    /// Y columns to work on, comma-separated short or long names (default: the columns a plot of
    /// the sheet would show).
    #[arg(short, long, value_delimiter = ',')]
    pub y: Vec<String>,
    /// For a project input: the workbook (default: the first one).
    #[arg(long)]
    pub book: Option<String>,
    /// The sheet, by name (default: the workbook's active sheet).
    #[arg(long)]
    pub sheet: Option<String>,
    /// Use only these worksheet rows, counted from 1, for example 120:480.
    #[arg(long, value_name = "FROM:TO", conflicts_with = "x_range")]
    pub rows: Option<String>,
    /// Use only the first unbroken run of rows whose X value lies in this interval, for example
    /// 0.2:0.6 (write negative limits as --x-range=-0.5:0.1).
    #[arg(long, value_name = "FROM:TO", allow_hyphen_values = true)]
    pub x_range: Option<String>,
    /// Save the project with the results to this file: .opj (also when no extension is given, with a
    /// companion .opj.sheaf if the .opj cannot hold everything) or .sheaf.
    #[arg(long, value_name = "PROJECT")]
    pub save: Option<PathBuf>,
    /// Write the sheet that holds the results as delimited text (.tsv or .txt gives tabs, anything
    /// else commas).
    #[arg(long, value_name = "FILE")]
    pub export: Option<PathBuf>,
    #[command(flatten)]
    pub read: ReadOptions,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum TTestKindArg {
    /// Each column against --mean.
    OneSample,
    /// Two columns against each other.
    TwoSample,
    /// Two columns, rows paired.
    Paired,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum TailArg {
    /// Differs from (either direction).
    Two,
    /// Larger than.
    Upper,
    /// Smaller than.
    Lower,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum SignRankKindArg {
    /// Each column against --median.
    OneSample,
    /// Two columns, rows paired.
    Paired,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum ComparisonArg {
    Tukey,
    Bonferroni,
    Sidak,
    /// Fisher's least significant difference.
    Lsd,
    Scheffe,
    /// No means comparison.
    None,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum SmoothKind {
    /// Savitzky-Golay: a local polynomial fit.
    SavitzkyGolay,
    /// Moving average.
    Average,
    /// Moving average with weights that fall off towards the window ends.
    WeightedAverage,
    /// A percentile of the window (50 is a median filter).
    Percentile,
    /// FFT low-pass filter; `--points` sets the cutoff frequency 1 / (2 dx points), larger values
    /// smooth more (it is not a window length).
    Fft,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum BoundaryKind {
    None,
    Reflect,
    Repeat,
    Periodic,
    Extrapolate,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum NormalizeKind {
    /// Divide by the largest value.
    Max,
    /// Map to the interval 0 to 1.
    Range,
    /// Map to the interval 0 to 100.
    Range100,
    /// Subtract the mean and divide by the standard deviation.
    ZScore,
    Min,
    Mean,
    Median,
    /// Divide by the standard deviation.
    Sd,
    /// Divide by the square root of the sum of squares.
    Norm,
    Mode,
    Sum,
    /// Divide by the number given with --value.
    Divide,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum MathKind {
    Add,
    Subtract,
    Multiply,
    Divide,
    /// The curve raised to the operand.
    Power,
}

/// The second operand of curve math: a number, or a reference column.
#[derive(Args, Debug, Clone)]
pub struct OperandArgs {
    /// A constant operand.
    #[arg(long, allow_hyphen_values = true, conflicts_with = "reference")]
    pub value: Option<f64>,
    /// The Y column of the reference curve (short or long name); its X column is the nearest
    /// X column to its left. By default it is searched in the same sheet.
    #[arg(long = "ref", value_name = "COLUMN")]
    pub reference: Option<String>,
    /// Take the reference from this data file (or project) instead.
    #[arg(long, value_name = "FILE", requires = "reference")]
    pub ref_file: Option<PathBuf>,
    /// The workbook that holds the reference (a project input, or the --ref-file project).
    #[arg(long, requires = "reference")]
    pub ref_book: Option<String>,
    /// The sheet that holds the reference.
    #[arg(long, requires = "reference")]
    pub ref_sheet: Option<String>,
    /// Only the X range that data and reference both cover, at the merged X values of both; the
    /// result goes to a new sheet. Without it every data row gets a value, the reference being
    /// continued as a straight line outside its range.
    #[arg(long, requires = "reference")]
    pub common: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum InterpKind {
    Linear,
    Spline,
    Akima,
    /// Cubic B-spline. With `--smoothing` above 0 it is Origin's smoothing spline; with 0 it is
    /// the interpolating spline, which is not what Origin computes for its default factor 0.
    #[value(name = "bspline")]
    BSpline,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum PeakKind {
    Both,
    Positive,
    Negative,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum PeakMethodKind {
    LocalMax,
    Window,
    FirstDerivative,
    SecondDerivative,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum PeakBaselineKind {
    None,
    EndPoints,
    Minima,
    Anchors,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum PeakLimitKind {
    Baseline,
    HalfHeight,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum PeakFitKind {
    Gauss,
    Lorentz,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum PeakFitBaselineKind {
    Constant,
    Linear,
}

#[derive(Subcommand, Debug)]
pub enum AnalyzeTool {
    /// Area under each curve, with peak position, height and width; adds the running integral.
    Integrate {
        #[command(flatten)]
        target: TargetArgs,
        /// Subtract the straight line through the first and last point of the row range first.
        #[arg(long)]
        baseline: bool,
        /// Integrate the absolute value of the curve instead of the signed curve.
        #[arg(long)]
        absolute: bool,
        /// Do not add a column with the running integral.
        #[arg(long)]
        no_cumulative: bool,
        /// Integrate several X intervals at once, for example 0.2:0.5,0.6:0.9 (write negative
        /// limits as --ranges=-0.5:0.1,0.2:0.4). One result row per interval and column, with
        /// the signed and the absolute area, peak position and height and the half-height width.
        /// Each interval is the first unbroken run of rows inside it; for the return sweep of a
        /// cyclic voltammogram use --row-ranges. The baseline option applies to each interval.
        #[arg(long, value_delimiter = ',', value_name = "FROM:TO,...", allow_hyphen_values = true)]
        ranges: Vec<String>,
        /// Like --ranges, but worksheet rows counted from 1, for example 1:200,201:400 (the
        /// charge of each cycle).
        #[arg(long, value_delimiter = ',', value_name = "FROM:TO,...")]
        row_ranges: Vec<String>,
    },
    /// Subtract a reference curve (a blank scan, a background) from each curve. The reference
    /// is interpolated linearly onto the X values of the data.
    SubtractRef {
        #[command(flatten)]
        target: TargetArgs,
        #[command(flatten)]
        operand: OperandArgs,
    },
    /// Combine each curve with a constant or a reference curve: add, subtract, multiply,
    /// divide or raise to a power (Origin's Simple Math).
    Math {
        #[command(flatten)]
        target: TargetArgs,
        #[arg(long, value_enum, default_value_t = MathKind::Subtract)]
        op: MathKind,
        #[command(flatten)]
        operand: OperandArgs,
    },
    /// Derivative of each curve with respect to its X column.
    Differentiate {
        #[command(flatten)]
        target: TargetArgs,
        /// Derivative order, 1 to 9.
        #[arg(long, default_value_t = 1)]
        order: usize,
        /// Smooth while differentiating, with a local polynomial fit of --poly over --points.
        #[arg(long)]
        smooth: bool,
        #[arg(long, default_value_t = 2)]
        poly: usize,
        #[arg(long, default_value_t = 20)]
        points: usize,
    },
    /// Smooth each curve.
    Smooth {
        #[command(flatten)]
        target: TargetArgs,
        #[arg(long, value_enum, default_value_t = SmoothKind::SavitzkyGolay)]
        method: SmoothKind,
        /// Points of the window.
        #[arg(long, default_value_t = 5)]
        points: usize,
        /// Savitzky-Golay: polynomial order.
        #[arg(long, default_value_t = 2)]
        poly: usize,
        /// Percentile filter: the percentile, 0 to 100.
        #[arg(long, default_value_t = 50.0)]
        percent: f64,
        /// How the window is filled where it sticks out of the data.
        #[arg(long, value_enum, default_value_t = BoundaryKind::None)]
        boundary: BoundaryKind,
    },
    /// Normalize each curve.
    Normalize {
        #[command(flatten)]
        target: TargetArgs,
        #[arg(long, value_enum, default_value_t = NormalizeKind::Max)]
        method: NormalizeKind,
        /// The divisor for --method divide.
        #[arg(long, allow_hyphen_values = true)]
        value: Option<f64>,
    },
    /// Subtract a baseline from each curve (default: its smallest value).
    Baseline {
        #[command(flatten)]
        target: TargetArgs,
        /// Subtract this constant.
        #[arg(long, allow_hyphen_values = true, group = "kind")]
        constant: Option<f64>,
        /// Subtract the straight line through the first and last point of the row range.
        #[arg(long, group = "kind")]
        end_points: bool,
        /// Subtract the straight line through two points, written as X1,Y1,X2,Y2.
        #[arg(long, value_delimiter = ',', allow_hyphen_values = true, value_name = "X1,Y1,X2,Y2", group = "kind")]
        line: Vec<f64>,
    },
    /// Interpolate each curve onto equally spaced X values; the result goes to a new sheet.
    Interpolate {
        #[command(flatten)]
        target: TargetArgs,
        #[arg(long, value_enum, default_value_t = InterpKind::Linear)]
        method: InterpKind,
        /// Number of points of the result.
        #[arg(long, default_value_t = 100)]
        points: usize,
        /// First X of the result (default: the smallest X of the data).
        #[arg(long, allow_hyphen_values = true)]
        x_min: Option<f64>,
        /// Last X of the result (default: the largest X of the data).
        #[arg(long, allow_hyphen_values = true)]
        x_max: Option<f64>,
        /// B-spline smoothing factor: the allowed sum of squared residuals (matches Origin's `sf`).
        /// 0 interpolates the points; Origin's automatic factor for 0 is not reproduced.
        #[arg(long, default_value_t = 0.0)]
        smoothing: f64,
    },
    /// Find the peaks of each curve; positions and heights go to a new sheet.
    Peaks {
        #[command(flatten)]
        target: TargetArgs,
        #[arg(long, value_enum, default_value_t = PeakKind::Both)]
        direction: PeakKind,
        /// A peak must stand out against this many neighbours on each side.
        #[arg(long, default_value_t = 2)]
        points: usize,
        /// Keep peaks at least this high (default: 20 % of the largest magnitude).
        #[arg(long, group = "keep")]
        min_height: Option<f64>,
        /// Keep only this many of the highest peaks.
        #[arg(long, group = "keep")]
        count: Option<usize>,
        /// Keep every local extremum.
        #[arg(long, group = "keep")]
        all: bool,
        /// How peaks are located. This and every option below switch to the peak chain
        /// (baseline, search, per-peak limits), which also writes a baseline column.
        #[arg(long, value_enum)]
        method: Option<PeakMethodKind>,
        /// Smooth the curve over this many points (Savitzky-Golay) before searching.
        #[arg(long, default_value_t = 0)]
        smooth: usize,
        /// Drop peaks narrower than this full width at half height (X units).
        #[arg(long, default_value_t = 0.0)]
        min_width: f64,
        /// Baseline to subtract before the search: none, end-points (straight line through the
        /// first and last point), minima (the lowest point of each of --baseline-points stretches),
        /// or anchors (--anchors).
        #[arg(long, value_enum)]
        baseline: Option<PeakBaselineKind>,
        /// Number of stretches (anchor points) of the minima baseline.
        #[arg(long, default_value_t = 5)]
        baseline_points: usize,
        /// Anchor points of the anchors baseline, written X1:Y1,X2:Y2,... (linear interpolation).
        #[arg(long)]
        anchors: Option<String>,
        /// Where a peak starts and ends: baseline (where the corrected curve reaches the baseline
        /// or the valley to the next peak) or half-height.
        #[arg(long, value_enum)]
        limits: Option<PeakLimitKind>,
        /// Add area, area percent, FWHM and limits of every peak.
        #[arg(long)]
        integrate: bool,
        /// Fit all peaks together (sum of peaks, started at the found peaks) and report each one.
        #[arg(long, value_enum)]
        fit: Option<PeakFitKind>,
        /// Baseline of the multi-peak fit.
        #[arg(long, value_enum, default_value_t = PeakFitBaselineKind::Linear)]
        fit_baseline: PeakFitBaselineKind,
    },
    /// Descriptive statistics of each column.
    Stats {
        #[command(flatten)]
        target: TargetArgs,
    },
    /// t-test: each column against a hypothesized mean (one-sample), two columns against each other
    /// (two-sample, equal variances unless --unequal) or two columns as pairs. The result goes to a
    /// new sheet; the columns are chosen with --y.
    Ttest {
        #[command(flatten)]
        target: TargetArgs,
        #[arg(long, value_enum, default_value_t = TTestKindArg::TwoSample)]
        test: TTestKindArg,
        /// One-sample test: the hypothesized mean.
        #[arg(long, default_value_t = 0.0, allow_hyphen_values = true)]
        mean: f64,
        /// Two-sample and paired test: the hypothesized difference of the means (first minus second).
        #[arg(long, default_value_t = 0.0, allow_hyphen_values = true)]
        difference: f64,
        /// Two-sample test: do not assume equal variances (Welch's test).
        #[arg(long)]
        unequal: bool,
        #[arg(long, value_enum, default_value_t = TailArg::Two)]
        tail: TailArg,
        /// Significance level.
        #[arg(long, default_value_t = 0.05)]
        alpha: f64,
    },
    /// One-way ANOVA: every chosen column is a group. The result (group statistics, ANOVA table,
    /// means comparison of all pairs) goes to a new sheet.
    Anova {
        #[command(flatten)]
        target: TargetArgs,
        #[arg(long, value_enum, default_value_t = ComparisonArg::Tukey)]
        comparison: ComparisonArg,
        /// Significance level.
        #[arg(long, default_value_t = 0.05)]
        alpha: f64,
    },
    /// Two-way ANOVA. Indexed data (default): --y FACTOR_A,FACTOR_B,DATA names three columns, the first two
    /// holding the levels (text or numbers) of each row. With --raw every --y column is one cell of the
    /// design and --levels-a / --levels-b give its levels. Sums of squares are Origin's (type III for
    /// unequal cell sizes). The tables go to a new sheet.
    Anova2 {
        #[command(flatten)]
        target: TargetArgs,
        /// Every chosen column is one cell of the design (Origin's "Raw Data").
        #[arg(long, requires_all = ["levels_a", "levels_b"])]
        raw: bool,
        /// Raw data: the level of factor A of each --y column, comma-separated.
        #[arg(long, value_delimiter = ',')]
        levels_a: Vec<String>,
        /// Raw data: the level of factor B of each --y column, comma-separated.
        #[arg(long, value_delimiter = ',')]
        levels_b: Vec<String>,
        /// Leave the interaction of the two factors out of the model.
        #[arg(long)]
        no_interaction: bool,
        #[arg(long, value_enum, default_value_t = ComparisonArg::Tukey)]
        comparison: ComparisonArg,
        /// Significance level.
        #[arg(long, default_value_t = 0.05)]
        alpha: f64,
    },
    /// Mann-Whitney test of two columns (two independent samples). The alternative is about the first
    /// column.
    Mannwhitney {
        #[command(flatten)]
        target: TargetArgs,
        #[arg(long, value_enum, default_value_t = TailArg::Two)]
        tail: TailArg,
        /// Exact p-value (permutation distribution of the rank sum) instead of the normal approximation.
        #[arg(long)]
        exact: bool,
        /// Significance level.
        #[arg(long, default_value_t = 0.05)]
        alpha: f64,
    },
    /// Wilcoxon signed rank test: each column against a hypothesized median (one-sample), or two columns
    /// as pairs.
    Signrank {
        #[command(flatten)]
        target: TargetArgs,
        #[arg(long, value_enum, default_value_t = SignRankKindArg::OneSample)]
        test: SignRankKindArg,
        /// One-sample test: the hypothesized median.
        #[arg(long, default_value_t = 0.0, allow_hyphen_values = true)]
        median: f64,
        #[arg(long, value_enum, default_value_t = TailArg::Two)]
        tail: TailArg,
        /// Decide with the asymptotic p-value even where an exact one exists (up to 80 differences).
        #[arg(long)]
        asymptotic: bool,
        /// Significance level.
        #[arg(long, default_value_t = 0.05)]
        alpha: f64,
    },
    /// Kruskal-Wallis test: every chosen column is a group.
    Kruskal {
        #[command(flatten)]
        target: TargetArgs,
        /// Significance level.
        #[arg(long, default_value_t = 0.05)]
        alpha: f64,
    },
    /// Correlation coefficients (Pearson unless --no-pearson, optionally Spearman and Kendall) with p-values
    /// for every pair of the chosen columns, as a table of pairs and as matrices on a new sheet.
    Corr {
        #[command(flatten)]
        target: TargetArgs,
        #[arg(long)]
        spearman: bool,
        #[arg(long)]
        kendall: bool,
        /// Leave Pearson's coefficient out.
        #[arg(long)]
        no_pearson: bool,
        /// Leave out every row where any column has no number (default: pair by pair).
        #[arg(long)]
        listwise: bool,
        /// Level for the sentence that names the significant pairs.
        #[arg(long, default_value_t = 0.05)]
        alpha: f64,
    },
    /// Frequency counts (the numbers of a histogram) of each column, and a graph with a column plot
    /// of the counts. Without --bin-size and --bins the bin size and range are chosen automatically.
    Histogram {
        #[command(flatten)]
        target: TargetArgs,
        /// Width of a bin.
        #[arg(long, conflicts_with = "bins")]
        bin_size: Option<f64>,
        /// Number of bins over the range (needs --min and --max, or takes the data range).
        #[arg(long)]
        bins: Option<usize>,
        /// Start of the first bin.
        #[arg(long, allow_hyphen_values = true)]
        min: Option<f64>,
        /// End of the last bin.
        #[arg(long, allow_hyphen_values = true)]
        max: Option<f64>,
        /// Add a row for the values below the first bin.
        #[arg(long)]
        below: bool,
        /// Add a row for the values above the last bin.
        #[arg(long)]
        above: bool,
        /// Do not make a graph.
        #[arg(long)]
        no_graph: bool,
    },
    /// Normality tests (Shapiro-Wilk, Kolmogorov-Smirnov, Lilliefors, Anderson-Darling, D'Agostino-Pearson)
    /// of each column; the result goes to a new sheet.
    Normality {
        #[command(flatten)]
        target: TargetArgs,
        /// Significance level.
        #[arg(long, default_value_t = 0.05)]
        alpha: f64,
    },
    /// Sort the rows of the sheet by one column.
    Sort {
        #[command(flatten)]
        target: TargetArgs,
        /// The column to sort by (short or long name).
        #[arg(long)]
        by: String,
        #[arg(long)]
        descending: bool,
    },
    /// Keep the rows for which a condition holds, for example --where "col(B) > 0.5 && col(A) < 10".
    Filter {
        #[command(flatten)]
        target: TargetArgs,
        /// The condition, in the column-formula language.
        #[arg(long = "where", value_name = "CONDITION")]
        condition: String,
        /// Remove the other rows from the sheet itself instead of copying the kept rows to a new sheet.
        #[arg(long)]
        in_place: bool,
    },
}
