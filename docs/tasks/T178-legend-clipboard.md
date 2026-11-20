# T178: Legend Cut/Copy, plot references in free text labels (B2 slice)

**Status:** done 2026-11-19 (Claude Windows "obj-align"), **not measured**.
**Size:** S. **Source:** greyed inventory package B2 ("Graph legend: Cut · Copy"), which waited on
`\l(n)` in free labels (T152 note).
**Spec:** `docs/spec/graphing.md` ("Legend order").

## Done

- `crates/plot/src/layout.rs`: a free text label whose text has `\l(n)` or `%(n)` (`legend::has_plot_refs`)
  is drawn by `draw_legend`. It uses the label's font and anchor and the layer's plots, with no frame
  or fill. Labels that are turned or have no plots keep the plain text path.
- The legend's right-click Cut/Copy and the graph Edit menu Cut/Copy with the legend selected
  (`Cmd::CopyLegend`, `Action::CopyLegend`, `shape_clip::copy_legend`). Cut hides the legend.
- Tests: `legend_tests::a_free_label_draws_plot_symbols_and_names_like_the_legend_without_its_frame`
  and `shape_clipboard::the_legend_is_copied_as_a_label_that_draws_its_symbols_and_cut_hides_it`.

## Left

- Measuring Origin's text-object spacing for `\l(n)`.
- Turned labels with plot references.
- Pasting into a layer whose plots differ: the references then name that layer's plots.
