# T213: File > New > Function Plot (B14 slice)

**Status:** done 2026-11-20 (Claude Windows "obj-align"), **not measured**.
**Size:** S. **Source:** greyed inventory package B14 ("New: Function Plot").
**Spec:** `docs/spec/graphing.md` 41.

## Done

- The T119 function window opened with no graph. OK makes the graph and the curve in one undo step,
  and Cancel makes nothing (`SheafApp::function_graph_new`).
- Test: `function_graph::file_new_function_plot_makes_its_graph_on_ok_in_one_undo_step`.

## Left

- Origin's 3D function plots and parametric functions.
