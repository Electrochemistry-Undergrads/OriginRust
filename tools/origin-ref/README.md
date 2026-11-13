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

## Origin's own dialogs (lists of controls, defaults, combo box items)

`origin-dialog.ps1` opens one of Origin's dialogs in a temporary visible Origin (same lock) and writes, per step, a picture
of the dialog, a list of its controls (class, text, position, tick state) and the items of every combo box. Either a
LabTalk command that opens the dialog (`-Command 'impasc fname:="%WORK%file.txt" -d;'`, which blocks, so the helper
`origin-dialog-watch.ps1` does the looking and closes the dialog with Escape) or a double-click on the main window
(`-DoubleClick x,y`, for example on a column header to open Column Properties). `-StepsFile` lists steps as
`name=actions`; actions are `@L:x,y` click, `@D:x,y` double click, `@S:x,y,n` mouse wheel, `@C:i,j` pick item j of the
i-th combo box (and tell the dialog), `@T:i,text` set the i-th edit box, `@B:text` click a button, `@W:ms` wait, or
SendKeys text; positions are pixels from the top-left corner of the dialog. `-After` runs LabTalk after the dialog closed
(`'$wks.col1.name$'` prints a string). Pictures and lists go to the git-ignored scratch folder; nothing is saved in Origin.

## Column format reference runs

`colfmt-*.lines.txt` are line scripts (run with `run-labtalk-lines.ps1`, which prints strings as raw bytes in the console
encoding, so micro signs come out wrong; run them through a copy that sets `[Console]::OutputEncoding` to UTF-8) that set a
column's `format`, `subformat`, `digitMode` and `digits` and print `cell(r,1)$`, the text Origin shows: Month and Day of
Week numbers (`colfmt-weekday-month`), Engineering digits and letters (`colfmt-engineering-digits`,
`colfmt-engineering-carry`, `colfmt-significant-digits`). The findings are in `docs/spec/workbook-model.md`.
