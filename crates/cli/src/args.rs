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
    /// 1 / y.
    Statistical,
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
    #[arg(long, value_delimiter = ',')]
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
    /// Save the project with the results to this .sheaf file.
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
    /// A data file, a .sheaf project or an Origin .opj project.
    pub path: PathBuf,
    #[command(flatten)]
    pub read: ReadOptions,
}

#[derive(Args, Debug)]
pub struct ImportArgs {
    /// Data files to import. Each becomes one workbook. An Origin .opj project adds all its workbooks.
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
    /// A data file, a .sheaf project or an Origin .opj project. Several data files are overlaid in one figure.
    #[arg(required = true)]
    pub inputs: Vec<PathBuf>,
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
    /// Save the project with the results to this .sheaf file.
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
    },
    /// Descriptive statistics of each column.
    Stats {
        #[command(flatten)]
        target: TargetArgs,
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
