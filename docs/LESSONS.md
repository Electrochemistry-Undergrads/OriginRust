# Lessons

One entry per lesson, summary first. Update an entry instead of adding a duplicate; delete entries
that turn out to be wrong.

## Creating the GitHub repository is blocked for the assistant in auto mode

`gh repo create` was refused twice by Claude Code's automatic permission check ("public
data-sharing upload"), even though the brief approves it. The owner has to run the command once
himself (it is in `HANDOFF.md`) or add a permission rule. After that, plain `git push` may or may
not be allowed; try it and record the result here.

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
