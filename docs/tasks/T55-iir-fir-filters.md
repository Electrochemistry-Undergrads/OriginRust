# T55: IIR Filter and FIR Filter

**Status:** claimed 2026-11-15 (session "violin", Claude macOS, no Origin), branch `digital-filters`.
**Size:** L. **Source:** the greyed Analysis > Signal Processing > IIR Filter... (Origin 2016's
`iirfilter`, T18 backlog "IIR remain") and `origin-2026-2027-gaps.md` (Analysis: FIR filter design
and application, Origin 2027 app).

## Goal

Analysis > Signal Processing > IIR Filter... designs a Butterworth, Chebyshev I/II, elliptic or
Bessel filter (low/high/band pass, band stop) as Origin's `iirfilter` documents and applies it to
a Y column; FIR Filter... designs a windowed-sinc FIR filter as Origin's FIR app documents and
applies it. Both write the filtered column next to the input like the other signal tools, with
CLI (`sheaf analyze iir|fir`) and MCP reach.

## Origin facts (docs, not measured)

To be filled from Origin's `iirfilter` X-function page and the FIR Filter app page before
building; Sheaf's choices go in analysis-tools.md 17 labelled as unmeasured. Measure on Windows
with `tools/origin-ref/` (`iirfilter` exists in Origin 2016).

## Where

- `crates/calc` new filter design/apply module (checked against `scipy.signal`).
- `crates/core/src/analysis.rs` (`AnalysisKind`, tool requests), the existing signal tools as models
  (FFT Filters, Smooth, Decimation).
- `crates/app/src/menus.rs` (Signal Processing), analysis dialogs.
- `crates/ops` / CLI analyze, MCP bridge (picks up CLI tools automatically).
- `docs/spec/analysis-tools.md` 17 (new).

## Done when

Menu, CLI and MCP filter a column; filter coefficients and outputs match `scipy.signal` to tight
tolerances on fixtures; the dialog follows Origin's documented controls; recalculation and `.opj`
warnings as the other signal tools; analysis-tools.md 17 records Origin's facts and Sheaf's choices.

## Don't

- Don't change FFT Filters, Smooth or Decimation results.
