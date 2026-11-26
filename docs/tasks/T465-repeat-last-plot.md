# T465: Plot > Repeat Last Plot (Origin 2026, repeat the last plotting action)

**Status:** done 2026-11-25 (Claude Windows "claude:work", recent-lists.md "Repeat Last Plot"). Plot > Repeat Last
Plot runs the most recent Plot-menu command (the first entry of the Plot menu's Most Recently Used list, T78) on the
current selection. It is greyed until a graph has been made from the Plot menu, and it is not itself recorded.
**Size:** S. **Source:** origin-2026-2027-gaps.md, "Repeat Last Action and Duplicate Operation" (plotting actions).

## Tests

- `crates/app/src/app/ui_tests/recent_lists.rs`: `repeat_last_plot_makes_the_last_plot_again_on_the_new_selection`.
