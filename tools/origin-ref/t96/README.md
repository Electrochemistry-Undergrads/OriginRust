# T96 wavelet reference runs (Origin 2016)

Generators and numbers for `docs/spec/analysis-tools.md` section 19 (Analysis > Signal Processing > Wavelet).
Run them from an empty scratch folder (they write `*.lines.txt` and `*.out` into the current folder; the raw
outputs stay local, `re-workspace/scratch/t96/`) with `python <script>.py` (not `-I`: they import each other).

- `lib.py` builds the line scripts for `../run-labtalk-safe.ps1` and parses the sheet dumps; `wt.py`, `wtm.py` are the
  numpy models used to read the results (periodic DWT in Origin's alignment, its inverse, tail zeroing).
- `d2.py` dwt, all 21 wavelets, both extension modes; `i1.py` idwt on unit coefficients; `n2.py`, `s1.py`, `s5.py`, `s9.py`
  wtdenoise; `sm1.py`, `sm6.py` wtsmooth; `c1.py`, `c2.py`, `c3.py` cwt; `m2.py` mdwt; `n1.py` the noisy test signal.
- `mkfix.py` turns the outputs into `t96-origin.json`, the fixture file for the Rust tests (inputs and Origin outputs).
- `a3.py`, `n4.py` are the fits that identified the filter alignment and the denoise rule.
