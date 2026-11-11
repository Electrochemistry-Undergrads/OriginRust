# Specification: 2D graphs, graph object model and graph export

Status: research draft, clean-room, written in our own words for a programmer who has never used Origin.

## Summary

- A graph is a **page** holding one or more **layers**. A layer is a rectangular frame with its own X and Y axes, data plots, text and drawn objects. A normal line plot has one layer (double-Y plots use two linked layers).
- Layer position and size are stored relative to the page (percent by default) or in absolute units. Text, line widths and arrowheads follow the layer size ("scale elements"), unless a fixed factor is chosen.
- Plots in one layer form a **plot group**: each member takes the next entry of increment lists (color, line style, symbol shape, symbol interior). We decoded the owner's default color list (section 1.5).
- An axis has a scale (from, to, type, rescale mode), major/minor ticks, tick labels, a title, grid lines, breaks and special ticks. Axis titles and legend text come from column metadata via the substitution notation `%(...)`.
- Text labels use a backslash escape language (`\b( )`, `\+( )`, `\g( )` ...), listed in section 7.
- Export is one function (`expGraph`) with a settings tree: margin mode, image size, DPI, per-format options. Raster default is 300 DPI; default image type is EPS.
- Themes (`.OTH`, XML) store partial formatting; templates (`.OTP`, binary) store a whole graph without data. The owner's install applies the system theme "Dark Colors & Light Grids" to new graphs.
- Public docs and installed text files do NOT state most numeric defaults (font size, line widths, tick lengths, page size). They are UNKNOWNS in section 11, to be measured in running Origin. Values tagged [REC] are unverified recollection.

Priority tags: **P0** owner's daily workflow (CH Instruments CV/chronoamperometry overlays, integration figures, publication export), **P1** common, **P2** later.
Source tags: **[F:path]** installed file read (paths under `C:\Program Files\OriginLab\Origin2016` unless "User:" = `OriginRust\OriginLab\2016\User Files`); **[W:page]** public page at docs.originlab.com/origin-help (or labtalk, quick-help, x-function); **[INF]** inference; **[REC]** recollection.

---

## 1. Object model (P0)

### 1.1 Hierarchy
```
Project
 └─ Graph page            short name, long name, size, resolution, background
     ├─ Layer 1..N        frame rect, scale-elements mode, links
     │   ├─ Axes          X bottom, X top, Y left, Y right
     │   ├─ Data plots    ordered; X col, Y col (+ error col), row range, type, style
     │   ├─ Plot groups   plots sharing increment rules
     │   ├─ Legend        a text object named "Legend"
     │   ├─ Text objects  axis titles, labels, layer title
     │   └─ Drawn objects lines, arrows, rectangles, circles, polygons
     └─ Page-level objects
```
The Plot Details dialog has the same three levels: page, layer, plot [W:pd-dialog-size-tab]. Theme files use the tree Root > Page > Layers > All Layers > Axes > All Axes > Ticks / Grids [F:Themes\Graph\*.OTH].

### 1.2 Page
| Property | Meaning | Default | Source |
|---|---|---|---|
| width, height | stored as printer dots; inches = dots / resolution | UNKNOWN U5 | [W:labtalk page object] |
| display unit | inch for US/UK locale, millimetre elsewhere | locale | [W:pd-dialog-printdim-tab] |
| keep aspect ratio | setting one side recomputes the other; resizing objects scale to fit | on | [W:pd-dialog-printdim-tab] |
| use printer default dimensions (width, height) | page fitted to printable area of the printer | checked | [W:print-option-dialog] |
| resolution | dots per inch | UNKNOWN | |
| page margin for generated layouts | percent of page | 3 % in 2025b (2016 UNKNOWN) | [W:graph-options-set-graph-default] |

Observed hint: the commented-out keys `defpagewidth=6400`, `defpageheight=4800`, `defpageres=600` in the `[Display]` section of Origin.ini describe a printer-dimension default of 10.667 x 8.0 inch (4:3) [F:Origin.ini]. They are an example, not a confirmed default. Theme "Physical Review Letters" stores page values 10.6 and 8.13 (unit not recorded; probably cm) [F:Themes\Graph\Physical Review Letters.OTH].

### 1.3 Layer
| Property | Meaning | Default | Source |
|---|---|---|---|
| left, top, width, height | frame rect of the plotting area | UNKNOWN U5 | |
| unit | % of page, inch, cm, mm, pixel, point, or % of linked layer; changing unit converts values and keeps geometry | % of page | [F:X-Functions\Graph Manipulation\laysetunit.OXF] [W:pd-dialog-size-tab] |
| width:height ratio | can be forced (`laysetratio`) | free | [F:laysetratio.OXF] |
| isometric link of axis lengths to scale | keep X:Y unit ratio on resize; hidden when an axis has a break | off | [W:pd-dialog-size-tab] |
| scale elements | scale with layer frame, or fixed factor (2.3) | scale with layer frame | [W:quick-help/change-size-of-textlabels] |
| clip data to frame (+ margin %) | hide data outside frame | UNKNOWN | [W:pd-dialog-display-speed-tab] |
| data on top of axes | draw data over axes | off | same |
| grid on top of data; reference lines on top | draw order | off [INF] | same |
| speed mode (max points per curve) | draw decimated data only; "Speed Mode is On" watermark never exported | off | same |
| show axes / labels / data / frame | visibility toggles | on | same |
| layer links | X and/or Y scale linked 1:1 to another layer | off | [F:laylink.OXF] |

Helper defaults seen in X-Functions: `layarrange` gaps 5 / margins 15 (percent of page); `merge_graph` gaps 5, margins left 15 right 10 top 10 bottom 15, scale mode auto, optional panel labels A/a/custom `Graph##` (positions top-left/right inside/outside); `plotstack` vertical gap 0, margins left 15 right 10 top 10 bottom 10, layers linked with X linked 1:1 [F:layarrange.OXF, merge_graph.OXF, plotstack.OXF].

### 1.4 Axes
Four slots per 2D layer: bottom X, top X, left Y, right Y. Bottom and left are normally visible; top and right can be shown, and may share settings with the opposite axis. Each axis owns scale, ticks, tick labels, title, grid, breaks, special ticks, line (section 3). A box (frame) is made by showing line and ticks on opposite axes without labels; theme "Opposite Lines" turns that on for all axes [F:Themes\Graph\Opposite Lines.OTH]. Whether 2016 shows opposite axes by default: UNKNOWN U15.

### 1.5 Data plots and plot groups (P0)
A data plot references X column (or row index), Y column, optional error column, and a row range (default all rows). Plot-type IDs: 200 line, 201 scatter, 202 line+symbol, 203 column, 204 area, 205 high-low-close, 206 box, 207 floating bar, 215 bar, 219 histogram, 220 image, 225 pie, 230 "use template", 231 Y error bar, 233 X error bar [W:labtalk plot-type-ids]. `plotxy`: input range list (X, Y, optional error as third column), plot type, symbol size in points, rescale (default 1), legend (default 1), target layer (default new graph) [F:X-Functions\Plotting\plotxy.OXF].

**Grouping.** Edit mode "dependent" applies increment rules; "independent" stops incrementing but keeps styles. Increment: None (all use the first list entry), By One (cycle, wrap), Stretch (spread list over members), Binned, or by metadata (workbook, sheet, long name, units, comments, sampling interval). Subgroups by fixed size, column label, or axis [W:pd-dialog-group-tab].

**Default color list (observed, decoded).** The owner's new-graph theme carries a color list identical to "Q02 Basic" [F:Themes\Graph\Dark Colors & Light Grids.oth, Themes\Color\Q02 Basic.OTH]. Each entry is a 0-based index into the base palette below, or a packed value (top byte 1, then B, G, R). Order:

| # | entry | RGB |
|---|---|---|
| 1 | idx 0 | 0,0,0 black |
| 2 | idx 1 | 255,0,0 red |
| 3 | idx 3 | 0,0,255 blue |
| 4 | idx 5 | 255,0,255 magenta |
| 5 | idx 11 | 0,128,0 olive (green) |
| 6 | idx 8 | 0,0,128 navy |
| 7 | idx 15 | 128,0,255 violet |
| 8 | idx 9 | 128,0,128 purple |
| 9 | idx 10 | 128,0,0 wine |
| 10 | idx 7 | 128,128,0 dark yellow |
| 11-18 | rgb | (43,99,162) (30,150,150) (155,100,26) (16,199,62) (185,36,122) (45,197,204) (63,65,152) (147,172,43) |
| 19 | idx 19 | 128,128,128 gray |
| 20-23 | rgb | (150,100,100) (100,150,100) (43,163,202) (50,100,150) |

The byte layout is [INF], cross-checked on "Q03 Rainbow7", which decodes to red, orange, yellow, green, blue, violet, purple. That a fresh overlay gets black, red, blue, magenta, ... must be confirmed (U1). The file "System Color List" holds 13 RGB values, role unconfirmed: (240,0,130) (250,60,60) (240,130,40) (230,175,45) (230,220,50) (160,230,50) (0,220,0) (0,210,140) (0,200,200) (0,160,255) (30,60,255) (110,0,220) (160,0,200).

**Base palette** (0-based index = LabTalk index minus 1) [W:labtalk list-of-colors]: 0 black 0,0,0; 1 red 255,0,0; 2 green 0,255,0; 3 blue 0,0,255; 4 cyan 0,255,255; 5 magenta 255,0,255; 6 yellow 255,255,0; 7 dark yellow 128,128,0; 8 navy 0,0,128; 9 purple 128,0,128; 10 wine 128,0,0; 11 olive 0,128,0; 12 dark cyan 0,128,128; 13 royal 0,0,160; 14 orange 255,128,0; 15 violet 128,0,255; 16 pink 255,0,128; 17 white; 18 light gray 192; 19 gray 128; 20 light yellow 255,255,128; 21 light cyan 128,255,255; 22 light magenta 255,128,255; 23 dark gray 64. Specials: -9 automatic, -4 none.

**Default line style list:** ids 0..7 in order (solid, dash, dot, dash dot, dash dot dot, short dash, short dot, short dash dot) [F:Themes\Graph\Line Style Default.OTH; addline.OXF]. **Default symbol shape list:** ids 1..16 (square, circle, up triangle, down triangle, diamond, plus, cross, star(*), h-bar, v-bar, numbers, capital letters, lowercase letters, right arrow, left triangle, right triangle) [F:Symbol Type Default.OTH]. **Default interior list:** ids 0..11 [F:Symbol Interior Default.OTH]. Other themes shorten lists (Triangles: 3,4,6,7; Dash and Dot: 1,2,3). [REC] a new overlay increments color only and keeps one solid style and one symbol shape; confirm (U2).

### 1.6 Text, legend, annotation objects
- Axis titles: text objects owned by the axis, default `%(?X)` / `%(?Y)` (3.4).
- Legend: text object "Legend", one line per plot `\L(n) %(n)` (6). Renaming it freezes it [W:legend-manualcontrol].
- Free text: position, font, size, color, rotation, escapes; attach to *page* (never moves/resizes), *layer frame* (follows frame, ignores axis range) or *layer and scales* (anchored in data coordinates) [W:attachment-scaling-object].
- Drawn objects: line, arrow, rectangle, circle, polygon: line color/style/width, fill, arrowhead shape/size, selectable, movable. `addline` adds a full-width vertical or horizontal line at a value with color, one of the 8 styles, optional value label (top/bottom/left/right), selectable (1), movable (1) [F:addline.OXF]. P1.

---

## 2. Units and sizing (P0)

| Quantity | Unit | Notes | Source |
|---|---|---|---|
| font size | points | `\p` escape uses percent | [W:escape-sequences] |
| line width (data, axis, tick, grid, drop line) | points; script unit is points x 500 | | [W:labtalk options_for_lines] |
| symbol size | points (alternatives: X scale, Y scale units) | circle can be sized by side, diameter or area | [W:pd-dialog-sym-tab] |
| symbol edge thickness | percent of symbol radius (1 to 100) when "scale by symbol size" (default on); else points | "default" = global border-width option | [W:pd-dialog-sym-tab, options-dialog-graph-tab] |
| tick length | points; "Auto" = minor is half of major | major default UNKNOWN | [W:axesref-lineticks] |
| layer rect | % of page by default | | |
| distance tick label / title from tick | percent of tick length (global option) | | [W:options-dialog-axis-tab] |
| label offsets | percent of point size (100 = one font width) | | [W:axesref-ticklabels] |
| spline step | `SplineStep=30` (units 0.1 pt; 10 = one step per point of X extent) | smaller is finer | [F:Origin.ini [Display]] |

### 2.1 Known defaults
- Dash dot dot pattern: 18 12 5 12 5 12 (dash, space, dot, space, dot, space), editable; a "scale dash pattern by line width" option exists; units unstated. Other seven styles: UNKNOWN U6 [W:options-dialog-graph-tab].
- Elements scale with the layer frame [W:quick-help/change-size-of-textlabels].
- Symbol color automatic = follows line color [W:pd-dialog-line-tab].
- Axis line color Auto = darker shade of plot fill; if no fill, of border; else follows line color [W:axesref-lineticks].
- Anti-aliasing covers plots, axes/grid and line objects; purely horizontal/vertical lines are never anti-aliased [W:options-dialog-graph-tab].

### 2.2 Font, width, size defaults: UNKNOWN (U3)
No public page and no installed text file gives them. [REC] Arial; axis titles, tick labels and legend 22 pt on the stock page; data line 1.5 pt; axis line 1.5 pt; major tick 9 pt, minor 4.5 pt; symbol 9 pt. Weak support: themes "Black on paper" and "Physical Review Letters" contain the numbers 1.5 and 8 [F:Themes\Graph]. Do not hard-code until measured.

### 2.3 Scale elements
Attached elements (font sizes, line and arrow widths, arrowheads, drawn objects) respond to layer resizing:
- **Scale with layer frame**: proportional; after each resize Origin updates the stored factor.
- **Fixed factor**: elements = stored size x factor; resizing leaves them alone. Text keeps its size but still moves with the frame; drawn objects attached to frame/scales are still resized; page-attached objects never change.
- Command "Fix scale factor" (Graph menu) sets factor 1 on all layers so point sizes are true; theme "Layer No Element Scaling" does the same [W:pd-dialog-size-tab; F:Themes\Graph\Layer No Element Scaling.oth].
[INF] Implement `s` per layer: effective size = stored pt x s; in scale mode recompute s from the new/old frame size ratio (which ratio, width/height/min: U7).

---

## 3. Axis options (P0)

### 3.1 Scale tab [W:axesref-scale]
| Option | Meaning | Default |
|---|---|---|
| from, to | axis limits | from data |
| reverse | swap limits | off |
| type | linear; log10; ln; log2; probability (norminv(x/100), valid 0.0001..99.999); probit (step = 1 sd, 5 = mean); reciprocal (1/x); offset reciprocal (1/(x+273.14)); logit (ln(y/(100-y))); double-log reciprocal / Weibull (ln(-ln(1-x))); custom (direct and inverse formula of x); discrete (labels from first plot's X, skips gaps) | linear |
| script type codes | linear, log10, prob, probit, recip, offset, logit, ln, log2 [F:laysetscale.OXF] | |
| symmetric log | allows negative values on log axes; linear threshold 1; linear length 1 major interval | off |
| rescale mode | Fixed (never refit; zoom prompts); Normal (manual/zoom allowed; refit only on explicit Rescale); Auto (also refits when data change); From and To can have different modes | UNKNOWN U17 |
| rescale margin % | pad min/max by percent of span; 0 with 0 minor ticks gives exact data limits; value >= system var @RRT uses the default padding method, else pad = n x span; rounding to increments blurs it; not for matrix-based graphs | UNKNOWN U4 |
| major ticks | by increment; by counts; min and max only; custom positions; plot column label | by increment |
| increment | spacing; time axes accept `1month`, `4Q` (sec,min,hour,day,week,month,quarter,year) | auto |
| count | wanted tick count; negative = round labels to integers | |
| anchor (first) tick | forces a tick at a value (anchor 1, range -3..3, increment 2 gives -3,-1,1,3) | auto |
| minor ticks | count between majors (1 = halves); or custom positions | UNKNOWN U4 |
| log tick rule | linear ticks when 10 x log10(max/min) <= @TL; @TL default 10 | 10 |
| max major ticks | numeric axes: cap (value unstated; above it Origin shows 6); text/date axes: 400 [W:options-dialog-axis-tab] | |
Script: `laysetscale` (axis x/y/z, from, to, inc, type; all default auto) [F:laysetscale.OXF].

### 3.2 Line and ticks tab [W:axesref-lineticks]
| Option | Meaning | Default |
|---|---|---|
| show line and ticks | per axis; "same for opposite" shares | UNKNOWN U15 |
| line color, thickness (pt) | color Auto rule in 2.1 | UNKNOWN |
| position | % offset from default (positive away from page centre), or at a scale value | at frame |
| arrow at begin/end | shape, width, length, offset % | off |
| tick direction | theme value 1 = in, 2 = out (for major and minor separately); none and both exist, ids [INF] 0 and 3 | UNKNOWN U15 ([REC] out) |
| tick length (pt), color, thickness | minor Auto = half major | UNKNOWN |
Observed themes: "Ticks All In" sets major=minor=1; "Ticks All Out" 2; "Ticks YL XB Out YR XT In" sets left and bottom 2, right and top 1. Property ids: 57 minor, 58 major [F:Themes\Graph\Ticks*.OTH].

### 3.3 Tick labels tab [W:axesref-ticklabels]
| Option | Meaning | Default |
|---|---|---|
| show | per axis | bottom/left on |
| type | numeric; text from dataset; time; date; month; day of week; column name or label; tick-indexed dataset; categorical; tick-indexed string | numeric |
| numeric display | decimal (1000 or 1,000); scientific (10^3 or 1E3); engineering (k, M, G, T, m, u, n, p, f); custom format | decimal |
| decimal places | integer, or auto | auto |
| display units | one shared power-of-ten (10^2..10^6, 10^-2..10^-6, custom) | off |
| divide by factor | all labels divided; expressions allowed | 1 |
| formula | label = f(x); data unchanged | none |
| prefix, suffix | text with escapes | empty |
| plus/minus sign text | | |
| "Zero is 0", "No trailing zero" | global options | UNKNOWN |
| 1E3 style | 1E+03 / 1E+3 / 1E3 | 1E+03 |
| rotate | degrees, positive counter-clockwise; auto = 45 then 90 on overlap; parallel; perpendicular | 0, auto on overlap |
| font, size, bold, white-out, color | color Auto follows axis | UNKNOWN |
| position, alignment | auto: centred for top/bottom, right-aligned on left axis, left-aligned on right axis | auto |
| minor tick labels | hide / each minor / specified indices; size % of major | hide, 100 % |
| auto-hide overlapped | | UNKNOWN |

### 3.4 Title tab [W:axesref-title, quick-help/customize-axistitle, options-dialog-axis-tab]
Default text: the long name of the first plot's X (or Y) column, else its short name, plus the units when the page-level "show units" option is on. The notation is `%(?X)` and `%(?Y)` ("?" = first plot of the layer; `%(1X, @LL)` explicit). Units are written in a style chosen globally: parentheses, brackets, braces, slash, dash, "in", two lines; default Auto (language-dependent). Options: show, color or follow plot, rotation (positive counter-clockwise), offset along axis (begin / middle / end), font, size, same-for-opposite. Origin.ini has commented keys `XTitle`, `YTitle`, `Legend` to redefine defaults; example `%(1Y, @LG)` = long name plus unit.

### 3.5 Grids [W:axesref-grids]
Per direction: major and minor grid each with show, color, style, thickness; plus "opposite" line and "line at value" (for columns/bars Y=0 is on). Shipped system theme stores two line sets per axis: {color 18 = light gray, style 1 = dash, width 0.3} and {color 19 = gray, style 0 = solid, width 0.5} with ids 66 color, 67 style, 68 width; which set is major is unconfirmed (U8) [F:Themes\Graph\Dark Colors & Light Grids.oth]. Grids off by default for new graphs [REC].

### 3.6 Breaks [W:axesref-breaks] (P1)
Enable; half-length (auto = one major tick distance); 1 to 7 breaks; per break from, to, position % of axis length (automatic position even or proportional); per-break "scale after break" and "tick labels after break" (both default to follow main axis).

### 3.7 Special ticks and reference lines (P1/P2)
Custom tick positions and labels per tick. Reference lines are not documented for 2016; emulate with `addline` objects.

---

## 4. Automatic scale and tick algorithm

**Observed:** rescale combines data min/max, the rescale margin percent, rounding to the tick increment and the minor tick count; the result is "approximately" min-margin..max+margin and lands on tick positions. With margin 0 and 0 minor ticks, Normal rescale gives exact data limits [W:axesref-scale]. The exact increment rule is not documented.

**Proposed rule (INF; validate against Origin on ~10 datasets):**
1. span = max - min (guard span 0).
2. wanted about 5 to 8 major intervals; raw = span/6; step = smallest of {1, 2, 2.5, 5} x 10^k that is >= raw.
3. from = floor((min - pad)/step) x step; to = ceil((max + pad)/step) x step; pad = margin% x span.
4. minor count default 1 (U4).
5. log axes: ticks at decades, minors 2..9; a span under one decade uses linear ticks (@TL rule).
Electrochemical CV axes the owner centres on zero are Fixed scale, not auto [INF].

---

## 5. Line, scatter, line+symbol (P0)

### 5.1 Plot types
Line (200), scatter (201), line+symbol (202) are one plot object differing in connect and symbol settings (line: no symbol; scatter: no line) [F:plotxy.OXF].

### 5.2 Connect [W:pd-dialog-line-tab, labtalk options_for_lines]
| Connect | id | Behavior |
|---|---|---|
| No line | 0 | |
| Straight | 1 | default for line and line+symbol [INF] |
| 2 point segment | 2 | alternating segment and gap |
| 3 point segment | 3 | |
| B-spline | 8 | smooth, winds around points, ends duplicated |
| Spline | 9 | cubic through points; X increasing; max 900 points; resolution = SplineStep |
| Step horizontal | 11 | |
| Step vertical | 12 | |
| Step horizontal centred | 13 | |
| Step vertical centred | not documented | |
| Bezier | 15 | groups of 4, passes through 1st and 4th; leftovers unused |
Also: connect begin and end (closes; off), connect within subset (on for line plots), Akima/modified Bezier (later versions).

### 5.3 Line options
| Option | Meaning | Default |
|---|---|---|
| style | 0 solid, 1 dash, 2 dot, 3 dash dot, 4 dash dot dot, 5 short dash, 6 short dot, 7 short dash dot | solid |
| width | points | UNKNOWN U3 |
| color | palette index, RGB, automatic (group), none | group |
| transparency | 0 opaque .. 100 clear | 0 |
| fill under curve | enable; base line (auto, bottom, top, left, right axis, value, extra line); fill to another plot; above/below colours; pattern | off |
| symbol/line gap | % of symbol diameter, max 256; global option | off |
| drop lines | horizontal and vertical, with colour, style, width | hidden |

### 5.4 Symbols [W:labtalk list-of-symbol-shapes, pd-dialog-sym-tab]
Shapes: 0 none, 1 square, 2 circle, 3 up triangle, 4 down triangle, 5 diamond, 6 plus, 7 cross, 8 star(*), 9 h-bar, 10 v-bar, 11 numbers, 12 capital letters, 13 lowercase letters, 14 right arrow, 15 left triangle, 16 right triangle, 17 hexagon, 18 star, 19 pentagon, 20 sphere; 56 data markers, 58 X-position marker lines.
Interiors: 0 solid, 1 open, 2 dot centre, 3 hollow, 4 + centre, 5 x centre, 6 - centre, 7 | centre, 8 half up, 9 half right, 10 half down, 11 half left. Hollow lets objects beneath show; open hides them unless fill is none.
Other: size (pt, default UNKNOWN), edge thickness (2), colour automatic = line colour, fill colour (global default), size from column with scaling factor and formula (`s`, `s1` min, `s2` max), user bitmap/SVG symbols (P2).

### 5.5 Error bars (P0)
Error column attached as third plot column; types 231 (Y) and 233 (X). Template ERRBAR exists. Options: direction plus/minus/both, cap width, line width/colour/style. Numeric defaults UNKNOWN U9.

### 5.6 Gaps and skipping
Missing values break the connecting line [INF]. Skip-points option exists in plot details [REC]. Speed mode only decimates drawing.

---

## 6. Legend (P0)

### 6.1 Syntax
Per entry: `\L(n) %(n)`. `\L(n)` (also `\l`) draws the sample of plot n; `\L(2.1)` = layer 2 plot 1; further options change symbol/size/width. `%(n)` yields text per translation mode. Installed ini: `Legend=\L(%d) %%(%d)\r\n` and `PlotLegend=%(?P)` [F:Origin.ini [Text]]. `@` options:

| Option | Meaning |
|---|---|
| @LL | long name (empty if none) |
| @LS, @C | short name |
| @LU | units |
| @LA | long name, else short name |
| @LC | comments, else long name, else short name |
| @LM | comment, first line |
| @LD n | user parameter n |
| @LP | parameter 1 |
| @R | data range |
| @D | dataset name |
| @W, @WS | workbook, worksheet name |
| @GF | graph filter condition |
| @V, @WT | cell access (point-by-point legend) |
Combine: `%(1,@(@LL(@LU)))` = long name followed by (units). Other legend kinds: categorical, box-chart components, point-by-point. Legend symbols not tied to a plot: `\L(O Shape:=..., ...)` (P2) [W:legend-substitution-notation].

### 6.2 Which label is used; translation modes
Documented default: the Y column's comments row supplies the text, switchable by the translation mode [W:quick-help/customize-axistitle]. `legendupdate` modes: comment first line (`@LM`, default, index 0), data range (`@R`), long name and units (`@U`), user parameter 1 (`@LD`), long name (`@LA`), short name (`@LS`), parameter 1 (`@LP`), custom format [F:legendupdate.OXF]. For CHI files with empty comments choose long name or short name [INF].

### 6.3 Update modes [F:legendupdate.OXF]
Scope: whole page, active layer, specific page, specific layer. Mode: reconstruct (rebuild from scratch) or update (keep manual text edits). Legend: one per layer or one for whole page. Entry order ascending/descending; plot order within layer ascending/descending. Hide entries for hidden plots (default on); hide entries for fit curves (default off). Which listed item is the default of mode and order: U10. A renamed legend becomes static.

### 6.4 Frame, position
Frame tab (none, line, shadow...), position by anchor corner, size, arrange vertical or horizontal, multi-column, line spacing %, `\sep:nn` gaps. Default frame and position of the stock template: UNKNOWN U11 ([REC] frame none, top-right inside layer).

---

## 7. Text escape sequences (P0) [W:escape-sequences, legend-substitution-notation]

| Sequence | Meaning |
|---|---|
| `\b(text)` | bold |
| `\i(text)` | italic |
| `\u(text)` | underline |
| `\s(text)` | strikethrough |
| `\+(text)` | superscript |
| `\-(text)` | subscript |
| `\=(sub,sup)` | sub and superscript together (`\v(,)` for literal comma) |
| `\g(text)` | Greek letters (`\g(a)` alpha, `\g(m)` mu) |
| `\f:Name(text)` | font by name |
| `\cN(text)` | colour by palette position N (base U12) |
| `\pN(text)` | size as percent of current (200 doubles) |
| `\xHHHH(...)` | Unicode by hex code, e.g. `\x(03b8)` |
| `\(code)` | character by ASCII code |
| `\ab( )` `\ad( )` `\at( )` | bar, dot, tilde above |
| `\y(')` | full-height prime |
| `\v(text)` | literal; stops `%` and `\` interpretation (`5\v(%)Cu`) |
| `\h(...)` | block substitution |
| `\q(text)`, `\_(text\)`, `\[text\]` | LaTeX equation (P2) |
| `\L(n)` | legend symbol |
| `\sep:nn` | extra line gap, % of font height |
| `\sc(...)` | one conditional space |
| `\ww(text)` | literal wrapped text in legends |
| `%(...)` | substitution |
Unsupported codes and a bare backslash are dropped silently. Literal backslash: `\\`. Examples: `i / \g(m)A`, `cm\+(-1)`, `H\-(2)O`.

---

## 8. 2D plot types as Origin groups them [F:User:Template.ini; F:*.otp names]

| Group | Plots | Priority |
|---|---|---|
| Line | Line | P0 |
| Symbol | Scatter, ScatterCentral, Errbar (scatter + error bars), DropLine, Bubble | P0 scatter and errbar; P1 dropline; P2 others |
| Line & Symbol | Line+Symbol, Lser3 (line series) | P0 |
| Columns & Bars | Column, ColumnLabel, gColumn (grouped), Bar, StackColumn, StackColP, StackBar, StackBarP, FloatCol, FloatBar, Pie, PieBW | P1 column/bar/stack; P2 rest |
| Multi-Curve | DoubleY (two Y axes), 3Ys_Y-YY, 3Ys_Y-Y-Y, 4Ys_Y-YYY, 4Ys_YY-YY, OffsetstackY, Waterfall, Pan2Vert, Pan2Horz, Pan4, Pan9, Stack, Inset, RightY, RightTop, OffsetY, OffsetXY | P0 DoubleY and many-curve overlay in one layer; P1 Stack, panels, OffsetY, RightY; P2 rest |
| 3D XYY | Water3D, Walls, Ribbon, Bar3D (+ OpenGL versions) | P2 |
| Statistics | Box, Hist, HistCumu, Xbarr, ParetoRaw, PPNormal, PPExp, PPLog, PPWeibull, QQ, ScatterMatrix, gBox | P1 Hist, Box; P2 rest |
| Area | Area, FillLine, StackArea | P1 (peak shading) |
| Contour | TriContour, TriContline, TriContgray, PolarContour, TernaryContour, ContLine, ContGray, Contour, Profile2 | P2 |
| Specialized | Polar, PolarXrYTheta, WindRose, WindRoseRaw, Ternary, SmithCht, Radar, Vector, VectXYXY, Zoom1 | P2 |
| Stock | HLClose, Candlestick, OHLCBarchart, OHLCVol | P2 |
| 3D Surface; 3D Symbols, Bars, Vector; Image | Mesh, Wirefrm, Cmap, 3D, Traject, 3DBars, Image, Profiles ... | P2 |

About 250 `.otp` templates are installed; many are internal (fit, FFT, report, preview). Plotting X-Functions: plotxy, plotm, plotms, plotstack, plotmyaxes, plotgroup, plotbylabel, plot_lineser, plotxyz, plot_prob ... [F:X-Functions\Plotting]. `plotmyaxes`: several Y axes left/right, optional middle Y axis (default 0), axis colour linked to plot (default 1), show Y titles (0), top X axis (0), gap auto. `plotgroup`: split by page, layer and data group columns; arrange vertical/horizontal; types line, scatter, line+symbol, column [F:plotmyaxes.OXF, plotgroup.OXF]. Exact Plot menu order for 2016: U13.

---

## 9. Graph export (P0)

### 9.1 The function [F:X-Functions\Import and Export\expGraph.OXF, expPDFw.OXF; W:x-function/ref/expgraph, settings-in-expgraph-dailog]
Variables: `type`, `export`, `pages`, `filename`, `path`, `overwrite`, `theme`, `sysopts`, `tr` (export settings), `keepsize`, `tr1` (size), `tr2` (format). `expPDFw` adds `tr3`; `expImage` exports workbook images.
Image types in the file: ai, bmp, cgm, dxf, emf, eps, gif, jpg, pcx, pdf, png, psd, tga, tif, wmf; docs also list SVG. Default: EPS. Raster: BMP, GIF, JPG, PCX, PNG, PSD, TGA, TIF. Vector: AI, CGM, DXF, EMF, EPS, PDF, SVG, WMF. For the owner P0: PNG, TIF, JPG, PDF, SVG, EMF, EPS.

| Variable | Meaning | Default |
|---|---|---|
| export | active page; all in folder; recursive; open (visible only); incl. embedded; project; specified | active page |
| filename | supports substitution: long name, short name, `Graph`+index, `Graph#`+index | |
| path | user files folder, or folder of the project | |
| overwrite | rename (auto enumerate), replace, skip, ask | ask |
| theme | graph theme applied before export | none |
| sysopts | take page settings from global options (script only) | 1 |
| keepsize | keep original graph size | 0 |

### 9.2 Export settings `tr`
| Setting | Meaning | Default |
|---|---|---|
| SpeedMode | 0 page setting, 1 force on, 2 force off | 0 |
| Margin | -2 auto (global copy-page settings + offsets); 0 border; 1 tight; 2 page; 3 tight in page | -2 |
| BorderWid | border width (only with Border) | -1 |
| OpenGLGraph.AntiAlias | 0 none, 1 multisample, 2 fast (3D only) | 1 |
| Advanced.Resolution | vector only: 0 set resolution for symbols/bars/rasters, 1 keep size | 0 |
| Advanced.DPI | resolution of rasterised parts of vector output | 300 |
Margin meaning: border = bounding box of everything plus border; tight = bounding box including off-page objects; page = full page rectangle; tight in page = bounding box of objects over 50 % inside the page. Margin offsets: whole-pixel offsets relative to the active mode, set interactively; no default [W:export-margin_offsets].

### 9.3 Size `tr1`
Unit 0 inch (default), 1 cm, 2 pixel, 3 page ratio %. Rescaling 0 by width (default) or 1 by height; aspect always kept; width/height value 0 = original page size. Pixel size = inches x DPI (raster only). Publisher recipe: Margin = page, size = page ratio 100 %, DPI as requested [W:quick-help/set-expgraph-size-resolution].

### 9.4 Raster options `tr2`
| Format | DPI default (choices) | Colour depth default | Extras |
|---|---|---|---|
| PNG | 300 (72,100,150,300,600,1200) | 256 colours (mono, 16, 256, 24-bit, 8-bit grey) | |
| TIF | 300 (+96) | 256 colours | colour space RGB (CMYK, YCbCr); compression none (PackBits, LZW) |
| JPG | 300 (+96) | 24-bit (or 8-bit grey) | quality 0..255 default 0, higher = smaller/lower quality |
| BMP | 300 (+96) | 256 (+16-bit) | |
| GIF | 300 (+96) | 256 | |
| PCX, PSD, TGA | 300 | 256 | |
Trap: PNG and TIF default to 256 colours; use 24-bit for colour figures. [W:details-of-treenodes-in-export-graph]

### 9.5 Vector options `tr2`
- **EPS**: PostScript level 2 (1/3), ASCII data (binary), RGB (CMYK), bitmap handling "individual bitmaps" (ignore, source copy, combine) at 150 dpi, preview none (TIFF/WMF), Adobe font type 1 (3, 42), embed outline fonts (substitution, built-in), vector line thickness 0, semi-transparency as raster at default resolution (opaque, or chosen dpi default 300), raster compression on.
- **PDF**: colour translation normal; line and fill mode device; background rectangle off; bitmap handling individual at 72 dpi (72..600); add text to metadata off; compression Flate for colour images, mono images, and text/graphics; fonts: embed = descriptor only (embedded, outline), stroke width 2, substitution off, TrueType off. For a self-contained publication PDF prefer embedded fonts [INF].
- **SVG, EMF**: no settings documented (U14). **AI, CGM, DXF, WMF**: legacy P2 (WMF placeable header on; DXF units inches).

### 9.6 Copy page / clipboard (P1)
Ctrl+J copies page (OLE), Ctrl+Alt+J copy as image, Ctrl+C copies as image by default. Size factor % of original: 100 from 2021b, 40 before, so 2016 default likely 40 [W:quick-help/export-graph-size-small]. Margin control as 9.2. 2016 ini: `AddEMFInCopyPage=1`, `AddWMFInCopyPage=1` (enhanced and Windows metafile put on clipboard) [F:Origin.ini]. View > Show Copy Page/Export Area (Ctrl+4) outlines the region.

---

## 10. Themes and templates

- **Graph theme `.OTH`** (XML, plain): a tree of numeric node ids with values; lists are base64 int32 arrays. A theme only changes the properties it contains; several can be applied, the topmost wins [W:themeref-themeorganizer]. Built-in: `Origin2016\Themes\Graph` (50 files, e.g. Black on paper, Color Publication, B&W Publication, FDA portrait/landscape, Physical Review Letters, Times New Roman Font, Ticks All In/Out, Line Style and Symbol variants, Fill Pattern variants, Layer No Element Scaling, Speed Mode ON/OFF, Night Sky, Opposite Lines). User themes live in the user Themes folder. Observed ids: page node 4 (6 colour, 10 with 11/12/13 incl. width and height); axes 1073741862 with X 536936487 and Y 537002023; ticks 1073741878 (57 minor, 58 major); grids 1073741887 (66 colour, 67 style, 68 width); list nodes 521 colour list, 522 line style list, 523 symbol shape list, 524 symbol interior list; 262 font for all text (opaque id; Times New Roman = 1852392883) [F:Themes\Graph\*].
- **Graph template `.OTP`** (binary; not read): "Save as template" keeps layout and style (page, layers, axes, plot types and styles, group rules, text, drawn objects, legend, speed-mode settings) and not the data; a new graph is built by plotting selected columns into it. Registry of user-visible templates and their categories: `Template.ini` (section 8). Base templates: LINE, SCATTER, LINESYMB, ORIGIN, STANDARD, DOUBLEY ... [F:*.otp names; INF for contents].
- **System theme for new graphs**: owner's `[Display] SysTheme=Dark Colors & Light Grids` and `[System Themes] GraphNewGraph=Dark Colors & Light Grids` [F:Origin.ini; User:Origin.ini]. Despite the name this theme contains only the grid line sets (3.5), the colour list (1.5) and one unexplained property (node 139 = 2); fonts/sizes/page come from the template.
- **Colour lists** `Themes\Color`: Q01 Black and RGB, Q02 Basic, Q03 Rainbow7, Q04 Paired Color, Q05-Q08 seasons, S01-S04 sequential, D01-D05 diverging, System Color List. **Palettes** `Palettes\*.pal` (about 60) are colour maps (P2).

Our app: store themes and templates as JSON (own format), with the same partial-override semantics.

---

## 11. UNKNOWNS: measure in running Origin 2016

Method: draw an overlay of four XY columns with the default template; read Plot Details; or run LabTalk queries (`page.width`, `layer.width`, `layer.unit`); export PNG variants and diff pixels.

- U1 default colour order of a fresh overlay (candidate black, red, blue, magenta; alternative System Color List).
- U2 which element lists increment by default (colour only?) and default symbol/line style.
- U3 default font name and sizes (axis title, tick label, legend), data/axis/tick line widths, tick lengths, symbol size and edge thickness.
- U4 default minor tick count, rescale margin %, @RRT, auto tick-count target.
- U5 stock LINE template: page size, resolution, layer rect %.
- U6 dash numbers for all 8 styles and their units.
- U7 scale-with-frame ratio rule.
- U8 grid defaults and which set is major.
- U9 error bar defaults (cap width, widths, direction).
- U10 legendupdate default mode/order mapping.
- U11 legend frame, position, font defaults.
- U12 `\cN` base (0 or 1).
- U13 exact Plot menu order and labels.
- U14 SVG and EMF export (text vs outlines, stroke handling).
- U15 default tick direction, which axes show, opposite lines.
- U16 axis title when long name and units are empty.
- U17 default rescale mode after adding plots.
- U18 default PNG export result of the GUI Export dialog (DPI, margin mode).
