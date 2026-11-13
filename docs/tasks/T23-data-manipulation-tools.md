# T23: Data Manipulation tools and Median Filter

**Status:** in progress (branch `math-manip`, built on macOS without Origin). Every Origin behaviour
below that is not in a spec table is from Origin's public documentation, paraphrased, and is not
measured; each is listed in the spec section "Data manipulation tools and Median Filter" as to be
measured on the Windows machine with `tools/origin-ref`.
**Size:** M. **Branch:** `math-manip`. **Model:** Sonnet.
**Detail:** audit D "Partial"; backlog row "Mathematics" of T18.
**Spec:** `docs/spec/analysis-tools.md` section 9 (Subtract Reference Data, the pattern to copy) and the
new section 11.

## Goal

The greyed items of Analysis > Data Manipulation (Subtract Straight Line, Reduce to Evenly Spaced X,
Curve Translate X) and Analysis > Mathematics > Median Filter work as tools of the shared analysis
dialog frame, with a recalculation link, a CLI subcommand and tests.

## Origin facts

- Measured earlier: the straight-line subtraction arithmetic and linear interpolation with straight
  end-segment continuation (`subtract_ref`, spec section 9; `calc::subtract_line`, `calc::interp1`).
- From Origin's public documentation, not measured: Subtract Straight Line subtracts the line through
  two points chosen on the graph; Reduce to Evenly Spaced X offers interpolation and averaging within
  bins; Curve Translate moves a curve by a constant or so that a chosen point lands on a chosen value;
  the Median Filter takes an odd window of points. Edge handling of the median filter is not documented
  in a way we can rely on.

## Where

- `crates/calc/src/manip.rs` (new): `value_at`, `subtract_line_at`, `reduce_evenly`, `translate`
  helpers, `median_filter`.
- `crates/core/src/analysis.rs`: `Tool::SubtractLine`, `Tool::ReduceX`, `Tool::Translate`,
  `Tool::MedianFilter` (specs in `crates/core/src/analysis_manip.rs`, new); `analysis_link.rs`: the four
  tools are linkable.
- `crates/app/src/dialogs/analysis.rs` (`AnalysisKind`), `crates/app/src/menus.rs` (greyed items).
- `crates/cli/src/args.rs`, `crates/cli/src/analyze.rs`: `analyze subtract-line`, `reduce-x`,
  `translate`, `median-filter`.

## Steps

1. Numerics in sheaf-calc with doc comments and tests on synthetic data worked out by hand.
2. Specs and `Tool` variants, run in `Project::analyze`, linkable.
3. Dialog kinds and menu items (Curve Translate Y added next to Curve Translate X).
4. CLI subcommands with tests.
5. Spec section; look at the dialogs with `--show analysis:NAME --screenshot`.

## Done when

The four menu items open working dialogs, results appear as columns or a new sheet, Recalculate
Manual/Auto follows edits, `sheaf analyze ...` works, and fmt, clippy and tests pass.

## Don't

Do not claim Origin's numbers for the unmeasured details; keep them listed in the spec.
