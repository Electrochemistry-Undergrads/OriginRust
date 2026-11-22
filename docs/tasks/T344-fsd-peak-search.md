# T344: Find Peaks on the Fourier self-deconvolved curve

**Status:** done 2026-11-21 (Claude Windows "claude:2"). Sheaf's own (Origin 2018b's FSD peak detection is
not in Origin 2016).
T341 left item. `FindOptions::fsd`: the peak search runs on the FSD-narrowed corrected curve so that
hidden bands are found; heights, widths and areas are measured on the data. Peak Analyzer Find Peaks page
tick, CLI `analyze peaks --fsd-fwhm W --fsd-k K`.
**Size:** S. **Spec:** `docs/spec/analysis-tools.md` 22.

## Tests

- `peakchain::tests::the_fsd_search_finds_hidden_peaks` (calc), `the_fsd_peak_search_finds_hidden_bands` (cli).

## Left

- A band width guess from the data; the FSD search in Batch Peak Analysis themes is saved (serde) but not
  shown in the theme list's summary.
