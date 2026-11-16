# T58: Text formatting on graphs (Format toolbar, underline, colour, text window buttons)

**Status:** open. Written 2026-11-15 from the owner's priority "basic formatting (bolding, italics,
etc.)", parity row "Text objects and escapes" (P0, partial) and the 13 greyed Format toolbar items.
**Size:** L (four slices, each mergeable on its own). **Branch:** `text-format`. **Model:** Sonnet.
Slices 1 and 2 need no Origin; slice 3's toolbar details are measured on Windows.
**Spec:** `docs/spec/graphing.md` 7 (escapes), 16 (free text labels); `docs/spec/gui-look.md` 1
(toolbar rows), 4 (graph window).
**Greyed items closed (15):** Toolbars > Format: Font name box, Font size box, Bold, Italic,
Underline, Superscript, Subscript, Greek, Increase Font Size, Decrease Font Size, Font Color, Fill
Color, Line Color; Format > Axis Titles > X Axis Title..., Y Axis Title... [Graph].

## Goal

A person formats graph text without typing escape codes. Select an axis title, a text label or the
legend and click Bold, Italic, Underline, a font, a size or a colour on the Format toolbar: the whole
object changes, in one undo step. In the text window, select some characters and click Bold, Italic,
Underline, Superscript, Subscript or Greek: the selection is wrapped in the matching escape, and a
preview under the text box shows the result as the graph will draw it. Underline, strikethrough and
text colour are drawn on screen and in every export. Typical target: `j / mA cm\+(-2)`, `E vs.
Ag/AgCl / V`, `H\-(2)O`, `\g(m)A`, a bold panel letter.

## Today

- `crates/plot/src/markup.rs` parses `\b`, `\i`, `\+`, `\-`, `\g`, `\pN`, `\f:Name`, `\x`, `\v`,
  `\(nnn)`; the text of `\u(..)`, `\s(..)` and `\cN(..)` is kept but drawn without underline,
  strikethrough or colour. A `Run` has `text`, `bold`, `italic`, `scale`, `rise` only (no font
  family, colour or line flags: check what `\f:Name` really does).
- `crates/plot/src/text.rs` turns runs into `Item::Text` items; the backends that draw them are
  `svg.rs`, `eps.rs`, `emf.rs` and the raster/PDF paths that go through the scene (grep `Item::Text`).
- `Font` (`crates/core/src/graph.rs`) has `family`, `size_pt`, `bold`, `italic`, `color`; no
  underline.
- The Format toolbar (`Bar::Format` in `crates/app/src/toolbar.rs`) is inert: `Item::FontBox` and
  `Item::SizeBox` are painted boxes (`text_box`), the buttons are `g(...)`.
- The graph view has one selected object: `selected: Option<(usize, ObjectKind)>` in
  `crates/app/src/graph_view.rs`; `ObjectKind` (`crates/plot/src/scene.rs`) has `XTitle`, `YTitle`,
  `Legend`, `Label(i)`, `XAxis`, `YAxis`, `Shape(i)`, `Plot(i)`.
- The text window (`crates/app/src/dialogs/text_object.rs`) edits a label's text, size, bold, italic
  and rotation. Axis titles and the legend are edited in `dialogs/graph_props.rs`.

## Origin facts

- Measured (gui-look.md 1 and 4): the Format toolbar's font box reads "Default: Arial"; with the
  legend selected the size box shows its size (22), with nothing selected it shows "0". The
  toolbar's buttons are font, size, bold, italic, underline, superscript, subscript,
  super-subscript, Greek, bigger, smaller (ui-command-map.md, toolbar table).
- Measured (ui-command-map.md, shortcut table): while a text object is edited in place, Ctrl+B, I,
  U, G, +, =, -, <, > give bold, italic, underline, Greek, superscript, subscript, super-subscript,
  font up and font down.
- Escapes: graphing.md 7 (from Origin's documentation).
- **Not measured, measure on Windows before slice 3 is called "as Origin":** what each toolbar
  button does to a selected (not edited) text object: does it set the object's font style or wrap
  the whole text in an escape; which objects the toolbar acts on (tick labels through a selected
  axis? data labels?); the steps of Increase/Decrease Font Size; the Font Color, Fill Color and Line
  Color buttons' palettes and what they change on a plot, a shape and a text object; the layout of
  Origin's text Object Properties window (button row and preview).
  Until measured, Sheaf's choices (label them in the spec): a selected object's own font changes
  (`Font.bold` and so on, not escapes); Increase/Decrease move the size along Origin's font-size
  list if the size box's list is known, else by 2 pt; Fill Color and Line Color act on a selected
  plot or drawn shape.

## Where

- `crates/plot/src/markup.rs`: add `underline`, `strike` and `color: Option<..>` to `Run`; parse
  `\u`, `\s`, `\cN` (palette position N; find Origin's base palette in the spec or measure it) and
  `\=(sub,sup)`. Keep `plain()` and the nesting limit.
- `crates/plot/src/text.rs` and `scene.rs`: carry the new run fields on `TextItem` (or emit line
  items beside the text); draw them in every backend, rotated with the text. Underline position and
  thickness: measure Origin's PNG at two sizes, else font metrics.
- `crates/core/src/graph.rs`: `Font.underline` (serde default, not written when false).
- `crates/app/src/toolbar.rs`: live Format bar. A small `FormatTarget` in the app says what the
  toolbar acts on: the selected graph object, the text window's selection, or (T59) worksheet
  cells. The boxes show the target's font and size and become combo boxes (font list from
  `crates/plot/src/fonts.rs`).
- `crates/app/src/dialogs/text_object.rs`: a button row (B, I, U, x², x₂, αβ, colour) that wraps
  the selected characters, the same shortcuts as Origin's in-place edit, and a preview drawn by
  `sheaf_plot`. Reuse the same editor part for axis titles and legend text in `graph_props.rs`.
- `crates/app/src/menus.rs`: Format > Axis Titles > X/Y Axis Title... open that title's text
  editor.
- `crates/io/src/opj`: label text keeps its escapes already; write `Font.underline` if the format
  has a place for it, else warn on save (ADR 0008).

## Steps

1. **Escapes drawn (no UI).** `\u`, `\s`, `\cN`, `\=`: parser tests, golden images for SVG and PNG,
   one EPS/EMF/PDF check each; legend and tick labels use the same path, so check one of each.
   Spec 7: mark which escapes are drawn.
2. **Text window.** Button row, shortcuts, selection wrapping (wrapping a selection that is already
   inside `\b(..)` removes it when the selection is exactly that escape; otherwise nests), live
   preview. Use it for labels, axis titles and legend text. Enable Format > Axis Titles items. UI
   tests: select "2" in `cm-2`, click Superscript, text becomes `cm\+(-2)`; undo is one step.
3. **Format toolbar on a selected graph object.** `FormatTarget`, live boxes and buttons for
   titles, labels and legend; greyed (state, not "Not available") when nothing formattable is
   selected. Font Color, then Fill Color and Line Color on plots and shapes. UI tests per button;
   `--screenshot` checks of the toolbar states. Delete the 13 toolbar items from greyed-items.md.
4. **Origin pass (Windows).** Measure the "not measured" list, record it in gui-look.md and
   graphing.md 7/16, change Sheaf where it differs.

## Done when

On a graph, a title can be made bold, italic, underlined, larger, another font and another colour
from the toolbar; a superscript, subscript or Greek letter can be made in the text window without
typing an escape; screen, PNG, SVG, PDF, EPS and EMF agree; `.sheaf` and `.opj` (with its companion)
round-trip the result; fmt, strict Clippy and workspace tests pass; the 15 items are gone from
greyed-items.md.

## Don't

- Don't build in-place editing on the graph page (typing into the graph) here; the text window is
  the editor. Note it as a leftover.
- Don't change how existing escapes are drawn: the goldens of the built-in templates must not move
  except where an underline or colour newly appears.
- Don't make the toolbar act on worksheet cells here (T59 adds that target).
- Don't copy Origin's icons; the toolbar icons are Sheaf's own drawings (`Icon` in `toolbar.rs`).
