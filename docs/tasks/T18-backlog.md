# T18: Backlog (not yet briefed)

Large or later items. Before starting one, write a proper brief (see README "Brief format") from
the audit section named and the parity table row.

| Item | Audit | Notes |
|---|---|---|
| Matrices, contour, heatmaps | E, C | owned by `originrust-3a`; do not start |
| Notes, Layout, Excel, Image, Function windows | A, E | `Window` is only Workbook/Graph (`crates/core/src/project.rs`) |
| Command Window, Script Window, scripting runtime | A, D | milestone 6 "embedded language"; ADR first |
| Analysis templates and recalculation of analysis results | D | relates to T03 Recalculate |
| Templates (save/load), themes (.OTH), Theme Organizer (F7) | C, B | 9 built-in templates today |
| Tools > Options (ten tabs), Fitting Function Builder/Organizer (F8/F9) | B | |
| Importers: Excel, HDF5, NetCDF, JCAMP, Matlab, Thermo, ODBC | A, E | only CH Instruments and ASCII today |
| `.opju` reading | E | refused in `crates/io/src/opj/mod.rs` |
| `.opj` read of matrices, notes, layouts, fits, masks, number formats, hidden columns, widths | E | plot groups and legend frames: branch `opj-groups-frames` |
| Signal processing: FFT/IFFT tool, FFT filters beyond low-pass, Hilbert, convolution, deconvolution, STFT, wavelets, IIR | D | |
| Mathematics: cumulative sum tool, level crossing, curve intersect, average curves, polygon area | D | Median Filter, Subtract Straight Line, Reduce to Evenly Spaced X, Curve Translate: brief T23 |
| Multivariate: PCA, k-means, ROC, Kaplan-Meier | D | |
| Toolbars as separate bars, View > Toolbars customisation | A | |
| Drawing tools: lines, arrows, rectangles, pictures, inset | C | lines, arrows, rectangles, ellipses: brief T24 (`draw-objects`); pictures and inset still open |
| Autosave and recovery; Append Project | E | |
| Print, Print Preview, Page Setup | A | |
| 3D graphs | C | last (owner's request) |
