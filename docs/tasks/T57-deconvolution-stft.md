# T57: Deconvolution and STFT

**Status:** claimed 2026-11-15 (session "violin", Claude macOS, no Origin), branch `deconv-stft`.
**Size:** M. **Source:** the greyed Analysis > Signal Processing > Deconvolution... and STFT...
(Origin 2016's `deconv` and `stft`; T18 backlog "deconvolution, STFT ... remain").

## Goal

Deconvolution... recovers a signal from a convolved signal and a response, as Origin's `deconv`
documents; STFT... computes the short-time Fourier transform of a column (window, size, overlap,
FFT length, output as Origin documents: amplitude/power/dB, a matrix or worksheet of time x
frequency). Both reach the CLI (`sheaf analyze`) and MCP like Convolution and FFT.

## Origin facts (docs, not measured)

To be filled from Origin's `deconv` and `stft` X-function pages before building; Sheaf's choices
go in analysis-tools.md 18 labelled unmeasured. Measure on Windows with `tools/origin-ref/`.

## Where

- `crates/calc` (`signal.rs` convolution, `fft.rs`), a new module for STFT; scipy/numpy checks.
- `crates/core/src/analysis.rs` tools and runners (Convolution, FFT as models).
- `crates/app/src/menus.rs` Signal Processing, dialogs; `crates/ops` / CLI / MCP bridge.
- `docs/spec/analysis-tools.md` 18 (new).

## Done when

Menu, CLI and MCP run both; outputs match numpy/scipy on fixtures (and Origin's documented
definitions); dialogs follow Origin's documented controls; analysis-tools.md 18 records the facts.

## Don't

- Don't change Convolution, Correlation, FFT, IIR/FIR results. No matrix-window work (another
  session's local matrix branch): if Origin outputs STFT to a matrix, write a worksheet and say so.
