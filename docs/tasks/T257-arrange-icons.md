# T257: Window > Arrange Icons

**Status:** done 2026-11-20 (Claude Windows "claude:3"), not measured. Window > Arrange Icons (greyed
item `B12`) is enabled: the bars of the minimised windows, which stand in a row along the bottom of the
workspace in stacking order, are lined up again in window order (the order the windows were made);
open windows keep their places and stacking order; the status line says how many were arranged (or
that none is minimised). This is the Windows MDI meaning of the command; Origin's own order of the
icons was not measured.
**Size:** S. **Source:** greyed inventory `B12`; feature-parity "Window menu".

## Tests

- `crates/app/src/workspace.rs`: `arrange_icons_lines_the_minimised_windows_up_in_window_order`.
