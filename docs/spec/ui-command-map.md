# Origin (9.3, 64-bit) user interface command map

Menu, command, dialog and shortcut reference for interface compatibility.

Summary

- Origin has one main window with a project (OPJ) holding many child windows. Each window type
  (workbook, graph, matrix, layout, notes, Excel workbook, image) swaps in its own menu bar.
- The menu bars are stored as English, German and Japanese resources inside the main
  executable. The menus for Analysis, Statistics, Matrix, Image and the "Import"/"Export"
  submenus are empty placeholders ("Please Wait...") that are filled at run time from the
  installed X-Function library, sorted by category folder.
- Workbook menu bar: File, Edit, View, Plot, Column, Worksheet, Analysis, Statistics, Image,
  Tools, Format, Window, Help. Graph menu bar: File, Edit, View, Graph, Data, Analysis,
  Gadgets, Tools, Format, Window, Help.
- Almost all dialogs for analysis (Integrate, Smooth, Peak Analyzer, Fit, Export Graph,
  Import) are generated automatically from an X-Function parameter list; their option names
  are listed in section C below and can be used directly as the settings model.
- Priority paths for the owner's workflow: File > Import (ASCII/CSV), Plot > Line / Scatter /
  Line+Symbol, Plot Details, Axis dialog, Analysis > Mathematics > Integrate, Analysis > Peaks and
  Baseline, File > Export Graphs.

Evidence legend: OBSERVED = directly verified reference detail (file references are given).
INFER = inferred behavior. UNKNOWN = requires verification.
Priority tags: P0 owner daily workflow, P1 common, P2 later.

Primary sources (all OBSERVED): menus, dialogs, strings, accelerators and toolbars decoded from
the resource sections of `Origin93_64.exe`, `ok9_64.dll`, `ou9_64.dll`, `od9_64.dll`,
`ogrid9_64.dll`, `okUtil9_64.dll` (English language, id 1033); X-Function parameter lists from
`X-Functions\*\*.OXF`; the menu placement index `X-Functions\xfdump_p_e.xfd`; toolbar layout
`Otb8.INI`; `OriginLiteMenu.ini`; `FULLMENU.CNF`. Decoder scripts are in the git-ignored scratch
folder `re-workspace\scratch\ui-map`.

---

## A. Menu trees per window type

Notation: items shown with their shortcut in brackets. "(dyn)" means the content is added at
run time from X-Functions. Separators omitted. The `&` marks an Alt-key mnemonic.

### A0. Which menu bar belongs to which window (OBSERVED resource ids in the exe)

| Resource id | Window it serves | Top-level menus |
|---|---|---|
| 501 | Workbook (worksheet) | File, Edit, View, Plot, Column, Worksheet, Analysis, Statistics, Image, Tools, Format, Window, Help |
| 503 | Graph | File, Edit, View, Graph, Data, Analysis, Gadgets, Tools, Format, Window, Help |
| 504 | Layout page | File, Edit, View, Layout, Tools, Format, Window, Help |
| 506 | Matrix book | File, Edit, View, Plot, Matrix, Image, Analysis, Tools, Format, Window, Help |
| 507 | Excel workbook window | File, View, Plot, Column, Analysis, Tools, Format, Window, Help |
| 508 | Notes window | File, Edit, View, Tools, Format, Window, Help |
| 526 | Image window | File, Edit, View, Image, Process, Analysis, Tools, Format, Window, Help |
| 510 / 511 | Minimal windows (function/other, no editing) | File, View, (Tools, Window,) Help |
| 480 | Menu shown when no document window is open | File, View, Tools, Window, Help |
| 532-534 | Small special windows (reduced File/Plot/Window) | UNKNOWN which window |

The ids 507, 510, 511, 532 are assigned by guess from their contents (INFER).

### A1. Menus shared by all document windows (OBSERVED)

**File** (workbook version; graph/layout/matrix differ only where noted)
- New: Project; Worksheet...; Matrix...; Graph; Image; From Template (None, More...); Excel; Layout;
  Notes; Function Plot (2D Function Plot, 2D Parametric Function Plot, 3D Function Plot,
  3D Parametric Function Plot). Creates a new window or project of that kind.
- Open... [Ctrl+O]; Open Excel... [Ctrl+E]; Open Sample OPJ; Append... (merge another project in);
  Close.
- Save Project [Ctrl+S]; Save Project As...
- Save Window As...; Save Template As...: write the active window alone or as a template.
- Save Workbook As Analysis Template...; Add Word Bookmarks to Analysis Template...;
  Batch Processing... (run a saved analysis template over many files).
- Save Project without Data...; Duplicate Project without Data.
- Print... [Ctrl+P]; Print Preview; Page Setup...
- **Import** (dyn): list of file-format importers, see section B3. Recent Imports submenu.
  Re-Import Directly [Ctrl+4]; Re-Import... (repeat the last import on the same sheet).
  Database Import: New / saved queries.
- **Export** (dyn): see section B4. Recent Exports submenu.
- Recent Books, Recent Graphs, Recent Projects; Exit. Recent Projects and Recent Imports work (T38,
  `docs/spec/recent-files.md`; 10 entries per list, the numbering and the path display are Sheaf's
  choices, not measured in Origin). Recent Books, Recent Graphs and Recent Exports stay greyed.
- Graph windows additionally have Export Page... in the Layout version and the graph File menu
  carries Batch Processing but no "Re-Import".

**Edit** (workbook): Undo [Ctrl+Z]; Cut; Copy submenu (Copy [Ctrl+C], Copy full precision
[Ctrl+Alt+C], Copy including label rows [Ctrl+Shift+C], Copy formula [Ctrl+J], Copy label rows
[Ctrl+Shift+J], Copy formula and label rows [Ctrl+Alt+J], Copy All); Paste [Ctrl+V]; Paste
Transpose; Paste Link [Ctrl+Alt+V]; Paste Link Transpose; Paste Special; Select...; Clear [Del];
Insert; Delete; Remove Links; Set As Begin / Set As End / Reset to Full Range (limit the active
row range used by analysis and plots); Find in Project... [F3]; Find in Sheets... [Ctrl+F];
Replace... [Ctrl+H]; Go to... [Ctrl+G]; Button Edit Mode [Ctrl+Alt+B].

**Edit** (graph): Undo; Cut; Copy; Copy Page [Ctrl+J]; Paste; Paste Link; Paste Special; Clear;
Insert New Object; Links; Button Edit Mode; Update Client; Copy Format submenu (All, All Style
Formats, Colors, Fonts, Background, Symbol/Line/Fill, Dimensions, Scales, Text); Paste Format...;
Find in Project.

**View**: Menu (choose a menu set, "Generic"); Toolbars... [Ctrl+T]; Status Bar; Command Window
[Alt+3]; Code Builder [Alt+4]; Quick Help [Alt+5]; Project Explorer [Alt+1]; Object Manager
[Alt+8] (graph only); View Windows (None / Windows in Active Folder / Active Folder and
Subfolders); Results Log [Alt+2] with View Mode (All Results / Active Folder / Active Folder and
Subfolders); Messages Log [Alt+6]; Smart Hint Log [Alt+7]. Workbook adds: Actively Update Plots,
Page Break Preview Lines. Graph adds: Print View [Ctrl+2], Page View [Ctrl+1], Window View, Draft
View [Ctrl+0], Zoom In [Ctrl+I], Zoom Out [Ctrl+M], Whole Page [Ctrl+W], Zoom All, Show submenu
(Layer Icons, Active Layer Indicator, Axis Layer Icons, Object Grid, Layer Grid, Frame, Labels,
Data, Active Layer Only, Master Items on Screen), Show Data Info, Full Screen [Ctrl+Shift+J].

**Tools** (same in every window): Options... [Ctrl+U] (see C10); System Variables...;
3D OpenGL Settings...; Protection (Protect Project, Audit Log); Apps on File Exchange;
Fitting Function Builder... [F8]; Fitting Function Organizer... [F9]; Theme Organizer... [F7];
Import Filters Manager...; Package Manager...; Custom Menu Organizer...; Virtual Matrix
Manager...; X-Function Builder... [F10]; X-Function Script Samples... [F11]; consoles for
MATLAB, R, Rserve, Python, LabVIEW VI browser, Mathematica link. Matrix windows add Region of
Interest Tools [Ctrl+Shift+T].

**Format** (workbook): Menu; Worksheet... [F4] (worksheet display properties); Column...
(column properties, C5); Snap to Grid; Merge cells [Ctrl+R]; Programming Control...; Object
Properties.

**Window**: Cascade; Tile Horizontally; Tile Vertically; Arrange Icons; Refresh; Duplicate (graph:
also Duplicate (Batch Plotting) with New Sheets/Books or New Columns); Split (workbook, matrix);
Properties... (window name, long name, comments); Command Window; Script Window; Folders list.

**Help**: online help, X-Functions help, programming help (LabTalk, Origin C, Python, automation
server, Code Builder), sample project, support and web links, About. (Licence/registration items
exist; deliberately not studied.)

### A2. Workbook window (resource 501)

**Plot** (P0; full list, OBSERVED). Grouping is the contract; the right column says what it draws.

| Group | Item | Draws |
|---|---|---|
| Line | Line | Y against X joined by straight segments |
| | Horizontal Step, Vertical Step | stair-step connection (horizontal-first or vertical-first) |
| | Spline Connected | smooth curve through points |
| Symbol | Scatter | symbols only |
| | Scatter Central | scatter with central axes through the origin |
| | Column Scatter | scatter against a category column |
| | Y Error, XY Error | symbols with error bars from the set-as-error columns |
| | Vertical Drop Line | line from each point to the axis |
| | Bubble, Color Mapped, Bubble + Color Mapped | third column controls size and/or colour |
| Line + Symbol | Line + Symbol, Line Series, 2 Point Segment, 3 Point Segment | symbols with connecting lines; segment types join point pairs/triples |
| Column/Bar/Pie | Column, Column + Label, Grouped Columns (indexed data), Bar, Stacked Column, Stacked Bar, 100% Stacked Column/Bar, Floating Column/Bar, 3D Color Pie, 2D B&W Pie | vertical/horizontal bars and pies |
| Multi-Y | Double-Y, 3Ys Y-YY, 3Ys Y-Y-Y, 4Ys Y-YYY, 4Ys YY-YY, Multiple Y Axes... | several Y axes in one layer |
| Y-offset/Waterfall | Stacked Lines by Y Offsets, Waterfall, Waterfall Y:Color Mapping, Waterfall Z:Color Mapping, 3D Waterfall and its two colour-mapped variants | curves shifted against each other |
| Multi-Panel | Vertical 2 Panel, Horizontal 2 Panel, 4 Panel, 9 Panel, Stack..., Multiple Panels by Label... | several layers on one page |
| 3D XYY | XYY 3D Bars, 3D Ribbons, 3D Walls, 3D Waterfall (+2 colour variants) | |
| 3D Surface | Color Fill Surface, X Constant with Base, Y Constant with Base, Color Map Surface, Colormap Surface with Projection, Wire Frame, Wire Surface, 3D Ternary Colormap Surface | |
| 3D Symbol/Bar/Vector | 3D Bars, 3D Scatter, 3D Trajectory, 3D Scatter + Error Bar, 3D Vector XYZ XYZ, 3D Vector XYZ dXdYdZ | |
| Statistics | Box Chart, Grouped Box Charts (indexed or raw), Interval Plot, Violin Plot and its variants (Violin with Box, Point, Quartile, Stick, Split Violin, Half Violin; Sheaf has all but Split Violin), Ridgeline Chart, Histogram, Histogram + Probabilities, Stacked Histograms, Marginal Histograms, Marginal Box Charts, 2D Kernel Density, QC (X bar R) Chart, Pareto (binned or raw), Scatter Matrix, Probability Plot, Q-Q Plot | |
| Area | Area, Stacked Area, Fill Area | |
| Contour/Heat Map | Color Fill, B/W Lines + Labels, Gray Scale Map, Heat Map, Polar Contour theta(X)r(Y), Polar Contour r(X)theta(Y), Ternary Contour | |
| Profile | Contour Profiles, Image Profiles | |
| Specialized | Polar theta(X) r(Y), Polar r(X) theta(Y), Wind Rose (binned or raw), Ternary, Piper, Smith Chart, Radar, High-Low-Close, Vector XYAM, Vector XYXY, Zoom | |
| Stock | High-Low-Close, Japanese Candlestick, OHLC Bar Chart, OHLC-Volume, Line | |
| Templates | System Templates..., User Templates (submenu of saved templates), Most Recently Used | plot with any saved .otp template |

P0 items: Line, Scatter, Line + Symbol, Y Error, Column, Double-Y, Multiple Y Axes (for
overlay CV plots), Stacked Lines by Y Offsets, Waterfall. P1: area, statistics group, contour,
templates. P2: 3D, specialised, stock.
The plot toolbars (C-toolbars below) show the same commands with the one-word internal names
`Line, Scatter, LineSymbol, Bar, Column, Pie, StackArea, Area, FillArea, DoubleY, LineSeries,
Waterfall, OffsetYs, YError, XYError, VertDropLine, Spline, HorzStep, VertStep, Segment2,
Segment3, BoxChart, Histogram, StackBar, StackCol, FloatBar, FloatColumn, IndexSize,
IndexColor, ScatterCentral, ColumnScatter, 3Ys_Y-YY` etc. (OBSERVED in `ou9_64.dll` toolbar
tooltips, with command text such as "@DoubleY"). Violin plots are newer than those 9.3 binaries (Origin 2019): the
Origin help names a "Violin with Box" button on the 2D Graphs toolbar, but its internal name is UNKNOWN (not observed).

**Column** (P0)
- Set As: X, Y, Z, Label, Disregard, Y Error, X Error (plot designation of the selected columns).
  Set as Categorical.
- Set Column Values... [Ctrl+Q]; Set Multiple Columns Values... [Ctrl+Shift+Q] (formula dialog, C4).
- Fill Column With: Row Numbers; Uniform Random Numbers; Normal Random Numbers; A set of
  Numbers...; A set of Date/Time values...; Arbitrary set of Text&Numeric values...
- Set Sampling Interval... (make X implicit with start and step); Show X Column....
- Add New Columns... [Ctrl+D].
- Filter: Add or Remove Filter, Enable/Disable Filter, Reapply Filter (rows not matching are hidden).
- Mask: Apply, Remove, Swap, Change Color, Disable Masking; Mask Cells by Condition....
  (Masked cells are excluded from analysis and plots.)

**Worksheet** (P1)
- Sort Range / Sort Columns / Sort Worksheet, each Ascending, Descending, Custom... (custom =
  nested sort dialog with multiple keys and a missing-value rule).
- Sort Columns by Label...; Clear Worksheet...; Worksheet Script...; Worksheet Query....
- (dyn) further entries from X-Functions "Data Manipulation\Worksheet": see A5.

**Analysis** (P0 for Mathematics, Peaks and Baseline, Fitting): dynamic; see A5.

**Statistics** (P2): dynamic; see A5. **Image** (P2): dynamic.

### A3. Graph window (resource 503)

**Graph** (P0)
- Layer Contents... [F12]: choose which datasets are plotted in the active layer (C-list "Layer
  Plot Contents": available data, layer contents, add/remove arrows, Edit Range, Plot Setup).
- Plot Setup...: choose X/Y/error columns and the plot type per dataset.
- Add Plot to Layer: Line..., Scatter..., Line + Symbol..., Column..., Area...
- Add Error Bars...; Add Function Graph....
- Rescale to Show All [Ctrl+R].
- Legend: Add Legend Title, Box Chart Components, Categorical Values, Point by Point, Data
  Plots; text colour follows plot; reverse order; visible plots only; hide legend for fitted
  curves; Reconstruct Legend [Ctrl+L]; Update Legend; arrange vertically/horizontally.
- New Table; New Color Scale; New Bubble Scale; New XY Scaler.
- Set Active Layer By Layer Icon Only; Fit Layer to Graph; Fit Page to Layers; Exchange X-Y Axes;
  Offset Grouped Data in Layer (None, Cumulative, Constant, Auto, Individual);
  Convert to Standard Font Size.

**Data** (P0)
- Set Display Range / Reset to Full Range (restrict the plotted rows of the active plot).
- Mark Data Range [Ctrl+Alt+M]; Clear Data Markers [Ctrl+Alt+N].
- Analysis Markers: Hide [Ctrl+Shift+M], Show Partial Ranges, Show All, size Tiny to Large.
- Lock Position: Layer Icon [Ctrl+Shift+K], First Point [Ctrl+Shift+F], Last Point [Ctrl+Shift+L].
- Mask Data Points; Move Data Points; Remove Bad Data Points; Dataset List.

**Analysis** (dyn): graph version is built from the same library with the active plot as input.
**Gadgets** (dyn, P0 for CV work): quick tools drawn from the "Data Exploration" folder; see A5.

**Format** (P0): Page Properties... [F2]; Layer Properties...; Plot Properties...; Snap Layers to
Grid; Snap Objects to Grid; Fit Page To Layers; Fit Page To Printer; Axes... (axis dialog, C2);
Axes submenu (X, Y, Z axis); Axis Tick Labels submenu; Axis Titles submenu; Programming Control;
Object Properties.

### A4. Matrix (506), Layout (504), Notes (508), Excel (507), Image (526)

- Matrix book: Edit as workbook plus Find/Replace/Go to; View adds Data Mode [Ctrl+Shift+D],
  Image Mode [Ctrl+Shift+I], Show Column/Row, Show X/Y, Show Image Thumbnails. Plot menu has only
  matrix-capable plots: 3D Surface group (Color Fill, X/Y Constant, Color Map, with Error Bar,
  Multiple surfaces, Projection, Wire Frame, Wire Surface), 3D Scatter/Bars, Histogram, Contour
  group (Color Fill, B/W Lines, Gray Scale Map, Heat Map, Polar Contour), Profile, Image Plot,
  templates. Matrix menu (dyn): set dimensions, properties, values, expand, shrink, transpose,
  flip, rotate 90, crop, replace. (P2)
- Layout page: Layout menu with Add Graph..., Add Worksheet..., Set/Clear Picture Holder, New
  Table, Global Speed Control; Format has Layout Page... [F2], Theme Gallery [F7]; View has Zoom
  In/Out/Whole Page and Show Grid. (P2)
- Notes: File and Edit with Select All, Find, Replace; View Word Wrap. (P2)
- Excel window: same plot subset as workbook (Line, Scatter, Bar, Column, Area, Polar, Pie ...),
  Column menu and Import ASCII. (P2)
- Image window: Image, Process and Analysis menus (dyn) fed by the "Image" X-Function folders. (P2)

### A5. Analysis, Statistics, Gadgets, Matrix and Image menus (dynamic)

OBSERVED: these five menus contain only "Please Wait..." in the resource; the real content is
generated at start-up. The index file `X-Functions\xfdump_p_e.xfd` lists, for each X-Function, a
menu path and a label (with an ampersand mnemonic). The first path component matches the
folder names in `X-Functions`: Data Exploration, Data Manipulation, Database Access, Fitting,
Graph Manipulation, Image, Import and Export, Mathematics, Miscellaneous, Plotting, Signal
Processing, Spectroscopy, Statistics, Utilities. INFER: Analysis menu entries are grouped by
these folders (with "Spectroscopy" appearing as Peaks and Baseline in the running menu), and the
Statistics menu uses the Statistics folder. UNKNOWN: exact ordering and any grouping not in
the folder names; check in running Origin.

Labels by folder (paraphrased lists of the English labels, P-tags in brackets):

- **Mathematics**: Simple Curve Math, Simple Column Math, Differentiate [P0], Integrate [P0],
  Interpolate/Extrapolate, Interpolate/Extrapolate Y from X, Trace Interpolation, Linear/Spline/
  B-Spline Interpolation, 2D and 3D interpolation, Intersect, Normalize (columns/curves), Average
  Multiple Curves, Polygon Area, XYZ Surface Area, Median Filter, Special Filter, Inverse. [P1]
  Sheaf: Intersect, Average Multiple Curves, Polygon Area and Trace Interpolation are live (rules in `analysis-curve-math.md`,
  unmeasured); the others of this folder not named in their own task stay grey.
- **Signal Processing**: Smooth [P0], FFT Filters, FFT (submenu FFT: FFT, IFFT, 2D FFT, 2D IFFT),
  Convolution, Deconvolution, Correlation, 2D Correlation, Coherence, Decimation, Envelope,
  Hilbert Transform, IIR Filter, STFT, Wavelet submenu (Decompose, Reconstruction, Denoise,
  Smooth, Continuous Wavelet, Multi-Scale DWT, 2D Decompose/Reconstruction, Wavelet
  Functions). [Smooth P0, rest P1/P2]
- **Fitting**: Linear Fit, Polynomial Fit, Multiple Linear Regression, Exponential Fit,
  Sigmoidal Fit, Single Peak Fit, Fit Linear with X Error, Nonlinear Curve Fit [P0],
  Nonlinear Implicit Curve Fit (ODR), Nonlinear Surface Fit, Simulate Curve, Simulate Surface,
  Compare Datasets, Compare Models, Rank Models, Fitting Function Wizard, Find Baseline Region.
  Each fit command remembers its last settings per dataset and writes a report sheet.
- **Spectroscopy (Peaks and Baseline)**: Peak Analyzer [P0] (entry opens a wizard; its theme
  folder "Peak Analyzer" holds the stored sessions), Multiple Peak Fit (Gauss/Lorentz/Voigt),
  Batch Peak Analysis Using Theme, baseline helpers. [P0]
- **Statistics (menu)**: Descriptive Statistics (Descriptive, Correlation Coefficient, Normality
  Tests, Frequency Counts, Distribution Fit, Grubbs, Dixon Q...), Hypothesis Testing (one- and
  two-sample t, paired t, proportions, variance), ANOVA (one-, two-, three-way, repeated
  measures), Nonparametric Tests, Multivariate Analysis (PCA, clusters, discriminant, PLS),
  Power and Sample Size, Survival Analysis, ROC Curve. [P2]
- **Data Manipulation** (Worksheet menu entries): Sort, Reverse Order, Transpose, Stack/Unstack
  Columns, Split Columns/Worksheet/Workbooks, Reduce Rows/Columns, Remove/Combine Duplicated Rows,
  Join Worksheets by Column/Label, Pivot Table, Convert to XYZ, Extract Worksheet Data, Find and
  Replace, Set Sampling Interval, Show X Column, Column Width, Row Height, Hide/Unhide
  Columns/Rows, Cell Color, Format Cells, Insert Sparklines/Notes/Graph/Arrow/Variables,
  Copy/Swap Columns, Subtract Reference Data, Subtract Straight Line, Reduce to Evenly Spaced X,
  Curve Translate X. Matrix entries: Set Dimension/Labels, Set Properties, Set Values, Expand,
  Shrink, Flip, Transpose, Rotate 90, Crop, Replace. Gridding: XYZ Gridding, XYZ Log Gridding,
  Generate Grid, Convert to Worksheet. [P1]
- **Data Exploration (Gadgets menu in a graph)**: Statistics, Quick Fit, Quick Sigmoidal Fit,
  Quick Peaks, Integrate [P0], Differentiate, Interpolate, Intersect, Rise Time, Cluster, FFT, 2D
  Integrate, Vertical Cursor..., Go To, Image/Contour Profile. A gadget draws a movable
  rectangle over the data and reports results for the data inside. [P0 for Integrate]
- **Image**: Adjustments (brightness, contrast, gamma, levels...), Spatial Filters, Geometric
  Transform (crop, resize, rotate, flip...), Conversion (binary, colour to grey, RGB split/
  merge...), Arithmetic Transform, Analysis (histogram). [P2]
- **Utilities**, **Miscellaneous**, **Database Access**, **Graph Manipulation** (extract layers,
  speed mode, scroll bar for axes...): not in the menu bar except as tools. [P2]

---

## B. File > Import and File > Export

### B1. Where the commands live (OBSERVED)
Menu ids `ID_DYN_XF_WKS_FILE` (import list in workbook) and `ID_DYN_XF_CUSTOMIZED_EXPORT`
(export list) in `OriginLiteMenu.ini` show that both submenus are dynamic X-Function lists
within the File menu. Shortcuts: Import ASCII [Ctrl+K], Import Wizard [Ctrl+3],
Re-Import Directly [Ctrl+4].

### B2. Import wizard / ASCII import (P0)

- The wizard (X-Function `impWiz`) offers: choose a data source type, pick file(s), then a
  filter-specific options page. Filters live as `.oif` files in `Filters` (ASCII, CSV, Excel,
  Image, Matlab, HDF5, ...); the Import Filters Manager edits them.
- `impASC` parameters (OBSERVED, `X-Functions\Import and Export\impASC.OXF`): `fname` (file
  name), `menu` (launch option: single ASCII, multiple ASCII), `trfiles` (info tree),
  `options` (import options tree).
- The older "ASCII Import Options" dialog (still in `ok9_64.dll`) shows the option groups:
  File Structure: Delimited with a delimiter choice or Fixed with column-width list; automatic
  determination of column types. File Header: skip main header lines (count; or auto-determine
  extra header lines); specify known sub-header lines (0,1,..) or Unknown; maximum lines kept in
  column labels. Other: action when non-numeric text is met in a numeric field; support numbers
  with leading zeros; "Import Into Worksheet as" (new book, new sheets, replace existing,
  new columns, etc.). A second page "Data Import Options": rename columns if first line holds
  (Measured 2026-11-13 in the 2016 `impASC` window: it has no "Data Import Options" page; see import-ascii.md section 10.)
  labels; rename worksheet to the file name; apply new range to associated plots; create column
  labels from the header.
- `impCSV` parameters: `fname`, `trfiles`, `options`, `orng` (destination range, default active
  sheet), `trheader`, `reimp` (repeat import). UNKNOWN: the full option tree (header lines,
  delimiters, decimal separator, date format, column-designation per header line); this belongs to
  the import topic and must be verified in the running program.
- After import the sheet remembers its source; Re-Import Directly repeats it.

### B3. Importer list (OBSERVED labels from `xfdump_p_e.xfd`)
ASCII (single/multiple), Comma Delimited (CSV), Excel (XLS, XLSX, XLSM), Image, Binary 2D
Array, Origin Project (OPJ, ORG, OGW), CDF, EDF/BDF, EarthProbe, Famos, HDF5, HEKA, IgorPro,
JCAMP-DX, KaleidaGraph, Matlab, Minitab, MZXML, NetCDF, NI DIAdem, NI TDM, pCLAMP,
Princeton Instruments SPE, Prism, SAS, SPSS, SigmaPlot JNB, Somat SIE, Sound WAV, Tektronix
ISF, Thermo SPC, Data Translation, BRUKER OPUS, ETAS INCA MDF, Import Wizard, ReImport,
database (ODBC / ADO). [P0: ASCII, CSV; P1: Excel, Image; rest P2]

### B4. Export (P0 for graphs)

Export commands (OBSERVED X-Function names): `expGraph` "Export Graphs", `expImage`, `expWks` (sheet
as image), `expPDFw` (multi-page PDF), `expASC` (worksheet as ASCII), `expMatASC`, `expWAV`,
`expNITDM`, `img2GIF`. Ctrl+G opens Export Graphs from the Project Explorer/graph.
Right-click on a graph page also has Copy Page, Export Graphs, Send Graphs to PowerPoint.

Export Graphs (`expGraph`) parameters (OBSERVED):

| Name | Meaning | Default | Source |
|---|---|---|---|
| type | Image Type: AI, BMP, CGM, DXF, EMF, EPS, GIF, JPG, PCX, PDF, PNG, PSD, TGA, TIF, WMF | index 5 (the list order puts EMF/EPS/GIF area; actual default to confirm) | expGraph.OXF |
| export | Which graphs: active page, all in active folder, all in folder recursive, all open in folder, embedded | active page | expGraph.OXF |
| pages | graph page names (when not "active") | active | |
| filename | output name(s); a name template with substitutions is supported | empty | |
| path | folder | empty (uses system last path) | |
| overwrite | Auto Rename, Replace, Skip, Ask | Ask (index 3) | |
| theme | graph theme to apply | none | |
| sysopts | Use Tools > Options settings | on | |
| keepsize | Keep original size | off | |
| tr1 (Image Size) | width and unit (`tr1.width`, `tr1.unit`; unit codes include inches, cm, pixels, percent) | use source | |
| tr2 (Image Settings) | per-format settings: resolution (dpi), colour depth, compression, background/transparency, fonts | UNKNOWN | |

The nested settings (dpi, colour depth, TIFF compression, transparency) are UNKNOWN; check in
the running dialog. Worksheet export (`expASC`) options OBSERVED: file type dat/txt/csv/all,
overwrite Replace or Append, selected data only, separator (TAB, comma, space, comma+TAB,
comma+space) or CSV separator (comma, semicolon), include short name, long name, units,
comments, user parameters, sampling interval, parameters, row labels; export with full
precision, export missing values as "--" (default on); text-cell number suffix.

---

## C. Toolbars

Names and docking state OBSERVED in `Otb8.INI` (a settings file) and button lists from resources
in `ou9_64.dll`. Toolbars are shown/hidden through View > Toolbars (Ctrl+T) and can be customised
(New Toolbar, Export Toolbars, Button Groups dialogs exist). Each button is a command id; a
button command is either a built-in id or a short script such as "@DoubleY" or
"worksheet -PU i line".

| Toolbar | Content (paraphrased) |
|---|---|
| Standard (large, 52 buttons) | New project/folder/workbook/Excel/graph/master graph/matrix/function/layout/notes/digitizer; Open, Open Template, Open Excel, Save Project, Save Template; Recalculate; Import Wizard, Import Single ASCII, Import Multiple ASCII, Import Excel; Print; Refresh, Duplicate, Zoom; Custom routine; Project Explorer, Object Manager, Results Log, Command Window, Code Builder toggles; Add New Columns; Template Organizer; Batch Processing; Send Graphs to PowerPoint; Slide Show; Video Builder; New 2D/3D (parametric) function plot; Duplicate with new sheet/columns |
| Edit | Cut, Copy, Paste, Undo |
| Graph | Zoom In/Out, Whole Page; anti-aliasing; Rescale; Exchange Axes; Speed Mode; Extract to Layers/Graphs; Merge; Add layers (Bottom-X Left-Y, Top-X, Right-Y, Top-X Right-Y); Add Inset Graph (with or without data) |
| 2D Graphs (two bars) | Line, Scatter, Line+Symbol, Bar, Column, Pie, Stacked Area, Area, Fill Area, Polar x2, Ternary, Smith, Radar, High-Low-Close, Vector x2, Template Library, Wind Rose, Candlestick, OHLC, Scatter Central, Probability/QQ, Multiple Y Axes, panel templates, Vertical Drop Line, 2/3-Point Segment, Steps, Spline, Double-Y, Line Series, Waterfall variants, Stacked Lines by Y Offsets, Zoom, Y Error, XY Error, Stacked/Floating bars/columns, Bubble, Color Mapped, Box Chart, Histogram, 3Ys/4Ys, Pareto, Piper, 100% stacked, 2D Kernel Density, Column Scatter |
| 3D and Contour Graphs | 3D Scatter, Trajectory, Vector variants, XYY bars/ribbons/walls/waterfall, surface plots (colour fill, X/Y constant, colormap, with error bars, multiple, projection), 3D bars, wire frame, contour (colour fill, B/W lines, gray scale), Image Plot, Image/Contour Profiles, polar and ternary contours, Heat Map |
| 3D Rotation | rotate clockwise/counterclockwise, tilt in four directions, perspective +/-, fit frame to layer, reset rotation, rotation angle box |
| Worksheet Data (Column) | Statistics on Column(s)/Row(s), Sort, Set Column Values, Set all column values, fill with row number/uniform/normal random, filter add/enable/reapply |
| Column | Set as X, Y, Z, Y Error, Labels, Disregard, Grouping, Subject; move first/left/right/last; swap |
| Layout | Add Graph, Add Worksheet |
| Mask | Mask range, Unmask range, Swap mask, Change mask colour, Hide/Show masked points, Disable/Enable masking; variants for active plot vs all plots |
| Tools | Pointer, Scale In/Out, Screen Reader, Data Reader, Data Selector, Draw Data, Text, Arrow, Curved Arrow, Line, Rectangle, Circle, Polygon, Freehand, Polyline, Region tools, Regional Data Selector, Regional Mask, Zoom-Panning, Scale Zoom-Panning, Rotate; insert Word/Excel/Equation/OLE objects; Insert Graph/Worksheet/Page |
| Object Edit | align left/right/top/bottom/horizontal/vertical; uniform width/height; Front, Back, Front(data), Back(data), Group, Ungroup |
| Arrow | horizontal/vertical snap, widen/narrow, lengthen/shorten head |
| Markers & Locks | Add/Clear Data Markers, Marker Size, Marker Hide/Show, Lock Positions |
| Style | line/border colour, style, width; fill pattern, colour, width; pattern colour; font colour; merge cells; apply palette; 3D lighting |
| Format | font, size, bold, italic, underline, superscript, subscript, super-subscript, Greek, bigger/smaller font |
| AutoUpdate | block autoupdate |
| Database Access | Load ODQ, Preview Import, Import data, Remove SQL, Query Builder, SQL Editor |
| Border toolbar (cells) | clear/left/top/right/bottom/frame/inside borders etc. |
| Docked panels (not toolbars) | Project Explorer, Data Display, Data Info, Quick Help, Results Log, Command Window |

A separate lightweight menu/toolbar definition (`OLitetb.xml`, `OriginLiteMenu.ini`) is the
stripped-down variant for the Origin Lite edition and not needed.

---

## D. Important dialogs and their option groups

Dialog titles and control captions are OBSERVED in the resource tables; behaviour not
described here is UNKNOWN.

### D1. Plot Details (P0) - title "Plot Details", tabbed, tree on the left
The left tree lists page, layers, and every plot; the right side shows tabs for the
selected node. Buttons: Apply, OK, Cancel, Worksheet (open source).
Tabs seen for a line/scatter plot:
- **Line**: Connect (straight, step, spline...), Style, Width, Color, Transparency, Fill Area
  Under Curve, gap to symbol, draw line in front/behind symbols.
- **Symbol**: shape (geometric, alphabetic, incremental, row-number, user-defined), size,
  scaling factor, edge colour and thickness, fill colour, interior style, transparency,
  overlapped points offset, "size based on" a column, skip-points rule via Drop Lines tab.
- **Drop Lines**: style, width, colour, skip points (show every Nth), drop to surface.
- **Label**: font, colour, size, justify, attach to, rotation, X/Y offset, white-out.
- **Group**: edit mode (independent, dependent), group members, increment rules (colour, line
  style, symbol shape/size cycle automatically across the plots of a group).
- **Pattern** (bars, area): fill colour, pattern, border colour/width, background.
- **Data** (histogram): bin size, number of bins, begin/end, distribution curve.
- **Error bar**: colour, line width, cap width, through symbol, direction, plus/minus,
  absolute or relative, "apply to all".
- **Spacing** (columns): gap between datasets (%), overlap (%), bar thickness (%).
- **Color Mapping / Colormap**: levels with colours, fill, contour lines, missing values.
- **Stack / Offset / Waterfall**: offset mode (none, cumulative, constant, auto, individual),
  X and Y offsets, gap, normalize to percent.
- **Layer-level tabs**: Size/Speed, Background, Display, Link Axes Scales (link a child layer's X or Y
  to the parent: none, straight 1:1, custom X = a*X' + b), Stack.
- **Page-level tabs**: Print/Dimensions, Legends/Titles, Miscellaneous, Display.

### D2. Axis dialog (P0) - opened by double-clicking an axis, or Format > Axes
Tabs: **Scale** (From, To, Type linear/log/etc., Rescale mode, Increment, First tick, minor
ticks count, ticks from dataset); **Title & Format** / Line and Ticks (show axis and ticks,
colour, thickness, major/minor tick length, tick direction inside/outside/none, axis position
at left/bottom or at a percent/value of the other axis); **Tick Labels** (show major labels,
type: numeric, text, date, from dataset; font, colour, bold, size; display: decimal,
scientific, engineering; divide by factor; decimal places; prefix, suffix; rotation);
**Minor Tick Labels**; **Custom Tick Labels** (rotation, special ticks at fixed values);
**Grids** (major/minor grid lines, colour, type, thickness, grid at opposite side, line at
axis = 0 (zero line), additional lines); **Break** (axis break from/to, position, scale after
break, minor ticks); an "Apply To" prompt for copying settings to other axes.
The axis has per-axis independent settings for each of bottom/top X and left/right Y.

### D3. Layer / Page properties (P1)
Layer Properties: size/position, background, link axes, "Stack". Page Properties (F2): width,
height, units, page colour, print preferences, speed mode (skip points when needed; maximum
points per curve), connect line across axis break and missing data, legend preferences.

### D4. Set Column Values (P0) - title "Set Column Values"
Controls: Row range "For row (i) ... to ..."; a formula edit box (with F(x) function list);
Add Function, Add Column (inserts a column reference); Function Description pane; Undo; OK;
Cancel; AutoUpdate (recalculate when inputs change); a "Before formula scripts" pane.
Also "Set Multiple Columns Values".

### D5. Worksheet / Column properties (P0)
Column Format dialog: Column Name, Plot Designation (X, Y, Z, Err, Label, Disregard, ...),
Display Type (text & numeric, numeric, text, date, time, month, day of week) with Format and
numeric display (decimal, scientific, engineering, decimal with commas), internal data type,
Column Width, column label rows, "apply to all columns to the right", Prev/Next. Window
Properties (Alt+Enter): long name, short name, comments, show-title option (name/long/both),
type, location, size, created/modified times. Worksheet Display Control (Format > Worksheet [F4]):
fonts, heading height and row heading width, column name/label rows to show, background and
grid lines, "Multiple X" columns. Right-click header menu on a column offers Plot, Set As,
Set Column Values, Fill with..., Filter, Mask, Sort, Normalize, Frequency Count, Statistics
on Column, Properties. (OBSERVED context menu resource 570.)

### D6. Import (see B2) (P0)
### D7. Nonlinear Curve Fit (NLFit) (P0)
Opened by Analysis > Fitting > Nonlinear Curve Fit (OXF `FitNL`; dialog code lives in
`nlsfWiz9_64.dll` and `NLSFwizard.ini`, not decoded). Observed structure: Settings page tree with
nodes Function Selection (category, function, function description, parameters), Parameters
(value, fixed, bounds, share), Bounds, Fitting (iterations, tolerance, algorithm Levenberg-Marquardt or
Simplex; the iterate X-Function `nlfit` exposes `n` (iterations, default -1) and `method` lm|s),
Weight, Output (report sheet, fitted curves, residuals, confidence/prediction bands). Buttons:
Fit, One Iteration, 100 Iterations, Revert, Done. Bottom action row: Fit, Reduce chi-sqr...
Menus in the Fit window (e.g. "Adjust" and "Baseline" menus, OBSERVED resource 2748, are for
Peak Analyzer baseline editing, see D9). UNKNOWN: exact option names; check in Origin.
Related: Fitting Function Builder [F8], Organizer [F9] (user fit functions), Fit Function Wizard.

### D8. Integrate / Differentiate / Smooth (P0)
OBSERVED parameters (X-Function `integ1`):

| Name | Meaning | Default |
|---|---|---|
| iy | input XY range | active |
| baseline | use straight line through end points as baseline (0/1) | 0 |
| type | Area Type: math (signed area), abs (absolute area) | math |
| plot | Plot Integral Curve: none, new graph, source graph | none |
| x1, x2 / i1, i2 | integration limits by X value or index | whole range |
| area, y0, x0, dx | outputs: area, peak height, X at peak, width at half height | |
| rescale | rescale source graph | 0 |

`differentiate`: derivative order (1-9, default 1); Savitzky-Golay smoothing before
differentiating (0/1) with polynomial order (default 2) and window points (default 20); plot
derivative curve.
`smooth` (Signal Processing > Smooth): Method: aav Adjacent-Averaging, sg Savitzky-Golay, pf Percentile
Filter, fft FFT Filter, lw Lowess, le Loess, bm Binomial (default index 1 = Savitzky-Golay);
points of window (default 5; range 5-205), polynomial order (default 2; 1-5), boundary condition
(none, reflect, repeat, periodic, extrapolate; default none), weighted average, span (0-1, default
0.1) for local regression, percentile (default 50), cutoff frequency, pad to power of 2, filter type,
baseline to subtract (end-point straight line, 1% mean ends, none).
`fft_filters`: filter type, lower/upper cutoff, cutoff, pass/stop frequency, threshold, keep DC
offset (default 1).
All three have an Input range, an output range, a Preview button, Auto Apply checkbox
(from the generic X-Function dialog), a theme dropdown (save and reuse settings) and "Recalculate"
mode (None/Manual/Auto) on the output so results update when data change (OBSERVED
variable labels; recalc modes INFERRED).

### D9. Peak Analyzer (P0)
Entry: Analysis > Peaks and Baseline > Peak Analyzer (`pa.OXF`, parameters: input range, theme
name, script mode). A wizard with pages: Goal (Subtract Baseline, Integrate Peaks, Find Peaks,
Fit Peaks - one chosen), Baseline Mode (None, Constant, Use Existing Dataset, User Defined,
Positive Peak/Negative Peak auto-detection...), Baseline Creation (anchor points, interpolation:
linear, spline, B-spline, etc.), Baseline Subtraction, Peak Finding (method, direction, thresholds),
Peak Integration, Peak Fitting. (Page names are INFERRED from public knowledge; OBSERVED
files: theme/session store `PeakAnalyzer.otw`, `PICKPEAK.OGS`, preview graph `PeakAnalyzerPreview.otp`,
report `PKREPORT.otp`, and a baseline editing menu with Refit Left/Right Segment, Move Segments by
Cursor, Progress Baseline, Linear Connect, Cubic Connect, Step at Peak, Step at Half Area, Move
Baseline by Cursor, OBSERVED in resource 2748.)
Peak finder `pkFind` parameters (OBSERVED): smooth points; direction p/n/b; method: local maximum,
window search, 1st derivative, 2nd derivative, residual after 1st derivative; local points;
size option raw or percent; height (default 20), width (default 50); filter by height/number/none;
value; maximum half width; foot height; outputs for centre, left, right marker indices.
`blauto`: auto baseline with threshold (default 0.05) and number of points (default 8).
`fitpeaks` (Multiple Peak Fit): type gauss/lorentz/voigt; number of peaks (1-10); report and fitted-curve outputs.

### D10. Tools > Options (P1) - tabs
Numeric Format (display digits, scientific thresholds, angular unit, ASCII import separator),
Graph (dashes, anti-aliasing, symbol gaps and default fill, OLE, log-scale floor), Axis (numeric
and text scales: ticks inside/outside, line colour and width, tick lengths, max number of ticks),
Text Fonts (default font, Greek, size, sub/superscript offsets and sizes), Page (redraw, preview,
margins, resolution), Miscellaneous (X/Y scale factors, long names, auto short-name,
multi-ASCII import template, printing, collapsible menus), Open/Close (default new-window types,
autosave interval, backup before saving, prompts), File Locations (user files folder, ASCII file
types), Excel (internal vs external workbooks), Windows, Toolbars.

### D11. Export Graphs - see B4. Preview button and per-format settings are present.

### D12. Other dialogs
Layer Contents (D-graph), Select Columns for Plotting (assign worksheet columns to X, Y, yEr,
xEr, Label; Add/Replace/Delete; Set X values from..step), Error Bars (add to dataset: percent
of data, standard deviation, square root, scaling factor), Plot Range (from/to row), Page
Control, Find (in project: names, labels, folders), Nested Sort (criteria list), Color Palette,
Set Matrix Values, Matrix Properties/Dimensions, Programming Control, Worksheet Script,
Extract Worksheet Data, Select Data Set, Delete Confirmation.

---

## E. Keyboard shortcuts (OBSERVED accelerator tables, `Origin93_64.exe` and `ok9_64.dll`)

| Key | Command |
|---|---|
| Ctrl+N / Ctrl+O / Ctrl+S / Ctrl+P | New, Open, Save (project), Print |
| Ctrl+E | Open Excel |
| Ctrl+K | Import ASCII |
| Ctrl+3 / Ctrl+4 | Import Wizard / Re-Import Directly |
| Ctrl+Q / Ctrl+Shift+Q | Set Column Values / Set Multiple Columns Values |
| Ctrl+D | Add New Columns |
| Ctrl+G | Go to cell |
| Ctrl+F / Ctrl+H | Find in sheets / Replace |
| F3 | Find in Project |
| Ctrl+Z, Alt+Backspace | Undo |
| Ctrl+C / X / V | Copy / Cut / Paste |
| Ctrl+Alt+C, Ctrl+Shift+C, Ctrl+J, Ctrl+Shift+J, Ctrl+Alt+J, Ctrl+Shift+A | Copy variants (full precision, with labels, formula, label rows, formula+labels, all) |
| Ctrl+J (graph) | Copy Page |
| Ctrl+Alt+V | Paste Link |
| Ctrl+R | Rescale to show all (graph); merge cells (sheet format) |
| Ctrl+L | Reconstruct Legend |
| Ctrl+I / Ctrl+M / Ctrl+W | Zoom in / Zoom out / Whole page |
| Ctrl+1 / Ctrl+2 / Ctrl+0 | Page view / Print view / Draft view |
| Ctrl+Shift+J | Full screen |
| Ctrl+Alt+M / Ctrl+Alt+N | Mark data range / Clear markers |
| Ctrl+Shift+M, K, F, L | Hide analysis markers; lock to layer icon / first point / last point |
| Ctrl+T | Toolbars dialog |
| Ctrl+U | Options |
| Alt+1 ... Alt+9 | Project Explorer, Results Log, Command Window, Code Builder, Quick Help, Messages Log, Smart Hint Log, Object Manager |
| Alt+Shift+3 | Script Window |
| F2 | Page Properties (graph/layout); rename in project explorer |
| F4 | Worksheet properties |
| F5 | Refresh |
| F7 / F8 / F9 / F10 / F11 | Theme Organizer / Fitting Function Builder / Fitting Function Organizer / X-Function Builder / Script Samples |
| F12 | Layer Contents |
| Alt+Enter | Properties |
| Ctrl+Shift+D / I / C / X / N | Matrix: data mode / image mode / column-row / X-Y / thumbnails |
| Ctrl+Shift+T | Region of Interest tools (matrix) |
| Ctrl+Alt+B | Button edit mode |
| Ctrl+B, I, U, G, +, =, -, <, > | Text object formatting: bold, italic, underline, Greek, superscript, subscript, super-subscript, font up/down (in-place text edit) |
| Ctrl+A | Select all windows (Project Explorer) |
| Del, Shift+Del | Delete / delete without prompt (project explorer) |

---

## F. Window types and the Project Explorer

OBSERVED window kinds (File > New and Project Explorer menus): Workbook (worksheets), Matrix
book (matrix sheets and image thumbnails), Graph (pages containing layers), Layout page,
Notes, Excel workbook (embedded or linked), Function window (a plot of an expression), Image
(single image window), Digitizing image window, and Master graph page. Non-document windows:
Project Explorer (Alt+1), Object Manager (Alt+8), Results Log (Alt+2, persistent text log with filter
modes), Command Window (LabTalk interactive, Alt+3), Script Window, Code Builder (Origin C
editor, Alt+4), Quick Help (Alt+5), Messages Log (Alt+6), Smart Hint Log (Alt+7), Data Display
and Data Info panels, status bar.

Project Explorer (OBSERVED context menus, resource 2753): a split view; the upper part (or
left, user choice) is a tree of folders with the project at the root, the lower part a list
of the windows in the selected folder (views: none, windows in active folder, in active folder
and subfolders). Context menus per selection: Root, One Folder, Multiple Folders, One Window,
Multiple Windows, None Selected, Notes window, Header. Folder commands: Find, Append Project,
Save As Project (saves a folder as its own project), Show/Hide All Windows, Delete Folder,
Rename, Restricted in Viewer, New Window (Worksheet, Graph, Matrix, Excel, Notes, Layout,
Function), New Folder, View Windows, Send Graphs to PowerPoint, Properties. Window commands:
Save As, Activate, Restore, Minimize, Maximize, Show, Hide, Skip Slide, Delete, Duplicate,
Duplicate (Batch Plotting), Rename, Auto Short Name, Dependents (lists windows that depend on
this one, e.g. graphs built from a book), Copy Page, Export Graphs, Send to PowerPoint, View
Full Screen, Print, Go to Original Folder, Properties. Keyboard: F2 rename, Del delete,
Ctrl+A select all windows, F5 refresh, Alt+Enter properties.
Every window has a short name (identifier for scripts, no spaces) and a long name (free
text); the title bar can show either or both. Sheets live inside books, layers inside graphs.

Right-click menus (OBSERVED, `ou9_64.dll` ids 570/571/581/584): worksheet (cell, row number,
column header, whole-sheet corner, outside area), graph (axis, layer icon, page, plot line, data
point, legend, text, objects), matrix (cell, corner, thumbnails).

---

## G. Open unknowns (need to be checked in a running Origin)

1. Exact order and grouping of the Analysis, Statistics, Gadgets and File>Import/Export dynamic menus
   (only category folders and labels are observed, `xfdump_p_e.xfd`).
2. Export Graphs per-format settings (dpi default, colour depth, compression, transparency)
   and the default image type index.
3. Full option tree of the ASCII/CSV import filter (`options`, header-line handling) and default values;
   the `.oif` filter files in `Filters` were not parsed here.
4. NLFit and Peak Analyzer dialog pages in detail (the dialogs are built by `nlsfWiz9_64.dll`
   and the Peak Analyzer wizard script, not by the resource tables).
5. Which menu resource id (507, 510, 511, 532-534) maps to which special window.
6. Plot Details tab availability per plot type and exact control ranges.
