# Figure rendering

**Summary.** A graph in the document model (`sheaf-core::graph`) says what to draw and in which
style. The `sheaf-plot` crate turns a graph plus the project's column data into a *scene*: a flat
list of lines, polygons and positioned text in page coordinates. One scene is written out as SVG.
PNG is made by rasterising that SVG, PDF by converting that SVG, and the application window shows
the same rasterised image. Because every output comes from the same scene, a figure looks the same
on screen and in every export. This document fixes the layout rules, the tick algorithms and the
SVG structure.

## 1. Coordinates and units

- Page coordinates are in points (1 pt = 1/72 inch), origin at the top-left corner, y pointing down.
- The SVG has `width`/`height` in `pt` and a `viewBox` of `0 0 <page width> <page height>`, so one
  user unit is one point. All style sizes of the model (line widths, font sizes, tick lengths,
  symbol sizes) are already in points and are used unchanged.
- PNG at `dpi` dots per inch has `ceil(page_width_pt * dpi / 72)` by `ceil(page_height_pt * dpi /
  72)` pixels. The DPI is recorded in the PNG `pHYs` chunk.
- TIFF is the same picture as PNG, laid on white (no alpha channel, since many programs that take
  TIFF figures mishandle one), stored as 8-bit RGB with LZW compression and the DPI in the
  `XResolution`/`YResolution` tags (unit inch).
- **Export size.** By default a figure has the size of the page. An export width and/or height
  (`ExportSize`; CLI `--image-width`, `--image-height` with a unit in, cm, mm, pt or px; the app's
  export window "Scale to" a width in in or cm) scales the whole page, text and lines included, as
  Origin's export image size does with its aspect ratio kept. With both, the page is scaled to fit
  inside both. Only the root `width`/`height` of the SVG change; the `viewBox` keeps the drawing in
  page points, so SVG, PDF, PNG and TIFF all follow. A scale below 0.01 or above 100 is refused.
  (The CLI's `--width` and `--height` are different: they change the page itself and keep text sizes.)
- The layer frame is the rectangle `left, top, width, height` (fractions of the page) converted to
  points. Data is clipped to the frame.

## 2. Resolving an axis (limits and ticks)

Input: the axis settings and the finite data values of all visible plots of the layer for that
axis (for a plot without an X column, the X values are the row numbers 1, 2, 3, ...). Rows where
X or Y is missing are ignored. With no finite data the range is 0..1. In a layer that stacks its
curves the values are taken as drawn, with each plot's offsets added (graphing.md section 13.5).

Three cases change the input (graphing.md sections 14.3 and 14.4):
- An X axis whose first plot has text in its X column (sample names), or no X column and labels
  from a chosen column, is a category axis: limits half a row beyond the first and last drawn row,
  never rounded, and a major step of 1 while the axis spans at most 200.
- An X axis over column plots gets half a bar spacing of room at each end and is not rounded.
- A linear Y axis over column plots includes their base (0, or the plot's offset).

Step 1, data range `[lo, hi]`. If `lo == hi`, widen: `lo -= 0.5*|lo|`, `hi += 0.5*|hi|`, or
`-1..1` when the value is 0. For a log10 axis only positive values count; with none, use 1..10.

Step 2, limits, by `auto_range` (a fixed `min` or `max` in the model overrides the computed one
afterwards):

- **Nice** (linear). `step = nice_step((hi - lo) / 5)` using the multipliers 1, 2, 5, 10 (smallest
  multiplier times a power of ten that is >= the raw value). Limits are `floor(lo/step)*step` and
  `ceil(hi/step)*step` (with a relative tolerance of 1e-9 so that values already on a tick stay
  there). When the user fixed `major_step`, that step is used for rounding instead.
- **Tight**. Limits are exactly `lo` and `hi`. Step as in Nice, computed from `hi - lo`.
- **SymmetricZero**. Let `m = max(|lo|, |hi|)`. `step = step15(m * 1.05 / 3)` where `step15` rounds
  up to the multipliers 1, 1.5, 2, 2.5, 3, 4, 5, 10 times a power of ten. `n = ceil(m * 1.05 / step
  - 1e-9)`. Limits are `-n*step` and `+n*step`. Check values: m = 33.55 gives ±45 with step 15;
  10.5 gives ±12 step 4; 7.632 gives ±9 step 3; 7.0 gives ±7.5 step 2.5; 0.9 gives ±1.2 step 0.4;
  5.041 gives ±6 step 2. When the axis has a fixed `major_step`, `n = ceil(m*1.05/major_step)`.
  If only one of `min`/`max` is fixed by the user, the other is its negative.
- **Log10 axes** ignore `auto_range`: limits are the enclosing powers of ten; major ticks at every
  power of ten (every k-th power when there are more than 8 decades, so that at most about 8 labels
  show); minor ticks at 2..9 times each power when `minor_ticks > 0`.

Step 3, major ticks (linear). `step` is the model's `major_step` if set, else the one from step 2
(for fixed limits: `nice_step((max - min) / 5)`). Ticks are at `k*step` for all integers k with
`min - eps <= k*step <= max + eps`, `eps = 1e-9 * (max - min)`. Tick values are cleaned of
floating-point noise by rounding to 12 significant digits (so 0.30000000000000004 prints as 0.3,
and a value within `eps` of 0 is exactly 0). If this would give more than 200 ticks the step is
doubled until it does not.

Step 4, minor ticks. `minor_ticks` evenly spaced ticks between each pair of neighbouring major
ticks, also continued before the first and after the last major tick while inside the limits.

Step 5, tick label text. The shown number is `value / tick_label_divisor`.
- `Auto`: the number of decimals is the smallest d in 0..=10 such that `step/divisor`, rounded to d
  decimals, equals itself within 1e-9 relative; all labels use that d. If the largest absolute
  label value is >= 1e6 or the step is < 1e-4, labels use scientific form `m×10^e`, written with a
  real multiplication sign and a superscript exponent, mantissa trimmed of trailing zeros.
- `Decimal(d)`: fixed d decimals. `Scientific(d)`: `m×10^e` with d mantissa decimals.
- Negative numbers use the real minus sign U+2212. Negative zero prints as `0`.
- Log axes label their powers of ten as Origin does (checked on two graphs exported from
  Origin): with the `Auto` format as plain numbers from `0.001` to `1000000`, and as `1E7`,
  `1E−4` beyond; with `Scientific(d)` as `10` with a superscript exponent.
- An axis with a label column (sample names) shows the cell of row k at a major tick at the whole
  number k, and nothing at other ticks (graphing.md section 14.3).

## 3. Layout

For each layer, in this drawing order (later items are on top). A layer linked to another layer
(the right Y axis of a double-Y graph) is laid out together with it: it shares its frame and,
when linked, its X axis, and the rules for axes on the top and right edges, hidden axes, axis
colours that follow the first curve and a legend for the whole graph are in graphing.md section 12.
A layer drawn as a waterfall lays its axes, labels and legend out on a smaller front frame, clips
data to the whole frame, draws each later curve moved right and up, and draws no grid and no
opposite lines or ticks (graphing.md section 13.6).

1. Page background rectangle (once per graph).
2. Grid lines, if set: minor first, then major; spanning the frame at each tick position.
3. Data plots in plot order, clipped to the frame (clip rectangle = the frame). For each visible
   plot: the line (if the kind has one), then error bars, then symbols.
4. Axis lines: bottom X and left Y always; top and right when `show_opposite` is set. With all four
   drawn they form a closed box; draw the box as one closed rectangle path when both axes have the
   same line width and color and both show their opposite side, so corners join cleanly. Line caps
   are square otherwise.
5. Ticks. Position: on the axis line. `In` points into the frame, `Out` away from it, `Both` both
   ways, `None` draws nothing. Length from the model. Opposite-side ticks are drawn when
   `opposite_ticks` is set, mirrored (an `In` tick still points into the frame).
6. Tick labels. X labels are centred under their tick; the top of the text is `gap` below the
   bottom end of the ticks, where `gap = 0.35 * tick font size` and the tick end is the axis line for
   `In`/`None` ticks or the outer tick end for `Out`/`Both`. Y labels are right-aligned, vertically
   centred on the tick, the same `gap` left of the left end of the ticks. X labels turned by
   `tick_label_angle_deg` hang from the highest corner of their text box, which sits directly
   under the tick; the label block is as deep as the deepest turned label (graphing.md
   section 14.5).
7. Axis titles. X title: centred under the frame, its top `0.6 * title font size` below the bottom
   of the tick labels. Y title: rotated 90° counter-clockwise, centred along the frame's height,
   its bottom edge (the side facing the axis) `0.6 * title font size` left of the widest Y tick
   label. A title that is `None` in the model is built from the first visible plot: the X (or Y)
   column's long name (short name if empty), followed by ` (unit)` when the unit is not empty; an
   X column of text counts too. When the plot has no X column the X title is that of the column
   that labels the ticks, if any, else `Row`.
8. Legend, when visible and the layer has at least one visible plot. The legend text (`Legend::text`,
   or the automatic text with one `\l(n) %(n)` line per visible plot) gives the rows: a line is a
   row, `\l(n)` or `\L(n)` puts the sample of plot n in the row, `%(n)` the plot's label, any other
   text is drawn as it is with the label markup; `n` counts the plots the legend lists, or is
   `l.p` (layer, plot) when it lists several layers; a space right after a sample is replaced by
   the gap, and an unknown plot number draws nothing. With `columns` > 1 the rows are filled
   column by column (rows per column = rows / columns, rounded up), each column as wide as its
   widest row, columns 1 × font size apart. One row, per visible plot: a
   sample (a horizontal line of length 2.2 × font size in the plot's line style, with one symbol
   centred on it for kinds that have symbols; symbol only for scatter; for a column plot a bar as
   long as the line sample and one font size high, filled and outlined like the bars), a gap of
   0.5 × font size, then the label text (see `Project::plot_label`). Row height 1.3 × font size. Padding inside the
   legend box 0.4 × font size. Corner positions put the box inside the frame with a margin of
   0.8 × font size from the two nearest frame edges (plus the major tick length when ticks point
   in). `Manual {x, y}` centres the box at that fraction of the frame. A frame rectangle and a
   background fill are drawn when set.
9. Free text labels.

### Keeping labels on the page

The frame in the model is a fixed fraction of the page, but tick labels and titles are placed
relative to the frame, so wide Y tick labels (for example `-4.67×10^5`) or large fonts could push the
Y title past the left page edge. Rule, applied to every layer before drawing: lay the labels out
for the nominal frame and find the left edge of the Y title (or of the widest Y tick label when
the title is hidden) and the bottom edge of the X title (or of the X tick labels when the title is
hidden). Keep a margin of 3 pt from the page edge. If the left edge is smaller than the margin, move
the frame's left side right by the difference (the right side stays); if the bottom edge is larger
than page height minus the margin, move the frame's bottom side up by the difference (the top stays).
A move never takes more than half of the frame's width or height. When everything already fits, the
frame is exactly the nominal one, so figures that fit are unchanged. Because labels move with the
frame, one move is exact. The same geometry is used by the screen view (`layout_geometry`).
Axes drawn on the right or top edge move the right or top side in the same way, and a layer with
linked layers takes the outermost labels of all of them into account (graphing.md section 12.3).
When linked layers lie outside the first layer's frame (the panels of a stack), the moves apply to
the rectangle that holds all their frames, and every frame is moved and scaled with it, so the
panels stay aligned (graphing.md section 15.5). An axis with `hide_end_label` leaves out the tick
label at its far end (top of a Y axis, right end of an X axis), and touching panels with the same
axis title draw it once, centred on them.

### Reversed axes

When both limits of an axis are fixed and the lower limit (`min`) is greater than the upper
(`max`), the axis is reversed, as in Origin where From greater than To makes values decrease along
the axis. Ticks, labels and grid are those of the range `max..min`; the first axis end is
at the larger value, and data, ticks and the inverse mapping all follow the reversed direction.
`ResolvedAxis::reversed` is true; `min` and `max` of the resolved axis stay ordered. Reversal
needs both limits: with only one fixed limit on the wrong side of the data, the old rule applies (the
other limit becomes the fixed one plus 1). Equal limits are rejected by the command-line program.
Previously reversed limits silently drew a one-unit-wide axis, which is why this is explicit.

Reversal also works on a log axis and with the symmetric-about-zero range (both limits fixed): the
axis is resolved as if the limits were in order and then flipped. Two finite limits whose span
overflows (for example 1e308 and -1e308) cannot be drawn; the renderer ignores both limits and uses
the automatic range, and the command-line program refuses them. The command-line program also
accepts a page width or height only from 0.1 to 100 inches. A single fixed limit that leaves the
data outside the axis is drawn as asked (as Origin does); no warning is given.

Text metrics: width from the font's advance widths; ascent 0.72 × size and descent 0.21 × size
are used for vertical placement regardless of the font, so that layout does not depend on which
font file is installed. The SVG `y` of a text element is its baseline.

## 4. Drawing data

- Convert each (x, y) row to page coordinates with the resolved axes (linear or log10). Rows with a
  missing or non-finite coordinate (or a non-positive value on a log axis) break the line: the
  curve is drawn as separate pieces on each side of the gap. In a layer that stacks its curves,
  the row is drawn at (x + ox, y + oy) with the plot's offsets, and column bars start at oy
  instead of 0 (graphing.md section 13.5).
- `Straight`: one polyline per piece. `StepHorizontal` / `StepVertical`: add the intermediate
  corner. `Spline`: a Catmull-Rom spline through the points (uniform parameterisation, tension 0.5),
  written as cubic Bézier segments: for points P0..P3 the segment P1→P2 has control points
  `P1 + (P2 - P0)/6` and `P2 - (P3 - P1)/6`; the end points are duplicated for the first and last
  segment. When a piece has more than 2000 points the spline is skipped and a polyline is drawn
  (at that density the two are indistinguishable).
- Decimation for speed: consecutive points that fall within 0.05 pt of the last kept point are
  dropped (the first and last point of a piece are always kept). This is invisible at any export
  resolution up to 1400 dpi.
- Line joins are round; caps are butt for dashed lines and round for solid data lines. Dash
  patterns are the model's multiples times the line width.
- Symbols are centred on the data point, `size_pt` wide: square, circle, triangle up/down, diamond
  (all filled or outlined with `edge_width_pt`), plus, cross and star (stroked).
- Error bars: a vertical line from `y - err` to `y + err` with caps of width `1.2 × symbol size`,
  in the plot's color, line width `max(0.5, line width / 2)`. A column plot's error bars stand in
  the middle of its own bar.
- Column plots: one rectangle per row from the base (0 on a linear Y axis, the lower limit on a log
  axis) to the value. The bars at one X position take `1 - gap` of the X spacing s together, and
  the visible column plots of a layer stand side by side in plot order (graphing.md section 14.4,
  with check values). s is 1 for row numbers and the mean X spacing otherwise.

## 5. Text markup

Label text may contain a small markup, compatible with the most used escapes of Origin text
labels: `\b(text)` bold, `\i(text)` italic, `\+(text)` superscript, `\-(text)` subscript,
`\g(text)` Greek (each Latin letter maps to the Greek letter of the Symbol-font layout, e.g. `m` →
μ, `D` → Δ). Also understood: `\pN(text)` at N percent of the surrounding size; `\f:Name(text)`, a
font switch, where the font is not changed (the label keeps its own font) but Greek mapping is on
inside the text only when the name is `Symbol` (so `\g(q \f:Arial((deg)))` gives "θ (deg)");
`\x(hhhh)` the character with that hexadecimal code (not a control character); `\v(text)` the text
literally, without escapes. `\u(text)`, `\s(text)` and `\cN(text)` keep their text but underline,
strikethrough and color are not drawn. Escapes nest. Superscripts and subscripts are drawn at 0.65
× size, raised by 0.35 × size or lowered by 0.15 × size. A backslash followed by anything else is
literal. Each run is
written as its own SVG `<text>` element at a computed position, so no renderer-specific text
layout is involved.

## 6. SVG structure

`<svg>` root with `xmlns`, size in pt and viewBox; a `<rect>` background; per layer one `<g>` with
the grid, a `<g clip-path>` with the plots (one `<path>` per piece, `fill="none"`), then axes,
ticks, labels and legend. Numbers are written with at most 3 decimals and no trailing zeros. Text
uses `font-family` (the model's family first, then the fallbacks `Liberation Sans`, `Helvetica`,
`DejaVu Sans`, `sans-serif`), `font-size`, optional `font-weight="bold"` / `font-style="italic"`,
`text-anchor`, and a `transform="rotate(...)"` for rotated text. Text is XML-escaped. The output is
deterministic: the same graph and data always give byte-identical SVG.

### Other vector formats

EPS and EMF are written from the scene, not from the SVG (`eps.rs`, `emf.rs`; the path data is
read back by `vector.rs`, which turns arcs into cubic curves). Both are vector and neither has
transparency: a colour with alpha is blended onto the page background.

- **EPS** is level 2. `%%BoundingBox` is the page size in points rounded up and
  `%%HiResBoundingBox` the exact size; an export size is a `scale` operator. Layers are clipped
  with `rectclip`. Text is set in Helvetica, Times or Courier (serif families go to Times,
  monospaced ones to Courier, the rest to Helvetica) with the bold and oblique variants, which
  every PostScript device has, so nothing is embedded. The fonts are re-encoded to ISO Latin-1
  (`ISOLatin1Encoding`, with the apostrophe, the grave accent and the hyphen put back to their
  ASCII meaning); Greek letters and a few signs (minus, infinity, comparison signs, arrows) are
  set in `Symbol`; a few typographic characters are replaced by a look-alike and any other
  character by `?`, and `write_eps` returns a warning for each kind. Centred and right-aligned
  text is placed with `stringwidth`, so its position follows the printer's font.
- **EMF** follows the public format description. One logical unit is 0.01 mm (the reference
  device is 10000 x 10000 units of 0.01 mm in 100 x 100 mm), the header's `rclFrame` is the page
  size in 0.01 mm and `rclBounds` the same in device units. Shapes are `EMR_BEGINPATH` ...
  `EMR_STROKEPATH` / `EMR_FILLPATH` / `EMR_STROKEANDFILLPATH` with `EMR_POLYLINETO` and
  `EMR_POLYBEZIERTO`, rectangles `EMR_RECTANGLE`; pens are geometric (`EMR_EXTCREATEPEN`) with
  the cap, join and user dashes, brushes solid; text is `EMR_EXTTEXTOUTW` in the graph's own
  family (`EMR_EXTCREATEFONTINDIRECTW`, negative height, escapement in tenths of a degree)
  with a baseline alignment and the advance of every character from the layout's measurer, so
  that the text keeps the layout's widths whatever font the viewer substitutes. Clipping per
  layer is `EMR_SAVEDC`, `EMR_INTERSECTCLIPRECT`, `EMR_RESTOREDC`. A file made this way was
  loaded and drawn by GDI+ (title, rotated text, clipping and dashes as in the PNG).

## 7. Fonts

The renderer loads the system fonts once. Text width is measured with the first family in the
fallback list that is installed. For tests, a fixed-metrics measurer (every character 0.55 × size
wide) can be chosen instead so that expected outputs do not depend on installed fonts.
