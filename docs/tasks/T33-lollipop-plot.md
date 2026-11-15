# T33: Lollipop plot

**Status:** claimed 2026-11-15 (session "mac-multipdf", macOS, no Origin), branch `lollipop`.
**Size:** M. **Source:** `origin-2026-2027-gaps.md` (Graphing: Chord and lollipop plots, Origin 2021;
the lollipop half only; chord, and the rug plot of 2019b, are left).

## Goal

Plot > Basic 2D > Lollipop Plot draws two or more Y columns as Origin's lollipop: symbols joined by
drop lines that go from each plot to the next data plot, with Origin's "Next Data Plot" drop-line
target and "Y Value: Plus-Minus" drop-line colour available in Plot Details.

## Origin facts (docs, not measured)

- Plot > Basic 2D : Lollipop Plot, template `Lollipop.OTPU`, Origin 2021 "New Graph Types".
  Needs two or more Y columns (or a range spanning two); X from the associated X column, else
  row number / sampling interval. [W:origin-help/lollipop-plot]
- It is a variation of the vertical drop-line scatter with X and Y exchanged, drop lines set to
  "Next Data Plot": each plot's drop line goes to the next plot at the same X, one by one; the last
  Y connects back to the first. [W:origin-help/lollipop-plot]
- Plot Details > Drop Lines tab: "Drop To" has "Next Data Plot", selected by default for Lollipop
  in the Vertical group. Colour "Y Value: Plus-Minus" (only for Lollipop) colours drop lines by the
  sign of the Y value. Colour "Auto" uses the symbol colour. [W:origin-help/pd-dialog-dropline-tab]
- Unconfirmed (no Origin here): default symbol, sizes and colours, stem width, whether the last-to-
  first connection is drawn for two columns, horizontal layout details, the Plus-Minus colours.
  Sheaf's choices go in graphing.md 18.7 labelled as such; measure on Windows with
  `tools/origin-ref/`.

## Where

- `crates/core/src/graph.rs` (`DropLine`, `DropLines`): a drop target (Axis, as now, or Next Data
  Plot) and a colour mode (Plus-Minus); serde defaults keep old files.
- `crates/core/src/template.rs`: `Template::Lollipop` (scatter, exchanged X-Y, vertical drop lines
  to the next data plot); `crates/plot/src/plots.rs` drop-line drawing.
- `crates/app/src/menus.rs` (Plot > Basic 2D), `crates/app/src/dialogs/graph_details.rs` (Drop
  Lines tab: Drop To, colour list) only for the drop-lines controls.
- `crates/ops` / CLI `--template lollipop`, MCP template list; `.opj` save warning for what the
  writer cannot store.
- `docs/spec/graphing.md` 18.7 (new).

## Done when

The menu item, CLI and MCP make a lollipop graph from two or more Y columns; drop lines to the next
plot and the Plus-Minus colour work and save in `.sheaf`; tests cover drawing geometry, the menu,
the Plot Details controls and loading old files; graphing.md 18.7 records Origin's facts and
Sheaf's choices.

## Don't

- Don't touch fill-under/sidelines code (`codex/t13-fill-sidelines`) or axis break drawing
  (`axis-breaks`).
- Don't change drop lines of existing graphs (defaults must stay Axis / Auto).
