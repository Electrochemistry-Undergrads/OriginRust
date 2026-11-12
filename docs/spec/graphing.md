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
| Multi-Curve | DoubleY (two Y axes), 3Ys_Y-YY, 3Ys_Y-Y-Y, 4Ys_Y-YYY, 4Ys_YY-YY, OffsetstackY (stacked lines by Y offsets, section 13), Waterfall (section 13), Pan2Vert, Pan2Horz, Pan4, Pan9, Stack, Inset, RightY, RightTop, OffsetY and OffsetXY (despite their names: multiple Y, and X and Y, axes, one layer per axis; GP-DY-01, not offset curves) | P0 DoubleY, stacked lines and many-curve overlay in one layer; P1 Stack, panels, OffsetY, RightY; P2 rest |
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

---

## 12. Double-Y graphs, right Y axes and linked layers (P0, GP-DY-01)

Tags as elsewhere in this document, plus **[COM]** for behaviour observed in Origin 2016 through COM/LabTalk on our own synthetic data (scripts and pictures in `re-workspace/scratch/graph-p0/`, git-ignored). The design decision is recorded in ADR 0008.

### 12.1 What Origin 2016 does

- **Two layers.** A double-Y graph is one page with two layers. Origin 2023 later added two Y scales inside one layer; Origin 2016 has only the two-layer form, template `DOUBLEY.OTP` (the current help calls it "2Ys Y-Y") [W:tutorials/double-y, origin-help/2ys-y-y-graph; F:DOUBLEY.OTP, no DblY.otp].
- **The link.** Layer 2 is linked to layer 1 [COM]:
  - its frame is given in "% of linked layer" as 0, 0, 100, 100, so it covers layer 1's frame exactly;
  - its X scale is linked "straight 1:1": the same limits as layer 1, and changing layer 1's X range moves layer 2's;
  - its Y scale is its own.
- **Moving and resizing.** A linked layer always moves and resizes with its parent; a straight or custom link makes its scale follow the parent's, and the link acts only when the parent changes [W:origin-help/link-layers, pd-dialog-linkaxesscale-tab].
- **Frame.** Layer 1's frame is about 17.9 / 11.6 / 68.2 / 71.9 percent of the page (left, top, width, height), practically the frame of the plain Line template, whose right margin already leaves room for a right axis [COM].
- **Axes per layer.** Layer 1 shows the bottom X and left Y axes with labels. Layer 2 shows the right Y axis with labels, and the top X line and ticks without labels, which closes the box. All ticks point outwards [COM].
- **Right Y title.** It belongs to layer 2 and is built from layer 2's first plot (long name and units). It is rotated 90 degrees counter-clockwise like the left title (it reads bottom to top), sits right of the right tick labels, centred on the frame height, and is black [COM].
- **Columns.** The Double-Y plot command puts the 1st, 3rd, 5th ... selected Y column in layer 1 and the 2nd, 4th ... in layer 2 [COM with one X column]. With two X columns the help says the split follows the X columns [W; not run]. Plotting with `plotxy` into the DoubleY template put every column into layer 1 [COM].
- **Legend.** One legend, in layer 1, lists the plots of both layers, one line per layer and plot; layer 2 has none. It is placed above the top-right corner of the frame, partly outside it [COM].
- **Colours** [COM]:
  - layer 1's curve is black and layer 2's curve is pure blue (the third entry of the default colour list, not the second);
  - curves placed in the same layer by the command did not change colour;
  - every axis colour in this template is "automatic": the axis line, its ticks and tick labels take the colour of the layer's plot, so the right Y axis (and the top X line, which belongs to layer 2) is blue like its curve; the Line template uses black axes;
  - titles stay black;
  - the help describes the automatic axis colour as a darker shade of the plot's fill, else of its border, else its line colour [W:axesref-lineticks].
- **Autoscale.** Each layer fits its Y axis to its own plots. X is shared through the link, and the layer that was rescaled last decides the shared X range; a rescale of layer 1 ignores layer 2's data and can cut off layer 2's curve [COM].
- **Adding a right Y axis to an existing graph.** "New Layer (Axes) > Right-Y (Linked X Scale and Dimension)" (`layadd type:=righty`) adds a linked layer that shows only a right Y axis [W:addlayers-insertlayer, x-function/ref/layadd; COM]:
  - black axis, no top line (the box stays open);
  - its own legend;
  - its own colour sequence, starting again at black.

### 12.2 Model

A double-Y graph in Sheaf is a graph with two layers, the second linked to the first. Every new field below has a serde default and is not written while it has its default value, so files written before this feature load unchanged, draw exactly as before, and graphs that do not use the feature are saved exactly as before.

```rust
pub struct Layer {
    // ... existing fields ...
    /// Set when this layer is drawn over another layer of the graph and follows it. While the
    /// link is valid, `frame` is read as fractions of the parent's frame instead of the page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<LayerLink>,
}

pub struct LayerLink {
    /// Index of the parent in `Graph::layers`.
    pub parent: usize,
    #[serde(default)]
    pub x: ScaleLink,
    #[serde(default)]
    pub y: ScaleLink,
}

pub enum ScaleLink {
    /// The layer's own scale (default).
    None,
    /// Exactly the parent's scale: limits, scale type, ticks and direction.
    Straight,
}

pub struct Axis {
    // ... existing fields ...
    pub side: AxisSide,                 // default Normal
    /// Axis line, ticks and tick labels in the colour of the layer's first visible plot.
    pub color_follows_plot: bool,       // default false
    /// The axis title in that colour too.
    pub title_color_follows_plot: bool, // default false
}

pub enum AxisSide {
    /// Bottom for X, left for Y (default).
    Normal,
    /// Top for X, right for Y.
    Opposite,
    /// Not drawn at all; the scale still places the data.
    Hidden,
}

pub struct Legend {
    // ... existing fields ...
    pub scope: LegendScope,             // default Layer
}

pub enum LegendScope {
    /// This layer's plots (default).
    Layer,
    /// Every layer's plots.
    Graph,
}
```

The enums are written in snake_case like the existing ones (`"straight"`, `"opposite"`, `"graph"`). A later `Custom` variant of `ScaleLink` can carry link formulas.

**Valid links.** A link is valid when `parent` is smaller than the layer's own index and the parent layer has no link itself (`Graph::link_parent`). An invalid link (to itself, to a later or missing layer, or to a linked layer) is ignored: the layer is drawn as an independent layer with its `frame` read as page fractions.

**Groups.** A layer without a valid link is a *root*. A root plus every layer validly linked to it, in layer order, is a *group* (`Graph::group`). The member of a group whose Y axis side is `Opposite` carries the group's right Y axis (`Graph::right_y_member`).

**Right-Y layer.** `template::right_y_layer(parent, parent_index)` builds the layer that gives a parent a right Y axis:

- start from a copy of the parent without its plots and free labels;
- `link = { parent: parent_index, x: Straight, y: None }` and `frame = {0, 0, 1, 1}`;
- `x_axis.side = Hidden`, without grids;
- `y_axis` is the parent's Y axis (fonts, ticks, scale type, automatic range mode, colour rule) with:
  - `side = Opposite`;
  - `min`, `max`, `major_step` and `title` set to `None`, and `tick_label_divisor = 1`;
  - `show_opposite = false` and `opposite_ticks = false`;
  - no grids;
- `legend.visible = false`, scope `Layer`;
- palette:
  - for a `Cycle` palette, the parent's colour list starting at its third colour (wrapping); a list of one or two colours starts at its last colour;
  - for a `Spread` palette, the parent's list in reverse order.

**Changes to the parent.** When a right-Y layer is added (`Project::add_right_y_layer`), the parent changes as follows:

- `y_axis.show_opposite = false` and `y_axis.opposite_ticks = false`, because the right edge now belongs to the new axis;
- `legend.scope = Graph`.

**Template `DoubleY`** (name `double-y`, label "Double Y"). The Line template with these changes:

- page 6.4 x 4.8 inch, as the Line template;
- layer 0:
  - frame left 0.16, top 0.07, width 0.68, height 0.75, which leaves room for the right labels and title;
  - `x_axis.show_opposite = true` and `x_axis.opposite_ticks = true` (the top line of the box);
  - `y_axis.color_follows_plot = true`;
  - legend scope `Graph`, position TopRight;
- layer 1 = `right_y_layer(layer 0, 0)`; its Y axis inherits `color_follows_plot = true`;
- titles do not follow the plot, X axes stay black, ticks point out.

`Template::new_graph` returns two layers for this template.

### 12.3 Resolving axes and frames

Groups are resolved one after the other in the order of their roots. Within a group:

1. **Data ranges.** For the root's X axis, take the finite X values of the root's visible plots and of every member whose `link.x` is `Straight`, with the row rules of figure-rendering.md section 2: rows with a missing X or Y are skipped, and on a log10 axis only positive values count, judged by the root's scale. The root's Y axis works the same way with `link.y`. Every other axis uses only its own layer's plots.
2. **Linked axes.** The root's axis is resolved as before. A member's axis with a `Straight` link *is* the root's resolved axis: the same limits, step, ticks, labels, scale type and reversal. The member's own `min`, `max`, `major_step`, `minor_ticks`, `scale` and `auto_range` on that axis are not used while it is linked.
3. **Nominal frames.** The root's nominal frame comes from `frame` as page fractions, as before. A member's frame is `x = R.x + f.left*R.w`, `y = R.y + f.top*R.h`, `w = f.width*R.w`, `h = f.height*R.h`, where R is the root's frame. For `{0,0,1,1}` it is the root's frame.
4. **Page fit** (extends "Keeping labels on the page" in figure-rendering.md). Lay out every member's labels at the nominal frames and take the outermost edges over the whole group:
   - the left edge of every `Normal` Y axis's title (or of its widest tick label when the title is hidden);
   - the right edge of every `Opposite` Y axis's title (or of its labels);
   - the bottom edge of every `Normal` X axis's title (or of its labels);
   - the top edge of every `Opposite` X axis's title (or of its labels).

   `Hidden` axes add nothing. The margin stays 3 pt from the page edge. Each shift moves one side of the root's frame and the opposite side stays where it is, in this order:
   - `left_shift = clamp(3 - left_edge, 0, 0.5*w)`, then `w1 = w - left_shift`;
   - `right_shift = clamp(right_edge - (page_w - 3), 0, 0.5*w1)`, taken from the width;
   - `bottom_shift = clamp(bottom_edge - (page_h - 3), 0, 0.5*h)`, then `h1 = h - bottom_shift`;
   - `top_shift = clamp(3 - top_edge, 0, 0.5*h1)`, moving the top side down.

   Then the members' frames are derived again from the adjusted root frame. A group whose labels already fit keeps its nominal frames, so existing one-layer figures are unchanged.
5. **Geometry.** `layout_geometry` still returns one `LayerGeometry` per layer, in layer order. A member's entry carries its derived frame, the root's X axis when X is linked, and its own Y axis.

Check values (fixed text metrics, every character 0.55 x size wide; DoubleY template, X = 0..10, left Y = x², right Y = (x - 5)/100): both frames are x 73.728, y 24.192, w 313.344, h 259.2; left Y 0..100 step 20; right Y -0.06..0.06 step 0.02; the right tick labels start at x = 396.272 (frame right edge 387.072 + 5 pt outward tick + 0.35 x 12 pt gap); the right title is anchored in its middle at (447.752, 153.792). With layer 0's width set to 0.78 the right title would end at 496.772, so the right side moves left by 38.972 and w becomes 320.452.

### 12.4 Drawing

Layers are drawn in layer order, each as before (grid, plots clipped to its frame, axes, ticks, labels, titles, legend, free text), with the rules below.

- **Axis colour.** If `color_follows_plot` is set and the axis's layer has a visible plot that can be drawn, the colour is that of the layer's first such plot: its line colour for Line and LineSymbol plots, its symbol (fill) colour for Scatter and Column plots. Otherwise the axis line and ticks use `axis.color` and the tick labels `tick_font.color`, as before. The title uses the plot's colour only when `title_color_follows_plot` is set; otherwise `title_font.color`.
- **Side `Normal`.** As before.
- **Side `Opposite`, Y axis:**
  - main line on the right frame edge;
  - ticks on that edge, `In` pointing left into the frame and `Out` pointing right;
  - tick labels left-aligned, vertically centred on their tick as on the left side, starting `0.35 * tick font size` right of the outer end of the ticks (of the axis line for `In`/`None` ticks);
  - title rotated 90 degrees counter-clockwise like the left title (it reads bottom to top), centred on the frame height, with its baseline at `labels_right + 0.6*size + 0.72*size`, where `labels_right` is the right edge of the widest right tick label (or the outer tick end when labels are hidden). The tops of its letters then face the labels at a distance of `0.6*size`, mirroring the left title;
  - `show_opposite` and `opposite_ticks` refer to the left edge.
- **Side `Opposite`, X axis:**
  - main line on the top edge;
  - labels centred above their ticks, the bottom of the text (baseline plus descent) `0.35 * size` above the outer tick end;
  - title centred above the labels, its bottom edge `0.6*size` above them;
  - `show_opposite` and `opposite_ticks` refer to the bottom edge.
- **Side `Hidden`.** No line, ticks, labels or title for that axis, and no opposite line. Grid lines follow the grid settings as usual.
- **Closed box.** One closed rectangle is drawn only when neither axis is `Hidden`, both have `show_opposite`, and their widths and line colours are equal. Otherwise each edge is drawn as its own line with square caps, in this order: X main edge, Y main edge, X opposite edge, Y opposite edge.
- **Title text.** A title that is `None` is built from the first visible plot of the axis's own layer, so the right title comes from the right layer's first plot. A layer without plots draws no title text.

So in the DoubleY template layer 0 draws the bottom, left and top lines and layer 1 draws only the right line, with its ticks, labels and title.

### 12.5 Legend and colours

- **Graph scope.** A legend with scope `Graph` lists every visible drawable plot of every layer: layer 0's plots in plot order, then layer 1's, and so on. Rows look as before. The box is placed in its own layer's frame. It is drawn after all layers of the graph (at the end of the last layer's drawing), so that no layer's curves cover it.
- **Margins.** A legend in a corner keeps `0.8 * font size` plus the inward length of the major ticks from each frame edge. The tick length counts for every axis of the legend's group that draws ticks pointing `In` or `Both` on that edge (its main edge, or the edge across with `opposite_ticks`). For a single layer this is the rule of figure-rendering.md; on a CV-style graph with a right axis the legend also keeps clear of the right axis's inward ticks.
- **No entries.** A legend with no entries is not drawn.
- **Automatic colours** are handed out per layer from that layer's palette (`Layer::recolor_automatic`). A plot that moves between layers takes the new layer's automatic colour unless its colour was chosen by hand. In the DoubleY template the first left curve is black and the first right curve is the third Sheaf palette colour, blue (30, 80, 220), as in Origin. Unlike Origin, a second curve on the same axis gets the next colour.

### 12.6 Commands

**Project API.**
- `Project::add_right_y_layer(graph, parent) -> Result<usize, ProjectError>`: when `parent`'s group already has a right-Y member, returns it and changes nothing; otherwise appends `right_y_layer(root)` (the root of `parent`'s group), changes the root as in 12.2 and returns the new index.
- `Project::move_plot_to_layer(graph, from, index, to)`: moves the plot to the end of layer `to` and hands out automatic colours again in both layers.
- `Project::plot_columns_double_y(template, left, right)`: makes the graph from `template`, adds the right layer (the DoubleY template already has it), plots `left` in layer 0 and `right` in the right layer, applies each layer's palette, and takes the long name from the first column's workbook.
- `Project::plot_columns(DoubleY, ys)` alternates the columns as Origin does (1st, 3rd, ... left; 2nd, 4th, ... right). `Project::default_plot_columns` returns every numeric Y column for the DoubleY template, because a double-Y graph exists to show different units. The desktop app's Plot menu lists the template through `Template::ALL`, so "Double Y" works there through these two functions.

**Command line** (`sheaf plot`):
- `--y2 COLS`: comma-separated short or long names, looked up like `-y` (per file when several files are overlaid). These columns go on a right Y axis added to the chosen template's graph. The left axis gets the `-y` columns, or the default columns without those named in `--y2`. Not allowed together with `--graph`.
- A column named in both `-y` and `--y2` is an error ("a column cannot be on both axes").
- `-t double-y` without `--y2`: the Y columns are those of `-y`, or every numeric Y column of the sheet; they alternate in column order. Fewer than one column per axis is an error ("a double-Y graph needs at least two Y columns, one for each axis").
- Several files with `--y2` or `-t double-y`: each file's columns are split as for one file; the right axis lists its curves after all left curves.

**Style options** (`sheaf plot` and `sheaf export`):
- `--y2-title TEXT`, `--y2-min`, `--y2-max`, `--y2-step` (negative values are written with `=`, for example `--y2-min=-1`) set the right Y axis of layer 0's group. On a graph without one they are an error ("the graph has no right Y axis ...").
- `--x-*` and `--y-*` set layer 0. Linked layers follow its X.
- `--axis-colors plot|fixed` sets `color_follows_plot` on the Y axes of layer 0's group. When it is absent the template decides.
- `--colors`, `--labels` and `--line-width` apply to the plots of layer 0's group in legend order (layer 0's plots first). `--no-legend` hides every legend of that group. Further layers of a graph read from an Origin project (multi-panel graphs) are left alone, as before.

Examples: `sheaf plot cycles.csv -y Capacity --y2 Efficiency -o fig.png`; `sheaf plot it.txt -t cv -y "i/mA" --y2 "Q/C" --y2-title "Charge (C)" -o fig.svg`; `sheaf plot data.csv -t double-y -o fig.pdf`.

**Desktop app** (later, with ADR 0007): the Plot menu's multi-axis group with "Double-Y" (today it appears as a template in the Plot menu); a Graph toolbar button that adds a Right-Y layer; a "Left axis / Right axis" choice per plot that calls `move_plot_to_layer`, creating the right layer the first time; layer icons 1 and 2 that choose the layer the dialogs edit. Today the graph settings dialog edits layer 0 only.

### 12.7 Origin project files (for the .opj reader and writer)

Observed on one Origin 2016 project [COM; one sample each]:
- **Frame.** Both layer records store the same absolute frame (layer header `0x71`, page units).
- **Linked-layer header bytes.** The linked layer differs from its parent at header `0x0AB` (= 1, probably the parent's number, unconfirmed), `0x12D` (= 1), `0x02E` (1 becomes 9) and bit `0x80` of `0x0FC`. Turning the X link off changed none of them.
- **X link.** A straight X link shows as annotations named `X1` and `X2` in the linked layer; with the X link off, `X1` was gone.
- **Axis visibility.** In each axis group, bit `0x40` of byte `0x26` of blocks 2, 3, 4 and 5 means: block 2 primary-side tick labels shown; block 3 primary line and ticks shown; block 4 opposite-side tick labels shown; block 5 opposite line shown. For Y, primary is left and opposite is right. This corrects opj-format.md section 10.3, which calls block 4 "title (not used)".
- **Automatic axis colour.** Stored as `F7 FF` at block 3 offset `0x0F` and as `F7 FF FF FF` at `0x92` and `0x16A`; black is 0 there.
- **Right title and legend.** The right title is the text object `YR` of the linked layer. The DoubleY template names its legend object `legend` in lower case.

Reader rule, not yet implemented (the reader today reads every layer as independent, drops `YR`, and matches only the exact name `Legend`):
- a layer k > 0 with the same rectangle as an earlier root layer, whose Y group shows the right line and labels and hides the left ones, becomes a member with `link = {parent, x: Straight, y: None}`, `frame = {0,0,1,1}` and `y_axis.side = Opposite`, with its title from `YR`;
- an X group with nothing shown becomes `x_axis.side = Hidden`;
- an X group showing only the top line becomes `Hidden` too, while the root gets `x_axis.show_opposite = true`;
- automatic axis colours set `color_follows_plot`;
- the legend object is matched without regard to case.

### 12.8 Deliberate differences from Origin 2016

- The shared X range covers the data of all linked layers, so no curve is cut off by a rescale of the other layer.
- Colours increment within each layer.
- The X axes and the top line of the box stay in the X axis colour (black); only Y axes follow their plot.
- Adding a right axis gives one legend for the graph, not a second legend in the new layer.
- The legend sits inside the frame (TopRight), not above it.
- Every plot is listed (Origin's four-column run showed one fewer).
- The right layer of a CV-style graph copies the CV style: ticks in, symmetric range about zero, black axes, closed box.

Tick positions of the left and right axes are not aligned with each other (Origin does not align them either).

### 12.9 Unknowns

- The meaning of the linked layer's header bytes listed in 12.7, and where Origin stores the straight X link.
- Whether Origin's blue for layer 2 is stored in the template or follows from a colour-list rule.
- Whether Origin 2016 offers "title colour follows plot" for axis titles (it appears in current help).
- How Origin splits a selection with two X columns (documented, not run).
- Origin's distance between the right tick labels and the right title, and the right title's size (not measured; Sheaf mirrors its left-side rule).
- Why Origin's DoubleY template uses finer automatic tick steps than the Line template for the same data (X -1..11 step 2 against -2.5..12.5 step 5); related to U4.
- Whether the owner wants coloured Y axes on double-Y figures (the DoubleY template colours them; `--axis-colors fixed` turns it off) and a symmetric right axis on CV-style figures.

---

## 13. Stacked lines by Y offsets and waterfall (P0, GP-OFF-01)

Status: researched on 2026-11-12 with Origin 2016 runs through COM on our own synthetic data (scripts and outputs in `re-workspace/scratch/graph-p0/`, git-ignored), plus a read-only look at the settings and counts of the owner's stacked XRD graphs. Tags as in section 12; **[COM]** is behaviour seen in Origin 2016. The model decision is recorded in ADR 0009.

### 13.1 What Origin 2016 does

- **The idea.** "Stacked Lines by Y Offsets" (template `OffsetStackY.otp`) draws several curves in one layer, each moved up by its own amount so that they do not overlap. Only the drawing changes; the worksheet data stay as they are [W:stacklineyoffset-graph].
- **Offset modes.** The plot group of a layer has one of these modes, chosen on the Stack tab of Plot Details or with Graph > Offset Grouped Data in Layer [W:pd-dialog-stack-tab, pd-offset-tab]:
  - None.
  - Cumulative: each plot's Y values are added point by point to those of the plots before it. Stacked columns and areas use it; that is GP-COL-01, not this section.
  - Constant: a fixed step per plot; the first plot does not move.
  - Auto: offsets computed from the curve heights and a gap.
  - Individual: each plot has its own X and Y offset (set on the Offset tab, or by dragging a curve after clicking it twice).
- **Only group members move.** A plot added to the layer later outside the group kept offset 0 [COM].
- **The 2016 template uses Auto with a 10 % gap** [COM]. The current help says the template uses Individual offsets [W]; that is probably a later change.
- **The Auto rule in Origin 2016** [COM, checked on seven data sets and on the owner's files]:
  - H is the largest height (largest minus smallest Y) of any curve of the group, and the gap is 10 % of H;
  - the first curve does not move;
  - every later curve is moved up so that its lowest point lies one gap above the highest point of the curve before it, after that curve was moved.

  The current help describes a gap computed pair by pair; for the first check row of 13.11 that rule would give 141 for the fourth curve, while Origin 2016 gave 150.
- **Details of Auto** [COM]:
  - missing values are ignored;
  - a flat curve has height 0, and when every curve was flat the gap was 1;
  - offsets are added in data units, also on a log10 Y axis;
  - hidden plots keep their place, so hiding one curve does not move the others;
  - the offsets are computed again when the data change (seen after a rescale; a hidden instance without a rescale kept the stored numbers, so Origin recomputes when it redraws [INF]);
  - the X offset is always 0.
- **Axes and legend** [COM]:
  - the automatic axis range includes the moved curves (data 0..151 gave a Y axis from -10 to 160);
  - Y tick labels show the axis coordinate, which is a true value only for the first curve; there is no option to label each curve with its own values;
  - the template shows Y tick labels, a Y title that says the values are offset, and a legend with a frame;
  - the legend keeps plot order, so the lowest curve is listed first.
- **How the owner uses it.** Eleven stacked graphs of 2 or 6 XRD patterns, all Auto 10 %, all with the Y tick labels hidden, a closed box, X ticks pointing out, no legend, and the curve names typed as text next to the curves. Offsets seen: 0, 1.1, 2.2, 3.3, 4.399, 5.499 on normalised patterns, and 0, 1052, 2251, 3143, 4029, 4847 on raw counts.
- **Scripting.** `get <plot> -sy` and `-sx` read the offsets; `set -sy` and `set -sx` are refused by 2016, so Individual offsets cannot be set by script in that version [COM].
- **Waterfall.** Origin's 2D waterfall (template `WATERFAL.otp`) is pseudo-3D [COM]:
  - each curve is drawn on its own plane, the first at the back and the last at the front;
  - the planes are shifted by a layer-wide X and Y percentage (`layer -y X,Y`; the 2016 template reports -31.97 and -89.77);
  - a Z axis names the curves, and the Y axis at the right of the back plane shows raw data;
  - curves have a white fill that hides the curves behind them ("Show Hidden Lines" is off by default [W:customizewaterfall]);
  - swapping the Z axis limits reverses the order;
  - there are no offsets per plot.

  Sheaf draws a simpler 2D form (13.6).

### 13.2 Model

```rust
pub struct Layer {
    // ... existing fields ...
    #[serde(default, skip_serializing_if = "Stack::is_none")]
    pub stack: Stack,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plot_offsets: Vec<PlotOffset>,
}

#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Stack {
    None,                                             // default
    Auto { gap_percent: f64 },                        // 13.4.1
    Constant { step: f64 },                           // 13.4.2
    Individual,                                       // 13.4.3
    Waterfall { dx_percent: f64, dy_percent: f64 },   // 13.6
}

pub struct PlotOffset {
    pub column: ColumnId,          // the Y column of the curve
    pub x: f64,                    // default 0; not written when 0
    pub y: f64,                    // default 0; not written when 0
    pub follows: Option<ColumnId>, // default None; not written when None
}
```

| `stack` | stored as | meaning |
|---|---|---|
| None | absent, or `{"kind":"none"}` | no offsets |
| Auto | `{"kind":"auto","gap_percent":10.0}` | 13.4.1 |
| Constant | `{"kind":"constant","step":500.0}` | 13.4.2 |
| Individual | `{"kind":"individual"}` | 13.4.3 |
| Waterfall | `{"kind":"waterfall","dx_percent":5.0,"dy_percent":10.0}` | 13.6 |

- Both new fields have serde defaults and are not written while they have their default, so files written before this feature load unchanged and graphs that do not stack are saved exactly as before. `FORMAT_VERSION` stays 1; an older Sheaf ignores the fields and draws the curves unstacked.
- **Per-curve settings live on the layer, found by the curve's Y column** (`Layer::plot_offset`, `plot_offset_mut`, `follows`). `Plot` itself is unchanged, so the code that builds plots (importers, fits, the app) needs to know nothing about stacking, and an offset stays with its curve when plots are reordered or removed. Two plots of the same column in one layer share one entry.
- `x` and `y` are offsets in axis units. They are used only in Individual mode; other modes leave them in place, so switching back to Individual restores them.
- `follows = Some(C)` gives the curve no place of its own in the stack: it takes the offset, and the waterfall shift, of the first member whose Y column is C. A curve cannot follow its own column (such an entry is ignored). When a fit adds its curve to a layer that shows the data, and when `Project::plot_fit` makes a graph of data and fit, the fitted curve gets an entry that follows the data column.
- The renderer treats a stored number that is not finite as 0. A negative gap or step is allowed: the curves overlap, or stack downwards.

### 13.3 Members of the stack

The members of a layer are its plots, in plot order, whose Y column exists and holds numbers and which do not follow another column (13.2). Hidden plots are members. The member index k counts members from 0.

The extent of a member is taken over its plotted rows (the row range, clamped to the column length as in drawing), using only rows where X and Y are both finite; without an X column, X is the row number. `lo` is the smallest and `hi` the largest Y of those rows. A member with no such row has no extent.

### 13.4 Offsets (ox, oy) per plot, in axis units

13.4.1 **Auto.**
1. H is the largest `hi - lo` over the members that have an extent. If no member has an extent, or H is 0, H = 10.
2. gap = `gap_percent * H / 100`.
3. Go through the members in order, with `top` unset at the start:
   - a member without an extent gets (0, 0), and `top` stays as it is;
   - the first member with an extent gets (0, 0), and `top = hi`;
   - every later member with an extent gets `oy = top + gap - lo`, and then `top = hi + oy`.
4. ox is always 0.

The offsets are computed again at every drawing, so they follow the data.

13.4.2 **Constant.** Member k gets (0, k × step).

13.4.3 **Individual.** Each member gets the `x` and `y` of the layer's entry for its Y column, or (0, 0) when there is none.

13.4.4 **None and Waterfall.** Every member gets (0, 0).

13.4.5 **Followers** get the pair (and the member index) of the first member whose Y column is the followed column; without such a member, (0, 0) and no member index. Plots whose Y column holds no numbers get (0, 0) and are not drawn.

`sheaf_plot::stack_offsets(project, graph, layer)` returns these pairs, one per plot in plot order. The app will use it to copy the offsets in effect into the layer when the user switches to Individual, so that the picture does not jump.

### 13.5 Drawing

- Every use of a data point (x, y) uses (x + ox, y + oy) instead:
  - lines and symbols;
  - error bars, from y + oy - e to y + oy + e;
  - column bars, from oy to y + oy;
  - the data range for automatic axis limits (figure-rendering.md section 2, step 1).
- When an offset is 0 the value is used exactly as stored, so a layer with `stack` None is drawn exactly as before, with byte-identical SVG.
- Worksheets, analysis tools and fits always use the stored values.
- Limits, ticks and tick labels follow figure-rendering.md section 2, applied to the moved values. Tick labels show axis coordinates; no curve is labelled with its own values. Read-outs in the app show axis coordinates.
- The legend is unchanged (plot order).

Check: the SVG of an Auto-stacked graph is byte-identical to the SVG of an unstacked graph whose Y columns were shifted beforehand by the offsets of 13.4.

### 13.6 Waterfall (Sheaf's 2D form)

The waterfall is drawn only for a layer that is not linked to another layer and has no linked layers. In a graph with a right Y axis, `Waterfall` is drawn like `None` (the command line refuses that combination, 13.8).

Let n be the number of members (13.3), `sx = dx_percent / 100` and `sy = dy_percent / 100`. Let F be the layer frame in points after the label fit of figure-rendering.md section 3.

The front frame B:
- `B.w = F.w / (1 + (n - 1) |sx|)`;
- `B.h = F.h / (1 + (n - 1) |sy|)`;
- `B.x = F.x` when sx >= 0, otherwise `F.x + F.w - B.w`;
- `B.y = F.y + F.h - B.h` when sy >= 0, otherwise `F.y`;
- when n <= 1, B = F.

So the last member, moved by n - 1 shifts, ends exactly at the opposite corner of F.

Drawing rules:
- The layer geometry's `frame` is B and its `clip` is F (`LayerGeometry::clip`; in every other mode the two are equal).
- Axis limits come from the unmoved data of the visible plots.
- Axes, ticks, tick labels, titles, the legend and free text are laid out on B.
- Member k and its followers are drawn with every page point moved by `(k sx B.w, -k sy B.h)`: positive values move later curves right and up.
- Plots are drawn in reverse plot order, so the first plot is on top.
- Data are clipped to F.
- Opposite axis lines, opposite ticks and grid lines are not drawn, even when set.
- No Z axis, planes or fill that hides the curves behind are drawn.
- The label fit of figure-rendering.md section 3 is done for F. With a negative DX (or DY) the later curves move left (or down) past the front frame's Y (or X) axis and are drawn over its tick labels and title; hide them (`--no-y-tick-labels`) or use positive shifts.

Check values (F = 100, 50, 300, 200 pt; 3 members; 5 % and 10 %): B = 100, 83.333, 272.727, 166.667; member 2 is moved by (27.273, -33.333). With -5 %: B.x = 127.273.

### 13.7 Templates

- **`stacked`** (`Template::StackedLines`, label "Stacked lines (Y offsets)"): the `line` template with
  - `stack = Auto { gap_percent: 10 }`;
  - Y tick labels hidden, and Y major and minor ticks `None`;
  - `show_opposite` on both axes, without opposite ticks (a closed box);
  - the X axis and the legend as in `line`.

  This follows the owner's XRD figures rather than Origin's template, which shows Y tick labels.
- **`waterfall`** (`Template::Waterfall`): the `line` template with `Waterfall { dx_percent: 5, dy_percent: 10 }`. These numbers are Sheaf's own.

Both are in `Template::ALL`, so the desktop app's Plot menu lists them already.

### 13.8 Command line

These options work on `sheaf plot` and `sheaf export` and act on the first layer:

| option | effect |
|---|---|
| `--stack none` | no offsets |
| `--stack auto` | Auto with a 10 % gap |
| `--stack auto:GAP` | Auto with GAP percent; GAP is a finite number of at least 0 |
| `--stack step:STEP` | Constant with STEP axis units; may be negative (`--stack=step:-500`) |
| `--offsets Y1,Y2,...` | Individual; plot k gets `y = Yk`; plots after the list keep their offsets; more values than plots is an error |
| `--waterfall DX,DY` | Waterfall; DX and DY in percent, each from -100 to 100 |
| `--no-y-tick-labels` | hide the Y tick labels |
| `--no-y-ticks` | Y major and minor ticks `None` |

- `--stack`, `--offsets` and `--waterfall` exclude each other.
- Values that start with a minus sign need the `=` form, for example `--offsets=-100,0`.
- Error messages name the option and the bad value.
- `--waterfall` on a graph with a right Y axis, and `-t waterfall` together with `--y2`, are refused.
- `-t stacked` and `-t waterfall` choose the templates.

Examples: `sheaf plot xrd.csv -t stacked -o xrd.png`; `sheaf export p.sheaf --graph Graph1 --stack step:500 --no-y-tick-labels -o g.pdf`; `sheaf plot a.xy b.xy c.xy --stack auto --no-legend -o fig.svg`.

### 13.9 Desktop app (later, with ADR 0007)

- The Plot menu lists "Stacked lines (Y offsets)" and "Waterfall" today, through `Template::ALL`.
- The Graph menu gets "Offset Grouped Data in Layer" with None, Constant..., Auto... and Individual. Cumulative stays disabled until GP-COL-01 exists.
- The Plots tab gets an "Offsets" section with the mode and its numbers; in Individual mode, X and Y fields for the selected plot.
- Choosing Individual first copies `stack_offsets` into the layer's entries, so the picture does not jump.
- Hit testing in the graph view will use the moved values. Dragging a curve to set its offset is P2.

### 13.10 Origin project files

In the 515-byte curve record (opj-format.md section 10.5) [COM, byte comparisons of files saved by Origin with and without stacking, and the same pattern in the owner's files]:
- f64 at `0x1CA` is the Y offset in axis units;
- f64 at `0x1B2` is probably the X offset; it was 0 in every file seen [INF];
- f64 at `0x1BA`, `0x1C2`, `0x1D2` and `0x1DA` is 1.0 in graph curves, probably multipliers [INF];
- u8 `0x13A` = `0x30` and u8 `0x148` = 10 in every curve of an Auto-stacked layer, and 0 in a plain line graph; probably the mode code and the gap percent [INF].

The 361-byte layer header also differs at f64 `0x13D` (0.1 stacked, 0.08 plain) and `0x135` (Origin's "no value" stacked, 0 plain); their meaning is unknown.

Import rule (not yet implemented; the reader ignores these fields today, which is why the owner's XRD curves are drawn on top of each other): if any curve of a layer has a non-zero offset, set `stack = Individual` and add an entry with that curve's offsets for its Y column. This redraws the graph exactly as Origin saved it. Mapping `0x13A`/`0x148` to Auto can follow once the mode codes are confirmed. A future writer needs the same bytes.

### 13.11 Check values (Origin 2016, Auto, gap 10 %, X = 0, 1, ..., 20)

| Y of the curves, in plot order | offsets |
|---|---|
| x/2; 1000+5x; -5+x/2; x/20 | 0; -980; 135; 150 |
| x; 50-x/2; -5+x/4; 2x-10 | 0; -16; 43; 57 |
| x/2; 1000+10x; -5+x/2; x/20 | 0; -970; 255; 280 |
| first row, with the last value of the third curve missing | 0; -980; 135; 149.5 |
| first row, with the second plot hidden | 0; -980; 135; 150 |
| 7; x/2; 7 | 0; 8; 12 |
| 7; 7; 7 | 0; 1; 2 |
| 10+x; 1+x/20, on a log10 Y axis | 0; 31 |

With the second plot of the first row hidden, the visible curves span 0..10, 130..140 and 150..151, so the data range for the axis is still 0..151.

### 13.12 Deliberate differences from Origin 2016

- The `stacked` template hides the Y tick labels and ticks and closes the box (the owner's style); Origin's template shows them.
- The waterfall is a flat 2D drawing without a Z axis, planes or hiding fill, with its own default shifts, and it draws the first curve in front.
- Individual offsets are stored per Y column, not per plot.
- A fitted curve moves with its data (`follows`); Origin's behaviour for fit curves in a stacked layer was not tested.
- The automatic axis range follows Sheaf's rule (figure-rendering.md section 2), not Origin's rescale rule, so the limits differ (0..200 instead of -10..160 for the first check row).

### 13.13 Unknowns

- Origin 2016's codes for the modes other than Auto: None, Constant, Individual and Cumulative cannot be set by script in 2016 (`set -sy` is refused and there is no layer property). Confirming them needs a click test of Graph > Offset Grouped Data in Layer and a byte comparison.
- The storage of X offsets and multipliers (13.10) is inferred; the layer header differences are unexplained.
- Whether the Auto gap percent can be edited in the 2016 dialog. Only 10 was seen, so the H = 10 rule for flat curves is confirmed only at 10 %.
- Whether Origin's Auto uses a plot's display row range for the curve heights; Sheaf uses the plotted rows.
- What Origin's Data Reader shows on a moved curve (the stored or the moved value).
- Origin's exact waterfall geometry.
- Owner preferences: whether the stacked template should hide the legend (he deletes it and types the curve names next to the curves); whether he wants an option that writes each curve's name at its right end (a possible follow-up); whether hidden curves should keep their place (Origin's behaviour, adopted here).
- The current online help describes Individual offsets in the template and a pairwise Auto gap; neither matches Origin 2016, and later versions may differ.
