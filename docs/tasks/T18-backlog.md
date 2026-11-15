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
| Importers: Excel, HDF5, NetCDF, JCAMP, Matlab, Thermo, ODBC | A, E | only CH Instruments and ASCII today; Excel import and xlsx export: brief T30 |
| `.opju` reading and writing | E | workbooks read since `39178fc` (`crates/io/src/opju`, spec `opju-format.md`); left: decoding drift in about 200 columns of the samples (section 5.4), designations and labels, graphs, matrices, folders; no writer (a save to `.opju` is refused) |
| `.opj` read of matrices, notes, layouts, fits, masks, number formats, hidden columns, widths | E | plot groups and legend frames: branch `opj-groups-frames` |
| Signal processing: FFT/IFFT tool, FFT filters beyond low-pass, Hilbert, convolution, deconvolution, STFT, wavelets, IIR | D | convolution, correlation, the Hilbert transform (13), envelope, coherence, decimation (13.2-13.4), FFT/IFFT and FFT filters (15) done; deconvolution, STFT, wavelets, IIR remain |
| Mathematics: cumulative sum tool, level crossing, curve intersect, average curves, polygon area | D | done: T22, T23, level crossing (analysis-tools 14), trace interpolation (16); cumulative sum dropped (Origin has no tool, only `cumsum()`) |
| Multivariate: PCA, k-means, ROC, Kaplan-Meier | D | brief T29 |
| Toolbars as separate bars, View > Toolbars customisation | A | |
| Drawing tools: lines, arrows, rectangles, pictures, inset | C | lines, arrows, rectangles, ellipses: brief T24 (`draw-objects`); pictures and inset still open |
| Autosave and recovery | E | brief T28 done (autosave.md); Append Project done (`21c2aea`: File > Append, the appended project in its own folder, its recalculation links dropped with a note) |
| Print, Print Preview, Page Setup | A | brief T27 |
| 3D graphs | C | last (owner's request) |
