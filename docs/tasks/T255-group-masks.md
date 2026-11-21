# T255: Masked group, censor and state cells

**Status:** done 2026-11-20 (Claude Windows "claude:3"). Bug fix, T254 sibling. Masked or
filter-hidden cells of Kaplan-Meier's censor and group columns, ROC's state column and the group
columns of the grouped summary statistics now leave their row out, as a masked data value already
did (`statistics::cell_left_out`). In the summary statistics a row whose group is masked belongs to no
group (as a blank group cell), so it is not counted as Missing in any group.
**Size:** S. **Source:** a check of every analysis tool's column reads after T253/T254.

## Tests

- `crates/core/src/analysis/survival.rs`: `masked_group_and_censor_cells_leave_the_case_out`.
- `crates/core/src/analysis/roc.rs`: `a_masked_state_cell_leaves_the_case_out`.
- `crates/core/src/analysis/summary.rs`: `a_masked_group_cell_leaves_the_row_out`.

## Left

- The second inputs of the signal tools (curve math reference, convolution response, deconvolution,
  coherence, wavelet detail and scales, IFFT, column math) read raw numbers; whether masks belong there
  (a transform needs every sample) is open.
