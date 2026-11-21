# T293: Underline for a whole font

**Status:** done 2026-11-21 (Claude Windows "claude:3"). feature-parity "Fonts" (P0) gap "no
underline". `Font::underline` beside bold and italic: every run of the text is underlined (titles,
tick labels, legend, labels; markup underline is not doubled), the font boxes of the graph windows
(`font_ui`: axis titles and tick labels, legend) get an Underline tick, and the `.opj` writer writes an
underlined title or label as `\u(..)`, as it writes bold as `\b(..)`. Not measured (Origin's own
Underline tick sits in the same places).
**Size:** S. **Spec:** feature-parity "Fonts".

## Tests

- `crates/plot/src/text_decoration_tests.rs`: `an_underlined_font_underlines_every_run`.
- `crates/io/src/opj/write/graph_tests.rs`: `an_underlined_title_is_written_with_origins_underline_markup`.

## Left

- The default-font setting of the parity row.
