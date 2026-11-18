# T96: Wavelets (B8 slice): Decompose, Reconstruction, Denoise, Smooth, Continuous Wavelet

**Status:** done 2026-11-16 (all five items built; Claude Windows, branch `wavelets`, worktree `.claude/worktrees/wavelets`, not merged). Origin 2016 measured
the same day (`docs/spec/analysis-tools.md` section 19, fixtures `tools/origin-ref/t96/t96-origin.json`). Built: Decompose, Reconstruction, Denoise, Smooth
(`crates/calc/src/wavelet.rs`, `crates/core/src/analysis/wavelet.rs`, `crates/app/src/dialogs/wavelet.rs`, `sheaf analyze wavelet-decompose | wavelet-reconstruct |
wavelet-denoise | wavelet-smooth`, MCP `wavelet_decompose | wavelet_denoise | wavelet_smooth`; tests `crates/calc/tests/origin_wavelet.rs`) and Continuous Wavelet
(`crates/calc/src/cwt.rs`, `Tool::ContinuousWavelet` in the same core module, `sheaf analyze cwt --with <scale column> --wavelet morlet|dgauss|mexhat --param N`,
the window in `dialogs/wavelet.rs`; test `crates/calc/tests/origin_cwt.rs`: 43 Origin runs, all three wavelets, 1e-12; the formula is the integrated, sampled mother
wavelet of 19.6). Not built, no script route in Origin and not measured: Continuous Wavelet's pseudo-frequency output, coefficient matrix and image plot, and the MCP
tool for `cwt` and `wavelet_reconstruct` (they need a second column). Open corners kept visible in the windows: `sqtwolog` is a fit to Origin's brackets, Denoise's first and last `L - 2` samples can
differ, Smooth is exact for periodic power-of-two lengths only, odd intermediate lengths of a periodic multi-level run are refused (Origin accepts n = 100).
**Size:** L. **Spec:** `docs/spec/analysis-tools.md` section 19 (19.1 wavelets, 19.2 `dwt`, 19.3 `idwt`, 19.4 `wtdenoise`,
19.5 `wtsmooth`, 19.6 `cwt`, 19.7 windows, 19.8 not measured).
**Inventory:** Analysis > Signal Processing > Wavelet: Decompose... · Reconstruction... · Denoise... · Smooth... · Continuous
Wavelet... (5 items, tagged `T96`; X-Functions `dwt`, `idwt`, `wtdenoise`, `wtsmooth`, `cwt`).

## Goal

The five items do what Origin 2016's do, built in the layout of the existing signal tools (FFT, FFT Filters, Hilbert, Envelope,
Coherence, Decimation: `analysis-tools.md` 13 to 16; `crates/calc/src/decimate.rs` is the closest model: options struct, pure
function, one result sheet, errors that mirror Origin's refusals). Order of work, one commit each and each item enabled only
when it matches the spec:

1. **Decompose** and **Reconstruction** (exact, simplest).
2. **Smooth** (position rule, one open corner) and **Denoise** (custom threshold exact, `sqtwolog` and boundary open).
3. **Continuous Wavelet** (closed form found, 19.6).

## Origin facts (measured 2026-11-16, `re-workspace/scratch/t96/`, local only; generators in `tools/origin-ref/t96/`)

- **21 wavelets**: Haar, DB2 to DB10, Bior1.1 to 3.7 (Bior1.1, 1.3, 1.5, 2.2, 2.4, 2.6, 2.8, 3.1, 3.3, 3.5, 3.7). No other
  families. Filters are PyWavelets' `haar`, `dbN`, `biorX.Y` (to 1e-15). Extension modes: Periodic (default) and Zero-padded.
- **`dwt`** (one level): `ca[k] = sum_j lo[j] x[2k+1-j]`, `cd[k]` with `hi`; periodic wraps (`n/2` values, odd `n` refused),
  zero-padded gives `floor((n+L-1)/2)` values (= `pywt.dwt(mode='zero')`). Output: two new columns `DWT_CA1`, `DWT_CD1`.
- **`idwt`**: `y[2k+j-(L-2)] += ca[k] rl[j] + cd[k] rh[j]`, periodic modulo `2N`, zero-padded drops outside `0..2N-L+1`
  (length `2N-L+2`); unequal lengths follow `ca`. Output column `IDWT_Result1`.
- **`wtdenoise`**: levels 1 to `level` of the periodic or zero-padded decomposition, **hard threshold, one common
  `T = thrd/100 * max|d|` (custom, `>=`)**, approximation untouched; `sqtwolog` has a fixed `T` that depends only on `n` (not
  found: table of brackets in the fixture, fit `0.6592 sqrt(2 ln(n/2))` for `n >= 256`, about 1.88 below). Level limit:
  periodic `floor(log2 n) - 1`, zero-padded `floor(log2 n)`; Haar with zero-padded refused for power-of-two lengths.
  A boundary effect of `L - 2` samples at each end in some cases is not explained (19.4).
- **`wtsmooth`**: decomposition to the deepest level, detail coefficients laid out coarsest first, **the last
  `floor(cutoff/100 * N)` set to 0** (position rule, finest level first), approximation kept. Corners: `K = 2` at cutoff 0 and 1
  for `n = 128`; zero-padded and non-power-of-two lengths not exact (19.5).
- **`cwt`** (closed form, 19.6): output sheet `CWTResults1`, rows = samples, columns = scales in the given order (real numbers); the mother
  wavelet sampled at 1024 points on [-8, 8] (Morlet, MexHat) or [-5, 5] (DGauss), running integral, scale cut to its integer part
  (scales below 1 refused), filter `ip[floor(i / (a step))]`, `-sqrt(a)` times the first difference of the convolution, the middle `n` values;
  wave number cut to an integer, Morlet 5 to 20, DGauss order 1 to 8. Matrix output, image plot and pseudo-frequency are not reachable from a script.
- **Windows** (19.7): titles `Decompose: dwt`, `Reconstruction: idwt`, `Denoise: wtdenoise`, `Smooth: wtsmooth`, `Continuous
  Wavelet: cwt`, fields and defaults in the table there; Denoise and Smooth have a preview pane.

## Where

- `crates/calc/src/wavelet.rs` (new): filter tables for the 21 wavelets (write the coefficients as numbers in the source with
  the Daubechies and biorthogonal tables from the literature or from a clean derivation, not copied from PyWavelets' files;
  check them against `t96-origin.json` to 1e-12), `dwt`, `idwt`, multilevel helpers, `denoise`, `smooth`; `crates/calc/src/cwt.rs`
  (new) for the continuous transform. One line each in `crates/calc/src/lib.rs`.
- `crates/core/src/analysis.rs`: additive `Tool::WaveletDecompose`, `WaveletReconstruct`, `WaveletDenoise`, `WaveletSmooth`,
  `ContinuousWavelet` (specs in a new module `crates/core/src/analysis/wavelet.rs`, as `Tool::Decimate(DecimateSpec)` does for
  its options); results as ordinary sheets/columns named as in 19.2 to 19.6 (long name, comment).
- `crates/app/src/menus.rs`: the `grey_sub("Wavelet", ...)` block becomes a `sub("Wavelet", [item(..)...])` with five
  `Cmd::Analyze(AnalysisKind::...)` items; `crates/app/src/dialogs/analysis.rs` (new `AnalysisKind` values and one option
  function each, like `decimate_options`; put the five windows in a new file `crates/app/src/dialogs/wavelet.rs` and call it
  from there); `crates/ops/src` (`sheaf analyze wavelet-decompose | wavelet-reconstruct | wavelet-denoise | wavelet-smooth | cwt`);
  `crates/mcp`. **Another session (T57, branch `deconv-stft`) edits the same menu block, `analysis.rs` and `dialogs/analysis.rs`:
  keep edits additive and local, merge `main` often.**
- Tests: `crates/calc/tests/origin_wavelet.rs` reading `tools/origin-ref/t96/t96-origin.json` (or a small Rust fixture file in
  the style of `crates/calc/tests/fixtures/*.rs`: copy the numbers, label the tolerance), a UI test in
  `crates/app/src/app/ui_tests.rs`, and `--show` screenshots of the five windows.

## Steps

1. `wavelet.rs`: tables, `dwt`/`idwt` for both extension modes; tests against the `dwt` and `idwt` blocks of the fixture (1e-12),
   the odd-length refusal, perfect reconstruction for all 21 wavelets. Tool, windows and menu items for Decompose and
   Reconstruction; output columns named as in 19.2 and 19.3.
2. Smooth: deepest level, tail zeroing; tests against the `smooth` block (periodic power-of-two lengths exact; the corner `K = 2`
   at cutoff 0 and 1 reproduced as measured and noted in the code). Zero-padded and other lengths: build the rule of 19.5, check it
   against the fixture cases, and where it does not match say so in the window's tooltip and in 19.5, or measure more.
3. Denoise: custom threshold as in 19.4 (test: the `denoise` block, 1e-12, all samples except the boundary cases); `sqtwolog` from
   the bracket table (a test that the kept/dropped sets of the bracket signals are reproduced); the level limits and the Haar
   zero-padded refusal as measured. Try to find the boundary rule from `denoise_boundary` (the model is exact in the interior; the
   first and last `L-2` samples differ when the wrap-around coefficients are involved); if it stays unknown, document and mark the
   test with the interior range.
4. Continuous Wavelet (done): found by recognising the half-sample centre as the first difference of an integrated, sampled
   wavelet; tested to 1e-12 on 43 Origin runs; output sheet `CWTResults1` in the input book (short names A, B, ... where Origin
   names the first column `Coefficient`).
5. Measure what 19.8 lists when cheap (Results Log lines, missing values, the matrix output and the image plot through the
   window, the pseudo-frequency), update section 19.
6. Each tool: one undo step, ordinary sheets, delete the five lines from `docs/tasks/greyed-items.md`, `python
   tools/greyed-count.py --write` (revert its change to `HANDOFF.md`), update this brief's Status and the README row.

## Done when

The five menu items are enabled and match the spec on the fixture within the stated tolerances (the open corners named in the
spec are either solved or still listed there with a visible note in the window); fmt, strict clippy, `cargo test --workspace` and
`python3 tools/greyed-count.py --check` pass; the inventory lines are deleted; the README row says done.

## Don't

- Do not use Symlets, Coiflets or other families: Origin offers 21 wavelets only.
- Do not use pywt's `periodization` mode or the symmetric mode for "Periodic": the alignment is the one in 19.2 (circular
  shift of `periodization`).
- Do not implement Denoise's `custom` as soft thresholding, per-level thresholds or a universal threshold with a noise estimate
  from the data: it is hard, one `T` over all levels, relative to the largest coefficient.
- Do not implement Smooth as "zero the smallest coefficients" or "zero the finest levels": it zeros the last `K` entries of the
  coarse-to-fine layout.
- Do not call the `cwt` numbers Origin's where they are fitted or scipy-checked; label them as in the spec.
- Do not edit `HANDOFF.md` beyond your own claim; keep edits to `analysis.rs`, `menus.rs`, `dialogs/analysis.rs` and
  `crates/ops/src/*.rs` additive.
