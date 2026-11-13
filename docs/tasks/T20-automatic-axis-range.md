# T20: Automatic axis range as Origin (linear and log)

**Status:** done, merged (`522c625`). Left: Rescale Margin is not stored in `.opj` (named in the save warnings); all-negative columns, other margins on log axes and the step at margins above 16 % are not measured (graphing.md section 4); the `Rescale` combo keeps Sheaf's names (Origin: Normal and others). **Size:** M.
**Branch:** `axis-range2`. **Model:** Sonnet.
**Detail:** audit C "Different, measured in the spec" and "Different, checked in Origin" (log axes); B
"Axis dialog" (Rescale Margin). **Spec:** `docs/spec/graphing.md` section 4 (read it whole, it is short).

## Today

- `crates/plot/src/ticks.rs`: `resolve_linear` with `AutoRange::Nice` rounds the `span/5` step outward
  with **no margin**; `nice_step` picks from {1, 2, 5} x 10^k. `resolve_log` always snaps to whole
  decades (at least one) and never switches to linear ticks.
- A second copy of the rule: `nice_range` in `crates/io/src/opj/write/graph.rs` (writes the range Origin
  shows on open). Change both, or make the writer call the plot crate's rule.
- `AutoRange` doc in `crates/core/src/graph.rs` says "with a small margin" (wrong today; T17 item 3).
- Axis dialog `crates/app/src/dialogs/graph_axis.rs`: "Rescale Margin(%)" is greyed and shows 0;
  Origin's default is 8. Rescale choices are Round out / Exact / Symmetric (Origin: Normal and others).
- `AutoRange::SymmetricZero` (the owner's CV rule, LESSONS) and `Tight` must not change.

## Origin facts (measured, 56 linear cases in `tools/origin-ref/autoscale*.lines.txt`)

- Limits: `floor((lo - 0.08 span) / h) * h` .. `ceil((hi + 0.08 span) / h) * h`, `h` = half the major
  step (the minor spacing; one minor tick between majors). Holds in all 56 cases.
- The major step is 1, 2 or 5 x 10^k but not a simple function of the span; spec section 4 lists the
  measured span → step pairs. Find a rule that reproduces all 56 (try `span/n` thresholds per decade;
  the spec's proposed `span/6` with {1, 2, 2.5, 5} is a guess and does not explain 0.91..1.2 → 0.5).
- Examples: -0.2..1.0 → -0.5..1.25 (0.5); 0..10 → -2.5..12.5 (5); 12..77 → 0..90 (20);
  100..1000 → 0..1100 (200); 0.4..0.995 → 0.3..1.1 (0.2).
- Log axes (checked, `re-workspace/scratch/audit-2/gr*.lines.txt`): after `layer -a` Origin gave
  1.7778..9 for data 2.06..8 and 1..300 for 1.5..150, so it pads and rounds on log axes too (not whole
  decades). Measure ~10 more log cases before writing the rule.

## Steps

1. Turn the 56 cases into a fixture test (`crates/plot/tests/` or `ticks.rs` tests), failing first.
2. Implement the margin and the step rule in `resolve_linear` for `AutoRange::Nice`; same in
   `nice_range`.
3. Measure log cases with `tools/origin-ref/run-labtalk-lines.ps1` (copy `autoscale.lines.txt`, set
   `layer.y.type=2`), implement, fixture.
4. Make Rescale Margin live in the axis dialog (default 8, stored on the `Axis`), `.opj` write/read if
   the field is known (opj-format.md), otherwise name it in the save warnings.
5. Golden images change: regenerate only after looking at each changed SVG (LESSONS on
   `SHEAF_UPDATE_GOLDEN`), and say in the commit which ones changed and why.

## Done when

All 56 linear cases and the measured log cases match; CV template (SymmetricZero) unchanged; dialog
margin live; checks pass; T17 item 3 (AutoRange doc) done with it.
