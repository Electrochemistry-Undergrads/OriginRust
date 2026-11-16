# T63: Greyed items whose feature already exists (second round of T01)

**Status:** open; groups can be claimed one by one (name the group letters in the claim). Written
2026-11-15 from a read of the greyed list against the code; **every group must be checked in the
code before it is wired, the notes below are leads, not facts about the running app.**
**Size:** M in all, each group S. **Branch:** `greyed-2-<group>`. **Model:** Sonnet. No Origin
needed to wire; where Origin's behaviour is unknown the group says so.
**Spec:** `docs/spec/ui-command-map.md` (menu order), `docs/spec/gui-look.md`; T01 for the method.
**Greyed items closed (31).**

## Goal

Enable every greyed item whose work is already done by existing code, so the grey list only shows
what is really missing. Each item must do what Origin's item does; if the existing code does only
part of it, leave the item grey and move it in greyed-items.md to the package that will finish it.

## Groups

| | Items | Lead (verify first) |
|---|---|---|
| A | Toolbars > 2D Graphs: Bar, Stacked Area, Box Chart, Histogram, Bubble (5) | `Template::Bar`, `StackedArea`, `BoxChart`, `Histogram`, `Bubble` exist and are in the Plot menu; the buttons are `g(PlotBar, ...)` etc. in `toolbar.rs`. Use `b(..., Cmd::Plot(Template::..), ...)` as the live buttons beside them do. |
| B | Plot > Line: Horizontal Step, Vertical Step, Spline Connected (3) | `Connect::StepHorizontal`, `StepVertical`, `Spline` exist in `crates/core/src/graph.rs` and are drawn. Plot as Line with that connection (a `Template` each, so `sheaf plot -t` gets them too). |
| C | Plot > Statistics: Probability Plot, Q-Q Plot (2) | Statistics menu items run `AnalysisKind::ProbPlot` / `QQPlot`. Unknown: whether Origin's Plot-menu item opens the same window or plots at once with defaults; until measured open the same window. |
| D | File > Open Excel... (1) | The Excel import exists (T30). In Origin this item opens the file in an Excel window, which Sheaf does not have. Either open the Excel import window (label it Sheaf's choice in gui-look.md 7) or move the item to B14. Ask nothing; choose the import window unless the owner has said otherwise. |
| E | File > Export: Graph as Image..., Multi-Page PDF... with no window or a workbook active (2) | Export Graphs (T05) and multi-page PDF (T32) work with a graph active. Check Origin: if it greys these without a graph, make them state-greyed (not "Not available") and delete them from the list; else open Export Graphs with the project's graphs to choose from. |
| F | Toolbars > Standard: Duplicate; Explorer window menu: Duplicate (2) | `Cmd::DuplicateWindow` (Window > Duplicate). |
| G | Toolbars > Object Edit: Bring to Front, Send to Back (2) | T24's shape ordering (graph shape menu "Bring to the front / Push to the back"); live when a drawn shape is selected, state-greyed otherwise. Other object kinds are B2. |
| H | Column > Fill Column With and column header > Fill Column with: Row Numbers, Uniform Random Numbers, Normal Random Numbers (6) | `sheaf-expr` has the row index, `rnd` and `grnd` (`crates/expr/src/funcs.rs`). Fill the selected columns with values (not a stored formula; check Origin if possible), one undo step. |
| I | Format > Axis Tick Labels: X Axis Tick Labels..., Y Axis Tick Labels... (2) | Open the axis window (`crates/app/src/dialogs/graph_axis.rs`) on its Tick Labels tab for that axis. |
| J | Graph > Add Plot to Layer: Line..., Scatter..., Line + Symbol..., Column..., Area... (5) | Layer Contents and Plot Setup add plots to a layer. Unknown: Origin adds the worksheet selection as that plot type, or opens a window to choose data; until measured open Plot Setup with the plot type preselected. |
| K | Column header > Frequency Count... (1) | Statistics > Descriptive Statistics > Frequency Counts... (`AnalysisKind::Histogram`) or Discrete Frequency: check which one Origin's context item opens. |

## Steps (per group)

1. Verify the lead by reading the code and running the existing feature (`--screenshot`, a UI test).
2. Wire the item; give it the same enabled/disabled state rule as its twin in the menus.
3. Test: a UI test that the item is live and dispatches the same command as its twin (T27's
   toolbar Print test is the pattern), plus one that it does the work.
4. Delete the items from greyed-items.md, run `python tools/greyed-count.py`, update the counts.

## Done when

All eleven groups are wired or moved with a written reason; the checks pass.

## Don't

- Don't build new features here. More than about two hours for a group means it is not a wire-up:
  move it to its package.
- Don't enable an item that does less than Origin's.
- Don't reorder menus; positions follow the measured menus (ui-command-map.md).
