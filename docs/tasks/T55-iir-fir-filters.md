# T55: IIR Filter and FIR Filter

**Status:** done 2026-11-15 (session "violin", Claude macOS, no Origin), branch `digital-filters`, ready to merge; documented in analysis-tools.md 17. Review fixes 2026-11-15: finite gains at high IIR orders (scaled products), non-finite FIR windows refused with Scale Passband off, dialog start values from the real Fs, scipy fixtures for high orders, window parameters, least squares of every response (even taps against a numpy optimum) and minimum-order assemblies; main merged and the parity counts recomputed (249 rows).
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

Full list in analysis-tools.md 17 (with the URLs); no Origin was available, so nothing is measured.

- The X-Function is `dfilter` (OriginPro; `iirfilter` is not an Origin name), with `dfilterapply` for
  applying a designed filter. Methods: Butterworth, Chebyshev Type I, Chebyshev Type II, Elliptic. **Bessel
  is not offered**, so Sheaf does not build it. Response types low, high, band pass, band stop. Defaults:
  order 10, Hz, ripple 1 dB, attenuation 80 dB, forward-backward off. A Minimum order checkbox. Fs is read
  from the X column. Outputs: SOS matrix, zeros/poles/gain, state space, coefficients, filtered signal.
  Algorithm: analog prototype, frequency transform, bilinear transform; forward-backward gives zero phase.
- The FIR Filter is a File Exchange app (Origin 2027): Least Squares or Window methods, order (Minimum
  with the Kaiser window for the Window method), 17 windows, Scale Passband, weights, ripple and
  attenuation (dB or linear), forward-backward. No Parks-McClellan. Defaults, formulas and the alignment of
  the output are not documented.

## What was built

`sheaf_calc::{iir, fir}` (design and application, checked against `scipy.signal` by
`tools/iir-fir-ref.py` and `crates/calc/tests/scipy_filters.rs`), `Tool::IirFilter` and `Tool::FirFilter`
with `LinkTool` variants (Recalculate, `.opj` warnings), the two windows under Analysis > Signal
Processing, `sheaf analyze iir-filter|fir-filter` (aliases `iir`, `fir`) and the MCP bridge
(`iir_filter`, `fir_filter`). Sheaf's unmeasured choices (the meaning of Fc per method, the passband gain of even-order Chebyshev I and elliptic filters, the high-order gain) are listed in analysis-tools.md 17.

Left: Origin measurements on Windows (`dfilter` exists in Origin 2016; the FIR app needs Origin 2027), the
SOS, ZPK, state-space and coefficient output sheets, `dfilterapply` and `dfilterfcf`, the preview panels
of the dialogs, a custom FIR window from data.

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
