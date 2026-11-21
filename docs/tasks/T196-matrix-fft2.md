# T196: 2D FFT and 2D IFFT of a matrix (B14 slice)

**Status:** done 2026-11-20 (Claude Windows "obj-align"). Built on numpy's `fft2` conventions,
**not measured** against Origin (measured in T287: the numbers match).
**Size:** S. **Source:** greyed inventory package B14 ("Signal Processing > FFT: 2D FFT... · 2D IFFT...").
**Spec:** `docs/spec/matrices.md` 7e.

## Done

- `sheaf_calc::fft2::fft2` (rustfft, rows then columns) and `Project::matrix_fft2`. `MatrixCmd::Fft2`
  and `MatrixCmd::Ifft2` are in the matrix bar's Analysis > Signal Processing > FFT and in the
  workbook and graph Analysis menus.
- Tests: `fft2::tests::matches_numpy_fft2_and_inverts` and
  `matrix_window::two_d_fft_and_ifft_round_trip_through_new_matrix_books`.

## Left

- Measuring Origin's `fft2` output, shift and normalization, and adding their options.
