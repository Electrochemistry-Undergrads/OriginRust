# User-facing gaps between Origin 2016 and Origin 2026–2027

Origin 2016 is the comparison baseline. The destination includes Origin 2026, 2026b and 2027,
including capabilities introduced in the intervening releases. OriginLab announced 2027 on
November 11, 2026. [2016 highlights][o16] · [Release history][history] · [2027 announcement][release27]

This task inventory covers capabilities absent from Sheaf's implementation and its existing task
briefs, handoff and roadmap when checked against `eaa6a5c` on 2026-11-15. The version column
identifies the documented addition or extension. Where the source describes an improvement,
the row names the added workflow. Product and App names are kept so each feature can be checked
against the release documentation. **Pro** identifies explicitly documented OriginPro features.
**App** identifies an optional OriginLab add-on. The comparison uses release documentation.
Turn individual rows into scoped briefs when work starts, with reference inputs and expected
outputs or UI behavior.

## Worksheets and project workflows

| Feature | Release | Functionality to add | Source |
|---|---|---|---|
| Cell formulas and Formula Bar | 2019 / 2021 | Calculate individual cells and edit their expressions in a formula bar, alongside column-wide calculations. | [2019][o19] · [2021][o21] |
| Named cells and ranges | 2022 | Assign a reusable name to a cell or selected range and reference it in calculations. | [2022][o22] |
| Cell notes | 2022 | Done (T36). Attach a comment to an individual data cell, show an indicator and read the note in place. | [2022][o22] |
| Worksheet frozen panes | 2023 | Done (T34). Keep selected data rows or columns visible while scrolling the data area. | [2023][o23] |
| Time Series Pivot | 2025 | Calendar periods done (T358: Worksheet > Time Series Pivot, minute to year, six statistics); fiscal calendars, custom intervals and a second grouping remain. Aggregate timestamped records by calendar intervals and a second period grouping, including fiscal calendars and custom intervals. | [2025][o25] |
| Project Browser | 2025b | Search a project-wide list of sheets, columns, graphs and folders, preview matches and apply metadata edits to multiple selections. | [2025b][o25b] |
| Window Switcher | 2025b | Done (T42). Cycle through open child windows with keyboard shortcuts and previews. | [2025b][o25b] |
| Interactive formula references | 2025b | Insert a cell, column or range into the formula being edited by clicking or dragging in the sheet. | [2025b][o25b] |
| Conditional highlight of extrema | 2025b | Done (T35). Mark the minimum or maximum within each row or each column as its values change. | [2025b][o25b] |
| Flash Fill and Smart Formula | 2026 | Infer a text extraction or combination rule from an example. Preview it and either fill static values or keep a calculation that updates. | [2026][o26] |
| Find in Project Files | 2026 / 2026b | Done for names, labels, comments, units, cells and graph text (T361: File > Find in Project Files...); notes windows and background searching remain. Search a collection of saved projects for text in metadata, graph labels, notes and data cells, then open the matching project. | [2026][o26] · [2026b][o26b] |
| Repeat Last Action and Duplicate Operation | 2026 | Analyses done (T360: Analysis > Repeat Last Analysis on the new selection); plotting actions and Duplicate Operation to other sheets remain. Repeat the last plotting action on another selection, reopen the last tool dialog and apply a configured analysis to other sheets. | [2026][o26] |
| Mirror Columns and sheets | 2026b | Create a live reference to source data elsewhere in a project with a single stored copy. Inspect the source and destination relationships. | [2026b][o26b] |
| Insert columns across sheets | 2026b | Workbook and project scope done (T359: Column > Add Column to All Sheets..., with a formula); folder scope and insertion at a position remain. Add or insert a matching column and its formula in multiple related sheets across a workbook, folder or project. | [2026b][o26b] |
| Sticky Notes | 2027 | Add searchable, collapsible notes to graph windows, workbooks and individual sheets with indicators and color choices. | [2027][o27] |
| Workbook operation tree | 2027 | Group analysis operations with their result sheets in Object Manager. Navigate to inputs or outputs, edit operation parameters and change recalculation mode from the tree. | [2027][o27] |

## Import, reporting and export

| Feature | Release | Functionality to add | Source |
|---|---|---|---|
| Multi-page PDF figure export | 2018b | Done (T32): select several graphs and export them as separate pages in one PDF. | [2018b][o18b] |
| Data Connectors and periodic refresh | 2019b / 2026 | Keep a linked source and its selected data subset in the workbook. Reload the link on a configurable, pausable timer and optionally keep linked data out of the saved project. | [2019b][o19b] · [2026][o26] |
| Linked HTML/Markdown analysis reports | 2019b | Build a report embedding live links to result values, metadata and figures. Reuse the report with batch analysis and export it as a web page. | [2019b][o19b] |
| Import PDF Tables | 2021 App | Extract table cells from a PDF into worksheets. | [2021][o21] |
| Formula-preserving Excel and Sheets import | 2026 | Import Excel cell expressions or paste spreadsheet formulas as editable calculations. Surface unsupported formulas in the affected cells. | [2026][o26] |
| GraphPad Prism connector | 2026b App | Import Prism project or template data and map it to structured tables that preserve group and replicate roles. | [2026b][o26b] |

## Graphing and data exploration

| Feature | Release | Functionality to add | Source |
|---|---|---|---|
| Manhattan plot | 2017 Pro App | Done (T362: Statistics > Manhattan Plot..., one scatter per group and a significance line). Plot significance across grouped genomic positions, typically with chromosome groups and a significance cutoff. | [2017][o17] |
| Violin and parallel-coordinate plots | 2019 | Violin done (T37); parallel plot and parallel index done (T49). Show each group's distribution as a density silhouette, or draw each record across parallel variable axes. | [2019][o19] |
| Rug plot | 2019b | Done (T41). Add a mark along an axis for each observation to reveal its distribution. | [2019b][o19b] |
| Sankey and alluvial diagrams | 2020 | Done: Sankey diagrams (T379, `sheaf plot -t sankey`) and alluvial diagrams (T381, `sheaf plot -t alluvial`), both under Plot > Categorical. | [2020][o20] |
| Browser Graph | 2020b / 2024 | Browse similarly structured datasets inside a graph and switch the displayed series, including datasets on different sheets. | [2020b][browser] · [2024][o24] |
| Chord and lollipop plots | 2021 | Lollipop done (T33). Draw relationships as chords between categories. Draw values as stems ending in markers. | [2021][o21] |
| Network and grouped marginal plots | 2021b | Network plots done (T383: Plot > Categorical > Network Plot, `sheaf plot -t network`). Grouped marginal plots done (T373: `sheaf plot -t marginal-histograms|marginal-box-charts --group`, a scatter and a histogram or box per group). | [2021b][o21b] |
| Sunburst and ridgeline plots | 2022 | Sunburst done (T51), ridgeline done (T44). Draw a hierarchy as concentric rings or compare distributions as vertically offset density curves. | [2022][o22] |
| Plot-only X sorting | 2023 | Done (T39). Sort points by X for line connections while preserving worksheet row order. | [2023][o23] |
| Built-in equation rendering | 2023b | Render LaTeX equations in graph labels and worksheet metadata, carry metadata equations into legends and axis titles, and retain vector output on export. | [2023b][o23b] |
| Time Series Explorer | 2024b | View the full series and a detail panel for a chosen interval. Step or play through successive intervals. | [2024b][o24b] |
| Extract data from regions on a graph | 2024b | Rectangle and ellipse done (T355: a drawn object's menu, Extract Data in Region; freeform and several regions remain). Draw one or more rectangular, oval or freeform regions and output the enclosed observations to a worksheet. | [2024b][o24b] |
| Graph Slicer | 2025 / 2025b | Filter plots from a common source sheet at page, layer or plot scope and extract the selected subset. | [2025][o25] · [2025b][o25b] |
| Treemap | 2025b | Done (T50). Show hierarchical magnitudes as nested rectangles sized by values. | [2025b][o25b] |
| On-graph trendlines | 2025b | Done (T43). Add a fitted trend or moving average directly to a plot with its own editable label and curve settings. | [2025b][o25b] |
| Cause-and-effect and nested frequency charts | 2026 | Fishbone diagram done (T375: Plot > Categorical > Cause-and-Effect Diagram, `sheaf plot -t fishbone`); nested frequency charts (columns, bars, pie or doughnut sections of counts, nested categories) open. | [2026][o26] |
| Slope Annotation | 2026b | Done (T52). Mark a slope directly on a graph with a data-linked annotation. | [2026b][o26b] |
| Hexbin, binned scatter and UpSet plots | 2027 | Hexbin done (T45); binned scatter done (T356, Statistics > Descriptive Statistics > Binned Scatter); UpSet remains. Summarize dense XY data in hexagonal bins or binned scatter points. Visualize sizes of set intersections. | [2027][o27] |

## Analysis and statistics

| Feature | Release | Functionality to add | Source |
|---|---|---|---|
| Fourier self-deconvolution peak detection | 2018b | Done (T341, Sheaf's own after Kauppinen 1981: Analysis > Peaks and Baseline > Fourier Self-Deconvolution..., `analyze fsd`; Origin's settings unknown). Enhance separation of overlapping peaks in a spectrum for peak detection. | [2018b][o18b] |
| Quantile regression and change-point analysis | 2019b Apps | Quantile regression done (T342, linear, exact; no standard errors); change-point analysis done (T346: mean, variance or both, PELT, exact). Fit conditional quantiles of a response. Detect points at which the statistical behavior of a series changes. | [2019b][o19b] |
| Time series modeling and forecasting | 2024b Pro App | Autocorrelation done (T363: Statistics > Autocorrelation, ACF with Bartlett bounds, PACF, Ljung-Box) and classical decomposition done (T370: Statistics > Seasonal Decomposition) stationarity tests done (T372: ADF and KPSS) exponential smoothing forecasts done (T374: simple, Holt, Holt-Winters) and ARIMA forecasts done (T376: conditional least squares); exact-likelihood and seasonal ARIMA remain. Check stationarity, inspect autocorrelation, separate trend and seasonality, and use ARIMA to forecast future observations. | [2024b][o24b] · [Time Series Analysis][tsa] |
| Best Subset Selection | 2025b Pro App | Done (T347, Sheaf's own definitions: Analysis > Fitting > Best Subset Selection..., `fit --regress ... --best-subsets`). Compare regression models over candidate predictor subsets and rank the models by fit criteria. | [2025b][o25b] |
| Piecewise Peak Fitting | 2025b Pro App | Done (T352, Sheaf's own: Peak Analyzer regions, `analyze peaks --regions`). Divide a single curve into fitting regions, set a separate baseline for each region and resolve peaks within each region. | [2025b][o25b] · [Piecewise Peak Fitting][piecewise] |
| Graphical Summary | 2026 | Done (T40, Separate Graphs). Produce a descriptive report with statistics, diagnostic charts and Anderson–Darling normality testing for individual or grouped data. | [2026][o26] |
| Life Science structured tables | 2026b Pro | Give experimental data explicit group, replicate, time or survival roles and surface table-specific graphing and analysis actions. | [2026b][o26b] |
| Random Forest and Acceptance Sampling | 2026b Pro Apps | Random forest done (T377: Statistics > Random Forest, seeded CART forests for classes and values). Train ensembles of decision trees for predictive modeling. Acceptance sampling done (T369: Statistics > Acceptance Sampling and `analyze acceptance`, attribute single sampling plans, OC, AOQ, ATI); Random Forest open. | [2026b][o26b] |
| Multiple Comparisons and Significance Plots | 2027 | Done (T64, one-way ANOVA and two-sample t-test; two-way, paired and one-sample left). Generate a chart directly from ANOVA or t-test reports, including comparisons, error bars and p-value annotations. | [2027][o27] |
| Assay analysis and dose-response extensions | 2027 Pro | Partly: the DoseResp, Logistic and Logistic5 fits already report EC50 (and EC20, EC80 where Origin's files define them) as derived values (checked in T354); not done: Western blot, qPCR, EC50 marks on the graph, robust-regression outliers. Analyze Western blot and qPCR experiments, mark IC50/EC50 on dose-response fits and support robust-regression outlier handling. | [2027][o27] |
| Monte Carlo simulation workflow | 2027 Pro App | Define a probabilistic model, sample its inputs, inspect sensitivity and capability, then optimize controllable inputs against tolerances. | [2027][o27] |
| Quality improvement additions | 2027 Pro Apps | G/T charts done (T364: Statistics > Rare Event Chart, geometric G and exponential or Weibull T limits); nonnormal capability done (T365: Statistics > Process Capability, normal and lognormal or Weibull percentile method); stability studies done (T368: Statistics > Stability Study, ICH Q1E poolability and shelf life); two-level split-plot designs done (T380 factorial base, T382 split-plot design and two-stratum effects under Statistics > Two-Level Experiments). Analyze rare events with G/T control charts, assess nonnormal process capability, run two-level split-plot experiments and model between-batch variation in stability studies. | [2027][o27] |
| FIR filter design and application | 2027 Pro App | Done (T55: Analysis > Signal Processing > FIR Filter..., windowed-sinc design and application, `analysis-tools.md` 17; the row was stale, corrected in T354). Design and apply a finite impulse response filter to sampled signals. | [2027][o27] · [FIR Filter][fir] |

## Added 2026-11-20 from the 2026, 2026b and 2027 release notes

Found by a web search of the release notes (search summaries, not a page-by-page read). Checked against Sheaf's code and specs by keyword on 2026-11-20; confirm each row against the release page and the code before writing its brief.

| Feature | Release | Functionality to add | Source |
|---|---|---|---|
| Graph Maker additions: Run chart, Pareto chart, Stem-and-Leaf plot, Grouped Histogram | 2026 | Run chart done (T270) and Grouped Histogram (T271), from the Plot menu (the Graph Maker itself is not built); Pareto exists (T83); Stem-and-Leaf done (T275) as Statistics > Descriptive Statistics > Stem-and-Leaf Plot..., from the options of OriginLab's "Stem and Leaf Plot" app page (not measured). The rebuilt Graph Maker gained these chart types. No Run chart or Grouped Histogram in Sheaf's code or specs; a Pareto mention exists in HANDOFF, so check what is built before starting. | [2026][o26] · [2026 notes](https://www.originlab.com/ReleaseNotes/index.aspx?v=2026) |
| Text-to-number functions `NumExtract()` and `NumBreak()` | 2026 SR1 | Done (T265). Extract numbers from mixed text and split number runs, to help move Excel formulas. Not in Sheaf's expression engine. | [2026 notes](https://www.originlab.com/ReleaseNotes/index.aspx?v=2026) |
| Distribution Overview plot, arbitrary censoring and life-data regression | 2026b | Overview plot for right-censored data done (T366: Statistics > Survival Analysis > Distribution Overview Plot, Weibull, lognormal, exponential and normal); arbitrary censoring done (T384: Distribution Analysis (Arbitrary Censoring), exact, right-, left- and interval-censored rows); frequency input and life-data regression remain. A Distribution Overview plot for right-censored data (Statistics menu), distribution analysis with arbitrary censoring, an optional frequency input, regression with life data. Sheaf has Kaplan-Meier but none of these. | [2026b][o26b] · [2026b notes](https://www.originlab.com/ReleaseNotes/index.aspx?c=1) |
| Poisson rate tests | 2026b | Done (T267: calc, analysis, CLI, dialogs; Summarized input only). Rate tests for Poisson counts. Sheaf mentions Poisson in some files; check whether the rate tests exist. | [2026b][o26b] |
| Mixture designs with several totals | 2026b | Design of experiments: mixture designs accept more than one total; factor names accept more characters. | [2026b notes](https://www.originlab.com/ReleaseNotes/index.aspx?c=1) |
| Reliability and Survival Analysis app; DMD, RPC3 and Ivium connectors | 2026 | An app for reliability and survival work, and import connectors for those three instrument formats. | [2026][o26] |
| Same sheet across several workbooks in a folder | 2027 | Sheet activation done (T277: sheet tab > Activate This Sheet in All Books in Folder, by name); the label-row system variables are left. Activate the same sheet in all workbooks of a folder at once; new system variables control rich text and dynamic merge in the long name and comments label rows. | [2027 notes](https://www.originlab.com/releasenotes/index.aspx?c=2) |
| Faster project save | 2026 | OriginLab reports about 40% faster saving of large mixed-series projects. A performance target for Sheaf's `.sheaf` save, not a feature. | [2026][o26] |

[o16]: https://www.originlab.com/2016
[history]: https://www.originlab.com/index.aspx?go=SUPPORT&pid=3325
[release27]: https://www.originlab.com/index.aspx?go=COMPANY%2FNEWSANDEVENTS%2FPRESSROOM&pid=5497
[o17]: https://www.originlab.com/2017
[o18b]: https://www.originlab.com/2018b
[o19]: https://www.originlab.com/2019
[o19b]: https://www.originlab.com/2019b
[o20]: https://www.originlab.com/2020
[browser]: https://blog.originlab.com/introducing-the-browser-graph
[o21]: https://www.originlab.com/2021
[o21b]: https://www.originlab.com/2021b
[o22]: https://www.originlab.com/2022
[o23]: https://www.originlab.com/2023
[o23b]: https://www.originlab.com/2023b
[o24]: https://www.originlab.com/2024
[o24b]: https://www.originlab.com/2024b
[o25]: https://www.originlab.com/index.aspx?go=Products%2FOrigin%2F2025&pid=5468
[o25b]: https://www.originlab.com/2025b
[o26]: https://www.originlab.com/2026
[o26b]: https://www.originlab.com/2026b
[o27]: https://www.originlab.com/index.aspx?go=Products%2FOrigin%2F2027&pid=5490
[tsa]: https://docs.originlab.com/app/tsa/
[fir]: https://www.originlab.com/fileExchange/details.aspx?fid=1077
[piecewise]: https://www.originlab.com/fileExchange/details.aspx?fid=1036
