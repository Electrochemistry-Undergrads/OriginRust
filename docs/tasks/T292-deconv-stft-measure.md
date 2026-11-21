# T292: Deconvolution and STFT measured against Origin 2016

**Status:** done 2026-11-21 (Claude Windows "claude:2"), measured.
HANDOFF "Next candidates" B for T57. Origin 2016's `deconv`: Linear (its default) pads signal and
response to 2N - 1 points and keeps N - m + 1 values (reproduced to 1e-12 on two signals); Circular is
Sheaf's former method; Normalize is on by default; the output has no Long Name and the Comments
"deconvolution of B, C". Sheaf now has both types (Linear by default, `--circular`), the window
starts with Normalize on, and the output is labelled as Origin's. Origin's `stft` amplitudes and dB
equal Sheaf's exactly; its defaults (256/256/128, dB) are now Sheaf's. Its matrix output is not built
(Sheaf still writes a sheet).
**Size:** S. **Spec:** `docs/spec/analysis-tools.md` 20. Probes: `re-workspace/scratch/deconv-measure/`,
`re-workspace/scratch/stft-measure/`.

## Tests

- `signal::tests::deconvolution_matches_origin_2016`, `stft::tests::defaults_match_origin_2016`, the
  analysis, CLI and UI deconvolution/STFT tests.

## Left

- STFT into a matrix book with an image plot, as Origin.
