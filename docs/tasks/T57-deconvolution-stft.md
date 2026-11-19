# T57: Deconvolution and STFT

**Status:** done 2026-11-19 (Claude Windows "t30-origin", taking over the 2026-11-15 claim of Claude
macOS "violin", which had no branch or code). Not measured: Origin's COM class is unregistered on the
test machine. Left: measure Origin's dialogs, defaults and outputs and match them; write STFT to a
matrix once matrix windows exist.
**Size:** M. **Source:** the greyed Analysis > Signal Processing > Deconvolution... and STFT...
(Origin 2016's `deconv` and `stft`; T18 backlog "deconvolution, STFT ... remain").
**Spec:** `docs/spec/analysis-tools.md` 20.

## Done

- `sheaf_calc::deconvolve` (FFT division over the signal's points) and the new `sheaf_calc::stft`
  module, checked against numpy.
- `Tool::Deconvolution` and `Tool::Stft` in `sheaf-core`, each writing a new sheet.
- `sheaf analyze deconvolution` and `sheaf analyze stft`; MCP reaches both through the analyze
  bridge.
- The two menu items are live, and both open the analysis window with their options.

## Don't

- Don't change Convolution, Correlation, FFT, IIR/FIR results. No matrix-window work.
