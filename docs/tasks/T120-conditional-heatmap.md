# T120: Worksheet > Conditional Formatting > Conditional Heatmap...

**Status:** done 2026-11-17 (Claude Windows, scheduled session "cond-heatmap", branch `cond-heatmap`; gui-look.md "Conditional Heatmap (T120)"), not measured (Origin 2016 has no conditional formatting). Left: Introducing Other Colors in Mixing and Load Palette (disabled in the window), editing a rule after creation (the Manager, B7).
**Size:** M. **Source:** greyed inventory package B7 (T35 leftover).
**Spec:** `docs/spec/gui-look.md` "Conditional formatting" (a new subsection).

## Goal

Conditional Heatmap colours each numeric cell of the selected range by its value: the range from a
low to a high level is cut into levels and each level gets a colour mixed between chosen colours.
Live as values change, saved with the sheet, removed by Remove Conditional Format, like T35's rules.

## Origin facts (docs, not measured: Origin 2016 has no Conditional Formatting)

From docs.originlab.com/origin-help/conditional-format, Conditional Heatmap dialog:

- Level: **Auto Rescale** (checked by default: the range's minimum and maximum), else **From** and
  **To**; **Find Min/Max**; scale **Type**; levels **By Increment** or **By Count** with a value;
  **First Level**.
- Colors: **Format Type** Fill Background or Color Text; **Contrast Color for Text**; **Color
  Generation** Limited Mixing (From to To), 3-Color Limited Mixing (From, Middle, To; Middle
  Position By Percentile, By Percent or By Value), Introducing Other Colors in Mixing, Load Palette;
  **Flip**; **Below** and **Above** colours for values outside From..To.
- A new rule goes to the top of the Manager list; the upper rule wins where rules overlap.

Not documented: default colours, default level count, how a level's colour is chosen, ties at level
bounds, missing values. Sheaf's choices are labelled unmeasured in the spec.

## Steps

1. Core: `Rule::Heatmap` in `crates/core/src/condformat.rs`, evaluation over the whole range,
   discrete levels as palette classes; tests.
2. App: the dialog (`crates/app/src/dialogs/cond_format.rs`), the menu item enabled, undo; a UI test.
3. Files: `.sheaf` saves it (serde); `.opj` warns as for the other rules; XLSX snapshot colours.
4. Spec subsection, greyed inventory recount, index.

## Done when

The item opens the dialog, the cells are coloured live, survive save/reload, are removed by Remove
Conditional Format; Introducing Other Colors and Load Palette stay disabled until palettes exist
for worksheets. fmt, strict Clippy and the full tests pass.
