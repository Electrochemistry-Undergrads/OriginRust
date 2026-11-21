# T287: Matrix 2D FFT measured against Origin 2016

**Status:** done 2026-11-21 (Claude Windows "claude:2"), measured.
HANDOFF "Next candidates" B for T196. Origin 2016's `fft2` (defaults) on a 4 x 4 matrix equals numpy's
`fft2` cell for cell (no normalization, no shift); `ifft2` returns the input. Its outputs are Complex,
Amplitude, Phase and Power sheets; Sheaf's 2D FFT book now has Power (|F|^2) besides Real,
Imaginary, Amplitude and Phase.
**Size:** S. **Spec:** `docs/spec/matrices.md` 7e. Probe: `re-workspace/scratch/fft2-measure/`.

## Tests

- `matrix_window::two_d_fft_and_ifft_round_trip_through_new_matrix_books` (the five sheets).
