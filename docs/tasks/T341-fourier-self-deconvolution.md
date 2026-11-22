# T341: Fourier Self-Deconvolution

**Status:** done 2026-11-21 (Claude Windows "claude:2"). Sheaf's own (Origin 2018b's tool is not in
Origin 2016 and not described in OriginLab's pages).
HANDOFF C / origin-2026-2027-gaps "Fourier self-deconvolution peak detection". Narrows Lorentzian bands K
times (Kauppinen 1981: line removed, FFT, exp(2 pi g |t|), apodization ending at L = c K / FWHM, inverse
FFT) so that overlapping peaks separate; the area is kept. Analysis > Peaks and Baseline > Fourier
Self-Deconvolution..., CLI `analyze fsd --fwhm W --k K --apodization ...`, result column beside the input.
**Size:** M. **Spec:** `docs/spec/analysis-tools.md` 22.

## Tests

- `fsd::tests` (calc: width / K and area for each apodization, two bands separate, bad input),
  `manip::tests::fourier_self_deconvolution_separates_two_bands` (core), `fsd_narrows_the_bands` (cli).

## Left

- Done in T344: FSD inside the Peak Analyzer's Find Peaks. Left: a band width guess from the data.
