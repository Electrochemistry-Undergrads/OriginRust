# Reference numbers from Origin

`run-labtalk.ps1` runs a LabTalk script in a temporary Origin instance through COM automation and
prints the values of chosen variables. The numeric parity tests in `crates/calc` compare our
results with numbers produced this way, on inputs that we generated ourselves.

- It needs a computer with Origin installed. Tests never call it; they use the numbers that were
  copied into the test fixtures, with the tolerance written next to them.
- The scripts in this folder are our own. They only call Origin's public scripting commands.
- One Origin instance at a time: the scripts take a system-wide lock, so a second copy started
  in parallel waits until the first has closed Origin.

Example (PowerShell):

```
.\run-labtalk.ps1 -Script integ-parabola.labtalk.txt -Vars area
```

## Nonlinear fit reference runs

`run-labtalk-lines.ps1` runs a script one line at a time in one Origin session (it takes the same
lock). A line starting with `?` prints a LabTalk number, `$` a string, and `#[Book1]FitNL1` dumps a
worksheet (the fit report sheet). `nlfit-suite.lines.txt` is the script that produced
`crates/fit/tests/data/origin_nlsf.json`; `nlfit-make-fixture.py nlfit-suite.out nlfit-suite-meta.json out.json`
turns the output into that file. Origin shows no dialog when `nlend output:=1` is called this way.

## Text import reference runs

The `import-*.lines.txt` scripts run `impasc` on the small files in `import-inputs/` (all written by
us) and print what landed in the worksheet: number separators (`import-numsep`), the import modes
(`import-modes`), header lines (`import-header`) and user parameter rows (`import-userparam`). Run
them with the input folder as the work folder, for example:

```
.\run-labtalk-lines.ps1 -Script import-modes.lines.txt -WorkDir import-inputs
```

What they showed is written down in `docs/spec/import-ascii.md`, section 15.

## Project file features (folders and column formulas)

`opj-verify-features.ps1` loads an `.opj` in a hidden Origin (through COM, same lock, with a watchdog) and prints the
folder tree with the windows of each folder and, per column, the recalculate mode and formula text that Origin reports.
`cargo run -p sheaf-io --example opj_write_features -- FOLDER` writes the files it was used on; the results are in
`docs/spec/opj-format.md`, sections 13.10 and 13.11.

## Graph features of written projects (stack offsets, double-Y, bar gap, axis sides)

`opj-verify-graph-features.ps1` is `opj-verify-graphs.ps1` with more per layer: `layer.link`, `x.link`, `y.link`,
`layer.unit`, `showAxes` and `showLabels` of both axes, the axis colours, and a PNG per graph. Pass several files in one call
(`-Files @('a.opj','b.opj') -OutDir DIR`). `cargo run -p sheaf-io --example opj_write_graph_features -- FOLDER` writes the
files it was used on (`gf-doubley`, `gf-stack`, `gf-bars`, `gf-axes`); the findings are in `docs/spec/opj-format.md`, section 13.12.

## Statistics breadth (sign test, Kolmogorov-Smirnov, median, Friedman, Grubbs, Dixon, row statistics)

`stats3/` holds the generators (`g2.py` to `g5.py` write the LabTalk scripts in the lines format, `gen.py` the dump
helpers), `parse.py` (turns the raw output of `run-labtalk-safe.ps1` into the compact `stats3-p*.compact.txt` files
kept here), the case lists `cases2..5.json` and `stats3-make-fixture.py`, which writes
`crates/calc/tests/fixtures/stats3_origin.rs`. Origin shows no dialog when `sign2`, `kstest2`, `mediantest` and
`friedman` run with `rt:=<new name:=...>`; `grubbs` and `qtest` need `alpha` and the output variables passed
explicitly. Report sheets are read cell by cell (`wcol(c)[r]`) because `GetWorksheet` does not return them.
What the runs showed is in `docs/spec/statistics.md`, section 10.

## Still to measure (cross tabulation, Dunnett)

`docs/spec/statistics.md` section 15.3 lists what the cross tabulation and Dunnett's comparison take from scipy and the
textbooks and what Origin has not been asked yet (the cross tabulation X-Function and report, whether the one-way ANOVA
tool offers Dunnett at all). Run those here on the Windows machine and write fixtures next to
`crates/calc/tests/fixtures/scipy_stats_compare.rs`.

## 2D frequency counts and binning

`stats4/g3.py`, `g4.py` and `g5.py` write the line scripts of the `twoDBinning` runs (worksheet output, each cell read
with `wcol(c)[r]`; run with `run-labtalk-safe.ps1`), `parse3.py` reads their raw output and `make-fixture3.py` turns
the three outputs into `crates/calc/tests/fixtures/bin2d_origin.rs` and `stats4/bin2d-origin.compact.json`. What they
showed is in `docs/spec/statistics.md`, section 13.

## Discrete frequency

`stats5/g1.py` writes the line script of the `discfreqs` runs (output sheet read cell by cell as shown), `parse1.py`
reads the raw output and `make-fixture1.py` writes `crates/calc/tests/fixtures/discfreq_origin.rs` and
`stats5/discfreq-origin.compact.json`. Findings: `docs/spec/statistics.md`, section 14.

## Origin's own dialogs (lists of controls, defaults, combo box items)

`origin-dialog.ps1` opens one of Origin's dialogs in a temporary visible Origin (same lock) and writes, per step, a picture
of the dialog, a list of its controls (class, text, position, tick state) and the items of every combo box. Either a
LabTalk command that opens the dialog (`-Command 'impasc fname:="%WORK%file.txt" -d;'`, which blocks, so the helper
`origin-dialog-watch.ps1` does the looking and closes the dialog with Escape) or a double-click on the main window
(`-DoubleClick x,y`, for example on a column header to open Column Properties). `-StepsFile` lists steps as
`name=actions`; actions are `@L:x,y` click, `@D:x,y` double click, `@S:x,y,n` mouse wheel, `@C:i,j` pick item j of the
i-th combo box (and tell the dialog), `@T:i,text` set the i-th edit box, `@B:text` click a button, `@W:ms` wait, or
SendKeys text; positions are pixels from the top-left corner of the dialog. `-After` runs LabTalk after the dialog closed
(`'$wks.col1.name$'` prints a string). Pictures and lists go to the git-ignored scratch folder; nothing is saved in Origin. `-MenuClicks "x,y;x,y"` opens a dialog through a menu path instead (screen positions of the Origin window placed at 0,0; the Peak Analyzer: `258,43;311,175;492,205;735,257`), `-AfterFile` reads the after-lines from a file, and an after-line `#[Book]Sheet` prints that sheet's cells.

## Column format reference runs

`colfmt-*.lines.txt` are line scripts (run with `run-labtalk-lines.ps1`, which prints strings as raw bytes in the console
encoding, so micro signs come out wrong; run them through a copy that sets `[Console]::OutputEncoding` to UTF-8) that set a
column's `format`, `subformat`, `digitMode` and `digits` and print `cell(r,1)$`, the text Origin shows: Month and Day of
Week numbers (`colfmt-weekday-month`), Engineering digits and letters (`colfmt-engineering-digits`,
`colfmt-engineering-carry`, `colfmt-significant-digits`). The findings are in `docs/spec/workbook-model.md`.

`import-shortnames*.lines.txt` import `import-inputs/shortn2.txt` and `shortn3.txt` with the short-name line option
(`options.Hdr.SNames`) and print the column names; the rules are in `docs/spec/import-ascii.md`, section 3.

`import-replace-plots.lines.txt` replaces the data of a sheet by `impasc` while two graphs plot it (one of all rows, one
with a row range) and prints each plot's begin and end row before and after (`get %C -b`, `get %C -e`); run it with
`import-inputs` as the work folder. See `docs/spec/import-ascii.md`, section 10.

## PCA graphs, report layouts and survival-curve limits (T29 leftovers)

`pca-plots3.lines.txt` runs `pca` seven times with all four plots on, then lists the graph windows, their axis limits per
layer and their datasets and exports each graph as a PNG (the folder is written into the `expGraph` lines: edit it).
`kaplan-plots2.lines.txt` and `kaplan-plots3.lines.txt` run `kaplanmeier` with `sfci:=1` (needs `plotinone:=0`) and dump the
survival-function sheet with its column labels and the graph windows. `kmeans-members.lines.txt` prints the labels of the
membership sheet. `t29-report-setup.labtalk.txt` makes one report of each of the three X-Functions, and
`origin-screenshot.ps1 -Setup t29-report-setup.labtalk.txt -StepsFile t29-report-steps.txt -OutDir DIR` takes the pictures
of the report sheet windows (maximized with `win -z`, sections opened by clicking their boxes) from which the table
layouts in `docs/spec/statistics.md`, sections 17 to 20, were read. Within a line script `$(x)` turns a number into text and
`%(s$)` a string; a `for` loop with a nested `doc -e` did not run, so the scripts are unrolled.

## Merge Graph Windows and Fit Page to Layers (T60)

`merge-graph-*.lines.txt` make three stock Line graphs (GA, GB, GC) and run `merge_graph option:=project` with one
setting changed at a time, printing the merged page size and every layer's position, `layer.factor`, links and axis
label switches, then deleting the merged graph (`win -cd %H`): `merge-graph-options` (rows/columns, scale modes, links,
Show Axes Frame, aspect ratio, portrait, a double-Y source, keep:=0), `merge-graph-factor1` and `-factor2` (the text
factor for many grids, gaps and margins), `merge-graph-source-size` (sources resized to 50 x 50, 30 x 30 and 80 x 40
percent) and `merge-graph-threshold` (where the factor stops being 1). `fit-page-to-layers.lines.txt` runs `pfit2l`
with its options. Run them with `run-labtalk-lines.ps1`; the findings are in `docs/spec/graphing.md` 15.7c and 15.7d.

## Add Straight Line (T67)

`addline-objects.lines.txt` runs `addline` on a stock Line graph (vertical, horizontal, a named dashed red line,
a second vertical one) and prints the objects it makes and their ends, attachment, colour, style, width and the
label's text, place and size; `addline-labels.lines.txt` adds lines with each label location and a fixed format,
then rescales Y to see whether the ends follow. Findings: `docs/tasks/T67-add-straight-line.md` and graphing.

## Partial correlation, three-way and repeated measures ANOVA (T81)

`stats6/gp.py` and `gp2.py` write the line scripts of the `pcorrcoef` runs (report sheet read cell by cell after
`page.active = 2`, plain sheet after `page.active = 3`; `pp.py` and `cells.py` read the raw output). The three ANOVA
tools (`ANOVAThreeWay`, `ANOVAOneWayRM`, `ANOVATwoWayRM`) have no script arguments, so they are run through their dialogs:
`origin-dialog.ps1 -Setup DATA.labtalk.txt -MenuClicks "1011,192;391,43;392,109;<item>;<Open Dialog...>" -StepsFile
dialog-steps/*.steps.txt -AfterFile CELLS.txt` (the first click closes a floating Results Log; menu positions move when
Origin adds "Last used" entries, so take a picture first with `origin-screenshot.ps1`; typed range boxes sometimes lose an
entry, hence the fill is repeated; the "Input Data" group remembers whether it was open). The result lands as a new sheet in
the input book; `$wcol(c)[r]$` lines in the after file print its cells. `g3.py` has the data generators. The numbers and
data of all runs are in `stats6/t81-origin.compact.json` (`t81-make-compact.py` rebuilds it from the raw output, which stays in
the scratch folder). Findings: `docs/spec/statistics.md`, sections 24 to 26.
