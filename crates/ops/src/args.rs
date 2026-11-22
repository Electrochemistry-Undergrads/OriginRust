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
    /// Make a figure from a data file or a project and write it as PNG, TIFF, JPEG, BMP, GIF, TGA, PCX, SVG, PDF, EPS or EMF.
    Plot(Box<PlotArgs>),
    /// Print a graph using the saved Page Setup, or write a paper-sized vector PDF.
    Print(PrintArgs),
    /// Write a graph of a project as PNG, TIFF, JPEG, BMP, GIF, TGA, PCX, SVG, PDF, EPS or EMF, or
    /// the project (or a data file) as an Origin .opj project or an Excel .xlsx workbook.
    Export(Box<ExportArgs>),
    /// Write a worksheet of a project as delimited text, or as an Excel .xlsx workbook.
    ExportData(ExportDataArgs),
    /// Run an analysis tool on worksheet columns: integrate, differentiate, smooth, normalize,
    /// baseline, interpolate, peaks, stats, sort, filter.
    Analyze(AnalyzeArgs),
    /// Fit a model to a column: straight line, polynomial, a built-in function, a formula or an
    /// Origin .FDF function. Prints the parameters, statistics and ANOVA table.
    Fit(Box<FitArgs>),
    /// Simulate Curve: the values of a built-in fit function for typed parameters, at X values from a range
    /// or from a column. Writes X and Y as delimited text or an Excel workbook, or adds them to a project.
    Simulate(SimulateArgs),
    /// Simulate Surface: XYZ data of a surface fit function (the 22 of Origin's window) for typed parameters on a
    /// grid, with optional noise. Writes X, Y and Z as delimited text or an Excel workbook, or adds them to a project.
    SimulateSurface(SimulateSurfaceArgs),
    /// Nonlinear Implicit Curve Fit: a circle or an ellipse fitted to the points of two columns by orthogonal
    /// distance regression. Prints the parameters and statistics.
    FitImplicit(FitImplicitArgs),
    /// Compare Datasets: the same model fitted to two Y columns and compared (Sheaf's F-test of one parameter set
    /// for both). Prints the report.
    CompareDatasets(CompareDatasetsArgs),
    /// Merge Graph Windows: copy graphs of a project into the cells of one new graph (Origin's
    /// merge_graph defaults), then write it as a figure and/or save the project with it.
    MergeGraphs(MergeGraphsArgs),
    /// Matrices: make one, set its values by a formula, transpose, flip, rotate, expand or shrink
    /// it, convert between worksheets and matrices, and plot heat maps and contour plots (T192).
    Matrix(MatrixArgs),
    /// Batch Processing: one workbook per data file from a saved workbook or analysis template (its
    /// analysis runs again on each file's data), optionally a summary book that stacks one sheet of
    /// every book; saves the project and/or writes the summary as delimited text.
    Batch(BatchArgs),
}

/// `sheaf batch`: File > Batch Processing... without a window (T235; workbook-model 13p).
#[derive(Args, Debug)]
pub struct BatchArgs {
    /// Data files; each becomes one workbook made from the template.
    #[arg(required = true)]
    pub files: Vec<PathBuf>,
    /// The workbook or analysis template file (.sheafbooktemplate, saved by File > Save Template As...
    /// or Save Workbook As Analysis Template... on a workbook).
    #[arg(long, short = 't', value_name = "FILE")]
    pub template: PathBuf,
    /// Save the project with the books (and the summary) here: .sheaf, or .opj for Origin.
    #[arg(long, short = 'o', value_name = "FILE")]
    pub output: Option<PathBuf>,
    /// Add to the existing project at --output instead of starting a new one.
    #[arg(long, requires = "output")]
    pub append: bool,
    /// The template sheet whose rows from every file are stacked in a new summary book.
    #[arg(long, value_name = "SHEET")]
    pub summary: Option<String>,
    /// The name of the summary's first column, which holds each row's file name.
    #[arg(long, value_name = "NAME", default_value = "Dataset", requires = "summary")]
    pub identifier: String,
    /// Delete the books made from the files once the summary holds their rows.
    #[arg(long, requires = "summary")]
    pub delete_books: bool,
    /// Write the summary sheet as delimited text (CSV, or TSV/TXT by the extension).
    #[arg(long, value_name = "FILE", requires = "summary")]
    pub summary_csv: Option<PathBuf>,
    #[command(flatten)]
    pub read: ReadOptions,
}

/// `sheaf merge-graphs`: Graph > Merge Graph Windows without a window (graphing spec 15.11).
#[derive(Args, Debug)]
pub struct MergeGraphsArgs {
    /// A .sheaf project or an Origin .opj project with the graphs.
    pub project: PathBuf,
    /// A graph to merge, by name; repeat it for each graph, in the order the cells are filled (row by
    /// row from the top left). At least two.
    #[arg(long, value_name = "NAME", conflicts_with = "all_graphs")]
    pub graph: Vec<String>,
    /// Merge every graph of the project, in project order (Origin's "All in Project").
    #[arg(long)]
    pub all_graphs: bool,
    /// Number of rows (default: as Origin, ceil(sqrt(n)) for n graphs, or as few as the columns allow).
    #[arg(long)]
    pub rows: Option<usize>,
    /// Number of columns (default: as Origin, ceil(n / rows)).
    #[arg(long)]
    pub cols: Option<usize>,
    /// Horizontal and vertical gap between cells, percent of the page (default 5,5 as Origin).
    #[arg(long, value_delimiter = ',', value_name = "H,V", allow_hyphen_values = true)]
    pub gap: Vec<f64>,
    /// Left, right, top and bottom margins, percent of the page (default 15,10,10,15 as Origin).
    #[arg(long, value_delimiter = ',', value_name = "L,R,T,B", allow_hyphen_values = true)]
    pub margins: Vec<f64>,
    /// A portrait page (the page's longer side upright).
    #[arg(long)]
    pub portrait: bool,
    /// The page size in inches, WIDTH,HEIGHT (default Origin's 11.41,7.973; --portrait turns it).
    #[arg(long, value_delimiter = ',', value_name = "W,H")]
    pub page_size: Vec<f64>,
    /// Keep each graph's layer aspect ratio, at the top left of its cell.
    #[arg(long)]
    pub keep_aspect: bool,
    /// Link the layers of the later graphs to the first merged layer in position (Origin's "Link
    /// Source Graphs").
    #[arg(long)]
    pub link: bool,
    /// Also share the first layer's X scale (implies --link): a vertical stack with one X axis.
    #[arg(long)]
    pub share_x: bool,
    /// Hide the X tick labels, title and ticks of a cell that touches the cell below it (with a
    /// vertical gap of 0), and the Y ones of a cell touching one on its left (Origin's "Show Axes
    /// Frame").
    #[arg(long)]
    pub hide_inner: bool,
    /// Text size: auto (Origin's factor for smaller cells), keep (every point size as it is) or a
    /// fixed factor such as 0.5.
    #[arg(long, default_value = "auto", value_name = "auto|keep|FACTOR")]
    pub text_scale: String,
    /// Delete the merged graphs from the saved project (Origin's "Keep Source Graphs" off).
    #[arg(long)]
    pub remove_sources: bool,
    /// Write the merged graph as a figure (.png, .tif, .jpg, .bmp, .gif, .tga, .pcx, .psd, .svg, .pdf,
    /// .eps or .emf).
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    /// Resolution of raster output in dots per inch.
    #[arg(long, default_value_t = 300.0)]
    pub dpi: f64,
    /// Save the project with the merged graph (.sheaf, or .opj with a companion for what it cannot hold).
    #[arg(long, value_name = "FILE")]
    pub save: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct PrintArgs {
    /// A .sheaf, .opj or .opju project containing a graph.
    pub project: PathBuf,
    /// The graph to print (default: the only graph). Repeat it to print several graphs in the
    /// order given: with --output one PDF with a sheet per graph, otherwise one print job per
    /// graph (Sheaf's choices). A name given twice prints that graph twice.
    #[arg(long, value_name = "NAME", conflicts_with = "all_graphs")]
    pub graph: Vec<String>,
    /// Print every graph of the project in project order, as with a repeated --graph.
    #[arg(long)]
    pub all_graphs: bool,
    /// Submit to this printer (default: the system default printer).
    #[arg(long, conflicts_with = "output")]
    pub printer: Option<String>,
    /// Write a paper-sized vector PDF to this .pdf path instead of submitting a print job.
    #[arg(long, conflicts_with = "printer")]
    pub output: Option<PathBuf>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum WeightKind {
    /// Every point counts the same.
    None,
    /// 1 / error² from the Y-error column.
    Instrumental,
    /// 1 / abs(y); a y of exactly 0 is refused.
    Statistical,
    /// The weights are the values of the column(s) given with --yerr, used as given.
    Direct,
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
    /// A data file, a .sheaf project or an Origin .opj or .opju project. Not needed with --list.
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
    /// The Y-error column(s) for --weight instrumental (or the weight column(s) for --weight direct), by short or long name: one for all fitted
    /// columns, or one per --y column in the same order.
    #[arg(long, value_delimiter = ',', value_name = "COLUMNS")]
    pub yerr: Vec<String>,
    /// The X-error column for --model linearx (a straight line fitted with errors in x and y), by
    /// short or long name; without it the sheet's own X-error column is used.
    #[arg(long, value_name = "COLUMN")]
    pub xerr: Option<String>,
    /// Multiple linear regression of the first Y column on these independent variables
    /// (comma-separated columns); --fix Intercept=VALUE holds the intercept.
    #[arg(long, value_delimiter = ',', value_name = "COLUMNS", conflicts_with_all = ["compare", "rank", "global"])]
    pub regress: Vec<String>,
    /// With --regress: best subset selection instead of one regression. Every subset of the --regress
    /// columns (at most 15) is fitted with an intercept and scored by R-square, adjusted R-square,
    /// Mallows Cp, AIC and BIC; the table goes to a new sheet BestSubsets<n>.
    #[arg(long, requires = "regress")]
    pub best_subsets: bool,
    /// Compare --model with this model (a model as for --model) on the first Y column: AICc,
    /// Akaike weights, BIC and the F-test.
    #[arg(long, value_name = "MODEL", conflicts_with_all = ["rank", "global"])]
    pub compare: Option<String>,
    /// Rank these built-in functions on the first Y column by AICc (comma-separated names).
    #[arg(long, value_delimiter = ',', value_name = "FUNCTIONS", conflicts_with = "global")]
    pub rank: Vec<String>,
    /// Do not scale the parameter errors by sqrt(reduced chi-square) (Origin's default is to scale;
    /// without weights Origin always scales, so this only matters with --weight).
    #[arg(long)]
    pub no_scale_errors: bool,
    /// Add the covariance matrix of the parameters to the parameter sheet (Origin's Covariance
    /// Matrix: one column per parameter).
    #[arg(long)]
    pub covariance: bool,
    /// Add the correlation matrix of the parameters to the parameter sheet (Origin's Correlation
    /// Matrix).
    #[arg(long)]
    pub correlation: bool,
    /// Fit all the --y columns together as one global fit (nonlinear models only); the
    /// parameters named by --share take one value for all columns.
    #[arg(long)]
    pub global: bool,
    /// Parameters shared between the columns of a --global fit.
    #[arg(long, value_delimiter = ',', value_name = "PARAMS", requires = "global")]
    pub share: Vec<String>,
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
    /// Turn off Auto Parameter Initialization: a nonlinear fit starts from the --param values,
    /// else the function's own starting values (1 for most), not from a guess from the data.
    #[arg(long)]
    pub no_auto_init: bool,
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
    /// Fit a surface z = f(x, y) to this Z column (short or long name); --model is then a surface
    /// function such as Gauss2D or Plane (see --list), or --formula in x and y. The Y column is
    /// the one given with --y and the X column the one given with --x; left out, they are the X
    /// column to the left of the Z column and the nearest Y column before it.
    #[arg(long, value_name = "COLUMN", conflicts_with_all = ["global", "regress", "compare", "rank", "xerr", "plot"])]
    pub z: Option<String>,
    /// The X column of a surface fit (--z).
    #[arg(long, value_name = "COLUMN", requires = "z")]
    pub x: Option<String>,
    /// Draw the data with the fitted curve and write the figure (.png, .tif, .jpg, .bmp, .gif, .tga,
    /// .pcx, .psd, .svg, .pdf, .eps or .emf). With
    /// several columns, only the first one is drawn.
    #[arg(long, value_name = "FIGURE")]
    pub plot: Option<PathBuf>,
    /// Resolution of a raster figure in dots per inch.
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

/// `sheaf simulate`: Analysis > Fitting > Simulate Curve without a window.
#[derive(Args, Debug)]
pub struct SimulateArgs {
    /// A data file, a .sheaf project or an Origin .opj or .opju project: the workbook that gets the new
    /// sheet, and the place --x-of looks in. Without it the curve is a project of its own.
    pub input: Option<PathBuf>,
    /// List the functions with their parameters and starting values, and exit.
    #[arg(long)]
    pub list: bool,
    /// The built-in fit function, such as Gauss, ExpDec1 or Boltzmann (see --list). Case does not matter.
    #[arg(short, long, default_value = "Gauss")]
    pub function: String,
    /// A parameter value, for example --param xc=5 (repeat the option or separate with commas). The
    /// parameters left out keep the starting values of --list.
    #[arg(short, long, value_delimiter = ',', value_name = "NAME=VALUE", allow_hyphen_values = true)]
    pub param: Vec<String>,
    /// The first X value (default: the X range of Origin's Simulate Curve window for the function, Gauss -1). A first value above --to gives falling X.
    #[arg(long, allow_hyphen_values = true, conflicts_with = "x_of")]
    pub from: Option<f64>,
    /// The last X value (default: the function's, Gauss 5).
    #[arg(long, allow_hyphen_values = true, conflicts_with = "x_of")]
    pub to: Option<f64>,
    /// How many X values, from 2 to 1000000 (default 200).
    #[arg(long, conflicts_with = "x_of")]
    pub points: Option<usize>,
    /// Space the X values evenly on a logarithmic scale (both ends must be positive).
    #[arg(long, conflicts_with = "x_of")]
    pub log: bool,
    /// Use the X values of this column (short or long name, in --sheet) instead of a range.
    #[arg(long, value_name = "COLUMN")]
    pub x_of: Option<String>,
    /// For a project input: the workbook (default: the first one).
    #[arg(long)]
    pub book: Option<String>,
    /// The sheet --x-of looks in, by name (default: the workbook's active sheet).
    #[arg(long)]
    pub sheet: Option<String>,
    /// Draw the curve and write the figure (.png, .tif, .jpg, .bmp, .gif, .tga, .pcx, .psd, .svg, .pdf, .eps
    /// or .emf). The graph is also kept with --save.
    #[arg(long, value_name = "FIGURE")]
    pub plot: Option<PathBuf>,
    /// Resolution of a raster figure in dots per inch.
    #[arg(long, default_value_t = 300.0)]
    pub dpi: f64,
    /// Draw the curve in a graph of the project that --save writes (--plot does this too).
    #[arg(long)]
    pub graph: bool,
    /// Write the X and Y columns as delimited text (.tsv or .txt gives tabs, anything else commas) or, for a
    /// .xlsx path, as an Excel workbook. Without --output and --save the curve is printed.
    #[arg(short, long, visible_alias = "export", value_name = "FILE")]
    pub output: Option<PathBuf>,
    /// Save the project with the new sheet to this file: .opj (also when no extension is given, with a
    /// companion .opj.sheaf if the .opj cannot hold everything) or .sheaf.
    #[arg(long, visible_alias = "project", value_name = "PROJECT")]
    pub save: Option<PathBuf>,
    #[command(flatten)]
    pub read: ReadOptions,
}

/// `sheaf fit-implicit`: Analysis > Fitting > Nonlinear Implicit Curve Fit without a window.
#[derive(Args, Debug)]
pub struct FitImplicitArgs {
    /// A data file, a .sheaf project or an Origin .opj or .opju project.
    pub input: PathBuf,
    /// The implicit function: Circle or Ellipse (Origin's PlaneMod is not available).
    #[arg(short, long, default_value = "Circle")]
    pub function: String,
    /// The column with the X values of the points.
    #[arg(short, long)]
    pub x: String,
    /// The column with the Y values of the points.
    #[arg(short, long)]
    pub y: String,
    /// A starting value, for example --init xc=2 (repeat or separate with commas); the others are estimated.
    #[arg(long, value_delimiter = ',', value_name = "NAME=VALUE", allow_hyphen_values = true)]
    pub init: Vec<String>,
    /// Fit only rows FROM:TO (counted from 1).
    #[arg(long, value_name = "FROM:TO")]
    pub rows: Option<String>,
    /// Fit only the rows whose X lies between FROM and TO.
    #[arg(long, value_name = "FROM:TO", conflicts_with = "rows", allow_hyphen_values = true)]
    pub x_range: Option<String>,
    /// Confidence level of the parameter limits (default 0.95).
    #[arg(long)]
    pub confidence: Option<f64>,
    /// Most iterations (default 400).
    #[arg(long)]
    pub max_iterations: Option<usize>,
    /// Convergence tolerance (default 1e-9).
    #[arg(long)]
    pub tolerance: Option<f64>,
    /// The workbook (default: the first one).
    #[arg(long)]
    pub book: Option<String>,
    /// The sheet (default: the active one).
    #[arg(long)]
    pub sheet: Option<String>,
    /// Write the parameter sheet as delimited text or, for a .xlsx path, as an Excel workbook.
    #[arg(short, long, visible_alias = "export", value_name = "FILE")]
    pub output: Option<PathBuf>,
    /// Save the project with the report sheets to this file: .opj or .sheaf.
    #[arg(long, visible_alias = "project", value_name = "PROJECT")]
    pub save: Option<PathBuf>,
    #[command(flatten)]
    pub read: ReadOptions,
}

/// `sheaf compare-datasets`: Analysis > Fitting > Compare Datasets without a window.
#[derive(Args, Debug)]
pub struct CompareDatasetsArgs {
    /// A data file, a .sheaf project or an Origin .opj or .opju project.
    pub input: PathBuf,
    /// The model: linear, poly:N or a built-in function (see `sheaf fit --list`).
    #[arg(short, long, default_value = "linear")]
    pub model: String,
    /// The Y column of the first dataset (its X column follows from the sheet).
    #[arg(long)]
    pub y1: String,
    /// The Y column of the second dataset.
    #[arg(long)]
    pub y2: String,
    /// A starting value for the fit of both, for example --init xc=2 (repeat or separate with commas).
    #[arg(long, value_delimiter = ',', value_name = "NAME=VALUE", allow_hyphen_values = true)]
    pub init: Vec<String>,
    /// Origin's "1st Dataset Name" (default Data1).
    #[arg(long)]
    pub name1: Option<String>,
    /// Origin's "2nd Dataset Name" (default Data2).
    #[arg(long)]
    pub name2: Option<String>,
    /// Leave the fit parameters out of the report (Origin's Fit Parameters box).
    #[arg(long)]
    pub no_parameters: bool,
    /// Leave the fit statistics out of the report (Origin's Fit Statistics box).
    #[arg(long)]
    pub no_statistics: bool,
    /// The workbook (default: the first one).
    #[arg(long)]
    pub book: Option<String>,
    /// The sheet (default: the active one).
    #[arg(long)]
    pub sheet: Option<String>,
    /// Write the report sheet as delimited text or, for a .xlsx path, as an Excel workbook.
    #[arg(short, long, visible_alias = "export", value_name = "FILE")]
    pub output: Option<PathBuf>,
    /// Save the project with the report sheet to this file: .opj or .sheaf.
    #[arg(long, visible_alias = "project", value_name = "PROJECT")]
    pub save: Option<PathBuf>,
    #[command(flatten)]
    pub read: ReadOptions,
}

/// `sheaf simulate-surface`: Analysis > Fitting > Simulate Surface without a window.
#[derive(Args, Debug)]
pub struct SimulateSurfaceArgs {
    /// A data file, a .sheaf project or an Origin .opj or .opju project: the workbook that gets the new sheet.
    /// Without it the surface is a project of its own.
    pub input: Option<PathBuf>,
    /// List the 22 functions with their parameters, ranges and noise levels of Origin's window, and exit.
    #[arg(long)]
    pub list: bool,
    /// The surface function, Origin's name such as Gauss2D, Plane or Cosine (see --list). Case does not matter.
    #[arg(short, long, default_value = "Gauss2D")]
    pub function: String,
    /// A parameter value, for example --param xc=0.5 (repeat the option or separate with commas). The parameters
    /// left out keep the values of --list.
    #[arg(short, long, value_delimiter = ',', value_name = "NAME=VALUE", allow_hyphen_values = true)]
    pub param: Vec<String>,
    /// X minimum (default: the function's range in Origin's window, Gauss2D -1).
    #[arg(long, allow_hyphen_values = true)]
    pub x_min: Option<f64>,
    /// X maximum (default: the function's, Gauss2D 1).
    #[arg(long, allow_hyphen_values = true)]
    pub x_max: Option<f64>,
    /// Number of X points (default 20).
    #[arg(long)]
    pub x_points: Option<usize>,
    /// Y minimum (default: the function's).
    #[arg(long, allow_hyphen_values = true)]
    pub y_min: Option<f64>,
    /// Y maximum (default: the function's).
    #[arg(long, allow_hyphen_values = true)]
    pub y_max: Option<f64>,
    /// Number of Y points (default 20).
    #[arg(long)]
    pub y_points: Option<usize>,
    /// Noise level in percent of the mean of Z (default: the function's, 10 for Gauss2D, Gaussian2D, Voigt2D and
    /// Voigt2DMod, 0 for the others). The noise is Sheaf's seeded generator, not Origin's sequence.
    #[arg(long)]
    pub noise: Option<f64>,
    /// Seed of the noise (default 1); the same seed gives the same noise.
    #[arg(long)]
    pub seed: Option<u64>,
    /// For a project input: the workbook (default: the first one).
    #[arg(long)]
    pub book: Option<String>,
    /// Write the X, Y and Z columns as delimited text (.tsv or .txt gives tabs, anything else commas) or, for a
    /// .xlsx path, as an Excel workbook. Without --output and --save the data is printed.
    #[arg(short, long, visible_alias = "export", value_name = "FILE")]
    pub output: Option<PathBuf>,
    /// Save the project with the new sheet to this file: .opj (also when no extension is given, with a
    /// companion .opj.sheaf if the .opj cannot hold everything) or .sheaf.
    #[arg(long, visible_alias = "project", value_name = "PROJECT")]
    pub save: Option<PathBuf>,
    #[command(flatten)]
    pub read: ReadOptions,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum Format {
    /// Decide from the file content; .xls, .xlsx and .xlsm files are Excel workbooks.
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
    /// Delimited text: the separator. One of comma, tab, semicolon, space, a single character, or a
    /// string of several characters that separates fields as a whole (for example "||").
    #[arg(long, value_name = "SEP")]
    pub delimiter: Option<String>,
    /// Delimited text: each character is a separator (for example "|;").
    #[arg(long, value_name = "CHARS", conflicts_with_all = ["delimiter", "fixed_width"])]
    pub delimiter_chars: Option<String>,
    /// Collapse runs of the characters supplied by --delimiter-chars to one separator.
    #[arg(long, requires = "delimiter_chars")]
    pub combine_delimiters: bool,
    /// Delimited text: fixed-width columns instead of a separator. The widths in characters, as a
    /// comma-separated list; the last width repeats for the remaining columns.
    #[arg(long, value_name = "W,W,...", value_delimiter = ',', conflicts_with = "delimiter")]
    pub fixed_width: Vec<usize>,
    /// Delimited text and Excel: number of lines (rows) before the first data row (default: detect).
    #[arg(long, value_name = "N")]
    pub header_lines: Option<usize>,
    /// Delimited text and Excel: first data row to import, counted after the header.
    #[arg(long, default_value_t = 1)]
    pub first_row: usize,
    /// Delimited text and Excel: last data row to import, inclusive (0: through the end).
    #[arg(long, default_value_t = 0)]
    pub last_row: usize,
    /// Delimited text and Excel: rows to read per repeating read/skip pattern.
    #[arg(long, default_value_t = 1)]
    pub read_rows: usize,
    /// Delimited text and Excel: rows to skip per repeating read/skip pattern.
    #[arg(long, default_value_t = 0)]
    pub skip_rows: usize,
    /// Delimited text and Excel: first source column to import (Excel: counted from column A).
    #[arg(long, default_value_t = 1)]
    pub first_column: usize,
    /// Delimited text and Excel: last source column to import, inclusive (0: through the end).
    #[arg(long, default_value_t = 0)]
    pub last_column: usize,
    /// Delimited text and Excel: columns to read per repeating read/skip pattern.
    #[arg(long, default_value_t = 1)]
    pub read_columns: usize,
    /// Delimited text and Excel: columns to skip per repeating read/skip pattern.
    #[arg(long, default_value_t = 0)]
    pub skip_columns: usize,
    /// Delimited text: numbers use a comma as the decimal separator.
    #[arg(long)]
    pub decimal_comma: bool,
    /// Delimited text and Excel: keep date and time columns as text instead of reading them as date numbers.
    #[arg(long)]
    pub dates_as_text: bool,
    /// Delimited text: read 11/12/2026 as 11 December (default: 12 November) when the file does not decide.
    #[arg(long)]
    pub day_first: bool,
    /// Delimited text: the digit-grouping character of numbers.
    #[arg(long, value_enum, default_value_t = ThousandsArg::Auto)]
    pub thousands: ThousandsArg,
    /// Delimited text and Excel: line (row, 1-based) holding the column long names (0: none; default: detect).
    #[arg(long, value_name = "N")]
    pub long_name_line: Option<usize>,
    /// Delimited text and Excel: line holding the units (0: none).
    #[arg(long, value_name = "N")]
    pub units_line: Option<usize>,
    /// Delimited text and Excel: line holding the comments (0: none).
    #[arg(long, value_name = "N")]
    pub comments_line: Option<usize>,
    /// Delimited text: last comments line; the lines from --comments-line to this one are joined with
    /// line breaks (Origin's Comments From .. To). Default: the end of the detected label lines.
    #[arg(long, value_name = "N")]
    pub comments_to: Option<usize>,
    /// Delimited text and Excel: header lines that become named parameter rows (for example the wavelengths
    /// of an ICP-OES export), as a comma-separated list. The first cell of the line is the name.
    #[arg(long, value_name = "N,N", value_delimiter = ',')]
    pub param_lines: Vec<usize>,
    /// Delimited text and Excel: line holding the short names of the columns (default: A, B, C, ...).
    #[arg(long, value_name = "N")]
    pub short_name_line: Option<usize>,
    /// Delimited text and Excel: leave the long name, units and comments of the columns empty; the
    /// header lines are still skipped.
    #[arg(long)]
    pub no_rename_columns: bool,
    /// Delimited text: name the sheet Sheet1 instead of after the file.
    #[arg(long)]
    pub no_rename_sheet: bool,
    /// Delimited text and Excel: leave the long name of the book empty instead of the file name.
    #[arg(long)]
    pub no_rename_book: bool,
    /// Delimited text: a file with several blocks of data separated by blank lines gets one sheet per block.
    #[arg(long)]
    pub blocks_as_sheets: bool,
    /// Delimited text: what a text cell in a numeric column does (Origin's NonNum): keep it as text,
    /// stop the import there, or put the rows after it in a new sheet or in new columns (the row
    /// itself left out).
    #[arg(long, value_enum, default_value_t = NonNumericArg::Text)]
    pub non_numeric: NonNumericArg,
    /// Delimited text: Add Sparklines (Origin's import option): the imported columns get sparklines
    /// and the sheet shows its Sparklines row; auto only when the sheet has fewer than 50 columns
    /// (Origin's default; Sheaf's is no).
    #[arg(long, value_enum, default_value_t = SparklinesArg::No)]
    pub sparklines: SparklinesArg,
    /// Excel (.xls, .xlsx, .xlsm): the sheets to read, as names or 1-based positions counted over
    /// all sheets, comma-separated (for example "Results,3"). A number is the sheet of that name if there
    /// is one, otherwise the sheet at that position.
    /// A sheet named here is read even when hidden. Default: every worksheet, hidden ones too (as
    /// Origin).
    #[arg(long, value_name = "LIST", value_delimiter = ',')]
    pub sheets: Vec<String>,
    /// Excel: read hidden sheets when --sheets is not given (the default; kept for older scripts).
    #[arg(long, conflicts_with = "skip_hidden_sheets")]
    pub hidden_sheets: bool,
    /// Excel: leave hidden sheets out when --sheets is not given.
    #[arg(long)]
    pub skip_hidden_sheets: bool,
    /// Start from a saved import theme (the Dialog Theme row of the import windows): a saved name,
    /// `<default>` or `<last used>`. Options given on the command line win over the theme; a flag
    /// can only turn an option on, so to switch one off save a theme without it. A name that is in
    /// no store lists the saved themes.
    #[arg(long, value_name = "NAME", conflicts_with = "last_used")]
    pub theme: Option<String>,
    /// Start from the settings of the last OK in the import windows (same as `--theme "<last used>"`).
    #[arg(long)]
    pub last_used: bool,
    /// The folder of the saved import themes (default: `$SHEAF_THEMES_DIR`, else `import-themes` in the
    /// settings folder). Only read with --theme or --last-used.
    #[arg(long, value_name = "DIR")]
    pub themes_dir: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct InfoArgs {
    /// A data file, a .sheaf project or an Origin .opj or .opju project.
    pub path: PathBuf,
    #[command(flatten)]
    pub read: ReadOptions,
}

#[derive(Args, Debug)]
pub struct ImportArgs {
    /// Data files, folders or patterns to import. A folder gives its data files (.txt, .csv, .tsv,
    /// .dat, .asc, .prn, .tab, .bin, .xls, .xlsx, .xlsm; an Excel file becomes one workbook with a sheet per Excel sheet); a name with * or ? in it (for example "exports/icp_*.csv")
    /// gives the matching files, in natural order (icp_2 before icp_10). An Origin .opj or .opju project adds
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
    /// Apply partial row/column selection to every data file except the first.
    #[arg(long)]
    pub partial_except_first: bool,
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

/// A text cell in a numeric column of delimited text (`sheaf import --non-numeric`, T312).
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum NonNumericArg {
    /// Keep the cell as text (Origin's default).
    Text,
    /// Import the rows above it only.
    Stop,
    /// The rows after it go to a new sheet.
    NewSheet,
    /// The rows after it go to new columns.
    NewColumns,
}

/// Add Sparklines of delimited text (`sheaf import --sparklines`, T231).
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum SparklinesArg {
    /// No sparklines.
    No,
    /// Every imported column.
    Yes,
    /// Every imported column when the sheet has fewer than 50 columns (Origin's default).
    Auto,
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
    /// Connect the points of line and line+symbol curves in increasing X order, leaving the
    /// worksheet unsorted (Origin 2023's LabTalk layer.plotN.line.sortx). Symbols, labels and
    /// error bars stay on their points.
    #[arg(long)]
    pub sort_x: bool,
    /// Add a trendline to each curve: linear, poly:N (order 2 to 10), power, log, exp or ma:N
    /// (moving average, window 2 to 10). Repeatable. Replaces the trendlines the curve has. A
    /// fitted trendline writes its equation and R-squared at the top left of the layer.
    #[arg(long, value_name = "KIND")]
    pub trendline: Vec<String>,
    /// Draw the trendlines without the equation and R-squared label.
    #[arg(long)]
    pub no_trendline_label: bool,
    /// Curve colors in legend order (left axis first, then right axis), comma-separated, as
    /// #RRGGBB or basic names.
    #[arg(long, value_delimiter = ',', value_name = "COLORS")]
    pub colors: Vec<String>,
    /// One of Origin's named colour lists for the curve colours: Default, Q01 Black and RGB, Q02 Basic,
    /// Q03 Rainbow7, ... S01 Gray Scale ... D05 Blue-Greens, System Color List (the code, for example
    /// Q03, is enough). --colors still sets single curves.
    #[arg(long, value_name = "NAME")]
    pub color_list: Option<String>,
    /// Legend labels in legend order (left axis first, then right axis), comma-separated.
    #[arg(long, value_delimiter = ',', value_name = "LABELS")]
    pub labels: Vec<String>,
    /// Hide the legend.
    #[arg(long)]
    pub no_legend: bool,
    /// Move the curves apart: none; cumulative (each curve is drawn at its Y plus the Y of the
    /// curves before it at the same row); auto (each curve above the one before it with a gap of 10 %
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
    /// Hexbin plots (-t hexbin): the number of bins along X (the page's regular hexagons set Y
    /// from it), from 2 to 1000 (default 20).
    #[arg(long, value_name = "N")]
    pub hex_bins: Option<usize>,
    /// Treemaps (-t treemap): the layout of every level: squarified (default), squarified-plus, split, pivot-size,
    /// pivot-middle or pivot-split-size.
    #[arg(long, value_name = "LAYOUT")]
    pub treemap_layout: Option<String>,
    /// Treemaps: the order of the rectangles: categorical (default, the order of first appearance), alphabetical,
    /// size or column:NAME (by a column, smallest first). The squarified layouts sort by size and ignore it.
    #[arg(long, value_name = "ORDER")]
    pub treemap_order: Option<String>,
    /// Treemaps: the number of levels shown, 1 or more (default: every level).
    #[arg(long, value_name = "N")]
    pub treemap_depth: Option<usize>,
    /// Treemaps: the gap between rectangles in pt, from 0 to 20 (default 1.5).
    #[arg(long, value_name = "PT")]
    pub treemap_gap: Option<f64>,
    /// Grouped columns (-t grouped-columns): the error bars on the bars (the mean of each group's rows): none
    /// (default), sd (standard deviation, n-1) or se (standard error).
    #[arg(long, value_name = "ERROR")]
    pub group_error: Option<String>,
    /// Interval plots (-t interval-plot): the error bar on each column's mean: ci (default, Student-t confidence
    /// interval), sd (standard deviation, n-1) or se (standard error).
    #[arg(long, value_name = "ERROR")]
    pub interval_error: Option<String>,
    /// Interval plots: the confidence level in percent of --interval-error ci, above 0 and below 100 (default 95).
    #[arg(long, value_name = "PERCENT")]
    pub interval_level: Option<f64>,
    /// Scatter matrices (-t scatter-matrix): the cells shown: square (default, every pair both ways round), upper or
    /// lower (the diagonal and the cells above or below it).
    #[arg(long, value_name = "DISPLAY")]
    pub matrix_display: Option<String>,
    /// Scatter matrices: what the diagonal cells show: none (default) or histogram of the column.
    #[arg(long, value_name = "SHOW")]
    pub matrix_diagonal: Option<String>,
    /// Scatter matrices: the gap between cells in percent of the page, from 0 to 10 (default 0).
    #[arg(long, value_name = "PCT")]
    pub matrix_gap: Option<f64>,
    /// Scatter matrices: leave the column names out of the diagonal cells.
    #[arg(long)]
    pub matrix_no_names: bool,
    /// Sunburst plots (-t sunburst): where the first wedge starts, in degrees counter-clockwise from 3 o'clock
    /// (default 90: 12 o'clock; the wedges go clockwise).
    #[arg(long, value_name = "DEGREES", allow_hyphen_values = true)]
    pub sunburst_start: Option<f64>,
    /// Sunburst plots: the hole in the middle as a percentage of the radius, 0 to 90 (default 30).
    #[arg(long, value_name = "PCT")]
    pub sunburst_hole: Option<f64>,
    /// Sunburst plots: the gap between rings in pt, from 0 to 20 (default 0).
    #[arg(long, value_name = "PT")]
    pub sunburst_gap: Option<f64>,
    /// Sunburst plots: what the wedges are labelled with: names (default), values, both or none.
    #[arg(long, value_name = "WHAT")]
    pub sunburst_labels: Option<String>,
    /// Sunburst plots: the order of the wedges of a ring: worksheet (default, first appearance) or size (largest first).
    #[arg(long, value_name = "ORDER")]
    pub sunburst_order: Option<String>,
    /// Add a free text label to the first layer: "TEXT@X,Y", where X and Y are fractions of the
    /// layer frame (0,0 is its top left corner, 1,1 its bottom right; the label's text starts
    /// there). The text may use Origin's escapes, for example --label "\b(Zn-LMO)@0.85,0.1".
    /// Repeat the option for more labels.
    #[arg(long, value_name = "TEXT@X,Y")]
    pub label: Vec<String>,
    /// Draw an object on the first layer: "KIND@X1,Y1,X2,Y2" with KIND line, arrow, rectangle or
    /// ellipse and the points as fractions of the layer frame (0,0 is its top left corner), for
    /// example --draw "arrow@0.2,0.2,0.5,0.4". Repeat the option for more objects.
    #[arg(long, value_name = "KIND@X1,Y1,X2,Y2")]
    pub draw: Vec<String>,
    /// Mark a slope between two data points of the first plot of the first layer: "ROW_A,ROW_B", the
    /// worksheet row numbers counted from 1 (for example --slope 3,8). The segment, dashed legs and the
    /// label "slope = ..." are computed in the axes' own units (per decade on a log10 axis). Repeat the
    /// option for more slopes; they all apply to the first plot.
    #[arg(long, value_name = "ROW_A,ROW_B")]
    pub slope: Vec<String>,
    /// Draw a vertical straight line at an X value on the first layer, with a label that shows the value:
    /// "VALUE" or "VALUE,LOCATION" with LOCATION top (the default), bottom, left or right (for example
    /// --vline 0.5 or --vline 0.5,bottom). The line runs across the whole frame in black, 0.5 pt. Repeat
    /// the option for more lines. A value outside the axis range draws nothing.
    #[arg(long, value_name = "VALUE[,LOCATION]", allow_hyphen_values = true)]
    pub vline: Vec<String>,
    /// Draw a horizontal straight line at a Y value on the first layer: like --vline.
    #[arg(long, value_name = "VALUE[,LOCATION]", allow_hyphen_values = true)]
    pub hline: Vec<String>,
}

/// Which part of the page a figure shows (Origin's export margin settings).
#[derive(Args, Debug)]
pub struct MarginArgs {
    /// Margin of the exported figure: border (everything drawn plus a border; Origin's default
    /// "auto"), tight (everything drawn), page (the whole page) or tight-in-page (the objects that
    /// are over half inside the page). --image-width and --image-height then size that area.
    #[arg(long, value_name = "MODE", default_value = "auto")]
    pub margin: String,
    /// Border width in pixels with --margin border (pixels at --dpi for PNG and TIFF, points for
    /// the vector formats; default 5 pt).
    #[arg(long, value_name = "PIXELS")]
    pub border_width: Option<f64>,
}

#[derive(Args, Debug)]
pub struct PlotArgs {
    /// A data file, a .sheaf project or an Origin .opj or .opju project. Several data files are overlaid in one figure.
    #[arg(required = true)]
    pub inputs: Vec<PathBuf>,
    /// The figure to write: .png, .tif, .jpg, .bmp, .gif, .tga, .pcx, .psd, .svg, .pdf, .eps or .emf.
    #[arg(short, long)]
    pub output: PathBuf,
    /// Resolution of raster output in dots per inch.
    #[arg(long, default_value_t = 300.0)]
    pub dpi: f64,
    /// Width of the exported picture, with a unit: in, cm, mm, pt, px or % (for example 3.5in or
    /// 8.5cm; px counts at --dpi; 50% is half the size of the exported area). The whole page is scaled to it, text and lines included, as
    /// Origin's export image size does; --width instead changes the page and keeps the text size.
    #[arg(long, value_name = "LENGTH")]
    pub image_width: Option<String>,
    /// Height of the exported picture, with a unit as for --image-width. With both, the picture is
    /// scaled to fit inside both.
    #[arg(long, value_name = "LENGTH")]
    pub image_height: Option<String>,
    #[command(flatten)]
    pub margin: MarginArgs,
    /// Figure template: line, scatter, line-symbol, column, cv, double-y, stacked or waterfall
    /// (default: what the importer suggests). column draws one group of bars per row, the Y columns
    /// side by side, with sample names from a text X column under the bars; double-y puts the 1st,
    /// 3rd, ... Y column on the left axis and the 2nd, 4th, ... on a right axis; 3ys-y-yy, 3ys-y-y-y, 4ys-y-yyy and
    /// 4ys-yy-yy make three or four layers, each with its own Y axis (colours and offsets as Origin's
    /// templates of these names) and deal the Y columns out one to a layer, round and round; stacked moves each
    /// curve above the one before it; lollipop (two or more Y columns) joins each Y column's points
    /// to the next column's at the same row with drop lines, X and Y exchanged; scatter-rug is a scatter
    /// with a rug (one short line per point) beside the bottom and left axes; violin draws each
    /// Y column as a mirrored kernel density silhouette, like box (also violin-box, violin-point,
    /// violin-quartile, violin-stick and half-violin, which differ in the marks inside); ridgeline draws
    /// each Y column as a density curve on its own baseline, the columns stacked up the page; hexbin bins
    /// the X column against the first Y column into hexagons coloured by their count (see --hex-bins);
    /// parallel (two or more Y columns) draws one vertical axis per column and one line per row, coloured by a
    /// column with a colour scale; parallel-index (three or more Y columns and a text group column, see --group)
    /// colours the lines by group, with a legend; treemap nests rectangles by the category columns (--group; default:
    /// the text columns) with areas proportional to the last numeric Y column (see the --treemap-* options); sunburst
    /// draws the same hierarchy as concentric rings, the first category innermost, angles proportional to the last
    /// numeric Y column, coloured by value (see the --sunburst-* options); grouped-columns and grouped-box split the one
    /// numeric Y column by one or two group columns (--group; the first on the X axis, the second one plot per level,
    /// shown in the legend): the mean of each group as columns (see --group-error) or a box per group;
    /// interval-plot draws the mean of each Y column with an error bar (see --interval-error); pareto counts the
    /// distinct values of the one Y column (text or numbers) as columns, most frequent first, with the cumulative
    /// percent on a right axis of 0 to 100; scatter-matrix plots every pair of the Y columns (2 to 8; -y may also
    /// name X columns) in an n x n grid of layers, column c on X and row r on Y, with the column names on the
    /// diagonal (see the --matrix-* options).
    /// A name that is none of these plots with the user template of that name (File > Save Template As in
    /// the desktop app; see --templates-dir).
    #[arg(short, long)]
    pub template: Option<String>,
    /// The folder of the user graph templates (default: `$SHEAF_TEMPLATES_DIR`, else `templates` in the
    /// settings folder). Only read when -t names no built-in template.
    #[arg(long, value_name = "DIR")]
    pub templates_dir: Option<std::path::PathBuf>,
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
    /// Parallel plots (-t parallel, -t parallel-index): the column that colours the lines. For a parallel index the
    /// group column (text; default: the first text column of the sheet), one colour and legend entry per distinct
    /// text. For a parallel plot a numeric column (default: the last Y column) shown with a colour scale; `none`
    /// draws every line in the plot colour. For a treemap or sunburst the category columns, comma-separated, the first the top level or innermost ring
    /// (default: the text columns among --y, else every text column of the sheet). For grouped-columns and grouped-box
    /// one or two group columns, comma-separated (required): the first the groups along the X axis, the second the
    /// subgroups side by side. With any other template (line, scatter, ...) one curve per value of the group column
    /// (Origin's plotgroup): the rows are split into a new sheet `Grouped`, the legend names the values.
    #[arg(long, value_name = "COLUMN", conflicts_with_all = ["graph", "panels"])]
    pub group: Option<String>,
    /// -t fishbone (a cause-and-effect diagram): the effect's text at the head of the spine (default
    /// "Effect"). The -y columns (default: every column of the sheet) are the categories, their cells the causes.
    #[arg(long, value_name = "TEXT")]
    pub effect: Option<String>,
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
    /// A .sheaf project or an Origin .opj project (its graphs are read) or .opju project (only its workbooks are read). For an output ending in
    /// .opj a data file (CH Instruments text, CSV or other delimited text) is also accepted.
    pub project: PathBuf,
    /// Name of the graph (default: the only graph of the project). Repeat it to export several
    /// graphs into one multi-page .pdf, one page per graph in the order given (Origin's Export
    /// Graphs "multipage" option), or into one .pptx (a slide per graph) or .docx (a page per graph); a name given twice gets a
    /// page each time. Not used for .opj or .xlsx output.
    #[arg(long, value_name = "NAME", conflicts_with = "all_graphs")]
    pub graph: Vec<String>,
    /// Export every graph of the project, in project order (Sheaf's choice of page order).
    /// More than one graph needs a .pdf output (one multi-page PDF, one page per graph), a .pptx
    /// output (one slide per graph) or a .docx output (one page per graph), with --image-width,
    /// --image-height and --margin applied to every page.
    #[arg(long)]
    pub all_graphs: bool,
    /// Excel output: name of the workbook to write (default: the first one).
    #[arg(long)]
    pub book: Option<String>,
    /// Excel output: name of the sheet to write (default: the workbook's active sheet; with
    /// --all-sheets, every sheet).
    #[arg(long)]
    pub sheet: Option<String>,
    /// What to write: a figure (.png, .tif, .jpg, .bmp, .gif, .tga, .pcx, .psd, .svg, .pdf, .eps or .emf),
    /// a PowerPoint presentation (.pptx, one slide per graph, the graph as a PNG at --dpi), a Word
    /// document (.docx, one graph per page with its name under it),
    /// with .opj, the whole project as an Origin project file (workbooks and graphs;
    /// what the file cannot hold is listed as warnings), or with .xlsx one workbook as an Excel file
    /// (a data file is also accepted; see --book, --sheet and the Excel options).
    #[arg(short, long)]
    pub output: PathBuf,
    /// Resolution of raster output in dots per inch.
    #[arg(long, default_value_t = 300.0)]
    pub dpi: f64,
    /// Width of the exported picture, with a unit: in, cm, mm, pt, px or % (for example 3.5in or
    /// 8.5cm; px counts at --dpi; 50% is half the size of the exported area). The whole page is scaled to it, text and lines included, as
    /// Origin's export image size does; --width instead changes the page and keeps the text size.
    #[arg(long, value_name = "LENGTH")]
    pub image_width: Option<String>,
    /// Height of the exported picture, with a unit as for --image-width. With both, the picture is
    /// scaled to fit inside both.
    #[arg(long, value_name = "LENGTH")]
    pub image_height: Option<String>,
    #[command(flatten)]
    pub margin: MarginArgs,
    #[command(flatten)]
    pub style: StyleArgs,
    #[command(flatten)]
    pub xlsx: XlsxArgs,
    #[command(flatten)]
    pub read: ReadOptions,
}

/// What an `.xlsx` output holds (`sheaf export`, `sheaf export-data`).
#[derive(Args, Debug, Clone, Default)]
pub struct XlsxArgs {
    /// Excel output: write every sheet of the workbook, one Excel sheet each (default: only the
    /// sheet picked with --sheet or the active one).
    #[arg(long, conflicts_with = "sheet")]
    pub all_sheets: bool,
    /// Excel output: do not write the row of column long names.
    #[arg(long)]
    pub no_long_names: bool,
    /// Excel output: do not write the row of units.
    #[arg(long)]
    pub no_units: bool,
    /// Excel output: do not write the row of comments.
    #[arg(long)]
    pub no_comments: bool,
    /// Excel output: write a first row with the short names of the columns (A, B, ...). Read it
    /// back with --short-name-line 1.
    #[arg(long)]
    pub short_names: bool,
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
    /// The file to write. A .xlsx extension gives an Excel workbook; a .tsv or .txt extension gives
    /// tab-separated text, anything else commas.
    #[arg(short, long)]
    pub output: PathBuf,
    #[command(flatten)]
    pub xlsx: XlsxArgs,
}

#[derive(Args, Debug)]
pub struct AnalyzeArgs {
    #[command(subcommand)]
    pub tool: AnalyzeTool,
}

/// Which data an analysis tool works on and where the results go.
#[derive(Args, Debug, Clone)]
pub struct TargetArgs {
    /// A data file, a .sheaf project or an Origin .opj or .opju project.
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

/// What the groups of a multiple comparisons plot are drawn as.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum ComparisonPlotTypeArg {
    /// Columns of the group means.
    Bar,
    Box,
    Violin,
    /// The raw points with the mean and its error bar.
    Scatter,
}

/// The error bar of a bar or scatter multiple comparisons plot.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum ErrorBarArg {
    /// One standard deviation.
    Sd,
    /// One standard error of the mean.
    Sem,
    /// Half the 95 % confidence interval of the mean.
    Ci95,
}

/// How the brackets of a multiple comparisons plot are labelled.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum BracketLabelsArg {
    /// `*` p < 0.05, `**` < 0.01, `***` < 0.001, `****` < 0.0001, `ns` otherwise.
    Asterisks,
    /// `p = 0.0123`, `p < 0.0001`.
    PValues,
}

/// The multiple comparisons plot (T64) that a one-way ANOVA with a means comparison or a two-sample t-test can draw.
#[derive(Args, Debug, Clone)]
pub struct ComparisonPlotArgs {
    /// Also draw the multiple comparisons plot of the report to this figure file (.png, .svg or .pdf): the groups
    /// with significance brackets between the compared pairs. The graph stays in the project (--save).
    #[arg(long, value_name = "OUT")]
    pub comparison_plot: Option<PathBuf>,
    /// The plot's groups: bar (means), box, violin or scatter (raw points).
    #[arg(long, value_enum, default_value_t = ComparisonPlotTypeArg::Bar)]
    pub plot_type: ComparisonPlotTypeArg,
    /// Error bars of the bar and scatter plots.
    #[arg(long, value_enum, default_value_t = ErrorBarArg::Sd)]
    pub error_bar: ErrorBarArg,
    /// Bracket labels.
    #[arg(long, value_enum, default_value_t = BracketLabelsArg::Asterisks)]
    pub labels: BracketLabelsArg,
    /// Bracket every compared pair, not only those with p below --threshold.
    #[arg(long)]
    pub all_pairs: bool,
    /// The p-value below which a pair counts as significant.
    #[arg(long, default_value_t = 0.05)]
    pub threshold: f64,
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
pub enum FamilyArg {
    Normal,
    Lognormal,
    Exponential,
    Gamma,
    Weibull,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum PowerTestArg {
    OneSample,
    TwoSample,
    Paired,
    Anova,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum ComparisonArg {
    Tukey,
    Bonferroni,
    Sidak,
    /// Fisher's least significant difference.
    Lsd,
    Scheffe,
    /// Bonferroni-Holm step-down (one-way ANOVA only).
    HolmBonferroni,
    /// Sidak-Holm step-down (one-way ANOVA only).
    HolmSidak,
    /// Dunnett: every group against the control group (see --control); one-way ANOVA only. Checked against
    /// scipy, not Origin.
    Dunnett,
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
    /// Lowess: local straight-line fits weighted by distance; `--span` or `--points` sets the window.
    Lowess,
    /// Loess: like Lowess with local parabolas.
    Loess,
    /// Binomial filter (1 2 1 / 4) applied `--order` times.
    Binomial,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum BoundaryKind {
    None,
    Reflect,
    Repeat,
    Periodic,
    Extrapolate,
}

/// Origin's `reducedup` methods.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum DupKind {
    Mean,
    Median,
    Min,
    Max,
    Sum,
    Sd,
    Se,
    Rms,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum ReduceKind {
    Mean,
    First,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum AxisKind {
    X,
    Y,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum ApodizationKind {
    Boxcar,
    Triangle,
    TriangleSquared,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum MedianEdgeKind {
    Shrink,
    Repeat,
    Reflect,
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
    /// Scale each column so that its --stat matches that of the --ref column (Origin's ref).
    Ref,
    /// Divide by the cell of the --ref column at --row (Origin's cell).
    Cell,
}

/// The statistic `--method ref` matches (Origin's `type`).
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum RefStatArg {
    Min,
    Max,
    Mean,
    Median,
    Sum,
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

/// Coherence window (Origin's names).
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum CoherenceWindowArg {
    Rectangle,
    Welch,
    Triangular,
    Bartlett,
    Hanning,
    Hamming,
    Blackman,
    Gaussian,
    Kaiser,
}

/// FFT filter type (Origin's names).
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum FftFilterArg {
    LowPass,
    HighPass,
    BandPass,
    BandBlock,
    Threshold,
    Parabolic,
}

/// Filter response type (Origin's Response Type of the IIR and FIR filters).
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum FilterTypeArg {
    LowPass,
    HighPass,
    BandPass,
    BandStop,
}

/// IIR design method (Origin's Method).
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum IirMethodArg {
    Butterworth,
    Chebyshev1,
    Chebyshev2,
    Elliptic,
}

/// Unit of the frequency specification.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum FreqUnitArg {
    /// Hz; the sample frequency comes from the X column unless `--fs` is given.
    Hz,
    /// 1 is the Nyquist frequency.
    Normalized,
}

/// FIR design method (Origin's Method).
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum FirMethodArg {
    LeastSquares,
    Window,
}

/// FIR window (Origin's list).
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum FirWindowArg {
    Bartlett,
    BartlettHanning,
    Blackman,
    BlackmanHarris,
    Bohman,
    Chebyshev,
    FlatTop,
    Gaussian,
    Hamming,
    Hanning,
    Kaiser,
    Nuttall,
    Parzen,
    Rectangle,
    Taylor,
    Triangular,
    Tukey,
}

/// Unit of the FIR magnitude specification.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum MagnitudeUnitArg {
    Db,
    Linear,
}

/// Envelope type (Origin's names).
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum EnvelopeArg {
    Upper,
    Lower,
    Both,
}

/// Decimation filter (Origin's names).
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum DecimateFilterArg {
    None,
    /// Moving average, a triangle of half width `factor`.
    Ave,
    /// Origin's FIR filter, a triangle of half width `order`.
    Fir,
}

/// Wavelet extension mode (Origin's `ext`).
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum WaveletExtArg {
    Periodic,
    /// Zero-padded (Origin's `pad0`).
    Pad0,
}

/// Mother wavelet of Continuous Wavelet (Origin's `type`).
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum CwtWaveletArg {
    Morlet,
    /// Derivative of Gaussian.
    Dgauss,
    Mexhat,
}

/// Wavelet threshold rule of Denoise (Origin's `thrt`).
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum WaveletThresholdArg {
    Custom,
    Sqtwolog,
}

/// Trace interpolation method (Origin's names).
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum TraceMethodArg {
    Linear,
    Spline,
    Bspline,
}

/// What each STFT value is.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum StftOutputArg {
    Amplitude,
    Db,
}

/// Plotting positions of a probability or Q-Q plot.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum ScoresArg {
    Filliben,
    Blom,
    Benard,
    Hazen,
    VanDerWaerden,
    KaplanMeier,
}

/// FFT window (Origin's names).
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum FftWindowArg {
    Rectangle,
    Welch,
    Triangular,
    Bartlett,
    Hanning,
    Hamming,
    Blackman,
}

/// FFT window correction.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum FftCorrectionArg {
    None,
    Amplitude,
    Power,
}

/// FFT power normalization: mean square, sum square or time-interval square amplitude.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum FftPowerArg {
    Msa,
    Ssa,
    Tisa,
}

/// The row given for a level crossing between two rows (Origin's `type`).
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum CrossingRow {
    /// The row whose X is nearest, the smaller X on a tie.
    Any,
    Left,
    Right,
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

/// Origin's Averaged X choices of Average Multiple Curves, and Sheaf's `any`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum AverageX {
    /// Same as Source X: curves that share an X column are averaged row by row (otherwise every X
    /// value of the curves over the range they all cover).
    Same,
    /// Common X Range: evenly spaced X over the range every curve covers.
    Common,
    /// Full X Range (Origin's default): evenly spaced X from the smallest to the largest X, each
    /// curve continued past its ends.
    Full,
    /// Evenly spaced X from the smallest to the largest X, each curve counted only where it has data.
    Any,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum AverageInterp {
    Linear,
    Spline,
    Akima,
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
    /// Origin's User Defined baseline: anchor points found by the 2nd derivative (at most
    /// --baseline-points of them, 8 when not given), joined by straight lines.
    SecondDerivative,
    /// Origin's Constant baseline at the minimum (its default), maximum, mean or median of the data.
    Minimum,
    Maximum,
    Mean,
    Median,
    /// Origin's Asymmetric Least Squares Smoothing (--als-asymmetric, --als-threshold,
    /// --als-smoothing, --als-iterations).
    Als,
}

/// Origin's Peak Analyzer goals that only make the baseline (analysis-tools 11.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum BaselineGoalKind {
    Create,
    Subtract,
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
    /// Origin's Voigt: a Gauss of FWHM wG convolved with a Lorentz of FWHM wL.
    Voigt,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum PeakFitBaselineKind {
    Constant,
    Linear,
}

#[derive(Subcommand, Debug)]
pub enum AnalyzeTool {
    /// Charge of each sweep of a cyclic voltammogram (X potential, Y current): the sweeps between turns
    /// of the potential, anodic and cathodic, each with integral I dE / scan rate, and the totals.
    CvCharge {
        #[command(flatten)]
        target: TargetArgs,
        /// Scan rate in potential units per second (V/s for a potential in V).
        #[arg(long)]
        scan_rate: f64,
        /// Subtract the straight line through each sweep's first and last point first.
        #[arg(long)]
        baseline: bool,
    },
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
    /// Combine each column with a constant or another column row by row, without X values
    /// (Origin's Simple Column Math): rows past the end of the operand column count as 0. The
    /// results go into new columns beside the input.
    ColumnMath {
        #[command(flatten)]
        target: TargetArgs,
        #[arg(long, value_enum, default_value_t = MathKind::Add)]
        op: MathKind,
        #[command(flatten)]
        operand: OperandArgs,
    },
    /// Convolve each column with a response column (Origin's Signal Processing > Convolution):
    /// the full linear convolution, or circular; X steps of the input's X spacing unless
    /// --interval is given. The results go to a new sheet.
    Convolution {
        #[command(flatten)]
        target: TargetArgs,
        /// The response column (short or long name).
        #[arg(long)]
        response: String,
        #[arg(long)]
        circular: bool,
        /// Divide the response by its Euclidean norm first.
        #[arg(long)]
        normalize: bool,
        /// Sampling interval of the result.
        #[arg(long)]
        interval: Option<f64>,
    },
    /// Deconvolve each column by a response column (Origin's Signal Processing > Deconvolution):
    /// the inverse of the convolution, done with the FFT, as many values as the column; X steps of
    /// the input's X spacing unless --interval is given. The results go to a new sheet.
    Deconvolution {
        #[command(flatten)]
        target: TargetArgs,
        /// The response column (short or long name), not longer than the signal.
        #[arg(long)]
        response: String,
        /// Divide the response by its Euclidean norm first.
        #[arg(long)]
        normalize: bool,
        /// Circular deconvolution (the FFT over the signal's points) instead of the default linear one.
        #[arg(long)]
        circular: bool,
        /// Sampling interval of the result.
        #[arg(long)]
        interval: Option<f64>,
    },
    /// The short-time Fourier transform of each column (Origin's Signal Processing > STFT): the
    /// amplitude spectrum of overlapping windowed segments, a frequency column and one column per
    /// segment in a new sheet.
    Stft {
        #[command(flatten)]
        target: TargetArgs,
        /// Sampling interval; by default the X step of the column (1 without an X column).
        #[arg(long)]
        interval: Option<f64>,
        #[arg(long, value_enum, default_value_t = FftWindowArg::Hanning)]
        window: FftWindowArg,
        /// Points in each segment (Origin's default 256).
        #[arg(long, default_value_t = 256)]
        window_len: usize,
        /// Points of each transform (at least the window length; zeros are padded).
        #[arg(long, default_value_t = 256)]
        fft_len: usize,
        /// Points shared by neighbouring segments.
        #[arg(long, default_value_t = 128)]
        overlap: usize,
        /// Amplitude in dB by default, as Origin.
        #[arg(long, value_enum, default_value_t = StftOutputArg::Db)]
        output: StftOutputArg,
    },
    /// Correlate each column with another column (Origin's Signal Processing > Correlation):
    /// lags -(N-1) to N-1, or 0 to N-1 when circular. The results go to a new sheet.
    SignalCorrelation {
        #[command(flatten)]
        target: TargetArgs,
        /// The second signal (may be the same column for an autocorrelation).
        #[arg(long = "with")]
        other: String,
        #[arg(long)]
        circular: bool,
        /// Divide by the square root of the product of the two sums of squares.
        #[arg(long)]
        normalize: bool,
        /// Lag step of the result.
        #[arg(long, default_value_t = 1.0)]
        interval: f64,
    },
    /// The FFT of each column (Origin's Signal Processing > FFT > FFT): frequency, real,
    /// imaginary, magnitude, amplitude, phase (degrees), power and dB in a new sheet.
    Fft {
        #[command(flatten)]
        target: TargetArgs,
        /// Sampling interval; by default the X step of the column (1 without an X column).
        #[arg(long)]
        interval: Option<f64>,
        #[arg(long, value_enum, default_value_t = FftWindowArg::Rectangle)]
        window: FftWindowArg,
        #[arg(long, value_enum, default_value_t = FftCorrectionArg::Amplitude)]
        correction: FftCorrectionArg,
        /// Divide the real and imaginary parts and the magnitude by the number of points.
        #[arg(long)]
        normalize: bool,
        /// All frequencies instead of 0 to the Nyquist frequency.
        #[arg(long)]
        two_sided: bool,
        /// Two-sided: keep the FFT order instead of putting zero in the middle.
        #[arg(long)]
        no_shift: bool,
        #[arg(long)]
        no_unwrap: bool,
        /// The +1 sign convention of the exponent.
        #[arg(long)]
        science: bool,
        #[arg(long, value_enum, default_value_t = FftPowerArg::Msa)]
        power: FftPowerArg,
    },
    /// The inverse FFT of each column as the real part of a spectrum (Origin's Signal Processing >
    /// FFT > IFFT): time, real, imaginary, amplitude and phase in a new sheet.
    Ifft {
        #[command(flatten)]
        target: TargetArgs,
        /// The column with the imaginary part (0 without).
        #[arg(long)]
        imaginary: Option<String>,
        /// Sampling interval of the spectrum; by default the X step of the column.
        #[arg(long)]
        interval: Option<f64>,
        /// The input is in FFT order (zero frequency first), not shifted.
        #[arg(long)]
        no_shift: bool,
        #[arg(long)]
        no_unwrap: bool,
        /// The +1 sign convention of the forward transform.
        #[arg(long)]
        science: bool,
    },
    /// Filter each column in the frequency domain (Origin's Signal Processing > FFT Filters), the
    /// result beside it. Frequencies left out take Origin's automatic values.
    FftFilter {
        #[command(flatten)]
        target: TargetArgs,
        #[arg(long = "type", value_enum, default_value_t = FftFilterArg::LowPass)]
        kind: FftFilterArg,
        /// Low and high pass.
        #[arg(long)]
        cutoff: Option<f64>,
        /// Band pass and block: the lower and upper cutoff.
        #[arg(long)]
        freq1: Option<f64>,
        #[arg(long)]
        freq2: Option<f64>,
        /// Threshold filter: the smallest magnitude kept.
        #[arg(long)]
        threshold: Option<f64>,
        /// Parabolic low pass: pass and stop frequencies.
        #[arg(long)]
        pass: Option<f64>,
        #[arg(long)]
        stop: Option<f64>,
        /// High pass, band pass and block: remove the zero frequency (the mean) too.
        #[arg(long)]
        no_offset: bool,
        /// Sampling interval; by default the X step of the column.
        #[arg(long)]
        interval: Option<f64>,
    },
    /// Design an IIR filter and apply it (Origin's Signal Processing > IIR Filter, OriginPro), the
    /// result beside each column. Frequencies left out take Sheaf's defaults (a quarter of the Nyquist
    /// frequency; documented, not measured). `--minimum` takes the order from the pass and stop
    /// frequencies and the ripple and attenuation.
    #[command(alias = "iir")]
    IirFilter {
        #[command(flatten)]
        target: TargetArgs,
        #[arg(long = "type", value_enum, default_value_t = FilterTypeArg::LowPass)]
        kind: FilterTypeArg,
        #[arg(long, value_enum, default_value_t = IirMethodArg::Butterworth)]
        method: IirMethodArg,
        /// Filter order.
        #[arg(long, default_value_t = 10)]
        order: usize,
        /// The smallest order that meets the pass and stop frequencies (`--order` is ignored).
        #[arg(long)]
        minimum: bool,
        #[arg(long, value_enum, default_value_t = FreqUnitArg::Hz)]
        unit: FreqUnitArg,
        /// Sample frequency in Hz; by default 1 / the X step of the column.
        #[arg(long)]
        fs: Option<f64>,
        /// Cutoff frequency (low and high pass) or the first cutoff (band pass and band stop). Butterworth: the
        /// -3 dB point; Chebyshev I and elliptic: the passband edge (gain down by --ripple); Chebyshev II: the
        /// stopband edge (gain down by --attenuation).
        #[arg(long)]
        cutoff: Option<f64>,
        /// The second cutoff of band pass and band stop.
        #[arg(long)]
        cutoff2: Option<f64>,
        /// Minimum order: pass and stop frequencies (the first of two for the band responses).
        #[arg(long)]
        pass: Option<f64>,
        #[arg(long)]
        pass2: Option<f64>,
        #[arg(long)]
        stop: Option<f64>,
        #[arg(long)]
        stop2: Option<f64>,
        /// Passband ripple in dB (Chebyshev I, elliptic, minimum order).
        #[arg(long, default_value_t = 1.0)]
        ripple: f64,
        /// Stopband attenuation in dB (Chebyshev II, elliptic, minimum order).
        #[arg(long, default_value_t = 80.0)]
        attenuation: f64,
        /// Filter forward and backward: zero phase.
        #[arg(long)]
        forward_backward: bool,
    },
    /// Design an FIR filter and apply it (Origin 2027's FIR Filter app), the result beside each column,
    /// aligned with the input. The Window method takes `--cutoff` (or the pass and stop frequencies
    /// with `--minimum`), Least Squares the pass and stop frequencies.
    #[command(alias = "fir")]
    FirFilter {
        #[command(flatten)]
        target: TargetArgs,
        #[arg(long = "type", value_enum, default_value_t = FilterTypeArg::LowPass)]
        kind: FilterTypeArg,
        #[arg(long, value_enum, default_value_t = FirMethodArg::Window)]
        method: FirMethodArg,
        /// Filter order (one less than the number of coefficients); high pass and band stop need an even one.
        #[arg(long, default_value_t = 20)]
        order: usize,
        /// Window method: the order from the Kaiser formula for the pass and stop frequencies.
        #[arg(long)]
        minimum: bool,
        #[arg(long, value_enum, default_value_t = FirWindowArg::Hamming)]
        window: FirWindowArg,
        /// Parameter of the Chebyshev, Gaussian, Kaiser, Taylor and Tukey windows.
        #[arg(long)]
        window_param: Option<f64>,
        /// Do not scale the passband centre to gain 1.
        #[arg(long)]
        no_scale: bool,
        #[arg(long, value_enum, default_value_t = FreqUnitArg::Hz)]
        unit: FreqUnitArg,
        /// Sample frequency in Hz; by default 1 / the X step of the column.
        #[arg(long)]
        fs: Option<f64>,
        #[arg(long)]
        cutoff: Option<f64>,
        #[arg(long)]
        cutoff2: Option<f64>,
        #[arg(long)]
        pass: Option<f64>,
        #[arg(long)]
        pass2: Option<f64>,
        #[arg(long)]
        stop: Option<f64>,
        #[arg(long)]
        stop2: Option<f64>,
        /// Least Squares weights of the passbands and the stopbands.
        #[arg(long, default_value_t = 1.0)]
        pass_weight: f64,
        #[arg(long, default_value_t = 1.0)]
        stop_weight: f64,
        /// Unit of `--ripple` and `--attenuation` (minimum order).
        #[arg(long, value_enum, default_value_t = MagnitudeUnitArg::Db)]
        magnitude_unit: MagnitudeUnitArg,
        #[arg(long, default_value_t = 1.0)]
        ripple: f64,
        #[arg(long, default_value_t = 60.0)]
        attenuation: f64,
        /// Filter forward and backward: zero phase.
        #[arg(long)]
        forward_backward: bool,
    },
    /// The upper and/or lower envelope of each column (Origin's Signal Processing > Envelope): the
    /// natural cubic spline through the extreme points, each envelope with its X in a new sheet.
    Envelope {
        #[command(flatten)]
        target: TargetArgs,
        #[arg(long = "type", value_enum, default_value_t = EnvelopeArg::Upper)]
        kind: EnvelopeArg,
        /// Points on each side of an extreme point (cut to a whole number, at least 1).
        #[arg(long, default_value_t = 2.0)]
        npts: f64,
    },
    /// The magnitude-squared coherence of each column with a second one (Origin's Signal
    /// Processing > Coherence): frequency and coherence in a new sheet.
    Coherence {
        #[command(flatten)]
        target: TargetArgs,
        /// The second signal (may be the same column).
        #[arg(long = "with")]
        other: String,
        /// FFT length: each segment is zero padded to it.
        #[arg(long, default_value_t = 256)]
        fft_len: usize,
        /// Segment length, at most the FFT length.
        #[arg(long, default_value_t = 256)]
        win_len: usize,
        /// Points shared by neighbouring segments, less than the window length.
        #[arg(long, default_value_t = 128)]
        overlap: usize,
        #[arg(long, value_enum, default_value_t = CoherenceWindowArg::Rectangle)]
        window: CoherenceWindowArg,
        /// Gaussian window: the reciprocal of the standard deviation.
        #[arg(long, default_value_t = 0.0)]
        alpha: f64,
        /// Kaiser window: the beta parameter.
        #[arg(long, default_value_t = 0.0)]
        beta: f64,
        /// Sampling interval (Origin: 1 unless given, not taken from the X column).
        #[arg(long, default_value_t = 1.0)]
        interval: f64,
    },
    /// Keep every n-th point of each column (Origin's Signal Processing > Decimation), with its X,
    /// in a new sheet. A factor of 3 or more needs ten times as many points as the factor.
    Decimate {
        #[command(flatten)]
        target: TargetArgs,
        /// Keep every n-th point.
        #[arg(long, short = 'd', default_value_t = 2)]
        factor: usize,
        #[arg(long, value_enum, default_value_t = DecimateFilterArg::None)]
        filter: DecimateFilterArg,
        /// FIR filter: half width of the smoothing, 8 to 60.
        #[arg(long, default_value_t = 30)]
        order: usize,
    },
    /// One level of the discrete wavelet transform of each column (Origin's Signal Processing >
    /// Wavelet > Decompose, `dwt`): approximation and detail coefficients in a new sheet.
    WaveletDecompose {
        #[command(flatten)]
        target: TargetArgs,
        /// haar, db2..db10, b11, b13, b15, b22, b24, b26, b28, b31, b33, b35, b37.
        #[arg(long, default_value = "haar")]
        wavelet: String,
        #[arg(long, value_enum, default_value_t = WaveletExtArg::Periodic)]
        ext: WaveletExtArg,
    },
    /// Rebuild a signal from approximation coefficients (the column) and detail coefficients
    /// (Origin's Wavelet > Reconstruction, `idwt`).
    WaveletReconstruct {
        #[command(flatten)]
        target: TargetArgs,
        /// The column of detail coefficients.
        #[arg(long = "with")]
        detail: String,
        #[arg(long, default_value = "haar")]
        wavelet: String,
        #[arg(long, value_enum, default_value_t = WaveletExtArg::Periodic)]
        ext: WaveletExtArg,
    },
    /// Continuous wavelet transform (Origin's Wavelet > Continuous Wavelet, `cwt`): one column of
    /// coefficients per scale in a new sheet CWTResults1.
    #[command(name = "cwt")]
    ContinuousWavelet {
        #[command(flatten)]
        target: TargetArgs,
        /// The column of scales (each cut to its integer part, 1 or more).
        #[arg(long = "with")]
        scales: String,
        #[arg(long, value_enum, default_value_t = CwtWaveletArg::Morlet)]
        wavelet: CwtWaveletArg,
        /// Wave number of the Morlet (5 to 20) or order of the DGauss (1 to 8).
        #[arg(long, default_value_t = 5.0)]
        param: f64,
    },
    /// Wavelet denoising (Origin's Wavelet > Denoise, `wtdenoise`): hard threshold of the detail
    /// coefficients of levels 1 to LEVEL, X and the denoised Y in a new sheet.
    WaveletDenoise {
        #[command(flatten)]
        target: TargetArgs,
        #[arg(long, default_value = "haar")]
        wavelet: String,
        #[arg(long, value_enum, default_value_t = WaveletExtArg::Periodic)]
        ext: WaveletExtArg,
        #[arg(long, value_enum, default_value_t = WaveletThresholdArg::Custom)]
        threshold: WaveletThresholdArg,
        /// Levels to threshold, 1 to 7.
        #[arg(long, default_value_t = 1)]
        level: usize,
        /// Threshold of every level in percent of the largest coefficient (custom only).
        #[arg(long, default_value_t = 50.0)]
        percent: f64,
    },
    /// Wavelet smoothing (Origin's Wavelet > Smooth, `wtsmooth`): the last CUTOFF percent of the
    /// detail coefficients are dropped; X and the smoothed Y in a new sheet.
    WaveletSmooth {
        #[command(flatten)]
        target: TargetArgs,
        #[arg(long, default_value = "haar")]
        wavelet: String,
        #[arg(long, value_enum, default_value_t = WaveletExtArg::Periodic)]
        ext: WaveletExtArg,
        /// Cutoff in percent.
        #[arg(long, default_value_t = 20.0)]
        cutoff: f64,
    },
    /// Resample each curve at evenly spaced points along its path (Origin's Mathematics > Trace
    /// Interpolation), for curves whose X is not monotonic such as a CV loop. The results, with
    /// their own X, go to a new sheet.
    TraceInterp {
        #[command(flatten)]
        target: TargetArgs,
        #[arg(long, value_enum, default_value_t = TraceMethodArg::Linear)]
        method: TraceMethodArg,
        /// Number of points of the result.
        #[arg(long, default_value_t = 100)]
        points: usize,
    },
    /// The Hilbert transform of each column (Origin's Signal Processing > Hilbert Transform), in a
    /// new column beside it.
    Hilbert {
        #[command(flatten)]
        target: TargetArgs,
    },
    /// Where each curve crosses a level (Origin's levelcrossing): the crossing X values in a column
    /// CrossingX<n> beside the curve, as Origin; --details adds the direction, row and Y.
    LevelCrossing {
        #[command(flatten)]
        target: TargetArgs,
        #[arg(long, default_value_t = 0.0, allow_hyphen_values = true)]
        level: f64,
        /// The row given for a crossing between two rows.
        #[arg(long, value_enum, default_value_t = CrossingRow::Any)]
        row: CrossingRow,
        /// Drop both crossings of a pair whose rows are closer than this.
        #[arg(long, default_value_t = 1)]
        min_gap: usize,
        /// Also write the direction of each crossing (1 up, -1 down, 0 on the level without
        /// passing through), its row and its Y.
        #[arg(long)]
        details: bool,
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
        /// Lowess and Loess: the share of the points in each local fit, 0 to 1 (default 0.1);
        /// with `--span 0` the window is `--points` points.
        #[arg(long, default_value_t = 0.1)]
        span: f64,
        /// Binomial filter: how many times the 1 2 1 / 4 filter is applied.
        #[arg(long, default_value_t = 1)]
        order: usize,
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
        /// The reference column for --method ref or cell (short or long name, same sheet).
        #[arg(long = "ref", value_name = "COLUMN")]
        reference: Option<String>,
        /// The statistic --method ref matches (Origin's default: min).
        #[arg(long, value_enum, default_value_t = RefStatArg::Min)]
        stat: RefStatArg,
        /// The row of the reference cell for --method cell, counted from 1.
        #[arg(long, default_value_t = 1)]
        row: usize,
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
    /// Interpolate each curve onto equally spaced X values; each result is a column at the end of the
    /// sheet, its X grid kept as the column's sampling interval (as Origin's interp1xy).
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
        /// 0 chooses the factor from the data as Origin does.
        #[arg(long, default_value_t = 0.0)]
        smoothing: f64,
    },
    /// Average the chosen curves on a common X: mean, standard deviation and count go to a new sheet.
    /// Curves with the same X values are averaged row by row; otherwise each curve's X must rise or
    /// fall all the way (use --rows for one sweep of a cyclic voltammogram).
    Average {
        #[command(flatten)]
        target: TargetArgs,
        /// Which X values (Origin's Averaged X; full by default).
        #[arg(long, value_enum, default_value_t = AverageX::Full)]
        x: AverageX,
        /// Number of evenly spaced X values (default Origin's Auto: five times the mean number of
        /// points per curve, at most 1000).
        #[arg(long, conflicts_with = "reference_curve")]
        points: Option<usize>,
        /// Evaluate at the X values of this curve, counted from 1 among the chosen columns.
        #[arg(long)]
        reference_curve: Option<usize>,
        #[arg(long, value_enum, default_value_t = AverageInterp::Linear)]
        method: AverageInterp,
        /// Add the standard deviation column (Origin's Std Dev).
        #[arg(long)]
        sd: bool,
        /// Add the standard error column.
        #[arg(long)]
        se: bool,
        /// Add the count column (Origin's N).
        #[arg(long)]
        count: bool,
        /// Add the minimum column.
        #[arg(long)]
        min: bool,
        /// Add the maximum column.
        #[arg(long)]
        max: bool,
    },
    /// Find where two curves meet; X and Y of each point go to a new sheet. Needs exactly two curves.
    Intersect {
        #[command(flatten)]
        target: TargetArgs,
        /// Also list the points where the curves touch without crossing.
        #[arg(long)]
        touching: bool,
    },
    /// The area enclosed by each curve taken as a closed polygon (last point joined to the first):
    /// area, signed area, centroid, perimeter and whether it crosses itself.
    PolygonArea {
        #[command(flatten)]
        target: TargetArgs,
    },
    /// The area of the surface through scattered X, Y, Z points (Delaunay triangulation of X and Y, sum of the 3D
    /// triangle areas; rows with the same X and Y are merged with the mean Z). Name the three columns with --y X,Y,Z
    /// in this order (default: the sheet's first Z column with its Y and X). One row: area, projected area, points,
    /// triangles, merged and skipped rows.
    XyzArea {
        #[command(flatten)]
        target: TargetArgs,
    },
    /// Subtract the straight line through two points from each curve. The points are X values;
    /// their Y values are read from the curve (straight lines between data points) unless
    /// --y1 / --y2 give them.
    SubtractLine {
        #[command(flatten)]
        target: TargetArgs,
        #[arg(long, allow_hyphen_values = true)]
        x1: f64,
        #[arg(long, allow_hyphen_values = true)]
        x2: f64,
        /// Y of the first point (default: the curve's value at --x1).
        #[arg(long, allow_hyphen_values = true)]
        y1: Option<f64>,
        /// Y of the second point (default: the curve's value at --x2).
        #[arg(long, allow_hyphen_values = true)]
        y2: Option<f64>,
    },
    /// Reduce Duplicate X Data: points whose X values are within the tolerance of their neighbour
    /// become one point; the result goes to a new sheet.
    ReduceDup {
        #[command(flatten)]
        target: TargetArgs,
        /// What replaces the Y values of a group (Origin's rms gives the mean, as measured).
        #[arg(long, value_enum, default_value_t = DupKind::Mean)]
        method: DupKind,
        /// Tolerance as a fraction of the X range (Origin's default 1e-8).
        #[arg(long, default_value_t = 1e-8)]
        tol: f64,
        /// Also write the X values that had duplicates and how many points each had.
        #[arg(long)]
        count: bool,
    },
    /// Reduce to Evenly Spaced X as Origin's reduce_ex: every N points (--every, 5 by default),
    /// or intervals of X (--step, --intervals or --frequency); the result goes to a new sheet.
    ReduceX {
        #[command(flatten)]
        target: TargetArgs,
        /// One point for every N points (Origin's Every N Points; 5 by default).
        #[arg(long, group = "grid")]
        every: Option<usize>,
        /// Intervals of this width in X (By Sampling Interval).
        #[arg(long, group = "grid")]
        step: Option<f64>,
        /// This many intervals over the X range (By Number of Intervals).
        #[arg(long, group = "grid")]
        intervals: Option<usize>,
        /// Intervals of width 1 / frequency (By Frequency).
        #[arg(long, group = "grid")]
        frequency: Option<f64>,
        /// mean (default): the mean of each group; first: its first point.
        #[arg(long, value_enum, default_value_t = ReduceKind::Mean)]
        method: ReduceKind,
        /// X Start of the intervals (default: the smallest X of the data).
        #[arg(long, allow_hyphen_values = true)]
        x_min: Option<f64>,
        /// X End of the intervals (default: the largest X of the data); a point there is left out.
        #[arg(long, allow_hyphen_values = true)]
        x_max: Option<f64>,
        /// Every N points: X steps further than this (percent) from even make the data be
        /// interpolated onto even X first.
        #[arg(long, default_value_t = 5.0)]
        tolerance: f64,
    },
    /// Translate each curve in X (a new sheet) or in Y (a new column): by --by, or so that the
    /// point at X = --at lands on --to.
    Translate {
        #[command(flatten)]
        target: TargetArgs,
        #[arg(long, value_enum, default_value_t = AxisKind::X)]
        axis: AxisKind,
        /// Shift by this amount.
        #[arg(long, allow_hyphen_values = true, group = "shift")]
        by: Option<f64>,
        /// X of the point to move (with --to).
        #[arg(long, allow_hyphen_values = true, requires = "to")]
        at: Option<f64>,
        /// Where the point lands: its new X (axis x) or its new Y value (axis y).
        #[arg(long, allow_hyphen_values = true, group = "shift", requires = "at")]
        to: Option<f64>,
    },
    /// Running median of each curve (Origin's percentile filter at 50 %).
    MedianFilter {
        #[command(flatten)]
        target: TargetArgs,
        /// Points of the window; an even number acts like the next odd one, as in Origin.
        #[arg(long, default_value_t = 5)]
        points: usize,
        /// shrink (default): the window shrinks symmetrically at the ends; repeat or reflect fill it.
        #[arg(long, value_enum, default_value_t = MedianEdgeKind::Shrink)]
        edge: MedianEdgeKind,
    },
    /// Fourier self-deconvolution: narrows the bands of each curve K times so that overlapping peaks
    /// separate (Sheaf's own; X evenly spaced). The result goes beside the curve.
    Fsd {
        #[command(flatten)]
        target: TargetArgs,
        /// Full width at half maximum of the bands, in X units.
        #[arg(long)]
        fwhm: f64,
        /// Resolution enhancement factor K (above 1).
        #[arg(long, default_value_t = 2.0)]
        k: f64,
        /// The apodization that ends the deconvolution.
        #[arg(long, value_enum, default_value_t = ApodizationKind::TriangleSquared)]
        apodization: ApodizationKind,
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
        /// second-derivative (Origin's User Defined anchors), or anchors (--anchors).
        #[arg(long, value_enum)]
        baseline: Option<PeakBaselineKind>,
        /// Number of anchor points of the minima (default 5) and second-derivative (default 8)
        /// baselines.
        #[arg(long)]
        baseline_points: Option<usize>,
        /// Only make the baseline (Origin's Peak Analyzer goals Create Baseline and Subtract
        /// Baseline): Baseline_Data Y<n> beside each curve, after Subtracted_Data Y<n> for
        /// subtract. Without --baseline, Origin's User Defined (second-derivative) baseline.
        #[arg(long, value_enum)]
        baseline_goal: Option<BaselineGoalKind>,
        /// Anchor points of the anchors baseline, written X1:Y1,X2:Y2,... (linear interpolation).
        #[arg(long)]
        anchors: Option<String>,
        /// Piecewise peak fitting: run the analysis (baseline, search, integration, fit) separately in
        /// each X region, written FROM:TO,FROM:TO,...; each region has its own baseline.
        #[arg(long, value_name = "REGIONS")]
        regions: Option<String>,
        /// Search on the Fourier self-deconvolved curve with bands of this FWHM (X units), so that
        /// hidden peaks are found; heights and areas are measured on the data. Needs evenly spaced X.
        #[arg(long, value_name = "FWHM")]
        fsd_fwhm: Option<f64>,
        /// Enhancement factor K of the --fsd-fwhm search.
        #[arg(long, default_value_t = 2.0, value_name = "K")]
        fsd_k: f64,
        /// ALS baseline: Asymmetric Factor, between 0 and 1 (close to 0 for positive peaks).
        #[arg(long, default_value_t = 0.001)]
        als_asymmetric: f64,
        /// ALS baseline: Threshold, the fraction of the largest distance above which a point counts
        /// as part of a peak.
        #[arg(long, default_value_t = 0.05)]
        als_threshold: f64,
        /// ALS baseline: Smoothing Factor, the base-10 logarithm of the smoothness penalty.
        #[arg(long, default_value_t = 5.0)]
        als_smoothing: f64,
        /// ALS baseline: Number of Iterations.
        #[arg(long, default_value_t = 10)]
        als_iterations: usize,
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
        /// A function per peak, in the order found, comma-separated (for example gauss,voigt);
        /// peaks beyond the list take the --fit function.
        #[arg(long, value_enum, value_delimiter = ',', requires = "fit")]
        fit_shapes: Vec<PeakFitKind>,
    },
    /// Descriptive statistics of each column. By default N, missing, mean, SD, SEM, sum, minimum, quartiles
    /// and maximum; --all adds IQR, variance, skewness, kurtosis, CV and the confidence limits of the mean.
    Stats {
        #[command(flatten)]
        target: TargetArgs,
        /// Also IQR, variance, skewness, kurtosis, coefficient of variation and confidence limits.
        #[arg(long)]
        all: bool,
        /// Add these quantities (comma separated): iqr, variance, skewness, kurtosis, cv, limits.
        #[arg(long, value_delimiter = ',')]
        add: Vec<String>,
        /// Confidence level of the limits of the mean.
        #[arg(long, default_value_t = 0.95)]
        level: f64,
        /// Percentiles in percent (comma separated, for example 5,95): one column each, P5, P95, ...
        #[arg(long, value_delimiter = ',')]
        percentiles: Vec<f64>,
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
        #[command(flatten)]
        plot: ComparisonPlotArgs,
    },
    /// Test of variance: each column against a hypothesized variance (chi-square, --variance) or two columns
    /// as a ratio of variances (F test, --ratio, first over second). Two --y columns, or --ratio, make the
    /// two-sample test. --levels (percent, comma-separated) adds the confidence intervals.
    Vartest {
        #[command(flatten)]
        target: TargetArgs,
        /// One-sample test: the hypothesized variance (default 1).
        #[arg(long, conflicts_with = "ratio")]
        variance: Option<f64>,
        /// Two-sample test: the hypothesized ratio of the variances (default 1).
        #[arg(long)]
        ratio: Option<f64>,
        #[arg(long, value_enum, default_value_t = TailArg::Two)]
        tail: TailArg,
        /// Significance level.
        #[arg(long, default_value_t = 0.05)]
        alpha: f64,
        /// Confidence levels in percent, for example 90,95,99; none means no intervals.
        #[arg(long, value_delimiter = ',')]
        levels: Vec<f64>,
    },
    /// Test of a proportion from counts (Origin's Summarized input): --successes and --size once for the
    /// one-sample test against --proportion, twice (or comma-separated) for the two-sample test of the
    /// difference against --difference. The data file only names the workbook that receives the result sheet.
    Proptest {
        #[command(flatten)]
        target: TargetArgs,
        /// Number of successes of each sample.
        #[arg(long, value_delimiter = ',', required = true)]
        successes: Vec<u64>,
        /// Sample size of each sample.
        #[arg(long, value_delimiter = ',', required = true)]
        size: Vec<u64>,
        /// One-sample test: the test proportion.
        #[arg(long, default_value_t = 0.5)]
        proportion: f64,
        /// Two-sample test: the hypothesized difference of the proportions (first minus second).
        #[arg(long, default_value_t = 0.0, allow_hyphen_values = true)]
        difference: f64,
        #[arg(long, value_enum, default_value_t = TailArg::Two)]
        tail: TailArg,
        /// Significance level.
        #[arg(long, default_value_t = 0.05)]
        alpha: f64,
        /// Also the exact test (binomial; Fisher's for two samples).
        #[arg(long)]
        exact: bool,
        /// Only the exact test, not the normal approximation.
        #[arg(long, requires = "exact")]
        exact_only: bool,
        /// Two-sample test: take the standard error of z from the pooled proportion (Origin's "Use pooled p for
        /// test", off by default).
        #[arg(long)]
        pooled: bool,
        /// Confidence levels in percent, for example 90,95,99; none means no intervals.
        #[arg(long, value_delimiter = ',')]
        levels: Vec<f64>,
        /// Power analysis: the power at the observed sample sizes (Origin's "Actual Power").
        #[arg(long)]
        power: bool,
        /// Power analysis: also the power at these sample sizes, for example 50,100,200 (Origin's "Hypothetical
        /// Power"; per sample for the two-sample test).
        #[arg(long, value_delimiter = ',')]
        power_sizes: Vec<u64>,
        /// Significance level of the power analysis (separate from --alpha, as in Origin).
        #[arg(long, default_value_t = 0.05)]
        power_alpha: f64,
    },
    /// Poisson rate test from counts (Origin 2026b, Summarized input; T267): --total and --size once for
    /// the one-sample test against --rate, twice (or comma-separated) for the two-sample test of the
    /// difference of the rates against --difference. The data file only names the workbook that receives
    /// the result sheet.
    Poissontest {
        #[command(flatten)]
        target: TargetArgs,
        /// Total occurrence (number of events) of each sample.
        #[arg(long, value_delimiter = ',', required = true)]
        total: Vec<u64>,
        /// Sample size (number of observations) of each sample.
        #[arg(long, value_delimiter = ',', required = true)]
        size: Vec<u64>,
        /// Exposure (length of each observation: time, area, ...) of each sample; default 1.
        #[arg(long, value_delimiter = ',')]
        exposure: Vec<f64>,
        /// One-sample test: the test rate.
        #[arg(long, default_value_t = 1.0)]
        rate: f64,
        /// Two-sample test: the hypothesized difference of the rates (first minus second).
        #[arg(long, default_value_t = 0.0, allow_hyphen_values = true)]
        difference: f64,
        #[arg(long, value_enum, default_value_t = TailArg::Two)]
        tail: TailArg,
        /// Significance level.
        #[arg(long, default_value_t = 0.05)]
        alpha: f64,
        /// Leave out the exact test.
        #[arg(long)]
        no_exact: bool,
        /// Leave out the normal approximation.
        #[arg(long, conflicts_with = "no_exact")]
        no_normal: bool,
        /// Two-sample test: the unpooled standard error even for a hypothesized difference of 0.
        #[arg(long)]
        unpooled: bool,
        /// Confidence levels in percent, for example 90,95,99; none means no intervals.
        #[arg(long, value_delimiter = ',')]
        levels: Vec<f64>,
    },
    /// One-way ANOVA: every chosen column is a group. The result (group statistics, ANOVA table,
    /// means comparison of all pairs) goes to a new sheet.
    Anova {
        #[command(flatten)]
        target: TargetArgs,
        #[arg(long, value_enum, default_value_t = ComparisonArg::Tukey)]
        comparison: ComparisonArg,
        /// Dunnett's control group: its number among the --y columns, counted from 1.
        #[arg(long, default_value_t = 1)]
        control: usize,
        /// Significance level.
        #[arg(long, default_value_t = 0.05)]
        alpha: f64,
        #[command(flatten)]
        plot: ComparisonPlotArgs,
    },
    /// Power or sample size of a t test or the one-way ANOVA (Origin's Power and Sample Size tools). Needs no
    /// data file: give --sizes to get the power of those sample sizes, or --powers to get the smallest sample
    /// sizes (per group for two samples and the ANOVA) that reach those powers.
    Power {
        #[arg(long, value_enum, default_value_t = PowerTestArg::TwoSample)]
        test: PowerTestArg,
        /// Sample sizes to find the power of, comma-separated.
        #[arg(long, value_delimiter = ',', conflicts_with = "powers")]
        sizes: Vec<f64>,
        /// Wanted powers (between 0 and 1) to find sample sizes for, comma-separated.
        #[arg(long, value_delimiter = ',')]
        powers: Vec<f64>,
        /// t tests: the mean (first group), or the mean of the sample; with --mean2 the hypothesized or second
        /// mean (paired: the mean difference with --mean2 0).
        #[arg(long, default_value_t = 0.0, allow_negative_numbers = true)]
        mean1: f64,
        #[arg(long, default_value_t = 1.0, allow_negative_numbers = true)]
        mean2: f64,
        /// Standard deviation (pooled; of the differences when paired; the root of the error mean square for the ANOVA).
        #[arg(long, default_value_t = 1.0)]
        sd: f64,
        /// ANOVA: the number of groups.
        #[arg(long, default_value_t = 3.0)]
        levels: f64,
        /// ANOVA: the corrected sum of squares of the group means.
        #[arg(long, default_value_t = 1.0)]
        css: f64,
        /// Significance level.
        #[arg(long, default_value_t = 0.05)]
        alpha: f64,
        /// One-sided (upper) test instead of two-sided.
        #[arg(long)]
        one_sided: bool,
        /// Write the table to this file (CSV or text, by extension).
        #[arg(long)]
        export: Option<std::path::PathBuf>,
    },
    /// Acceptance sampling by attributes (T369): the single sampling plan with the smallest sample that
    /// meets both risks at the AQL and LTPD, or a plan given with --n and --c, with its OC curve; with --lot
    /// also AOQ, AOQL and ATI. Needs no data file. Fractions defective are given as fractions (0.01 is 1 %).
    Acceptance {
        /// Acceptable quality level.
        #[arg(long, default_value_t = 0.01)]
        aql: f64,
        /// Lot tolerance percent defective, as a fraction.
        #[arg(long, default_value_t = 0.05)]
        ltpd: f64,
        /// Producer's risk.
        #[arg(long, default_value_t = 0.05)]
        alpha: f64,
        /// Consumer's risk.
        #[arg(long, default_value_t = 0.10)]
        beta: f64,
        /// Evaluate this sample size (with --c) instead of designing a plan.
        #[arg(long, requires = "c")]
        n: Option<u64>,
        /// The acceptance number of the plan given with --n.
        #[arg(long, requires = "n")]
        c: Option<u64>,
        /// Lot size, for AOQ, AOQL and ATI.
        #[arg(long)]
        lot: Option<u64>,
        /// Write the OC curve table to this file (CSV or text, by extension).
        #[arg(long)]
        export: Option<std::path::PathBuf>,
        /// Save the project (the Acceptance Sampling book and its OC graph) to this file.
        #[arg(long)]
        save: Option<std::path::PathBuf>,
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
    /// Three-way ANOVA of indexed data: --y FACTOR_A,FACTOR_B,FACTOR_C,DATA names four columns, the first
    /// three holding the levels (text or numbers) of each row. Type III sums of squares as Origin's. The
    /// tables (descriptive statistics, ANOVA, means comparisons of A, B, C and A*B, optional Levene and power)
    /// go to a new sheet.
    Anova3 {
        #[command(flatten)]
        target: TargetArgs,
        /// Leave the interaction A*B out of the model.
        #[arg(long)]
        no_ab: bool,
        /// Leave the interaction A*C out of the model.
        #[arg(long)]
        no_ac: bool,
        /// Leave the interaction B*C out of the model.
        #[arg(long)]
        no_bc: bool,
        /// Leave the interaction A*B*C out of the model.
        #[arg(long)]
        no_abc: bool,
        /// Means comparison methods (comma-separated; `none` for no comparison). Dunnett is not offered.
        #[arg(long, value_enum, value_delimiter = ',', default_values_t = [ComparisonArg::Tukey])]
        comparison: Vec<ComparisonArg>,
        /// Significance level of the means comparisons.
        #[arg(long, default_value_t = 0.05)]
        comparison_alpha: f64,
        /// Levene's test of equal variances.
        #[arg(long)]
        levene: bool,
        /// Power of the F tests for the actual sample size.
        #[arg(long)]
        power: bool,
        /// Power for these hypothetical sample sizes (comma-separated, for example 50,100,200).
        #[arg(long, value_delimiter = ',')]
        power_sizes: Vec<f64>,
        /// Significance level of the power analysis.
        #[arg(long, default_value_t = 0.05)]
        power_alpha: f64,
        /// Leave the descriptive statistics out.
        #[arg(long)]
        no_descriptive: bool,
        /// Significance level of the F tests.
        #[arg(long, default_value_t = 0.05)]
        alpha: f64,
    },
    /// Repeated measures ANOVA of indexed data. One-way: --y FACTOR,DATA,SUBJECT. With --two-way:
    /// --y FACTOR_A,FACTOR_B,DATA,SUBJECT. Every subject needs one number in every cell. Within-subjects table
    /// with the Greenhouse-Geisser, Huynh-Feldt and lower-bound corrections, Mauchly's test, the one-way
    /// multivariate tests, the between-subjects rows and descriptive statistics go to a new sheet.
    Rmanova {
        #[command(flatten)]
        target: TargetArgs,
        /// Two repeated factors (Origin's Two-Way Repeated Measures ANOVA).
        #[arg(long)]
        two_way: bool,
        /// The A*B interaction of the two-way design (off by default, as in Origin).
        #[arg(long)]
        interaction: bool,
        /// One-way means comparison methods (comma-separated; `none` for no comparison; Tukey when not given).
        /// Not offered for the two-way design. Dunnett is not offered.
        #[arg(long, value_enum, value_delimiter = ',')]
        comparison: Vec<ComparisonArg>,
        /// Significance level of the means comparisons.
        #[arg(long, default_value_t = 0.05)]
        comparison_alpha: f64,
        /// Leave the descriptive statistics out.
        #[arg(long)]
        no_descriptive: bool,
        /// Significance level (confidence limits of the descriptive statistics and the verdicts).
        #[arg(long, default_value_t = 0.05)]
        alpha: f64,
    },
    /// Partial correlation coefficients (Pearson) of the --y columns, controlling for the --control columns.
    /// A table of pairs and the matrices go to a new sheet.
    PartialCorr {
        #[command(flatten)]
        target: TargetArgs,
        /// The controlling variables (columns of the same sheet, comma-separated; at least one).
        #[arg(long, value_delimiter = ',', required = true)]
        control: Vec<String>,
        /// Leave out every row where any column has no number (default: pair by pair).
        #[arg(long)]
        listwise: bool,
        /// One-tailed instead of two-tailed test.
        #[arg(long)]
        one_tailed: bool,
        /// Add the p-value and df blocks ("Show Significance in Result Table").
        #[arg(long)]
        show_significance: bool,
        /// Flag the coefficients that are significant at 0.05 with a `*`.
        #[arg(long)]
        flag_significant: bool,
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
    /// Statistics on Rows: per worksheet row the mean and standard deviation (and optionally the standard
    /// error, minimum, maximum, count and sum) across the chosen columns, as new columns behind the last one.
    Rowstats {
        #[command(flatten)]
        target: TargetArgs,
        /// Add these quantities (comma separated): sem, min, max, n, sum.
        #[arg(long, value_delimiter = ',')]
        add: Vec<String>,
        /// Leave out the mean and the standard deviation.
        #[arg(long)]
        no_mean_sd: bool,
    },
    /// Grubbs test for one outlier in each chosen column (the value farthest from the mean).
    Grubbs {
        #[command(flatten)]
        target: TargetArgs,
        /// Significance level (two-sided).
        #[arg(long, default_value_t = 0.05)]
        alpha: f64,
    },
    /// Dixon's Q test for one outlier in each chosen column of 3 to 10 numbers; the level is 0.1, 0.05 or 0.01.
    Dixon {
        #[command(flatten)]
        target: TargetArgs,
        /// Significance level: 0.1, 0.05 or 0.01.
        #[arg(long, default_value_t = 0.05)]
        alpha: f64,
    },
    /// Paired sign test of two columns. The alternative is about the first column.
    Sign {
        #[command(flatten)]
        target: TargetArgs,
        #[arg(long, value_enum, default_value_t = TailArg::Two)]
        tail: TailArg,
        /// Significance level.
        #[arg(long, default_value_t = 0.05)]
        alpha: f64,
    },
    /// Two-sample Kolmogorov-Smirnov test of two columns. The alternative is about the first column
    /// ("upper": its distribution function lies above the second one's).
    Ks2 {
        #[command(flatten)]
        target: TargetArgs,
        #[arg(long, value_enum, default_value_t = TailArg::Two)]
        tail: TailArg,
        /// Significance level.
        #[arg(long, default_value_t = 0.05)]
        alpha: f64,
    },
    /// Median test (Mood's): every chosen column is a group.
    Mediantest {
        #[command(flatten)]
        target: TargetArgs,
        /// Significance level.
        #[arg(long, default_value_t = 0.05)]
        alpha: f64,
    },
    /// Friedman ANOVA: every chosen column is a treatment, every row a block (no missing numbers).
    Friedman {
        #[command(flatten)]
        target: TargetArgs,
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
    /// Cross tabulation of two categorical columns (--y ROW_VARIABLE,COLUMN_VARIABLE; text or numbers):
    /// counts with row, column and total percentages, expected counts, Pearson and likelihood ratio
    /// chi-square and, for a 2 x 2 table, Fisher's exact test. The tables go to a new sheet.
    Crosstab {
        #[command(flatten)]
        target: TargetArgs,
        /// Leave out the percentages of the row totals.
        #[arg(long)]
        no_row_percent: bool,
        /// Leave out the percentages of the column totals.
        #[arg(long)]
        no_column_percent: bool,
        /// Leave out the percentages of the grand total.
        #[arg(long)]
        no_total_percent: bool,
        /// Leave out the expected counts.
        #[arg(long)]
        no_expected: bool,
        /// Level for the sentence about the tests.
        #[arg(long, default_value_t = 0.05)]
        alpha: f64,
    },
    /// Discrete frequency (Origin's discfreqs): how often each distinct value of each column occurs, text or
    /// numbers; empty and missing cells count as values of their own. Writes Data and Count, and with the
    /// options the relative and cumulative frequency, to a new sheet.
    Discfreq {
        #[command(flatten)]
        target: TargetArgs,
        /// Add the relative frequency.
        #[arg(long)]
        relative: bool,
        /// Add the cumulative (relative) frequency.
        #[arg(long)]
        cumulative: bool,
        /// Give the frequencies in percent instead of fractions.
        #[arg(long)]
        percent: bool,
        /// Leave the Count column out.
        #[arg(long)]
        no_count: bool,
        /// Tell "a" and "A" apart.
        #[arg(long)]
        case_sensitive: bool,
        /// Order: cdesc (descending counts, the default), casc, vasc (ascending values), vdesc.
        #[arg(long, default_value = "cdesc", value_parser = ["cdesc", "casc", "vasc", "vdesc"])]
        sort: String,
    },
    /// 2D frequency counts and binning (Origin's twoDBinning): the points of one Y column against its X
    /// column, or of two columns (--y X --y Y), counted in a grid of X and Y bins; one row per X bin and
    /// one column per Y bin go to a new sheet. Each axis takes a bin size or a number of bins, and a
    /// range; what is left out is chosen from the data as Origin does.
    #[command(name = "bin2d")]
    Bin2d {
        #[command(flatten)]
        target: TargetArgs,
        /// What each cell holds: count, freq (percent of the binned points), or sum, mean, min, max,
        /// median of the Y values.
        #[arg(long, default_value = "count", value_parser = ["count", "freq", "sum", "mean", "min", "max", "median"])]
        stat: String,
        /// Width of an X bin.
        #[arg(long, conflicts_with = "x_bins")]
        x_inc: Option<f64>,
        /// Number of X bins.
        #[arg(long)]
        x_bins: Option<usize>,
        /// First X edge (the first bin's centre with --x-centers).
        #[arg(long, allow_hyphen_values = true)]
        x_min: Option<f64>,
        /// Last X edge (the last bin's centre with --x-centers).
        #[arg(long, allow_hyphen_values = true)]
        x_max: Option<f64>,
        /// Label the X bins by their centres, and read --x-min and --x-max as centres.
        #[arg(long)]
        x_centers: bool,
        /// Add an X bin for the values below the range.
        #[arg(long)]
        x_below: bool,
        /// Add an X bin for the values at or above the range.
        #[arg(long)]
        x_above: bool,
        /// Width of a Y bin.
        #[arg(long, conflicts_with = "y_bins")]
        y_inc: Option<f64>,
        /// Number of Y bins.
        #[arg(long)]
        y_bins: Option<usize>,
        /// First Y edge (the first bin's centre with --y-centers).
        #[arg(long, allow_hyphen_values = true)]
        y_min: Option<f64>,
        /// Last Y edge (the last bin's centre with --y-centers).
        #[arg(long, allow_hyphen_values = true)]
        y_max: Option<f64>,
        /// Read --y-min and --y-max as the centres of the first and last Y bin.
        #[arg(long)]
        y_centers: bool,
        /// Add a Y bin for the values below the range.
        #[arg(long)]
        y_below: bool,
        /// Add a Y bin for the values at or above the range.
        #[arg(long)]
        y_above: bool,
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
    /// Stem-and-leaf display of each column (Origin 2026's Stem and Leaf Plot app, T279): a new sheet with
    /// one line per stem (Data, Count, Stem, Leaf); the leaf unit is in the notes.
    Stemleaf {
        #[command(flatten)]
        target: TargetArgs,
        /// The value of one leaf digit; chosen from the range when left out.
        #[arg(long)]
        leaf_unit: Option<f64>,
        /// Two lines per stem (leaves 0-4 and 5-9) when there would be fewer than five lines.
        #[arg(long)]
        split: bool,
        /// The most leaves written on a line; more are counted as +N.
        #[arg(long, default_value_t = 50)]
        max_leaves: usize,
        /// Put values more than 1.5 IQR outside the quartiles on Low and High lines.
        #[arg(long)]
        trim_outliers: bool,
    },
    /// Fit normal, lognormal, exponential, gamma and Weibull distributions to each column by maximum
    /// likelihood: parameters, log-likelihood, AIC, BIC, Kolmogorov-Smirnov and Anderson-Darling statistics.
    Distfit {
        #[command(flatten)]
        target: TargetArgs,
        /// The families to fit, comma separated (default: all five).
        #[arg(long, value_enum, value_delimiter = ',')]
        family: Vec<FamilyArg>,
    },
    /// Probability plot: the data against the plotting position in percent on a probability axis, with the
    /// fitted distribution function; makes a graph.
    Probplot {
        #[command(flatten)]
        target: TargetArgs,
        #[arg(long, value_enum, default_value_t = FamilyArg::Normal)]
        family: FamilyArg,
        /// Do not make a graph.
        #[arg(long)]
        no_graph: bool,
        /// Plotting positions (Origin documents Blom as its default; scipy uses Filliben).
        #[arg(long, value_enum, default_value_t = ScoresArg::Filliben)]
        scores: ScoresArg,
        /// A confidence band of this level (0.95 for 95 %) round the fitted line, Origin's for every family.
        #[arg(long)]
        band: Option<f64>,
    },
    /// Q-Q plot: the quantiles of a distribution against the ordered data, with the least-squares line; makes
    /// a graph.
    Qqplot {
        #[command(flatten)]
        target: TargetArgs,
        #[arg(long, value_enum, default_value_t = FamilyArg::Normal)]
        family: FamilyArg,
        /// Do not make a graph.
        #[arg(long)]
        no_graph: bool,
        /// Plotting positions (Origin documents Blom as its default; scipy uses Filliben).
        #[arg(long, value_enum, default_value_t = ScoresArg::Filliben)]
        scores: ScoresArg,
        /// The ordered data on X and the quantiles on Y, as Origin's guide describes.
        #[arg(long)]
        data_on_x: bool,
    },
    /// Principal component analysis of the --y columns (the variables): eigenvalues, percent of variance,
    /// loadings and scores; makes a scree plot, a loading plot and a biplot as graphs.
    Pca {
        #[command(flatten)]
        target: TargetArgs,
        /// Analyze the covariance matrix instead of the correlation matrix.
        #[arg(long)]
        covariance: bool,
        /// Leave missing values out pairwise instead of dropping the whole row.
        #[arg(long)]
        pairwise: bool,
        /// Number of components whose loadings and scores are written.
        #[arg(long, default_value_t = 2)]
        components: usize,
        /// Divide each column of scores by the square root of its eigenvalue.
        #[arg(long)]
        standardize_scores: bool,
        /// Do not write the scores.
        #[arg(long)]
        no_scores: bool,
        /// Do not make the scree plot, loading plot and biplot.
        #[arg(long)]
        no_graph: bool,
        /// Do not make the loading plot (the loadings of two components as vectors).
        #[arg(long)]
        no_loading_plot: bool,
        /// Do not make the biplot (scores as points with the loading vectors on the top and right axes).
        #[arg(long)]
        no_biplot: bool,
        /// Also make a score plot (the scores of two components as points).
        #[arg(long)]
        score_plot: bool,
        /// The component on the horizontal axis of the loading plot, score plot and biplot.
        #[arg(long, default_value_t = 1)]
        x_component: usize,
        /// The component on the vertical axis of those plots.
        #[arg(long, default_value_t = 2)]
        y_component: usize,
    },
    /// ROC curve: the --y columns are the test variables followed, last, by the state column. Area under the
    /// curve with standard error and confidence interval (as Origin's), the curve and the best cut-off; makes a graph.
    Roc {
        #[command(flatten)]
        target: TargetArgs,
        /// The value of the state column that is the positive state (a number or a text).
        #[arg(long, default_value = "1")]
        positive: String,
        /// Lower test values point to the positive state (default: higher values do).
        #[arg(long)]
        lower_is_positive: bool,
        /// Confidence level of the interval of the area, in percent.
        #[arg(long, default_value_t = 95.0)]
        confidence: f64,
        /// Do not make the graph.
        #[arg(long)]
        no_graph: bool,
    },
    /// Manhattan plot: the --y columns are the group (chromosome), the position and the p-value; each group's
    /// positions laid end to end, -log10 p per point, a graph with a significance line.
    Manhattan {
        #[command(flatten)]
        target: TargetArgs,
        /// The significance level drawn as a line.
        #[arg(long, default_value_t = 5e-8)]
        threshold: f64,
        /// Do not make the graph.
        #[arg(long)]
        no_graph: bool,
    },
    /// ARIMA forecast: ARIMA(p, d, q) of each --y column by conditional least squares, forecasts with confidence
    /// bands, with a graph.
    Arima {
        #[command(flatten)]
        target: TargetArgs,
        /// Autoregressive order.
        #[arg(long, default_value_t = 1)]
        p: usize,
        /// Differences.
        #[arg(long, default_value_t = 0)]
        d: usize,
        /// Moving-average order.
        #[arg(long, default_value_t = 0)]
        q: usize,
        /// Leave out the constant (the mean of the differenced series).
        #[arg(long)]
        no_constant: bool,
        /// Steps to forecast.
        #[arg(long, default_value_t = 10)]
        horizon: usize,
        /// Confidence level of the forecast bands, in percent.
        #[arg(long, default_value_t = 95.0)]
        confidence: f64,
        /// Do not make the graph.
        #[arg(long)]
        no_graph: bool,
    },
    /// Exponential smoothing forecast: simple, Holt or Holt-Winters smoothing of each --y column with fitted or
    /// given constants, and forecasts after the last value, with a graph.
    Forecast {
        #[command(flatten)]
        target: TargetArgs,
        /// simple, holt, additive (Holt-Winters) or multiplicative (Holt-Winters).
        #[arg(long, default_value = "holt")]
        model: String,
        /// Points per season (Holt-Winters).
        #[arg(long, default_value_t = 12)]
        period: usize,
        /// Fixed smoothing constants alpha[,beta[,gamma]] (default: fitted by least squares).
        #[arg(long)]
        constants: Option<String>,
        /// Steps to forecast.
        #[arg(long, default_value_t = 10)]
        horizon: usize,
        /// Do not make the graph.
        #[arg(long)]
        no_graph: bool,
    },
    /// Stationarity tests: the augmented Dickey-Fuller statistic (MacKinnon asymptotic critical values) and the KPSS
    /// statistic with its table probability, for each --y column.
    Stationarity {
        #[command(flatten)]
        target: TargetArgs,
        /// ADF deterministic terms: constant, trend or none.
        #[arg(long, default_value = "constant")]
        adf_terms: String,
        /// ADF lagged differences (default: chosen by AIC).
        #[arg(long)]
        adf_lags: Option<usize>,
        /// KPSS null hypothesis: level or trend.
        #[arg(long, default_value = "level")]
        kpss_null: String,
        /// KPSS autocovariance lags (default ceil(12 (n/100)^(1/4))).
        #[arg(long)]
        kpss_lags: Option<usize>,
    },
    /// Seasonal decomposition: trend (centred moving average), seasonal indices and residual of each --y column,
    /// additive or multiplicative, with graphs.
    Decompose {
        #[command(flatten)]
        target: TargetArgs,
        /// Points per season (12 for monthly data with a yearly pattern).
        #[arg(long, default_value_t = 12)]
        period: usize,
        /// additive or multiplicative.
        #[arg(long, default_value = "additive")]
        model: String,
        /// Do not make the graphs.
        #[arg(long)]
        no_graph: bool,
    },
    /// Autocorrelation: ACF (Bartlett bounds), PACF and Ljung-Box Q per lag of each --y column, with ACF and PACF
    /// graphs.
    Acf {
        #[command(flatten)]
        target: TargetArgs,
        /// The largest lag (default min(10 log10 n, n - 1)).
        #[arg(long)]
        lags: Option<usize>,
        /// Confidence level of the bounds, in percent.
        #[arg(long, default_value_t = 95.0)]
        confidence: f64,
        /// Do not make the graphs.
        #[arg(long)]
        no_graph: bool,
    },
    /// Distribution overview of right-censored life data: --y TIME,CENSOR; a Weibull, lognormal, exponential or
    /// normal maximum likelihood fit, and the probability, density, survival and hazard graphs.
    Lifedist {
        #[command(flatten)]
        target: TargetArgs,
        /// weibull, lognormal, exponential or normal.
        #[arg(long, default_value = "weibull")]
        distribution: String,
        /// The value of the censor column that marks a censored case.
        #[arg(long, default_value = "1")]
        censor: String,
        /// Do not make the graphs.
        #[arg(long)]
        no_graph: bool,
    },
    /// Stability study (ICH Q1E): --y TIME,RESPONSE[,BATCH]; per-batch lines, poolability tests and the shelf life
    /// where a confidence bound of the mean meets a specification limit, with a graph.
    Stability {
        #[command(flatten)]
        target: TargetArgs,
        /// Lower specification limit.
        #[arg(long)]
        lsl: Option<f64>,
        /// Upper specification limit.
        #[arg(long)]
        usl: Option<f64>,
        /// Confidence level of the bounds, in percent.
        #[arg(long, default_value_t = 95.0)]
        confidence: f64,
        /// Significance level of the poolability tests.
        #[arg(long, default_value_t = 0.25)]
        alpha: f64,
        /// Do not make the graph.
        #[arg(long)]
        no_graph: bool,
    },
    /// Process capability: Cp, Cpk, Pp, Ppk and parts per million outside the limits of each --y column, normal
    /// or nonnormal (lognormal or Weibull, percentile method), with a histogram and the fitted curve.
    Capability {
        #[command(flatten)]
        target: TargetArgs,
        /// Lower specification limit.
        #[arg(long)]
        lsl: Option<f64>,
        /// Upper specification limit.
        #[arg(long)]
        usl: Option<f64>,
        /// Subgroup size for the within deviation (1: moving ranges of individual values).
        #[arg(long, default_value_t = 1)]
        subgroup: usize,
        /// normal, lognormal or weibull.
        #[arg(long, default_value = "normal")]
        model: String,
        /// Do not make the graphs.
        #[arg(long)]
        no_graph: bool,
    },
    /// Rare event chart: a G chart (opportunities between events) or T chart (time between events) of each --y
    /// column, with its center line and control limits.
    Rarechart {
        #[command(flatten)]
        target: TargetArgs,
        /// g (counts between events) or t (times between events).
        #[arg(long, default_value = "g")]
        chart: String,
        /// G chart limits: probability (geometric quantiles) or sigma (gbar +- k sqrt(gbar (gbar + 1))).
        #[arg(long, default_value = "probability")]
        limits: String,
        /// T chart distribution: weibull or exponential.
        #[arg(long, default_value = "weibull")]
        distribution: String,
        /// The limits are at this many sigmas (or the matching normal tail probability).
        #[arg(long, default_value_t = 3.0)]
        sigma: f64,
        /// Do not make the graphs.
        #[arg(long)]
        no_graph: bool,
    },
    /// Time series pivot: each --y column summarized per calendar period of its date-time X column.
    Timepivot {
        #[command(flatten)]
        target: TargetArgs,
        /// minute, hour, day, week (from Monday), month, quarter or year.
        #[arg(long, default_value = "day")]
        period: String,
        /// sum, mean, count, min, max or median.
        #[arg(long, default_value = "mean")]
        stat: String,
    },
    /// Binned scatter: X cut into bins of equal width or equal counts; the mean, SD, SE and count of each --y
    /// column's Y per bin; makes a graph of the data with the bin means.
    Binscatter {
        #[command(flatten)]
        target: TargetArgs,
        /// The number of bins.
        #[arg(long, default_value_t = 10)]
        bins: usize,
        /// Bins of equal width instead of equal counts.
        #[arg(long)]
        equal_width: bool,
        /// Do not make the graph.
        #[arg(long)]
        no_graph: bool,
    },
    /// Change point analysis: each --y column is a series of its own (in row order); finds where its mean, its
    /// variance or both change (exact optimum of normal costs plus a penalty per change) and lists the segments;
    /// makes a graph of the data with the segment means.
    Changepoint {
        #[command(flatten)]
        target: TargetArgs,
        /// What changes: mean, variance or meanvar.
        #[arg(long, default_value = "mean")]
        change: String,
        /// The penalty per change: mbic, bic, aic, hq (Hannan-Quinn) or a number.
        #[arg(long, default_value = "mbic")]
        penalty: String,
        /// The fewest points in a segment (2 or more when the variance changes).
        #[arg(long, default_value_t = 2)]
        min_segment: usize,
        /// Do not make the graph.
        #[arg(long)]
        no_graph: bool,
    },
    /// Linear quantile regression: the --y columns are the independent variables followed, last, by the
    /// dependent variable. One row of coefficients per quantile (exact solution), the sum of check losses and
    /// the pseudo R-square; with one independent variable a graph of the data and the fitted lines.
    Quantreg {
        #[command(flatten)]
        target: TargetArgs,
        /// The quantiles, each between 0 and 1.
        #[arg(long, value_delimiter = ',', default_value = "0.1,0.25,0.5,0.75,0.9")]
        quantiles: Vec<f64>,
        /// Fit without an intercept.
        #[arg(long)]
        no_intercept: bool,
        /// Do not make the graph.
        #[arg(long)]
        no_graph: bool,
    },
    /// K-means cluster analysis of the --y columns (the variables): cluster centres, sizes and distances, analysis
    /// of variance by variable and the cluster of every row; makes a scatter graph of the first two variables.
    Kmeans {
        #[command(flatten)]
        target: TargetArgs,
        /// Number of clusters.
        #[arg(long, default_value_t = 3)]
        clusters: usize,
        /// Upper limit of the passes of single-case moves.
        #[arg(long, default_value_t = 100)]
        max_passes: usize,
        /// Leave out the analysis of variance.
        #[arg(long)]
        no_anova: bool,
        /// Do not write the cluster of every row.
        #[arg(long)]
        no_membership: bool,
        /// Do not make the graph.
        #[arg(long)]
        no_graph: bool,
    },
    /// Kaplan-Meier estimator: the --y columns are time, censor and, optionally, group. Survival table, quartile
    /// and mean survival times, tests of groups; makes a graph of the curves.
    KaplanMeier {
        #[command(flatten)]
        target: TargetArgs,
        /// The value of the censor column that marks a censored case (a number or a text).
        #[arg(long, default_value = "1")]
        censor: String,
        /// Confidence level of the quartile and mean limits, in percent.
        #[arg(long, default_value_t = 95.0)]
        confidence: f64,
        /// Leave out the quartile estimates.
        #[arg(long)]
        no_quartiles: bool,
        /// Leave out the mean estimate.
        #[arg(long)]
        no_mean: bool,
        /// Log-rank test of the groups.
        #[arg(long)]
        log_rank: bool,
        /// Breslow test of the groups.
        #[arg(long)]
        breslow: bool,
        /// Tarone-Ware test of the groups.
        #[arg(long)]
        tarone_ware: bool,
        /// Add the confidence limits of the curves to the graph.
        #[arg(long)]
        band: bool,
        /// Do not make the graph.
        #[arg(long)]
        no_graph: bool,
    },
    /// Graphical summary (Statistics > Descriptive Statistics > Graphical Summary): for each --y column (and each
    /// group of the --groups columns) a report of descriptive statistics, confidence intervals of the mean, median
    /// and standard deviation and the Anderson-Darling normality test, and a graph with a histogram and normal
    /// curve, an interval plot and a normal probability plot.
    #[command(name = "graphsummary", alias = "graph-summary")]
    GraphSummary {
        #[command(flatten)]
        target: TargetArgs,
        /// Group columns, comma-separated short or long names; each group (combination of values) is summarized
        /// on its own.
        #[arg(long, value_delimiter = ',')]
        groups: Vec<String>,
        /// Confidence level of the intervals, in percent.
        #[arg(long, default_value_t = 95.0)]
        level: f64,
        /// Graph arrangement: separate (one graph per variable and group). The other three Origin arrangements
        /// (per-variable, per-group, all-in-one) are not available yet.
        #[arg(long, default_value = "separate")]
        arrangement: String,
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

// ----- sheaf matrix ---------------------------------------------------------------------------

#[derive(Args, Debug)]
pub struct MatrixArgs {
    #[command(subcommand)]
    pub action: MatrixAction,
}

#[derive(Subcommand, Debug)]
pub enum MatrixAction {
    /// Add a new matrix (optionally filled by a formula) to a project, which is made if needed.
    New(NewArgs),
    /// Convert worksheet columns (or XYZ columns on a regular grid) to a matrix.
    Convert(Box<ConvertArgs>),
    /// Change the size or the ranges of a matrix, or set its values by a formula in i, j, x and y.
    Set(SetArgs),
    /// Transpose, flip, rotate, expand or shrink a matrix.
    Transform(TransformArgs),
    /// Write a matrix as worksheet columns (direct, or three XYZ columns).
    ToSheet(ToSheetArgs),
    /// Statistics of the numbers in a matrix: n, missing, mean, sd, min, max, sum.
    Stats(StatsArgs),
    /// Plot a matrix or XYZ columns as a heat map or contour plot (PNG, SVG or PDF).
    Plot(Box<PlotMatrixArgs>),
}

/// Which matrix of a project a command works on.
#[derive(Args, Debug, Clone)]
pub struct MatrixTarget {
    /// The matrix window, for example MBook1 (default: the first one).
    #[arg(long)]
    pub matrix: Option<String>,
    /// The sheet of the matrix, by name (default: its active sheet).
    #[arg(long)]
    pub sheet: Option<String>,
}

#[derive(Args, Debug)]
pub struct NewArgs {
    /// The project to write: an existing project gets the matrix added; .opj (with a companion
    /// .opj.sheaf, because Origin files cannot hold matrices yet) or .sheaf.
    #[arg(short, long)]
    pub output: PathBuf,
    /// Number of columns.
    #[arg(long, default_value_t = 32)]
    pub cols: usize,
    /// Number of rows.
    #[arg(long, default_value_t = 32)]
    pub rows: usize,
    /// X coordinate of the first and the last column, FROM:TO (default 1:10, as in Origin).
    #[arg(long, value_name = "FROM:TO", allow_hyphen_values = true)]
    pub x_range: Option<String>,
    /// Y coordinate of the first and the last row, FROM:TO (default 1:10).
    #[arg(long, value_name = "FROM:TO", allow_hyphen_values = true)]
    pub y_range: Option<String>,
    /// Fill the matrix from this formula in i (row), j (column), x and y, for example
    /// "sin(x)*cos(y)".
    #[arg(long)]
    pub formula: Option<String>,
    /// Long name of the matrix window.
    #[arg(long)]
    pub long_name: Option<String>,
}

/// Where the coordinates of a worksheet that is converted to or from a matrix are kept.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum LayoutArg {
    /// No coordinates: columns and rows are numbered 1, 2, 3, ...
    None,
    /// X values across the columns (first data row or a label row); the Y values in the first
    /// column with --coords-in-first-column.
    XAcross,
    /// Y values across the columns; the X values in the first column with --coords-in-first-column.
    YAcross,
}

#[derive(Args, Debug)]
pub struct ConvertArgs {
    /// A data file, a .sheaf project or an Origin .opj project with the worksheet to convert.
    pub input: PathBuf,
    /// The project to write: the input's windows and the new matrix.
    #[arg(short, long)]
    pub output: PathBuf,
    /// The workbook (default: the first one) and sheet (default: its active sheet).
    #[arg(long)]
    pub book: Option<String>,
    #[arg(long)]
    pub sheet: Option<String>,
    /// The columns to convert, comma-separated names (default: all). With --xyz exactly the X, Y
    /// and Z columns (default: the columns designated X, Y and Z, else the first three).
    #[arg(short, long, value_delimiter = ',')]
    pub columns: Vec<String>,
    /// The columns are XYZ points on a regular grid (Origin's XYZ gridding for regular data), not
    /// a table of Z values.
    #[arg(long)]
    pub xyz: bool,
    /// Where the coordinates are (Direct conversion).
    #[arg(long, value_enum, default_value_t = LayoutArg::None)]
    pub layout: LayoutArg,
    /// Where the coordinates across the columns are: none, row1 (the first data row), or
    /// param:L / param:U / param:C (long name, units or comments row) or param:NAME (a parameter row).
    #[arg(long, default_value = "none")]
    pub labels: String,
    /// The first column holds the other coordinate. (The import options own --first-column.)
    #[arg(long)]
    pub coords_in_first_column: bool,
    /// How unevenly spaced the coordinates may be, in percent of the mean step (Origin: 5).
    #[arg(long, default_value_t = 5.0)]
    pub tolerance: f64,
    /// Place the first and last coordinate on a straight line fitted through all of them.
    #[arg(long)]
    pub linear_fit: bool,
    /// Keep rows and columns that hold no number (Origin trims them).
    #[arg(long)]
    pub no_trim: bool,
    #[command(flatten)]
    pub read: ReadOptions,
}

#[derive(Args, Debug)]
pub struct SetArgs {
    /// A .sheaf project (or .opj) that has a matrix.
    pub project: PathBuf,
    #[command(flatten)]
    pub target: MatrixTarget,
    /// The formula, in i (row, from 1), j (column, from 1), x and y (the cell's coordinates).
    #[arg(long)]
    pub formula: Option<String>,
    /// New number of columns (the X range stays, so the spacing changes).
    #[arg(long)]
    pub cols: Option<usize>,
    /// New number of rows.
    #[arg(long)]
    pub rows: Option<usize>,
    /// New X range, FROM:TO.
    #[arg(long, value_name = "FROM:TO", allow_hyphen_values = true)]
    pub x_range: Option<String>,
    /// New Y range, FROM:TO.
    #[arg(long, value_name = "FROM:TO", allow_hyphen_values = true)]
    pub y_range: Option<String>,
    /// Write the result here instead of back to the project.
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum TransformKind {
    Transpose,
    FlipHorizontal,
    FlipVertical,
    /// Quarter turn clockwise.
    RotateCw,
    /// Quarter turn counter-clockwise.
    RotateCcw,
    /// Half turn.
    Rotate180,
    /// More columns and rows by bilinear interpolation (--cols and --rows are the factors).
    Expand,
    /// Fewer columns and rows by averaging blocks (--cols and --rows are the factors).
    Shrink,
}

#[derive(Args, Debug)]
pub struct TransformArgs {
    /// A .sheaf project (or .opj) that has a matrix.
    pub project: PathBuf,
    /// What to do.
    #[arg(value_enum)]
    pub op: TransformKind,
    #[command(flatten)]
    pub target: MatrixTarget,
    /// Expand and shrink: the factor for the columns (Origin: 2).
    #[arg(long, default_value_t = 2)]
    pub cols: usize,
    /// Expand and shrink: the factor for the rows (Origin: 2).
    #[arg(long, default_value_t = 2)]
    pub rows: usize,
    /// Flip: mirror the X (horizontal) or Y (vertical) range too, so every value keeps its
    /// coordinate.
    #[arg(long)]
    pub coordinates: bool,
    /// Write the result here instead of back to the project.
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum ToSheetMethod {
    /// A worksheet column per matrix column (per row with --layout y-across).
    Direct,
    /// Columns X, Y and Z with a row per matrix cell.
    Xyz,
}

#[derive(Args, Debug)]
pub struct ToSheetArgs {
    /// A .sheaf project (or .opj) that has a matrix.
    pub project: PathBuf,
    #[command(flatten)]
    pub target: MatrixTarget,
    #[arg(long, value_enum, default_value_t = ToSheetMethod::Direct)]
    pub method: ToSheetMethod,
    /// Direct: write the coordinates in the first row and column (x-across, y-across) or not (none).
    #[arg(long, value_enum, default_value_t = LayoutArg::None)]
    pub layout: LayoutArg,
    /// XYZ: let Y run in the inner loop with X fixed (default) or the other way round with this flag.
    #[arg(long)]
    pub y_const_first: bool,
    /// XYZ: leave out the cells that hold no number.
    #[arg(long)]
    pub skip_missing: bool,
    /// Also write the new worksheet as delimited text (.tsv or .txt gives tabs, else commas).
    #[arg(long, value_name = "FILE")]
    pub export: Option<PathBuf>,
    /// Write the project with the new workbook here (default: back to the project, unless --export
    /// is given).
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct StatsArgs {
    /// A .sheaf project (or .opj) that has a matrix.
    pub project: PathBuf,
    #[command(flatten)]
    pub target: MatrixTarget,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum PaletteArg {
    /// Blue, cyan, green, yellow, red.
    Rainbow,
    /// Blue, white, red.
    BlueWhiteRed,
    /// Black to white.
    Gray,
}

#[derive(Args, Debug)]
pub struct PlotMatrixArgs {
    /// A .sheaf or .opj project with a matrix (or a data file or project with XYZ columns).
    pub input: PathBuf,
    /// The figure to write: .png, .svg or .pdf.
    #[arg(short, long)]
    pub output: PathBuf,
    /// contour (colour fill with lines at the major levels), contour-lines (lines with labels) or
    /// heatmap (a block per cell).
    #[arg(short, long, default_value = "contour")]
    pub template: String,
    /// The matrix window (default: the first one) and its sheet.
    #[arg(long)]
    pub matrix: Option<String>,
    #[arg(long)]
    pub sheet: Option<String>,
    /// Plot XYZ worksheet columns instead of a matrix: X,Y,Z. XYZ points on a regular grid are
    /// used as the grid; scattered points are triangulated (contour plots only). The default when
    /// the project has no matrix: the columns designated X, Y, Z, else the first three.
    #[arg(long, value_delimiter = ',', value_name = "X,Y,Z")]
    pub xyz: Vec<String>,
    /// For XYZ columns: the workbook (default: the first one).
    #[arg(long)]
    pub book: Option<String>,
    /// Number of major levels (steps of the colour scale).
    #[arg(long)]
    pub major: Option<u32>,
    /// Number of bands in each major step.
    #[arg(long)]
    pub minor: Option<u32>,
    /// Lowest and highest level (default: the data range rounded outwards as Origin does).
    #[arg(long, allow_hyphen_values = true)]
    pub z_min: Option<f64>,
    #[arg(long, allow_hyphen_values = true)]
    pub z_max: Option<f64>,
    /// Draw level lines / leave them out.
    #[arg(long, conflicts_with = "no_lines")]
    pub lines: bool,
    #[arg(long)]
    pub no_lines: bool,
    /// Write the level on the lines.
    #[arg(long)]
    pub labels: bool,
    /// Leave out the colour scale.
    #[arg(long)]
    pub no_scale: bool,
    /// Title of the colour scale (default: the Z label of the matrix).
    #[arg(long)]
    pub scale_title: Option<String>,
    #[arg(long, value_enum)]
    pub palette: Option<PaletteArg>,
    /// Axis titles (default: the X and Y labels of the matrix).
    #[arg(long)]
    pub x_title: Option<String>,
    #[arg(long)]
    pub y_title: Option<String>,
    /// Page size in inches (default 6.4 x 4.8).
    #[arg(long)]
    pub width: Option<f64>,
    #[arg(long)]
    pub height: Option<f64>,
    /// Resolution of PNG output in dots per inch.
    #[arg(long, default_value_t = 300.0)]
    pub dpi: f64,
    /// Also save the project with the graph here.
    #[arg(long, value_name = "PROJECT")]
    pub save: Option<PathBuf>,
    #[command(flatten)]
    pub read: ReadOptions,
}
