# T489: Browser Graph steps every plot of the layer

**Status:** done 2026-11-26 (Claude Windows "claude:3", graphing "Browser Graph"). Left item of T410/T456 (claude:work's,
unclaimed); Origin 2020b's Browser Graph, Sheaf's own design.
**Size:** S. **Source:** origin-2026-2027-gaps.md, "Browser Graph": several browsed plots remain.

## What it does

Graph > Browser Graph... has Browse all plots of the layer (by sheet or by book): every plot of the layer steps with the browsed one to
its own column of the sheet or book shown. Plots whose lists do not line up are left out.

## Tests

- `crates/core/src/browser.rs`: `all_plots_of_a_layer_step_through_the_sheets_together`.
