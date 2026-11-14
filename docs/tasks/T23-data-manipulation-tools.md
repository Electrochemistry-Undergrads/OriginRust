# T23: Data Manipulation tools and Median Filter

**Status:** done, merged (`f66dbea`), built on macOS without Origin. The four menu items
(and Curve Translate Y, Sheaf's own, next to X) open working dialogs, run as tools of the shared analysis
frame with Recalculate Manual/Auto, and have `sheaf analyze` subcommands. Every Origin behaviour that is
not in an earlier spec table is from Origin's public documentation, paraphrased, and is not measured; each
is listed under "Not measured" in spec section 12 to be measured on the Windows machine with
`tools/origin-ref`. On 2026-11-13 (Windows) Reduce to Evenly Spaced X was measured and rebuilt as Origin's `reduce_ex` (spec 12.2), and the X-Function names were read (12, item 1): Subtract Straight Line and Curve Translate are interactive tools on the graph in Origin, and Origin has no curve median filter in Mathematics (left: making those two interactive, deciding on Median Filter).
**Size:** M. **Branch:** `math-manip`. **Model:** Sonnet.
**Detail:** audit D "Partial"; backlog row "Mathematics" of T18.
**Spec:** `docs/spec/analysis-tools.md` section 9 (Subtract Reference Data, the pattern to copy) and the
new section 12.

## Goal

The greyed items of Analysis > Data Manipulation (Subtract Straight Line, Reduce to Evenly Spaced X,
Curve Translate X) and Analysis > Mathematics > Median Filter work as tools of the shared analysis
dialog frame, with a recalculation link, a CLI subcommand and tests.

## Origin facts

- Measured earlier: linear interpolation between points and missing values outside the data
  (`calc::interp1` with `miss`, spec section 5) and subtracting a reference curve (`subtract_ref`,
  section 9). The two-point line subtraction (`calc::subtract_line`, section 6) is Sheaf's own and
  was not compared with Origin; the new tools do not use the end-segment continuation.
- From Origin's public documentation, not measured: Subtract Straight Line subtracts the line through
  two points chosen on the graph; Reduce to Evenly Spaced X offers interpolation and averaging within
  bins; Curve Translate moves a curve by a constant or so that a chosen point lands on a chosen value;
  the Median Filter takes an odd window of points. Edge handling of the median filter is not documented
  in a way we can rely on.

## Where

- `crates/calc/src/manip.rs` (new): `value_at`, `subtract_line_at`, `reduce_evenly`, `translate`
  helpers, `median_filter`.
- `crates/core/src/analysis.rs`: `Tool::SubtractLine`, `Tool::ReduceX`, `Tool::Translate`,
  `Tool::MedianFilter` (specs in `crates/core/src/analysis/manip.rs`, new); `analysis_link.rs`: the four
  tools are linkable.
- `crates/app/src/dialogs/analysis.rs` (`AnalysisKind`), `crates/app/src/menus.rs` (greyed items).
- `crates/cli/src/args.rs`, `crates/cli/src/analyze.rs`: `analyze subtract-line`, `reduce-x`,
  `translate`, `median-filter`.

## Steps

1. Numerics in sheaf-calc with doc comments and tests on synthetic data worked out by hand.
2. Specs and `Tool` variants, run in `Project::analyze`, linkable.
3. Dialog kinds and menu items (Curve Translate Y added next to Curve Translate X; done).
4. CLI subcommands with tests.
5. Spec section; look at the dialogs with `--show analysis:NAME --screenshot`.

## Done when

The four menu items open working dialogs, results appear as columns or a new sheet, Recalculate
Manual/Auto follows edits, `sheaf analyze ...` works, and fmt, clippy and tests pass.

## Don't

Do not claim Origin's numbers for the unmeasured details; keep them listed in the spec.
