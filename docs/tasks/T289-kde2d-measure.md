# T289: Measure 2D Kernel Density against Origin 2016

**Status:** done 2026-11-21 (Claude Windows "claude:fe4b", graphing.md 38). Origin's `plot_kde2` makes a
32 x 32 density matrix over min - range / 4 .. max + range / 4 per axis with the Gaussian product kernel.
Sheaf now uses the same grid. Rule of Thumb (`sd * n^(-1/6) / 2`) matches Origin's matrix to 1e-15 of
its peak. Bivariate (Origin's default) uses the published diffusion estimator of Botev et al. (2010):
bandwidths +2.4 % and +0.4 % on the test data, the matrix within 1.3 % of the peak.
**Size:** S. **Follows:** T116. **Spec:** graphing 38.

## Left

- Origin's exact Bivariate rule (not documented; the diffusion estimator is the closest found).
- Origin's default contour drawing, its levels and colours, and its default of showing 0 points.
- The method and plot-type choices in Sheaf's UI.

## Tests

- `crates/calc/tests/origin_kde2d.rs`: the grid, Rule of Thumb exact, Bivariate within 3 %.
- `crates/calc/src/kde.rs`: `density_2d_tests`.
