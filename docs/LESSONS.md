# Lessons

One entry per lesson, summary first. Update an entry instead of adding a duplicate; delete entries
that turn out to be wrong.

## Creating the GitHub repository is blocked for the assistant in auto mode

`gh repo create` was refused twice by Claude Code's automatic permission check ("public
data-sharing upload"), even though the brief approves it. The owner has to run the command once
himself (it is in `HANDOFF.md`) or add a permission rule. After that, plain `git push` may or may
not be allowed; try it and record the result here. Tried again on 2026-11-11 in the second
session with the owner present: refused again ("data exfiltration"). Do not keep retrying; commit
locally and wait for the owner.

## Do not patch source files with `node -e` or `sed` scripts that contain backticks or `$`

A one-line Node script passed through bash had its template literal eaten by the shell and wrote
the word `undefined` into a source file. Use the Edit tool for source changes. Shell one-liners are
fine only for simple literal `sed` substitutions.

## Clippy rejects number literals close to pi or e

`3.14159` and `2.71828` in tests fail `clippy::approx_constant`. Use other numbers in tests.

## CH Instruments scan splitting follows the owner's own script

The rule "cut at every second potential reversal; the vertex row opens the next scan" comes from
the owner's `cv_overlay.py` (his Excel workflow) and reproduces his existing spreadsheets
(scans of 2200, 2400 and 2400 rows for his reference file). Do not "improve" it without asking.

## The owner's rule for a current axis symmetric about zero

Limits are `-L..+L` with at most three major steps per side, 5 % headroom, and steps rounded up to
1, 1.5, 2, 2.5, 3, 4 or 5 times a power of ten (33.55 mA gives ±45 with step 15, not ±40 with step
20). This is implemented as the `SymmetricZero` axis mode and has check values in
`docs/spec/figure-rendering.md`.

## Look at the desktop app without a person: `--screenshot`

`sheaf-app FILE --show ITEM --screenshot OUT.png` starts the app, optionally opens a dialog
(`graph-settings:x|y|plots|legend|page`, `column`, `set-values`, `export`, `import:FILE`,
`focus:Book1`), saves a picture of the window after 20 frames and quits. Read the PNG to check
layout. This shows that windows draw correctly; it does not exercise clicks.

## Background agents share the working tree: build and commit with care

While agents are writing crates, (1) commit only your own paths (`git add crates/app ...`), not
`git add -A`, or half-written agent code gets committed; (2) run `cargo fmt -p <crate>`, not
`cargo fmt --all`, so you do not reformat files an agent is editing; (3) give each agent its own
build folder (`CARGO_TARGET_DIR=target/agent-<name>`) so builds do not wait on each other; (4) a
half-written module of an agent can break the build of crates that depend on it for a while.

## `log` means two different things in Origin

In worksheet formulas (LabTalk) `log` is base 10 and `ln` is natural. Fit-function bodies in
`.FDF` files are written in Origin's C dialect, where `log` is the natural logarithm and `log10`
is base 10. `sheaf-expr` follows the worksheet meaning. The `.FDF` importer (milestone 4) has to
translate `log` to `ln`. Not yet confirmed by a run of Origin; check with a fit function that
uses `log` before relying on it.

## Default plot columns keep one unit

A CH Instruments current-vs-time import has the current in A and in mA and the charge in C as Y
columns. Plotting all of them on one axis is useless, so `Project::default_plot_columns` plots
only the mA columns when there are any, otherwise the columns that share the first Y column's
unit.
