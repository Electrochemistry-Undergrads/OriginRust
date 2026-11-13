# T06: `.opj` writer: remaining graph features

**Status:** items 1, 3 (as offsets), 4 (month/day/year) and 8 (written, not read) done on branch `opj-graph-rest`, verified in Origin (opj-format.md 13.14); items 2, 5, 6, 7 not found (probes listed in 13.14). **Size:** L (one commit per feature).
**Branch:** `opj-graph-write3`. **Model:** Sonnet (probes and writer).
**Spec:** `docs/spec/opj-format.md` sections 13.12 and 13.13 (what is written), section 10 (graphs).
**ADR:** 0008 (`.opj` default with companion).

## Background

Saving to `.opj` writes what the writer knows; anything else is named in the save warnings, and the
`<name>.opj.sheaf` companion keeps it. Already written: double-Y, stack Y offsets, bar gaps, axis
sides, automatic axis colours (13.12); area plots, decimal/scientific tick formats, column plot groups,
panels as linked layers with scale links (13.13). Round trip checked on the owner's XRD project.

## Still not written (the companion keeps them)

1. X offsets of stacked curves.
2. Bar overlap.
3. Waterfall.
4. Date/time tick formats.
5. Group colours that differ from Origin's list.
6. Grouped columns with error bars.
7. The hidden end tick label of a stack.
8. Window places (saved in `.sheaf` only).

For 1-3 the way Origin stores them **was not found**.

## Method (worked for 13.12/13.13)

1. Make two Origin files that differ only in the feature: `tools/origin-ref/opj-make-from-lines.ps1`
   (LabTalk lines → saved `.opj`).
2. Diff them: `tools/origin-ref/opj-diff-curves.py` (and `opj-dump-blocks.py`).
3. Write the field in `crates/io/src/opj/write/graph.rs` / `graph_tables.rs` / `glayout.rs`, read it
   in `crates/io/src/opj/graph.rs`, remove the feature from the "not written" warning list in
   `crates/io/src/opj/write/mod.rs`.
4. Verify: write with `sheaf export ... -o x.opj`, open in Origin
   (`opj-verify-graph-features.ps1`), compare the exported picture by eye; add a case to
   `crates/io/src/opj/write/graph_feature_tests.rs`.
5. Record the layout in opj-format.md as a new 13.14.

## Done when

Each feature is written and re-read, opens in Origin as drawn by Sheaf, and is gone from the
warnings; or the brief says "not found, probe X tried". Checks pass.

## Don't

Never commit `.opj` files except `crates/io/tests/fixtures/sheaf-written.opj` (gitignore). The owner's
projects in the repo root are read-only inputs.
