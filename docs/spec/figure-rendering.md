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
- The layer frame is the rectangle `left, top, width, height` (fractions of the page) converted to
  points. Data is clipped to the frame.

## 2. Resolving an axis (limits and ticks)

Input: the axis settings and the finite data values of all visible plots of the layer for that
axis (for a plot without an X column, the X values are the row numbers 1, 2, 3, ...). Rows where
X or Y is missing are ignored. With no finite data the range is 0..1.

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
- Log axes label powers of ten as `10` with a superscript exponent.

## 3. Layout

For each layer, in this drawing order (later items are on top). A layer linked to another layer
(the right Y axis of a double-Y graph) is laid out together with it: it shares its frame and,
when linked, its X axis, and the rules for axes on the top and right edges, hidden axes, axis
colours that follow the first curve and a legend for the whole graph are in graphing.md section 12.

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
   centred on the tick, the same `gap` left of the left end of the ticks.
7. Axis titles. X title: centred under the frame, its top `0.6 * title font size` below the bottom
   of the tick labels. Y title: rotated 90° counter-clockwise, centred along the frame's height,
   its bottom edge (the side facing the axis) `0.6 * title font size` left of the widest Y tick
   label. A title that is `None` in the model is built from the first visible plot: the X (or Y)
   column's long name (short name if empty), followed by ` (unit)` when the unit is not empty.
   When the plot has no X column the X title is `Row`.
8. Legend, when visible and the layer has at least one visible plot. One row per visible plot: a
   sample (a horizontal line of length 2.2 × font size in the plot's line style, with one symbol
   centred on it for kinds that have symbols; symbol only for scatter), a gap of 0.5 × font size,
   then the label text (see `Project::plot_label`). Row height 1.3 × font size. Padding inside the
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
  curve is drawn as separate pieces on each side of the gap.
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
  in the plot's color, line width `max(0.5, line width / 2)`.

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

## 7. Fonts

The renderer loads the system fonts once. Text width is measured with the first family in the
fallback list that is installed. For tests, a fixed-metrics measurer (every character 0.55 × size
wide) can be chosen instead so that expected outputs do not depend on installed fonts.
