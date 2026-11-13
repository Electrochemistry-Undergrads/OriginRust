# T16: Small numeric and behaviour differences

**Status:** item 2 done, merged (`94063b4`) (natural spline, 900 limit, increasing X, B-spline and Bezier added; spec 5.2); items 1 and 5 done, item 4 investigated, merged (`b967898`) (not fixed, findings in statistics.md); 3 done; 6 open (a different thing: the analysis smoothing factor, not the line connection). **Size:** S each; independent, one commit each. **Branch:** `small-diffs`.
**Model:** Sonnet (Haiku to run the suites). **Detail:** audit C "Missing objects", D "Numerically
or behaviourally different".

1. **Mann-Whitney print:** with "exact" selected and the asymptotic p printed, Sheaf prints the
   continuity-corrected value where Origin prints the uncorrected one, for the **upper tail only** (DONE, `d3f2e4b`)
   (two-sided and lower match). `mann_whitney()` in `crates/calc/src/nonparam.rs`; spec
   `docs/spec/statistics.md` (search "Mann-Whitney", around lines 193-196). Signed rank unaffected.
2. **Spline connect:** Sheaf's Spline is Catmull-Rom with duplicated ends, needs no increasing X, and
   above `SPLINE_MAX_POINTS = 2000` falls back to straight segments silently
   (`crates/plot/src/plots.rs`). Spec graphing.md 5.2: Origin uses a cubic spline, max 900 points.
   Measure what Origin draws above 900 points before copying.
3. **Error bar cap:** fixed at 1.2 x symbol size (`half_cap = 0.6*size` in plots.rs); Origin has a
   cap width setting (default to measure). Pairs with T12 item 2: do it there if T12 is running. Done, merged (`d0dbb37`): default 8 pt, setting in Plot Details > Error Bar.
4. **Kendall p with missing data** differs up to 17 %; **Lilliefors p above 0.1** about 6 %; **KS p**
   wrong for 5 or 40 numbers. Scripts and outputs in `re-workspace/scratch/audit-3/` (`kend.py`,
   `ks.py`, `ref.py`, `*.out.txt`). Fix only where Origin's rule is found; else record in
   statistics.md. (INVESTIGATED: none of the three rules found; KS and Lilliefors now have a 70-run fixture,
   Kendall implied variances are recorded; see statistics.md, normality and correlation sections.)
5. **Histogram automatic bin size** agrees with Origin in about 57 % of cases
   (`crates/calc/src/freq.rs`); measured cases in `tools/origin-ref/stats-bins*.lines.txt`. Find the
   rule from them. (DONE: `m = round(1.75 ln n)`, size `range/(m-0.19)` to 1 digit (n<14) or 1-2-5 (n>=14); 593/600.)
6. **B-spline smoothing factor 0** differs (Origin's automatic factor unknown,
   `crates/calc/src/interp.rs`). Low priority; documented already.

Done when: each fixed with an Origin fixture, or the brief says why not; checks pass.
