# T322: Measure Graph > Add Function Graph against Origin 2016

**Status:** done 2026-11-21 (Claude Windows "claude:fe4b", graphing 41). In Origin 2016, Graph > Add Function Graph...
adds a function plot F1 and opens Plot Details on its Function tab. The tab has Points 100, From/To greyed under
Auto (the X axis range), an empty Y(x), Before Formula Script, Plot Type (Line, Scatter, Line + Symbol, Column / Bar)
and a Workbook button. Sheaf's window now starts over the layer's X axis range and has the Plot Type choice.
**Size:** S. **Follows:** T119, T213. **Spec:** graphing 41.

## Left

- A curve that keeps following the X axis (Origin's Auto); Before Formula Script; the function as its own dataset.
- Reading back what Origin's OK draws (the collapsible Graph menu moved the item between runs).

## Tests

- `crates/app/src/app/ui_tests/function_graph.rs`: `the_function_curve_goes_into_the_active_layer` (range from the
  axis, 100 points, the Plot Type).
