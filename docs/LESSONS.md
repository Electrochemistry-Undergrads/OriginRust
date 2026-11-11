# Lessons

One entry per lesson, summary first. Update an entry instead of adding a duplicate; delete entries
that turn out to be wrong.

## The GitHub repository exists; pushing works

The private repository is https://github.com/UlanS1/OriginRust. In the first two sessions
`gh repo create` was refused by Claude Code's automatic permission check in auto mode ("data
exfiltration"). In the fourth session (2026-11-11), with the owner's permission mode set to
bypass, `gh repo create OriginRust --private --source . --remote origin --push` worked at once.
If a later session in auto mode is refused a `git push`, commit locally and note it in
`HANDOFF.md`; do not retry in a loop.

## Do not patch source files with `node -e` or `sed` scripts that contain backticks or `$`

A one-line Node script passed through bash had its template literal eaten by the shell and wrote
the word `undefined` into a source file. Use the Edit tool for source changes. Shell one-liners are
fine only for simple literal `sed` substitutions.

The same holds for shell here-documents (`cat >> file <<'EOF'`, `python - <<'EOF'`) whose body
contains Rust or Python code with single quotes, such as a `'\t'` character or a `'''` string: three
times in one session bash refused the whole command ("unexpected EOF while looking for matching
quote"), so nothing ran. For a patch that touches many places, write a small Python script with the
Write tool into the scratch folder and run that file.

## `cargo test` does not refresh the program in `target/debug`

After changing the command-line program, `cargo test -p sheaf-cli` passed while
`target/debug/sheaf.exe` was still the old build without the new subcommand. Run
`cargo build -p sheaf-cli` (or `cargo run -p sheaf-cli -- ...`) before trying it by hand.

## Several agents can use Origin: the driver scripts take a lock

`tools/origin-ref/run-labtalk.ps1` and `run-labtalk-data.ps1` hold the system-wide lock
`Global\SheafOriginReference` while Origin runs, so a second run waits instead of starting a second
Origin instance. New helper scripts that start Origin must take the same lock.

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
