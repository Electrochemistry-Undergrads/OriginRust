# Feature parity: Sheaf against Origin 2016 (9.3)

A snapshot. Update a row when its feature lands; re-run the audit after each milestone.

Date: 2026-11-11.

This document answers the owner's question: does Sheaf have all of Origin's features? The short answer is no. Sheaf does the owner's daily jobs well, but it has about a third of what Origin offers in some areas and almost nothing in others. It writes .opj files from the command line only (the app does not yet), and its window layout and menus differ clearly from Origin's.

## How the audit was made

Five auditing agents compared the specs in `docs/spec/` with the Rust code. Status was judged from the code: the agents opened the function and checked that a menu entry, dialog or command-line subcommand reaches it. They merged the several hundred audit rows into the 231 rows below, so one row can stand for several Origin features. The counts therefore count rows, not individual Origin features.

What was not done: nobody built or ran the app or the CLI, so wiring was judged from reading code and not by clicking. Nobody ran Origin through `tools/origin-ref/`, so the Origin side of every row comes from the specs, and many spec items are themselves marked unverified. No test suite was run, so numeric accuracy is only as good as the claims already recorded in the specs.

Statuses: ✓ done, ◐ partial, ✗ missing. Priorities: P0 is the owner's daily work and the owner's explicit requests, P1 is useful, P2 is rare.

## Counts

| | Done ✓ | Partial ◐ | Missing ✗ | Total |
|---|---|---|---|---|
| All rows | 38 | 99 | 93 | 230 |
| P0 | 30 | 64 | 7 | 101 |
| P1 and P2 together | 8 | 35 | 86 | 129 |

By priority alone: 103 rows are P0, 86 are P1 and 42 are P2. The P1 and P2 rows were not counted separately by status.

Reading the counts: the P0 rows are mostly present but often only partly (61 of 103 are partial). The 14 missing P0 rows are the real holes. The main ones are .opj saving, toolbars and the Origin look, copy and paste of cells, date and time columns, recalculation of column formulas, Double-Y graphs, offset and waterfall plots, per-peak integration with the Peak Analyzer, subtracting a reference curve, simple curve math, graph gadgets and analysis from a graph, and the shared analysis dialog frame.

Cross-cutting findings:
- Column formulas now recalculate (None, Auto, Manual; see workbook-model.md 10a). The CHI import writes `col*1000` formulas that stay one-time values (mode None).
- Analysis and fit results are static sheets, not linked to their source data.
- The core model holds several layers per graph, but the app edits only the first.
- `crates/io/src/opj/write` writes workbooks and graphs as Origin 2016 projects (CLI `sheaf export -o x.opj`); the app does not use it yet, and matrices, notes, folders and fit results are not written.

## 1 Project, windows and menus

| Feature | Origin | Sheaf | Status | Gap | Priority |
|---|---|---|---|---|---|
| Project save and open, own format | File > Save/Open, Ctrl+S/O | .sheaf zip; New/Open/Save/Save As; prompt on quit; CLI | ✓ | none | P0 |
| Save project as .opj (owner's request) | File > Save Project As | `io/src/opj/write`; CLI `export -o x.opj` | ◐ | workbooks and graphs are written and open in Origin 2016 (spec 13.9); the app has no menu entry yet; folders, matrices, notes, grid style, legend icons for spline curves not written | P0 |
| Open .opj workbooks and basic graphs | File > Open | `io/src/opj`; app, drag and drop, CLI | ◐ | skips matrices, notes, layouts, folders, fit results, formulas, masks, column formats; .opju not read; saved as .sheaf copy; pre-9 graphs unverified | P0 |
| Undo and redo | Edit > Undo, Ctrl+Z | full history, labelled, Ctrl+Z/Y | ✓ | superset of Origin's one level; Alt+Backspace unbound | P0 |
| Project Explorer | View > Project Explorer, Alt+1 | flat window list, right-click Show/Rename/Export/Delete | ◐ | no folder tree or editing UI (model has folders), no filters, Duplicate, Properties, search; cannot be hidden | P0 |
| Menu bar per window type | menus differ for book, graph, layout, matrix | one bar: File, Edit, Worksheet, Plot, Analysis, Graph, Window, Help | ◐ | no View, Column, Statistics, Gadgets, Tools, Format menus; Edit holds only undo and redo; bar does not change with the active window | P0 |
| Analysis and Statistics menu structure | Mathematics, Signal Processing, Fitting, Peaks and Baseline submenus; Statistics menu | one flat Analysis menu: 8 tools and 3 fit entries | ◐ | no submenus, no Statistics or Gadgets menu, not extensible | P0 |
| Toolbars | about 30, View > Toolbars | none (a small button row inside a graph) | ✗ | all families missing: standard, edit, graph, 2D plots, column, worksheet data, style, format, mask, tools | P0 |
| Origin look: MDI windows, classic chrome | main window | egui default style, tabs, left panel | ✗ | layout and chrome differ | P0 |
| Keyboard shortcuts | Ctrl+K import, Ctrl+Q set values, Ctrl+D, Ctrl+R, F-keys, Alt+n | Ctrl+N/O/S/Shift+S/I/Z/Y | ◐ | import is Ctrl+I (Origin: Ctrl+K, where Ctrl+I is zoom); most Origin keys unbound | P0 |
| New window kinds | File > New: Worksheet, Matrix, Graph, Image, Excel, Layout, Notes, Function Plot, from Template | Worksheet > New Workbook only | ◐ | no empty graph, matrix, image, layout, notes, function plot, new-from-template | P1 |
| Append project | File > Append | CLI `import --append` (workbooks only) | ◐ | none in the app; graphs not merged | P1 |
| Open Excel, sample project, Close | File menu | none | ✗ | Close of active window missing; Explorer Delete only | P1 |
| Save Window As, Save Template As, analysis template | File menu | none | ✗ | no .otp/.otw-style files; only 4 built-in figure templates | P1 |
| Print, Print Preview, Page Setup | File > Print, Ctrl+P | none | ✗ | graphs can only be exported to files | P1 |
| Recent projects, books, graphs, imports | File > Recent | none | ✗ | | P1 |
| Exit | File > Exit | File > Quit with save prompt | ✓ | | P1 |
| Window properties and rename | Alt+Enter | long name only | ◐ | short name automatic; no comments field or Properties dialog | P1 |
| Window menu | cascade, tile, duplicate, split, refresh | lists windows and focuses one | ◐ | no cascade, tile, Duplicate, Split, Refresh | P1 |
| Preferences | Tools > Options, Ctrl+U | none | ✗ | no settings stored | P1 |
| Panels | Results Log, Command Window, Object Manager, Quick Help, Data Display | one-line status bar | ✗ | results go to sheets; no log | P1 |
| Graph view modes and zoom | Page, Print, Window, Draft view; zoom commands | page fitted to window | ◐ | no user zoom or view commands | P1 |
| Find, replace, go to | Ctrl+F, Ctrl+H, F3, Ctrl+G | none | ✗ | | P1 |
| Set As Begin, End, Reset | Edit menu | none | ✗ | analysis dialogs take row ranges individually | P1 |
| Copy graph page to clipboard | Edit > Copy Page, Ctrl+J | none | ✗ | | P1 |
| Copy and paste format between graphs | Edit > Copy Format | "Use this style for all plots" button | ◐ | | P1 |
| Save or duplicate project without data | File menu | none | ✗ | | P2 |
| Other window types | Matrix, layout, notes, image, Excel windows; matrix operations, image processing, gridding, 3D | Window enum is workbook and graph only; project note text exists | ✗ | no matrix, layout, notes, image windows; no per-type menus | P2 |
| Autosave and recovery | Tools > Options | atomic save only | ✗ | | P2 |
| Tools managers and protection | Theme, Function Builder, Import Filters, Package, Custom Menu | none | ✗ | | P2 |
| Help | Help menu | About only | ◐ | no help pages | P2 |

## 2 Import

| Feature | Origin | Sheaf | Status | Gap | Priority |
|---|---|---|---|---|---|
| Delimited text engine, CSV | File > Import > ASCII, Ctrl+K | `io/src/ascii.rs`; dialog; CLI | ✓ | single-character delimiters only | P0 |
| Data-start detection | auto header lines | `detect()` with preview | ✓ | rule differs slightly from Origin's, not compared by black-box run | P0 |
| Header, long name, units, comments lines | File Header group | detected and overridable; header lines can become named parameter rows (`--param-lines`) | ◐ | no name+unit splitter, no leading-character rule, no count from bottom, comment lines are not joined, label detection looks at three lines at most | P0 |
| Non-numeric text, missing markers | NonNum, LeadingZeros | marker list, 80 percent rule, warning | ◐ | no choice of treatment, no mixed text and number column | P0 |
| Date and time detection | DateFormat, TimeFormat | `ascii.rs`: ISO, M/d/yyyy, d/M/yyyy, dd.MM.yyyy, d-Mon-yyyy, clocks with AM or PM; order rule and warning; option to keep text | ◐ | no custom pattern list, no per-column choice, no two-digit years | P0 |
| Import mode and naming | replace, new columns, rows, books, sheets | `io/src/batch.rs`: new book, new sheet, replace, new columns, new rows; import window; file name in sheet name or comment row | ✓ | no name trimming (FNameToShtFrom/To), no separate first-file mode | P0 |
| Number separators | NumSep | comma, dot, space, apostrophe grouping and decimal comma; found when unambiguous; option in window and CLI | ✓ | stricter than Origin: groups must have three digits | P0 |
| CH Instruments files | owner's workflow | `io/src/chi.rs`: CV, i-t, LSV, scan splitting, mA, charge, sign, clip warning, run parameters | ✓ | no real CHI file checked yet per the spec | P0 |
| Batch import of many files | multiple files | dialog, drops, CLI `sheaf import <folder or pattern> --mode books/sheets/columns/rows`; CHI files go through the CHI importers | ✓ | | P0 |
| Partial import | row, column ranges, read/skip patterns | skip rows, max rows | ◐ | no column range or patterns; max rows has no dialog control | P1 |
| Fixed width, multi-character delimiter | ASCII options | none | ✗ | | P1 |
| Encoding | none in Origin 2016 | BOMs, UTF-8, Windows-1252 | ✓ | no manual choice | P1 |
| Import filters and saved settings | .oif, Import Filters Manager | none | ✗ | | P1 |
| Import wizard, re-import | Ctrl+3, Ctrl+4 | one-page dialog; source path recorded | ◐ | no re-import command, no remembered settings | P1 |
| Excel import | File > Import > Excel | none | ✗ | no spreadsheet crate | P1 |
| Drag and drop | drop on window | projects open, data goes to dialog | ✓ | | P1 |
| Other importers, image, database | HDF5, NetCDF, JCAMP-DX, SPC, WAV, ODBC and more | none | ✗ | | P2 |
| Sparklines, auto width, write protection on import | ASCII options | none | ✗ | | P2 |

## 3 Workbook and columns

| Feature | Origin | Sheaf | Status | Gap | Priority |
|---|---|---|---|---|---|
| Book, sheet, column containment | workbook model | `core/worksheet.rs`, `column.rs` | ✓ | | P0 |
| Names and limits | 13-char book, 18-char column | validation of characters only | ◐ | length limits and uniqueness not enforced in the UI | P0 |
| New worksheet default shape | A(X), B(Y), 30 rows | `Project::add_workbook` | ◐ | not compared with Origin's | P0 |
| Label rows: long name, units, comments | worksheet header | grid rows and properties dialog | ✓ | | P0 |
| Parameter, user parameter, F(x) rows, show or hide label rows | label rows | stored in model, not shown | ◐ | not shown or editable; no sampling-interval row | P0 |
| Plot designations | Column > Set As | X, Y, Z, yEr, xEr, Label, Disregard | ✓ | no Group or Subject; Y-finds-X is nearest-left only | P0 |
| Column data formats | Numeric, Text, Time, Date, Month, Weekday, mixed | numeric, text, date, time, date-time (Origin's numbers) | ◐ | no month, weekday, mixed; fewer display patterns than Origin; Column Properties dialog sets the display pattern only | P0 |
| Numeric display format | decimals, significant, scientific, engineering | General, Decimal, Scientific, Significant | ◐ | no engineering or custom, no column width | P0 |
| Missing values | shown as `--` | NaN shown as `--` | ✓ | cannot tell empty from missing | P0 |
| Reference notation in formulas | `[Book]Sheet!Col`, cell(), `col(A)$`, `[L]` | `col`, `wcol`, index, range, `i` | ◐ | no other sheet or book, no cell(), no label access, no x/y shorthand | P0 |
| Sorting | nested, range, columns | one key for the whole sheet | ◐ | no nested keys, no range or column sort | P0 |
| Add, delete, move columns | Ctrl+D | add one, delete | ◐ | no insert at position, count dialog, move, swap, hide, no row insert or delete | P0 |
| Sampling interval, Show X Column | Column menu | none | ◐ | no Set Sampling Interval, no Show X Column | P0 |
| Cell selection, editing, copy, cut, paste, clear, fill | grid and Edit menu | double-click edit with undo, column select | ◐ | no cell or range selection, no clipboard code at all | P0 |
| Multiple sheets per book | tabs menu | tabs switch only; analysis adds result sheets | ◐ | cannot add, rename, move, delete a sheet | P0 |
| Set Column Values | Ctrl+Q | dialog with formula, rows from and to | ◐ | no Ctrl+Q, no function picker | P0 |
| Built-in functions | Origin function list | about 50 (estimate) in `crates/expr` | ◐ | no string, date, random, cumsum, diff, rank, lookup; no user functions | P0 |
| Recalculation (None, Auto, Manual) | Set Column Values | mode stored with the formula; Auto columns follow their inputs inside the same undo step, Manual ones show `pending` until Recalculate; cycles refused; spec in workbook-model.md 10a | ◐ | a coloured word in the header instead of the lock icon; same-sheet inputs only; analysis and fit results are not linked | P0 |
| Simple math on curves | Analysis > Mathematics > Simple Math | Simple Math window, `analyze math`, `Tool::CurveMath` | ✓ | add, subtract, multiply, divide, power with a constant or a reference; checked against Origin (spec section 9); no graph rescale | P0 |
| Row filter | Column > Filter | Filter Rows, in place or new sheet | ◐ | removes or copies, not a live hidden filter | P1 |
| Masking | Column > Mask | none | ✗ | model has no mask; .opj reader skips it | P1 |
| Internal types | double, short, long, complex | f64 only | ✗ | | P1 |
| Categorical columns | Set as Categorical | none | ✗ | | P1 |
| Worksheet display and view options | Format > Worksheet F4; hide, freeze, zoom | fixed layout | ✗ | | P1 |
| Before-formula script, multi-column set | Set Column Values | none | ✗ | one column only | P1 |
| Fill Column With | row numbers, random, patterns | only formula `i` | ◐ | no random, series or dates | P1 |
| Date and time arithmetic | date columns | columns are plain numbers in formulas (days) | ◐ | no `date()`, `time()` or date functions | P1 |
| String functions and text in formulas | text functions | text reads as missing | ✗ | | P1 |
| Worksheet operations | transpose, stack, unstack, join, reduce, copy, duplicates, pivot | none | ✗ | | P1 |
| External links, Paste Link | Edit menu | none | ✗ | | P2 |
| Complex numbers | complex type | none | ✗ | | P2 |
| Worksheet script, query, clear | Worksheet menu | none | ✗ | | P2 |

## 4 Scripting, batch and interoperability

| Feature | Origin | Sheaf | Status | Gap | Priority |
|---|---|---|---|---|---|
| Batch processing with analysis template | File > Batch Processing | shell loops over the CLI | ◐ | no batch command, glob, saved template or combined result table | P0 |
| X-Function mechanism | X-Function Builder | analysis dialogs and `sheaf analyze` subcommands | ◐ | no common mechanism, no user tools, nothing callable by name | P1 |
| Command-line control | Origin automation | `info`, `import`, `plot`, `export`, `export-data`, `analyze`, `fit` | ✓ | commands are independent; `export-data` reads .sheaf only | P1 |
| Batch plotting | Duplicate (Batch Plotting) | one overlay figure per CLI call | ◐ | no graph per book or sheet, no app command | P1 |
| Export worksheet as text | File > Export > Worksheet | `write_delimited`; CSV or TSV by extension | ◐ | no options dialog; no semicolon, label rows, selected range, full-precision switch | P1 |
| Write xlsx | export worksheets | none | ✗ | | P1 |
| Dependents and linked results | Window Properties > Dependents | none | ✗ | results unlinked from sources | P1 |
| Summary table across sheets | batch summary | none | ✗ | | P1 |
| LabTalk, Origin C, Python, MATLAB, R consoles | scripting windows | none | ✗ | only the formula language exists | P2 |
| COM server | Origin COM object | none | ✗ | | P2 |
| Read themes and templates | .oth, .otp | none | ✗ | | P2 |
| Read .opju | File > Open | none | ✗ | | P2 |
| Other exports | sheet as image, multi-page PDF, matrix ASCII, GIF | none | ✗ | | P2 |

## 5 Mathematics and data manipulation

| Feature | Origin | Sheaf | Status | Gap | Priority |
|---|---|---|---|---|---|
| Integrate | Mathematics > Integrate | `calc/integrate.rs`, checked against Origin; dialog; CLI | ✓ | | P0 |
| Integrate range, baseline, charge | integ1 with limits and baseline | typed rows or X range, end-point baseline, unit-aware area | ◐ | baseline is end-point line only; range typed, not picked on graph | P0 |
| Differentiate | order 1 to 9, smoothing | `calc/differentiate.rs` | ✓ | higher orders with smoothing not checked against Origin | P0 |
| Subtract straight line | subtract_line | Subtract Baseline: Line, End points | ◐ | typed coordinates, no on-graph pick | P0 |
| Subtract reference curve | subtract_ref | Subtract Reference Data window, `analyze subtract-ref` | ✓ | linear interpolation, common range option, checked against Origin (spec section 9) | P0 |
| Normalize | 12 methods | `calc/normalize.rs`, checked | ◐ | no reference column or cell methods; no row normalize | P0 |
| Interpolate and resample | interp1xy | linear, spline, Akima, B-spline | ◐ | extrapolation option not exposed; B-spline default does not match; new sheet only | P0 |
| Interpolate onto given X, trace | interp1, interp1trace | library function only | ✗ | no app or CLI route | P1 |
| Translate X or Y | cxt, vshift | none | ✗ | | P1 |
| Level crossing and curve intersect | levelcrossing, intersect | none | ✗ | | P1 |
| Data reduction | reducerows, reducedup | none | ✗ | | P1 |
| Sampling interval check | vinc | internal use only | ✗ | | P1 |
| Average multiple curves | avecurves | none | ✗ | | P1 |
| Cumulative sum | Mathematics | running integral only | ✗ | | P1 |
| Convolution, deconvolution | Signal Processing | none | ✗ | | P2 |
| XYZ gridding, matrix conversion | Convert to Matrix | none | ✗ | no matrix window | P2 |
| Rank, remove duplicates | wdeldup | none | ✗ | | P2 |
| Rise time, polygon area, roots, 2D integration | gadgets and Mathematics | none | ✗ | | P2 |
| 2D and 3D interpolation, matrix math, white noise | Mathematics folder | none | ✗ | | P2 |

## 6 Signal processing

| Feature | Origin | Sheaf | Status | Gap | Priority |
|---|---|---|---|---|---|
| Smooth: Savitzky-Golay, adjacent average, weighted, percentile, FFT filter | Signal Processing > Smooth | `calc/smooth.rs`, checked against Origin; boundary options | ✓ | no Preview, auto-apply or recalculate | P0 |
| Smooth: LOWESS, LOESS, binomial | Smooth dialog | none | ✗ | | P1 |
| FFT and inverse FFT | fft1, ifft1 | only inside smoothing | ✗ | | P1 |
| FFT filters | low, high, band pass, block, notch | low-pass inside Smooth only | ✗ | | P1 |
| Correlation, wavelets, decimation, Hilbert, STFT, digital filters | Signal Processing | none | ✗ | | P2 |

## 7 Peaks and baseline

| Feature | Origin | Sheaf | Status | Gap | Priority |
|---|---|---|---|---|---|
| Peak finding | Peak Analyzer, local maximum | `calc/peaks.rs`, checked; `calc/peakchain.rs` adds window search, derivative methods, smoothing, width filter (own rules, spec section 11); Find Peaks and Analyze Peaks windows | ◐ | only the local maximum method is checked against Origin | P0 |
| Baseline modes | constant, points, dataset, end points, ALS | Minimum, Constant, End points, Line | ◐ | no anchors (library function not reachable), dataset baseline, ALS, auto detect, preview | P0 |
| Baseline subtraction | pa_basesubtr | new Y column beside source | ✓ | limited by modes | P0 |
| Per-peak integration | area, centre, height, FWHM per peak | `analyze peaks --integrate`, Analysis > Analyze Peaks (automatic limits at the baseline or valley, result table); Integrate Ranges for typed ranges | ◐ | areas, heights and widths checked against `integ1` over the same rows; the choice of limits is Sheaf's own; no picking on a graph | P0 |
| CV charge with sloped baseline | integ1 with baseline | end-point baseline over typed range | ◐ | no foot picking, no anodic and cathodic split, no scan-rate division | P0 |
| Peak Analyzer wizard | goal, baseline, peaks, results | one dialog runs baseline, search, integration and fit (spec section 11) | ◐ | no wizard pages or preview, baseline anchors not picked on a graph | P0 |
| Multiple peak fit | Peaks and Baseline | sum of Gauss or Lorentz peaks on a constant or sloped baseline, started from found peaks, per-peak report (`fit/multipeak.rs`) | ◐ | one shape for all peaks, no constraints, not compared with Origin's own multi-peak fit | P0 |
| Peak results layout | Origin result sheets | Integrate and Find Peaks sheets | ◐ | no area percent, FWHM, separate baseline sheet | P0 |
| Multi-curve peak analysis | paMultiY | tools accept several Y columns | ◐ | not one run with one result set | P1 |

## 8 Gadgets and graph analysis

| Feature | Origin | Sheaf | Status | Gap | Priority |
|---|---|---|---|---|---|
| Gadgets and data cursors | integrate, statistics, quick fit, quick peaks, FFT, rise time on a graph region | coordinate read-out only | ✗ | none exist | P0 |
| Analysis menu on the active graph | graph Analysis menu | Analysis menu is workbook-only | ✗ | | P0 |
| Data menu | display range, markers, mask points, remove bad points | none | ✗ | | P1 |

## 9 Fitting

| Feature | Origin | Sheaf | Status | Gap | Priority |
|---|---|---|---|---|---|
| Linear regression | Linear Fit | `fit/linear.rs`; dialog; CLI | ✓ | | P0 |
| Polynomial regression | Polynomial Fit | `fit_polynomial` | ✓ | | P0 |
| Nonlinear fit engine | Levenberg-Marquardt, bounds, fixed, weights | `fit/nlsf.rs`; verified against Origin on 21 cases | ✓ | | P0 |
| Fit report | parameters, errors, t, p, limits, chi-square, R-square, ANOVA | `fit/report.rs`; separate sheets | ✓ | covariance and correlation matrices not written | P0 |
| Core function set | Gauss, Lorentz, Voigt, exponentials, Boltzmann, Hill | `functions.json`, categorised list | ✓ | | P0 |
| Automatic initial guess | nlfn | own guesses for 53 functions: peaks, exponentials, sigmoids, power laws, hyperbolas, polynomials to order 5 | ◐ | the rest of the 206 start from fixed values; no toggle | P0 |
| Fitted curve and residual output | NLFit Output | curve sheet, plot to graph | ✓ | number of points and output choices not in dialog | P1 |
| .FDF import | Fitting Function Builder | `fit/fdf.rs` | ✓ | 188 of 237 installed files build | P1 |
| Full function library | 237 functions, 30 categories | 206 in JSON, 188 more via .FDF | ◐ | 22 surface, 3 multi-variable and some others missing; some categories not offered. The UI auditor's estimate of about 23 functions conflicts with this; the fitting auditor's grep of the JSON is taken as correct | P1 |
| Fit control options | iterations, tolerance, Simplex, One Iteration, Revert | LM only; maximum iterations and tolerance in the API, `sheaf fit` and the Fit window | ◐ | no Simplex, One Iteration or Revert | P1 |
| Confidence and prediction bands | NLFit Output | `fit/bands.rs`; extra FitCurve columns and dashed plots; `--bands`/`--level`; Fit window | ✓ | band columns checked against Origin's closed form for a line, not read from Origin | P1 |
| Linear fit report details | covariance on sheets, residual plots | covariance only in library | ◐ | | P1 |
| User function and builder | F8, F9 | formula only; `.FDF` | ◐ | cannot save or reuse; no builder | P1 |
| Exponential, Sigmoidal, Single Peak shortcuts | Fitting menu | choose in Nonlinear Curve Fit | ◐ | no dedicated entries | P1 |
| Simulate curve | simcurve | none | ✗ | | P1 |
| Fit range and several Y columns | nlbeginr | rows or X range; app one column per run | ◐ | | P1 |
| Global fit | shared parameters | none | ✗ | | P2 |
| Surface fit | Fitsurface | none | ✗ | one variable only | P2 |
| ODR, fit with X error | FitODR | none | ✗ | | P2 |
| Multiple linear regression | fitMR | none | ✗ | | P2 |
| Compare models and datasets, rank models | F-test, AIC, BIC | none | ✗ | | P2 |
| Find baseline region, function wizard | findBase | none | ✗ | | P2 |

## 10 Statistics

| Feature | Origin | Sheaf | Status | Gap | Priority |
|---|---|---|---|---|---|
| Descriptive statistics on columns | Statistics on Columns | N, missing, mean, SD, SEM, sum, min, Q1, median, Q3, max | ◐ | variance, skewness, kurtosis, CV, IQR computed in `calc` but not shown; no confidence limits, mode, choice of items | P0 |
| Aggregate functions in formulas | Set Column Values | sum, mean, median, sd, var, count | ✓ | | P0 |
| Row statistics | Statistics on Rows | none | ✗ | | P1 |
| Moments, quantiles | quantiles, moments | quartiles only | ◐ | no arbitrary percentile | P1 |
| Correlation | corrcoef | Pearson r inside line fit only | ◐ | no matrix, no rank correlations | P1 |
| t-tests | Hypothesis Testing | one-sample, two-sample (pooled and Welch) and paired t-test with tails, level and confidence interval; Statistics menu, `analyze ttest`; checked against `ttest1`, `ttest2`, `ttestpair` (`statistics.md`) | ✓ | no test of equal variance, no summary-data input | P1 |
| Frequency counts, binning | freqcounts | counts, cumulative, percent, given or automatic bins, outlier rows; Statistics menu, `analyze histogram`; checked against `freqcounts` | ◐ | the automatic bin size is a fitted rule (about 57 % agreement) | P1 |
| Variance and proportion tests | vartest | none | ✗ | | P2 |
| ANOVA | one, two, three-way, RM | one-way with summary table and Tukey, Bonferroni, Sidak, Fisher LSD, Scheffe comparisons; checked against Origin's library | ◐ | no two-way, three-way or repeated measures | P2 |
| Nonparametric tests | Mann-Whitney, Kruskal-Wallis and others | none | ✗ | | P2 |
| Normality, outlier tests, distribution fit | Shapiro-Wilk, Grubbs | Shapiro-Wilk, Kolmogorov-Smirnov, Lilliefors, Anderson-Darling, D'Agostino-Pearson | ◐ | no outlier tests, no distribution fit, no probability plot | P2 |
| Multivariate analysis | PCA, clustering | none | ✗ | | P2 |
| Survival, power, ROC | Statistics menu | none | ✗ | | P2 |

## 11 Graph types

| Feature | Origin | Sheaf | Status | Gap | Priority |
|---|---|---|---|---|---|
| Line, Scatter, Line + Symbol | Plot menu | `PlotKind`; Plot menu; CLI | ✓ | | P0 |
| Cyclic voltammetry overlay | owner's workflow | built-in template; CLI `cv` | ✓ | | P0 |
| Overlay of many curves with grouping | Layer Contents | works across sheets and books | ◐ | grouping steps colour only | P0 |
| Error bars | Y, X, XY | symmetric Y only | ◐ | no X, direction, cap, style options; no dialog to attach error column | P0 |
| Double-Y, multiple Y axes | Multi-Curve | none | ✗ | one layer, one Y axis in the UI | P0 |
| Y offsets, waterfall | Multi-Curve | none | ✗ | | P0 |
| Column and bar charts | Column/Bar/Pie | `PlotKind::Column` drawn, created only by .opj import | ◐ | not creatable from menu, dialog or CLI; no bar, stacked, floating | P0 |
| Plot from selection | plotxy | uses designations | ◐ | no plotxy-style dialog | P0 |
| Layer Contents, add and remove data | F12 | Plots tab add, remove, reorder | ◐ | no separate X/Y/error assignment, no row range | P0 |
| Spline and step connect | Line > Connect | Catmull-Rom spline, steps | ◐ | no B-spline, Bezier, segments; spline skipped above 2000 points | P1 |
| Multi-panel and stack | 4 Panel, 9 Panel | none | ✗ | | P1 |
| Area and fill | Plot > Area | none | ✗ | | P1 |
| Drop lines, bubble, colour mapped | Plot > Symbol | none | ✗ | | P1 |
| Histogram, box chart, statistics plots | Plot > Statistics | histogram tool makes a column plot of the bin counts | ◐ | bars have gaps, no distribution curve, no box chart or other statistics plots | P1 |
| Pie | Plot > Pie | none | ✗ | | P2 |
| Polar, ternary, radar, Smith, wind rose | Specialized | none | ✗ | | P2 |
| Stock | Plot > Stock | none | ✗ | | P2 |
| Vector | Plot > Vector | none | ✗ | | P2 |
| 3D plots | Plot > 3D | none | ✗ | no 3D renderer | P2 |
| Contour, heat map | Plot > Contour | none | ✗ | needs matrices | P2 |
| Inset, pictures in graphs | Zoom, Insert picture | none | ✗ | | P2 |
| Other plot-type IDs when reading .opj | area, box, bar, histogram, pie | only types 200 to 203 mapped | ✗ | | P2 |
| Plotting X-Functions | plotgroup, plotbylabel, plotxyz | none | ✗ | | P2 |

## 12 Graph styling

| Feature | Origin | Sheaf | Status | Gap | Priority |
|---|---|---|---|---|---|
| Graph object model | page, layers, axes, plots, objects | `core/graph.rs` | ◐ | only layer 0 used; no top and right axes with own scale; no drawn objects | P0 |
| Units and sizing | cm, mm, pixel, scale modes | inches and percent | ◐ | | P0 |
| Axis scale and range | many scale types, rescale modes | linear, log10, from and to, nice, tight, symmetric | ◐ | no reverse, ln, log2, probability, custom ticks, rescale margins | P0 |
| Axis line and ticks | Line and Ticks tab | direction, length, width, colour | ◐ | no separate tick colour, offset, arrows | P0 |
| Tick labels | Tick Labels tab | font, number format, divisor | ◐ | no rotation, prefix and suffix, text labels, minor labels; date labels follow a date X column automatically, not chosen in the dialog and no month or year steps | P0 |
| Axis titles | Title tab | default from long name and units | ◐ | no `%(?X)` substitution, rotation, offset | P0 |
| Opposite and central axes | top and right, axis at value | top and right line and ticks only | ◐ | no labels or titles there; no axis at zero | P0 |
| Symmetric Y about zero | set by hand | automatic | ✓ | | P0 |
| Auto-scale and tick algorithm | automatic | `ticks.rs` from a proposed rule | ◐ | never validated against Origin | P0 |
| Rescale command | Ctrl+R | none | ◐ | fixed range cannot be re-fit except by hand | P0 |
| Line style, width, colour | Line tab | 8 dash styles, connect | ◐ | no transparency, fill under curve; dash patterns are guesses | P0 |
| Symbols | Symbol tab | 8 shapes, size, filled or open | ◐ | Origin has 20 shapes, 12 interiors; no separate fill and edge colour | P0 |
| Plot groups and increments | Group tab | colour only | ◐ | no line or symbol increments, modes | P0 |
| Colour palettes | Q01 to Q08, S, D lists | editable list of 9 | ◐ | not Origin's decoded default list; no named lists | P0 |
| Legend | Legend object | corners or manual, font, frame | ◐ | no substitution syntax, update modes, multi-column, drag | P0 |
| Text objects and escapes | text tool, escapes | `markup.rs` renders many escapes | ◐ | cannot create or edit free text labels in app or CLI; several escapes unsupported | P0 |
| Fonts | Format > Font | per axis and legend | ◐ | no underline, no default-font setting | P0 |
| Direct editing on the graph | select, drag, double-click | double-click opens a tab by region | ◐ | no object selection, handles, dragging | P0 |
| Plot Details dialog | tree, Apply, OK, Cancel | tabs, live apply | ◐ | no tree, no Worksheet button | P0 |
| Grids | Grids tab | major and minor with style | ◐ | no opposite grid or zero line | P1 |
| Axis breaks | Breaks tab | none | ✗ | | P1 |
| Reference lines, special ticks | add line | none | ✗ | | P1 |
| Drawn lines, arrows, shapes | Tools toolbar | none | ✗ | | P1 |
| Themes | Theme Organizer | none | ✗ | | P1 |
| Graph templates | Save Template As | 4 built-in only | ◐ | cannot save or load; .otp not read | P1 |
| Layer arrangement, merge | Arrange Layers | one layer frame editable | ◐ | | P1 |
| Backgrounds and frames | Background tab | page background, legend background | ◐ | no layer fill, border | P1 |
| Data reader, zoom, pan | Gadgets, zoom tool | free-cursor read-out | ◐ | no snapping, zoom or pan | P1 |
| Layer display options | clip, data on top | none | ✗ | | P2 |

## 13 Export

| Feature | Origin | Sheaf | Status | Gap | Priority |
|---|---|---|---|---|---|
| PNG with dpi and size | File > Export Graphs | `render_png`; dialog; CLI | ✓ | always 24-bit RGBA | P0 |
| SVG | export type | `render_svg` | ✓ | | P0 |
| PDF | expPDFw | `render_pdf`, text kept as text | ◐ | no PDF options, multi-page, several graphs | P0 |
| Size, margin and units | Image Settings | page size and dpi only | ◐ | no width and height override, unit choice, margin modes | P0 |
| One scene renderer for screen and exports | | `layout.rs` to `scene.rs` | ✓ | | P0 |
| Other image types | EMF, WMF, TIF, JPG, EPS, BMP, GIF, AI and more | none | ✗ | EMF, TIF, EPS are P1; the rest P2 | P1 |
| Batch export and naming | Export all, overwrite rules | one graph at a time | ◐ | no folder export, name templates | P1 |
| Export preview | Preview button | none | ✗ | | P1 |
| Send to PowerPoint or Word | Graph menu | none | ✗ | | P2 |

## 14 Dialog details

| Feature | Origin | Sheaf | Status | Gap | Priority |
|---|---|---|---|---|---|
| Generic analysis dialog frame | input and output ranges, Preview, Auto Apply, theme, recalculate | each tool writes a new result sheet | ✗ | | P0 |
| Column Properties dialog | Format dialog | name, long name, units, comments, plot as, number format incl. date, time, date and time | ◐ | no text type, width, Prev and Next | P0 |
| Context menus | header, cell, row, corner, graph objects | column header menu; graph menu | ◐ | no Fill, Mask, Normalize, Statistics entries; no cell, row, corner menus | P0 |
| Set Column Values dialog extras | function list, descriptions | none | ✗ | | P1 |

## Order of work

Each stage assumes the earlier ones. (Stages, not the numbered milestones of `docs/ROADMAP.md`.) Effort is not estimated here.

**Stage 1. Daily electrochemistry and ICP-OES work (P0 gaps).**
1. CH Instruments: confirm against real CV and i-t files, since the spec says none was checked.
2. CSV and ICP-OES: date and time columns, non-numeric handling, import into an existing sheet or new sheets, thousands separators, parameter rows, and a batch command over folders.
3. Overlay and column plots: make column plots creatable, line-style and symbol increments, X error bars and error-bar options, Double-Y, offset and waterfall plots.
4. Integration and peaks: per-peak integration with a results table, anchor-point baselines, picking limits on a graph, CV charge helper, then the Peak Analyzer chain and multi-peak fit.
5. Subtract reference curve and simple curve math.
6. Publication figures: free text labels, legend options, Origin's default colour list, rescale command, size and margin options, extra image types (TIFF, EMF, EPS).
7. Fitting: expose iteration and tolerance, confidence and prediction bands, initial guesses for more functions.
8. Column formulas that recalculate, and copy, cut and paste of cell ranges.

**Stage 2. The look and feel and .opj saving the owner asked for.**
1. Write .opj files. This needs a format study of the writer side, checked by opening the result in Origin 2016 through `tools/origin-ref/`. liborigin may be read for understanding, not copied. Start with workbooks, then graphs, so saved work reopens in Origin.
2. Origin-style chrome: MDI-style child windows, toolbars (standard, edit, graph, column, worksheet data, 2D graphs), Origin's menu names and order (View, Column, Statistics, Gadgets, Tools, Format) and menus that change with the active window.
3. Origin's shortcuts (Ctrl+K, Ctrl+Q, Ctrl+D, Ctrl+R, F2, F4, F12 and others), a Project Explorer with folders, Plot Details as a tree, the generic analysis dialog with Preview and Auto Apply.
4. Layers: a real multi-layer graph UI.

**Stage 3. Everyday gaps (P1).** Gadgets and graph analysis, recent lists, print, options dialog, Excel import and export, templates and themes, row statistics, t-tests and histograms, FFT and the other smoothers, worksheet operations, masking, live filters, simulate curve, plus the remaining P1 rows above.

**Stage 4. The rest (P2).** Matrix and image windows, 3D, contour, specialised plot families, ANOVA, nonparametric and multivariate statistics, global and surface fits, scripting, COM, other importers.

## Areas and sources the auditors could not judge

- Nothing was run: no app, CLI, test suite or Origin. Every status is from reading code.
- Numeric accuracy of each tool beyond what the specs and tests already claim.
- Whether the app shows import warnings for every .opj fidelity limit.
- The default shape of a new workbook in the app, and the exact count of expression functions (about 50 is an estimate).
- `dialogs/analysis.rs` and `fit.rs` bodies were mostly read by labels, so the sort, filter and some dialog-option rows may be slightly off.
- In the graph area, `layout.rs`, `ticks.rs` and parts of `markup.rs` were grepped, not read in full. The legend, tick-label, spline and free-label rows depend on them.
- The fit library size: one auditor estimated about 23 functions, another counted 206 in the JSON. The larger figure is used.
- Dynamic Origin menus (Analysis, Statistics, Gadgets, Import and Export) follow the spec's folder-level lists, since the spec marks the exact running menus unknown.
- Priorities for rows without an inventory entry are the auditors' judgement. The UI auditor did not read `feature-inventory.md`.
- The full per-category fit-function tables and the 592-line X-Function index were only read in part.
