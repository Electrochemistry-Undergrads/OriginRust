# Lessons

One entry per lesson, summary first. Update an entry instead of adding a duplicate; delete entries
that turn out to be wrong.

## The GitHub repository exists; pushing works

The private repository is https://github.com/Electrochemistry-Undergrads/OriginRust (moved there
from the owner's personal account on 2026-11-12). In the first two sessions
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
Write tool into the scratch folder and run that file. It happened again with bodies that held an
apostrophe in a comment (`layer's`) or a Rust lifetime (`&'static`), even inside `<<'EOF'`; the
Write and Edit tools always work.

Python scripts that write Rust source through a here-document also break backslashes silently:
inside a normal Python string `'\r'` and `'\n'` become real line breaks in the Rust file, and in a
raw Python string `"\\b(111)"` reaches Rust as `"\b(111)"`, an invalid escape. On 2026-11-13 this
broke the build three times while writing `.opj` reader code and its tests (Origin's text escapes
`\b(`, `\l(`, `\g(` are full of backslashes). Make such edits with the Edit tool.

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

## Run the whole test suite after every merge of parallel branches

Two branches can each pass their own tests and still break together. In the fourth session the
app branch changed how a new plot gets its color and the figure branch did not touch colors;
only after both were merged did the stored reference image of the cyclic voltammetry template
fail (two scans had the same color). Merge one branch at a time, run `cargo test --workspace`
after each, and treat a changed golden image as a real signal, not as something to regenerate.

## Parallel agents in git worktrees: what worked

Each agent got its own worktree (`.claude/worktrees/...`, its own branch) and its own build
folder (`CARGO_TARGET_DIR=target/wt-<name>`), built and tested only its crates, committed on its
branch, and was followed by a fresh reviewer and a fix pass before the lead merged it. Nothing
half-written reached `main`, and merges were clean because each agent owned named files. Shared
types that two agents need (the fit report) were written by the lead before they started.

## The workspace's test profile hides debug stack overflows

`[profile.dev] opt-level = 1` makes stack frames smaller than in an unoptimised build, so a
recursion test on a 1 MB thread passed even with the protection removed. Tests of stack depth
use a smaller thread (256 KiB) and were checked to fail without the fix.

## An empty `SHEAF_UPDATE_GOLDEN` still rewrites the golden images

The golden-image tests of `sheaf-plot` check only whether the variable exists, so
`SHEAF_UPDATE_GOLDEN= cargo test` (meant to clear it) rewrote every stored SVG instead of comparing
them. Run the tests without the variable, and after an accidental run check `git status
crates/plot/tests/golden` before committing.

## Clap reads a negative number as an unknown option

`--x-min -1` is rejected; write `--x-min=-1`. Tests and examples of the command-line program
use the `=` form for values that can be negative.

## Parallel build folders fill the disk

Each agent's build folder (`target/wt-<name>`) grows to several gigabytes with debug info. On
2026-11-12 the disk reached 97% full with fourteen such folders. Agents now build with
`CARGO_PROFILE_DEV_DEBUG=0`, and the lead deletes a branch's build folder when the branch is merged
with `tools/clean-target.sh wt-<name> ...`, which uses Windows' `rd` (much faster than `rm -rf`)
and checks that each folder is gone. A first version of this cleanup ran `rd` in a Bash loop with
`target\$d` in the path; Bash passed the name through unexpanded, `rd` failed in the background,
and nobody noticed until the disk was at 98% again. Check `df -h /c` before starting several
builds at once, and look at the result of every cleanup.

## Two sessions on one repository need a written split

The owner sometimes starts a second Claude session on the same repository. The sessions agree by
message which crates and files each one owns. Only the lead merges to `main` and edits HANDOFF.md,
ROADMAP.md and LESSONS.md. The other session works on named branches and reports them to the lead.

## One build folder shared by several worktrees can run tests with a dead source path

Cargo's fingerprints use paths relative to the workspace, so a test binary built in worktree A is
reused unchanged in worktree B when the sources are equal. `env!("CARGO_MANIFEST_DIR")` is baked
into that binary and still points at A. After A was removed, the fit fixtures failed to load
("file not found") on `main` although the files were there. When a worktree that shared the build
folder is removed, run `cargo clean -p <crate>` for the workspace crates (cheap: dependencies stay)
before trusting a test failure about a missing fixture.

## A shared build folder keeps the source path of the worktree that built a test

Tests that read data files through `env!("CARGO_MANIFEST_DIR")` (for example
`crates/fit/tests/origin_nlsf.rs`) have that path compiled in. When several worktrees build into
one `CARGO_TARGET_DIR` and the worktree that last built a crate is deleted, cargo may reuse the
test program and the test fails with "The system cannot find the path specified" although the file
is there. It is not a real failure: `cargo clean -p <crate>` for the workspace crates (or a build
folder per worktree) fixes it. Seen on 2026-11-12 after removing the `export-options` worktree.

## Origin writes to the owner's user folder when an export path is empty

`expGraph` with an empty `path:=` (for example when a `%WORK%` placeholder ends in a backslash
inside quotes and the command fails over to defaults) writes into
`Documents/OriginLab/2016/User Files`, the owner's folder. Always pass an explicit folder without a
trailing backslash, as `tools/origin-ref/opj-verify-graphs.ps1` does, and check the folder after a
failed export. On 2026-11-12 two probe PNGs landed there and were moved back out at once.
