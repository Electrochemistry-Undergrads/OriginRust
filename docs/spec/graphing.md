# Specification: 2D graphs, graph object model and graph export

Status: draft graphing and project-compatibility specification.

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
Reference tags: **[F:path]** file reference; **[W:page]** public OriginLab documentation (origin-help, labtalk, quick-help or x-function); **[INF]** inference; **[REC]** recollection.

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
| speed mode (max points per curve) | draw decimated data only; "Speed Mode is On" watermark never exported | worksheet data ticked, 3000 points per curve; matrix data unticked, 32 x 32 (measured in Origin 2016's Size/Speed tab, 18.7a) | same |
| show axes / labels / data / frame | visibility toggles | on | same |
| layer links | X and/or Y scale linked 1:1 to another layer | off | [F:laylink.OXF] |

Helper defaults seen in X-Functions: `layarrange` gaps 5 / margins 15 (percent of page); `merge_graph` gaps 5, margins left 15 right 10 top 10 bottom 15, scale mode auto, optional panel labels A/a/custom `Graph##` (positions top-left/right inside/outside); `plotstack` vertical gap 0, margins left 15 right 10 top 10 bottom 10, layers linked with X linked 1:1 [F:layarrange.OXF, merge_graph.OXF, plotstack.OXF].

### 1.4 Axes
Four slots per 2D layer: bottom X, top X, left Y, right Y. Bottom and left are normally visible; top and right can be shown, and may share settings with the opposite axis. Each axis owns scale, ticks, tick labels, title, grid, breaks, special ticks, line (section 3). A box (frame) is made by showing line and ticks on opposite axes without labels; theme "Opposite Lines" turns that on for all axes [F:Themes\Graph\Opposite Lines.OTH]. Whether 2016 shows opposite axes by default: UNKNOWN U15.

### 1.5 Data plots and plot groups (P0)
A data plot references X column (or row index), Y column, optional error column, and a row range (default all rows). Plot-type IDs: 200 line, 201 scatter, 202 line+symbol, 203 column, 204 area, 205 high-low-close, 206 box, 207 floating bar (and floating column), 213 stacked column, 214 stacked area, 215 bar, 216 stacked bar, 219 histogram, 220 image, 225 pie, 230 "use template", 231 Y error bar, 233 X error bar [W:labtalk plot-type-ids]. `plotxy`: input range list (X, Y, optional error as third column), plot type, symbol size in points, rescale (default 1), legend (default 1), target layer (default new graph) [F:X-Functions\Plotting\plotxy.OXF].

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

The byte layout is [INF], cross-checked on "Q03 Rainbow7", which decodes to red, orange, yellow, green, blue, violet, purple. **Measured (U1 closed, 2026-11-13):** a fresh `plotxy iy:=(1,2:26) plot:=200` of 25 Y columns, exported with `expGraph type:=png path:="DIR" filename:=NAME overwrite:=1 tr1.unit:=2 tr1.width:=800` and read pixel by pixel from the legend, colours the plots with the BASE palette below in order, not with the theme list above: black, red, green, blue, cyan, magenta, yellow, dark yellow, navy, purple, wine, olive, dark cyan, royal, orange, violet, pink, white (the 18th plot is invisible), light gray, gray, light yellow, light cyan, light magenta, dark gray (24 entries; the 25th plot is black again). Lines, scatter, line+symbol and columns all use this order. Sheaf's templates copy it (`ORIGIN_COLOR_LIST` in `template.rs`); the `.opj` writer's `PALETTE` has the same 24 entries. The file "System Color List" holds 13 RGB values, role unconfirmed: (240,0,130) (250,60,60) (240,130,40) (230,175,45) (230,220,50) (160,230,50) (0,220,0) (0,210,140) (0,200,200) (0,160,255) (30,60,255) (110,0,220) (160,0,200).

**Named colour lists (T332).** Origin 2016 installs 18 lists in `Themes\Color` (Q01 Black and RGB, Q02 Basic, Q03
Rainbow7, Q04 Paired Color, Q05-Q08 Season Spring/Summer/Fall/Winter, S01 Gray Scale, S02 Cyan, S03 Green, S04 Red,
D01 Pumpkin Patch, D02 Maple, D03 Sea and Land, D04 Watermelon, D05 Blue-Greens, System Color List), each one blob
decoded as above (4 to 23 entries). Q02 Basic decodes to exactly the list measured on the area templates
(`THEME_COLOR_LIST`). Sheaf keeps them as `template::ORIGIN_COLOR_LISTS` (`color_list` finds one by name or code,
`Default` is the base-palette list of a new graph); Plot Details > Group > Line Color > Details chooses one (the
plots are recoloured; editing a colour makes the list the layer's own) and `Palette::list` keeps its name in the
project; `sheaf plot --color-list Q03` does the same from the command line. Choosing a list in Origin's dialog was not
measured: `themeApply2g` with a colour theme file left a grouped graph's colours unchanged.

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
- Dash patterns (U6 closed, measured 2026-11-13): one plot per graph (`plotxy iy:=(1,2) plot:=200`, Y constant), then `set %C -d N`, `set %C -w W`, `set %C -c 1` as three separate calls, PNG at 3200 px wide (4.145 px per pt: the page is 10.72 in), run lengths read along the line. In a plot group `set -d` is ignored, so use one plot per graph. Patterns grow with the width but not exactly linearly (below 1 pt they are relatively longer); the values are on, off, ... in multiples of the line width, mean of widths 1.5 and 2 pt: dash 5.1 4.9; dot 1.6 4.3; dash dot 6.1 2.4 1.6 2.4; dash dot dot 4.7 2.35 1.6 2.4 1.55 2.5; short dash 2.6 1.95; short dot 0.95 1.05; short dash dot 3.95 1.5 0.95 1.5. (At 0.5 pt: dash 3.5 2.35 pt, dot 1.5 2.1 pt.) Sheaf's `Dash::pattern` uses these. The older 18 12 5 12 5 12 for dash dot dot is in other units [W:options-dialog-graph-tab].
- Elements scale with the layer frame [W:quick-help/change-size-of-textlabels].
- Symbol color automatic = follows line color [W:pd-dialog-line-tab].
- Axis line color Auto = darker shade of plot fill; if no fill, of border; else follows line color [W:axesref-lineticks].
- Anti-aliasing covers plots, axes/grid and line objects; purely horizontal/vertical lines are never anti-aliased [W:options-dialog-graph-tab].

#### Thin-width dash follow-up (2026-11-15, Origin 2016)

A verified two-row vector (`newbook; wks.nCols=2; col(A)=data(1,2,1);
col(B)=col(A)*0+1; plotxy iy:=(1,2) plot:=200;`) avoids the empty-line result
from assigning a scalar Y value. Set X to 1..2, Y to 0..2, hide axes/labels,
remove the legend and set blue (`set %C -c 4`). For each of the seven dashed
styles, set `-d` and `-w` separately and read them back. The 35 PNG cases at
3200 px cover 0.5, 1, 1.5, 2 and 3 pt. A further 24 PNG cases at 1600/6400 px,
14 thin-line cases at 6400 px, six EPS/PDF pairs and 35 EMFs succeeded. The first
35 blank-line exports and six unsupported SVG exports are excluded. All data
and outputs are synthetic; only numerical drawing behavior is used.

At 6400 px, the visible on/off runs in points are approximately:

| Style | 0.5 pt pen: on, off, ... | 1 pt pen: on, off, ... |
|---|---|---|
| Dash | 3.24, 2.40 | 5.88, 4.32 |
| Dot | 1.32, 2.16 | 2.28, 3.84 |
| Dash Dot | 3.84, 0.96, 1.32, 1.08 | 6.84, 1.80, 2.28, 1.92 |
| Dash Dot Dot | 3.00, 0.96, 1.32, 0.96, 1.32, 1.08 | 5.40, 1.80, 2.28, 1.80, 2.28, 1.92 |
| Short Dash | 1.80, 0.84 | 3.24, 1.44 |
| Short Dot | 0.96, 0.36 | 1.80, 0.48 |
| Short Dash Dot | 2.64, 0.48, 0.96, 0.60 | 4.68, 0.84, 1.80, 0.96 |

These are visible lengths, not a recovered pen formula. Raster on/off lengths
shift by about a pixel when resolution changes, even when their sum stays the
same: 0.5 pt Dash at 1600 px is about 2.64/3.00 pt, versus 3.24/2.40 at 6400 px.
PDF/EMF use separate round-ended segments and quantized pen widths; for example,
PDF's requested 0.5/1/1.5 pt widths become about 0.72/1.20/1.68 pt. The complete
width rule is therefore not inferred from a single bitmap, and a global minimum
pattern length would not describe the observations.

**Sheaf decision:** the shared line-stroke conversion uses the visible 0.5/1 pt
samples above, interpolates between them, and interpolates from 1 to the existing
`Dash::pattern` times 1.5 at 1.5 pt. Below 0.5 pt it scales the half-point sample
proportionally. These intermediate/below-range rules are deliberate approximations,
not measured Origin rules. At 1.5 pt and above, the previous ratios stay unchanged.
Every existing caller of that stroke conversion (including legend samples) gets
the same absolute-point array; fill-border strokes that use `Dash::pattern`
directly retain their earlier ratios. Sheaf keeps its existing butt-ended dashed
strokes and requested pen width; Origin's round caps and backend-specific width
rounding remain fidelity differences. No file/model fields or plot geometry change.
The raster regressions compare all seven thin styles at two resolutions, with
SVG/EPS/EMF unit-conversion and normal-width/continuity controls.

### 2.2 Font, width, size defaults (U3 measured 2026-11-13)
Measured on a fresh `plotxy` with `?layer.x.label.pt`, `?layer.x.thickness`, `get %C -z` (symbol size), `get %C -w` (line width x 500), `get %C -k` (shape), and from PNG exports at 3200 px (4.145 px per pt): page 6432 x 4923 units of 1/600 in (10.72 x 8.205 in); layer frame left 17.91 %, top 11.58 %, width 68.16 %, height 71.83 % (`layer.left/top/width/height`); tick labels 18 pt, axis titles about 22 pt (cap height 66 px), legend text about 22 pt, all Arial; axis line 1.5 pt; major tick 9 pt and minor 4.5 pt long (pixels), direction out, bottom and left only, one minor tick; data line 0.5 pt (`get -w` = 250); symbol square, 9 pt, solid; legend has a black frame. Sheaf's generic templates copy all of these (page 10.72 x 8.205 in); the four measured stock legend positions follow section 2.4; other template positions remain separate. Earlier recommendation, kept for the record:
No public page and no installed text file gives them. [REC] Arial; axis titles, tick labels and legend 22 pt on the stock page; data line 1.5 pt; axis line 1.5 pt; major tick 9 pt, minor 4.5 pt; symbol 9 pt. Weak support: themes "Black on paper" and "Physical Review Letters" contain the numbers 1.5 and 8 [F:Themes\Graph]. 

### 2.3 Scale elements
Attached elements (font sizes, line and arrow widths, arrowheads, drawn objects) respond to layer resizing:
- **Scale with layer frame**: proportional; after each resize Origin updates the stored factor.
- **Fixed factor**: elements = stored size x factor; resizing leaves them alone. Text keeps its size but still moves with the frame; drawn objects attached to frame/scales are still resized; page-attached objects never change.
- Command "Fix scale factor" (Graph menu) sets factor 1 on all layers so point sizes are true; theme "Layer No Element Scaling" does the same [W:pd-dialog-size-tab; F:Themes\Graph\Layer No Element Scaling.oth].
[INF] Implement `s` per layer: effective size = stored pt x s; in scale mode recompute s from the new/old frame size ratio (which ratio, width/height/min: U7).

---

### 2.4 Stock legend anchor (T21, measured 2026-11-15)

Nine verified OriginPro 2016 exports use synthetic five-row data: Line with one/four curves,
a long Y name, a resized layer, a resized page, and two-curve Scatter, Line + Symbol, Column
and Double-Y. `plotxy iy:=(1,2:3) plot:=200|201|202|203` creates the ordinary plots; Double-Y
uses `plot:=202 ogl:=[<new template:=doubleY>]`. `doc -uw` refreshes before `expGraph` writes
a 1600-pixel PNG. Scripts, numeric results and inspected PNGs are local in
`re-workspace/scratch/t21-legend-position/`; the first five failed exports are excluded.

For Line, Scatter, Line + Symbol and Column, `legend.attach=0` (layer frame). The upper-left
anchor, in fractions of the frame from its upper-left, is approximately **(0.8223, -0.1044)**.
It is calculated from `legend.x1/y1` and the axis ranges: `x=(x1-xfrom)/(xto-xfrom)` and
`y=1-(y1-yfrom)/(yto-yfrom)`. In the ordinary 0..6 by 1..11 Line case, x1=4.933850365,
y1=12.043552036. Scatter/Line+Symbol and Column give the same fractions with different ranges.
The stock legend is opaque white: in the four-curve export the blue curve stops precisely
at its frame; a further Column export with both axis sides enabled also hides the upper
frame line behind it. Both visual checks use the same default legend. The four measured
template defaults therefore use a white background, preventing a frame line crossing the text.

The anchor remains fixed when rows or text widen the legend: it grows right/down, can extend
past the page with a long label, and four entries cross the upper frame edge. A single entry
is above the frame in Origin. The resized-layer/page PNGs follow the same relative anchor;
raw `legend.left/top` retain cached page coordinates, so they are not used to infer resizing.

New graphs from those four templates use an **Automatic**
legend position with that upper-left anchor. Existing corner/manual positions and the CV,
Double-Y and other template placements stay as saved. The existing position chooser shows and resets Automatic. Native files retain it. The `.opj` writer places its rectangle at the same anchor and reports
that raw import uses a fixed centre; a companion retains Automatic for later Sheaf text/column
changes. Text measurement, padding, row spacing, sample size,
fonts, other frame styles and manual dragging remain the current renderer's behavior; this copies
placement, and does not claim identical legend dimensions. Double-Y's smaller page-attached
legend is a separate measured difference, outside this change.

A generated `.opj` reopened in Origin 2016 with the same anchor (0.822308, -0.104355).
Both its original and a longer edited legend exported successfully and were inspected; the
longer label retained its upper-left point. Five regressions cover the measured templates,
content/columns/font and layer/page changes, the position chooser with Cancel/Undo/Redo,
native/companion saves and raw `.opj` rectangle placement.

The probe uses the documented [graphical object coordinates](https://docs.originlab.com/labtalk/guide/creating-and-accessing-graphical-objs/)
and [expGraph](https://docs.originlab.com/x-function/ref/expgraph/); measurements above come
from the installed 2016 application rather than current documentation defaults.

## 3. Axis options (P0)

### 3.1 Scale tab [W:axesref-scale]
| Option | Meaning | Default |
|---|---|---|
| from, to | axis limits | from data |
| reverse | swap limits | off |
| type | linear; log10; ln; log2; probability (norminv(x/100), valid 0.0001..99.999); probit (a tick every 1 sd, 5 = mean; the increment is ignored); reciprocal (1/x); offset reciprocal (1/(x+273.14)); logit (ln(y/(100-y))); double-log reciprocal / Weibull (ln(-ln(1-x))); custom (direct and inverse formula of x); discrete (labels from first plot's X, skips gaps) | linear |
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
| line color, thickness (pt) | color Auto rule in 2.1 | 1.5 pt, black (2.2) |
| position | % offset from default (positive away from page centre), or at a scale value | at frame |
| arrow at begin/end | shape, width, length, offset % | off |
| tick direction | theme value 1 = in, 2 = out (for major and minor separately); none and both exist, ids [INF] 0 and 3 | UNKNOWN U15 ([REC] out) |
| tick length (pt), color, thickness | minor Auto = half major | 9 major, 4.5 minor, direction out (2.2) |
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

**Rotation and position (T291, measured in Origin 2016 on 2026-11-21, `re-workspace/scratch/t291`).**
Origin's titles are text objects `XB`, `YL` (and `XT`, `YR`): `yl.rotate` is 90 and `xb.rotate` 0 by
default, and a rotation turns the title about its own middle (`yl.rotate = 45` leaves the centre of
the "B" where the upright title had it). `xb.x`, `xb.y`, `yl.y` place a title in the layer's axis units
(+3 X units and -10 Y units moved the X title right and down by those amounts on the page) and a
moved title stays where it was put. Sheaf: the Axis window's Title tab has Rotate (deg.) (Default
or a number) and Offset (pt) right and down; the title turns about its middle and then moves by the
offset (`Axis::title_rotation_deg`, `Axis::title_offset_pt`, `turned_title` in
`crates/plot/src/layout.rs`). The `.opj` reader keeps a title's angle when it is not the default; the
writer writes the angle and the offset (the moved position as the object's centre). Dragging a title
with the mouse (T335) adds the move, in page points, to that offset in one undo step ("Move axis title"), so
the title stays where it was dropped, as Origin keeps a moved title; the Title tab shows the new offset.

**Tokens inside a title (T246).** A title of Sheaf's own may hold `%(?X)` and `%(?Y)` anywhere, with
or without options after a comma (`%(?X,@LG)`, letter case ignored); when the graph is drawn each is
replaced by the title Sheaf builds for that axis from the first plot (the automatic title of the X or
Y axis), so `%(?Y) per cycle` shows `Current (mA) per cycle`. Other `%(?...)` forms and an unclosed
token are drawn as typed. The title editor shows the text with the tokens. The `.opj` reader takes a
title that is only one token as the automatic title (as before) and now keeps a title with text
around the token instead of dropping that text. **Options (T281, measured in Origin 2016 on 2026-11-21
with `yl.text$ = "[%(?Y,option)]"` and exported pictures, `re-workspace/scratch/t281`):** for a
column with long name Volt, units mV and comments cmt, Origin draws: no option, `@LG` and `@U`
`Volt (mV)` (the built title); `@LL` and `@L` `Volt`; `@LU` `mV`; `@LC` `cmt`; `@LS` and `@C` the
short name `B`; `@LA` `Volt`; `@LN` `cmt (mV)`; `@D` `Book1_B`; `@W` `Book1`; `@WS` `Sheet1`; `@WL`
and `@LP` nothing (the book had no long name); an unknown option (`@X`) `###`. Without labels, `@LL`,
`@L`, `@LU` and `@LC` give nothing while the built title, `@LG`, `@LA` and `@LN` fall back to the
short name; with comments only `@LN` gives them and `@LA` the short name. Sheaf draws the same
(`token_option_text`); a title not named by a column (Count, Row, a sampling interval) shows its
built text for every option. Tests: `axis_title_tokens_are_replaced_by_the_built_titles`, `axis_token_options_give_what_origin_2016_draws`
(`crates/plot/src/offset_axis_tests.rs`), `only_a_lone_axis_token_is_the_automatic_title`
(`crates/io/src/opj/graph.rs`).

### 3.5 Grids [W:axesref-grids]
Per direction: major and minor grid each with show, color, style, thickness; plus "opposite" line and "line at value" (for columns/bars Y=0 is on). Shipped system theme stores two line sets per axis: {color 18 = light gray, style 1 = dash, width 0.3} and {color 19 = gray, style 0 = solid, width 0.5} with ids 66 color, 67 style, 68 width; which set is major is unconfirmed (U8) [F:Themes\Graph\Dark Colors & Light Grids.oth]. Grids off by default for new graphs [REC].

### 3.6 Breaks [W:axesref-breaks] (P1)
Enable; half-length (auto = one major tick distance); 1 to 7 breaks; per break from, to, position % of axis length (automatic position even or proportional); per-break "scale after break" and "tick labels after break" (both default to follow main axis).

**Measured 2026-11-15 in Origin 2016** (axis dialog through `tools/origin-ref/origin-dialog.ps1`, PNG exports 1600 px
wide of a line plot of 1..100 against itself, axes -25 to 125; raw files in `re-workspace/scratch/axis-breaks`):
- The Breaks tab (Horizontal, Vertical): tick **Enable** (ticked), **Break Half Length** (an editable list: `<auto>`, 3, 5,
  8, 10, 15), **Number of Breaks** (None, 1 to 7, starts at None), a table with the columns Break From, Break To, Auto
  Position (a tick) and Position (greyed while Auto Position is ticked), buttons Delete and Details....
- Rows the dialog fills in on the axis -25..125: going to 1 gives 12.5..87.5 (the middle half); going on to 2 adds
  96.875..115.625 and to 3 adds 117.96875..122.65625 (each new row the middle half of what lies after the last break).
  Jumping from None straight to 2 gave 5..35 and 65..95 instead (the axis in five equal parts); after None again, 3 gave
  -10..20, 35..65, 80..110 and 4 gave -10..20, 35..50, 65..80, 95..110; 7 gave rows 15 % apart from -17.5..-2.5; going
  down from 7 to 5 kept the first five rows. The jump rules depend on what the table held before and are not copied.
- Auto Position puts break k of n at k/(n+1) of the axis: 50; 33, 67; 25, 50, 75; 20, 40, 60, 80; 17 ... 83; 13 ... 88
  (the table shows them rounded).
- Each part between breaks is drawn linearly in its share of the axis (the parts of the three-break axis had 2.8, 10.8,
  44 and 45 px per unit). The gap of the automatic Break Half Length, from the tick positions: 15.4 px of the 879 px
  Y axis with one break (1.75 %), 15.5 to 16 px with two (1.77 to 1.82 %), 18.6 px of the 1090 px X axis with one
  (1.71 %) and 18.2 to 18.4 px with two (1.67 to 1.69 %); 7.4 to 9 pt at the export's 2.07 px per point, so it follows
  the axis length rather than a fixed size. Typing 5 or 10 into Break Half Length in the scripted dialog did not take,
  so its unit is unmeasured.
- The mark: the axis line is cut between the two ends, and at each end a straight stroke as thick as the axis line
  crosses it, centred on the end, reaching about half the gap along the axis to either side (the two strokes meet
  above and below the middle of the gap) and about 8 pt to either side of the line (35 px across, stroke width
  included), rising to the right on both the X and the Y axis.
- Tick steps: with one break on -25..125 (step 50 before) every part was labelled every 5 (`layer.y.inc` became 5), the
  same on -2500..12500 (5000 to 500); on 1..9 (step 2) and -5..40 (step 10) the step stayed. With three breaks the two
  short parts at the top had their own step 0.5. No rule fitting all four cases was found; Sheaf keeps the axis step
  and offers a step after each break.

Sheaf: `Axis::axis_breaks`, up to seven (`MAX_AXIS_BREAKS`); files with the former single `axis_break` read as one
break. Each break cuts its values from..to out; with Auto Position (`auto_position`, on for new breaks, off for breaks
read from older files so that they keep their `position_pct`) break k of n sits at k/(n+1), otherwise its middle is at
`position_pct`; the gap is `gap_pct` of the axis length, 1.75 by default (`BREAK_GAP_PCT`, the measured automatic
gap). Breaks outside the limits, empty, overlapping the one before or whose gap would not fit after the gap before
are left out. Ticks inside a break go; `step_after` gives the part after a break its own step up to the next break.
Values inside a break cannot be placed. The dialog's Enable starts with one break (the middle half), Number of
Breaks adds rows by the one-at-a-time rule above and removes rows from the end, Delete removes a row, and one "Gap
(% of axis)" value stands for Origin's Break Half Length. The mark is drawn as measured, 8 pt to either side.

**Lines across a break (measured 2026-11-15, `re-workspace/scratch/break-lines`):** the page Display tab of Origin
2016's Plot Details has "Connect line across Axis Break" and "Connect line across Missing Data", both unticked.
With the default, a line+symbol plot of 0, 0.5, ..., 2 then 18, ..., 20.5 on a Y axis broken at 3.75..16.25 drew the
segment from (4, 2) to (5, 18) straight between the two points on the page but left it out over exactly the
rows of the axis line's gap (574 to 588 px in a 1600 px export, the same rows as the cut in the axis line).
Sheaf: line pieces are cut over the band of every X or Y break (`break_cut::cut_at_breaks`) unless the page's
"Connect line across Axis Break" (`PageStyle::connect_across_breaks`) is ticked; then they are drawn through.
Not measured: the ticked case (saving the probe project failed, so it could not be reopened), closed outlines
of filled runs (Sheaf leaves those whole) and Origin's behaviour with spline connections.

### 3.7 Special ticks and reference lines (P1/P2)
Custom tick positions and labels per tick. Reference lines are not documented for 2016; emulate with `addline` objects.

**Additional Lines (Grids tab, measured 2026-11-14 in Origin 2016's axis dialog, `re-workspace/scratch/addlines`):**
the Grids tab's last group, "Additional Lines", has `Opposite` (a check box) and a check box with a value box,
labelled `Y =` on the page of the vertical grid lines (the X axis) and `X =` on the page of the horizontal ones
(the Y axis); the value starts at 0 and the box is grey until ticked. Exported pictures: `Opposite` on the X axis
page draws the X axis line along the top edge (as the Line and Ticks tab's top line); `Y =` draws a straight line
across the whole layer at Y = 0, `X =` one at X = 0, both solid and in the axis line's colour and weight. Not
measured: a value other than 0 (typing into the box did not take in the scripted dialog), whether the line hides
with a hidden axis, and its order against the data.
Sheaf: `Axis::additional_line` (`Some(value)` on the axis whose page has the box); the line is drawn with the axis
lines in that axis' width and colour, and only when the value lies inside the other axis' range; `Opposite` on the
Grids tab is the same setting as the opposite line of Line and Ticks. `.opj` saves warn that the line was not written.

---

### 3.8 Scale types other than linear and log10 (measured)

Measured in Origin on a Y axis by setting `layer.y.type`, then `layer -a` (Normal rescale, margin 8 %) and reading
`layer.y.from`, `.to`, `.inc`, with pictures of the axis exported to find the tick positions (raw outputs are local
only). LabTalk type numbers: 1 linear, 2 log10, 3 probability, 4 probit, 5 reciprocal, 6 offset reciprocal, 7 logit, 8 ln,
9 log2, 10 Weibull (the file byte of opj-format.md 10.2 is one less); 11 and higher do nothing. Rule common to every scale:
the axis draws the transformed value (`forward` below) along a straight line, increasing upwards (a reciprocal axis has the
larger values at the top). Limits are in data units. The margin is 8 % of the span of the transformed data on each side,
then the limits are rounded outwards as below.

| Scale | Transform | Valid data | Automatic limits | Ticks |
|---|---|---|---|---|
| ln, log2 | ln x, log2 x | x > 0 | with `b` the base: lower = `floor(x / b^k) b^k`, `k = floor(log_b x)`; upper = next tenth of `b^k`, `k = floor(log_b x) + 1` (the same rule as log10, giving its 1..9 ticks). Data 1..9 on ln gives 0.73576..12.0513; 1..1000 gives 0.36788..1788.57; on log2 1..9 gives 0.5..11.2, 1..1000 gives 0.5..1843.2 | `inc` = 1 (step counts powers). Majors at `b^e`, `e = floor(log_b from) + j inc` (so with `inc` 2 and from 0.01 on log2: odd exponents). Labels with up to 5 decimals, trailing zeros dropped (0.36788, 2.71828, 1096.63316, 2980.95799; log2: 0.03125, 0.5, 2048). Minor ticks: the skipped powers when `inc` > 1 (none seen otherwise) |
| reciprocal | -1/x | x > 0 | margin in the transformed value; limits rounded outwards to multiples of half the step; a lower limit of 0 becomes 1. Data 1..9 gives 1..25, inc 2; 10..100 gives 1..360, inc 20; 2..3 gives 1.75..3.25, inc 0.5. The measured increment rule and its limits are below (57 positive ranges) | majors at multiples of `inc` in data units; one minor tick (`minorticks` = 1) in the middle of each gap in data units |
| offset reciprocal | -1/(x + 273.14) | x > -273.14 | as reciprocal. Data 0..100 gives -25..125, inc 50; 20..300 gives 0..350, inc 100; 5..40 gives 0..45, inc 10 (step: nearest of 1, 2, 5 x 10^k to span/3, fits 3 of 3) | as reciprocal |
| probability | norminv(p/100) | 0 < p < 100 | margin in z; then the list entry at or below (above) the limit. 1..99 gives 0.1..99.9; 5..60 gives 2..70; 10..90 gives 5..95; 0.1..99.9 gives 0.01..99.99 | list 0.01, 0.1, 0.5, 1, 2, 5, 10, 20, 30, 40, 50, 60, 70, 80, 90, 95, 98, 99, 99.5, 99.9, 99.99 in percent; every `inc`-th entry counted from the first (0.01) is major, `inc` 3 by default (majors 0.01, 1, 10, 40, 70, 95, 99.5; a lower limit of 0.1 is labelled as well); the lower limit is always labelled; entries in between are minor ticks |
| probit | norminv(p/100) + 5 | 0 < p < 100 | margin in z, then rounded to the nearest whole sd. 1..99 gives 0.135..99.865 % (probit 2..8); 5..60 gives 2.275..50 (3..5); 10..90 gives 15.87..84.13 (4..6, inside the data) | a major tick and label at every whole probit (labels 2..8), no minor ticks. `inc` reads 3 after a rescale (as for probability) but is not used: inc 3, 1 and 2 draw the same axis (T263, Origin 2016 pictures of data 0.5..80, 2026-11-20) |
| logit | ln(p/(100-p)) | 0 < p < 100 | as probability, within 0.1..99.9. 1..99 gives 0.1..99.9; 5..60 gives 1..75 | list 0.01, 0.1, 1, 5, 10, 25, 50, 75, 90, 95, 99, 99.9, 99.99; `inc` 1 by default; with `inc` 2 the labels are 0.1, 1, 10, 50, 90, 99 |
| Weibull | ln(-ln(1 - p/100)) | 0 < p < 100 | as probability. 1..9 gives 0.5..50; 0.1..0.9 gives 0.01..5; 5..99 gives 1..99.9 | list 0.001, 0.01, 0.1, 0.5, 1, 5, 10, 50, 90, 99, 99.9; default `inc` 2 for limits 0.5..50, 1 for 0.01..5 and 0.001..5, 3 for 1..99.9 (ours: 1 below span 10, 2 below 60, else 3) |

**Reciprocal increment follow-up (Origin 2016, 2026-11-14).** Generated 101-row ramps,
Normal rescale and 8% margin, with idle pauses and repeated-rescale controls, give the following
empirical model. Start with the **linear** increment for the original data span (section 4,
including its one-decade exception). Apply the margin in `-1/x` coordinates and invert to raw
limits `a..b`. If `(b-a)/inc > 20`, replace the increment with `b/10` rounded **up** through
`1, 2, 5` times powers of ten. Test this before rounding the limits; then round them out to
multiples of half the final increment, replacing a nonpositive lower limit with 1.
This is an inference from the measurements, not a documented internal Origin algorithm.
Origin's [major-increment FAQ](https://docs.originlab.com/quick-help/why-does-major-axis-increment-not-update)
documents a default numeric tick limit of 20; Sheaf does not expose Origin's global tick-limit setting.

All 57 recorded ranges agree in increment and extent, including independent holdouts and
promotion boundaries: 1000..1001 gives 999.75..1001.25, inc 0.5; 2..16.9 gives 1..42, inc 2,
whereas 2..17 gives 1..42.5, inc 5; 1..12.7 gives 1..200, inc 20, whereas 1..12.8 gives
1..250, inc 50. Preserve floating-point comparisons before nice-step rounding at decade
boundaries (1..12 promotes to 20). Fixtures: `crates/core/tests/reciprocal_autorange.rs`;
`crates/plot/tests/reciprocal_autorange.rs` checks resolved ticks/labels and explicit-step override.
Raw scripts/results are local in `scratch/reciprocal-autorange` (`batch`, `settled`, `holdout`,
`count`, `promotion`). One measured Origin case, 0.04..0.48, returned reversed limits 4..1;
Sheaf keeps extent 1..4 and leaves direction to the explicit Reverse control. Constant data,
projections reaching infinity, margins other than 8%, and global tick-limit overrides remain
unmeasured; their existing fallback/parameter handling is retained. Offset reciprocal has its
separate unchanged rule above.

Implemented in `sheaf_core::scales` (transforms, limits) and `crates/plot/src/ticks_scales.rs` (ticks, labels); the
fixtures are the tests of those two files. Not measured: minor tick counts beyond the pictures, the effect of
`minorticks` on the list scales, negative data on the reciprocals, how date ticks work on these scales.

## 4. Automatic scale and tick algorithm

**Observed:** rescale combines data min/max, the rescale margin percent, rounding to the tick increment and the minor tick count; the result is "approximately" min-margin..max+margin and lands on tick positions. With margin 0 and 0 minor ticks, Normal rescale gives exact data limits [W:axesref-scale]. The exact increment rule is not documented.

**Measured in Origin (2026-11-12 and 2026-11-13).** Probes: `tools/origin-ref/autoscale*.lines.txt` (56 cases) and `autorange-*.lines.txt` (about 600 more); raw outputs are local, in `re-workspace/scratch/axis-range2/` (`lin*.out.txt`, `log*.out.txt`, `col*.out.txt`, `mar.out.txt`). The numbers are the fixture `crates/core/tests/data/origin_axis_range.json` (464 linear and 137 log cases), tested by `crates/core/tests/origin_axis_range.rs`; the rule is `crates/core/src/autorange.rs`, used by the plot crate and the `.opj` writer. For a new line graph (`plotxy`, the default template) with Y data from `lo` to `hi` (`span = hi - lo`) the Rescale Margin is 8 % (`layer.y.rescaleMargin`):

**Linear axis.**
- The minor tick count is 1 in every case, so `h = step / 2` is the minor spacing.
- Limits: `from = floor((lo - 0.08 span) / h) * h`, `to = ceil((hi + 0.08 span) / h) * h`. So the limits need not be multiples of the major step: 0..19.2 gives -2..21 with step 2. Ticks are at multiples of the step (0, 5, 10 for 0..10), the limits are minor ticks.
- Major step: let `E = 1.16 * span` and bring `E` into `[1.05, 10.5)` by powers of ten (`b` the power, `x = E / b`). Then the step is `0.5 b` for `x < 2.25`, `b` for `x < 4.5`, `2 b` otherwise. In terms of the span in one decade: 0.2 up to 0.9052, then 0.5 up to 1.94, 1 up to 3.88, 2 up to 9.05, 5 up to 10.34 (all times powers of ten). **One exception, measured but not understood:** for `E` from 12 to 40 (span 10.35 to 34.48) the step is 2, in that decade only (the same scaled spans in the decades 100 and 1000 or 1 and 0.1 follow the rule). All 464 cases match, including the 56 of the first probe.
- The step does not depend on the data offset (100..102.5 and 0..2.5 agree), only on the span; it does not follow the Rescale Margin either: data 0..10 give step 5 at margins 2, 4, 8, 12, 16 and 20 (the only deviation: 0..30 at 20 % gives 5 instead of 2). Margin 0 gives the data limits exactly (0..10, step 5), no rounding to `h`. Margins above 0 round as above (margin 4 on 0..30: -2..32).
- A script that sets `layer.y.from`/`to` leaves the increment where it was (it stayed 5 for all 23 ranges tried); Sheaf recomputes the step for fixed limits with its older rule (`nice_step(span / 5)`), which was kept.
- Column plots (`plot:=203`): the base end gets no margin and the span runs from the base: data 1..4 give 0..4.5 with step 1, 1..4.5 give 0..5 with step 2, 1..10 give 0..12.5 with step 5, 1..100 give 0..125 with step 50. This is the rule above with the margin on one side and `E = 1.08 * span`. Data that cross zero follow the two-sided rule (-5..3 gives -6..4, step 2). All-negative columns mirror the positive ones only roughly (-1..-4 gives -4.5..0, step 1, but -1..-4.5 gives step 1 where the mirror would be 2, and -1..-40 gives -50..10): only the positive case is implemented, the negative one uses the mirror.
- Date axes and axes with a fixed major step keep the older rule (round times; limits rounded to the step with no margin). Axes with text tick labels from a column keep the older step so that every row has a label.

**Log axis** (`layer.y.type = 2; layer -a`; 137 cases). The margin is 8 % of the span in log10 units, taken on each side: `lo' = lo * 10^(-0.08 s)`, `hi' = hi * 10^(0.08 s)` with `s = log10(hi / lo)`. The limits are not whole decades (data 2.06..8 give 1.7778..9, 1.5..150 give 1..300). By `s`:
- `s > 5` (5.079 and above; exactly 5.0 is in the next class): whole decades outside `lo'..hi'` (1..1e6 gives 0.1..1e7).
- `0.92 <= s <= 5` (0.903 is in the next class, 0.929 here): the next tick of the form `k * 10^d`, `k` = 1..9, outside `lo'..hi'`: 1..1000 gives 0.5..2000, 10..1000 gives 6..2000, 1..10 gives 0.8..20, 3..30 gives 2..40. Major ticks are at the powers of ten, minor ticks at 2..9 (`minorticks = 8`).
- `s < 0.92`: linear ticks. The limits are `floor(lo' / g) * g` and `ceil(hi' / g) * g` with `g = inc / 9` (`inc` the major step, `minorticks = 8` so `g` is the minor spacing), and `inc` is taken from `E = hi' - lo'`: the table of the linear axis (without the exception) for `E < 1.45`; 1 up to `E = 20.4`; above that `0.5 b` for `x < 4.6`, `b` for `x < 9.3`, `2 b` beyond (`b` the power of ten of `E`, `x = E / b`). Origin labels the multiples of `inc` inside the range (2..9 for data 2.06..8; 20, 30, ..., 60 for 20..60; 1, 1.2, 1.4 for 1..1.5), no minor tick labels. The table matches every narrow case of the fixture; between `E` 1.45 and 20.4 the step stays 1 where the linear table would give 2 or 5, and the boundary between 4.5 and 4.6 and between 9.0 and 9.3 are narrow intervals, not pinned.
- Fixed limits with less than a decade (`log10(max / min) <= 1`, the `@TL` rule of section 3.3) get linear ticks in Sheaf too, with `inc` from the same table for `E = max - min`; Origin's choice there was not measured.

**Still not measured:** the margin rule on log axes with a margin other than 8, all-negative columns, the step of a linear axis for margins above 16 %, `layer.y.rescaleMargin` on column plots, and the X axis of line graphs with text labels.

Sheaf's own earlier rule (`span / 5`, rounded out to the step, no margin) is gone for `AutoRange::Nice`; `Tight` and `SymmetricZero` (the CV rule) are unchanged.

**Previous proposal (superseded, kept for the other axis kinds; INF):**
1. span = max - min (guard span 0).
2. wanted about 5 to 8 major intervals; raw = span/6; step = smallest of {1, 2, 2.5, 5} x 10^k that is >= raw.
3. from = floor((min - pad)/step) x step; to = ceil((max + pad)/step) x step; pad = margin% x span.
4. minor count default 1.
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
| Step horizontal centred | 13 | horizontal to the middle between two X values, vertical, horizontal on (Sheaf T252; measured T268) |
| Step vertical centred | 14 (measured) | vertical to the middle between two Y values, horizontal, vertical on (Sheaf T252; measured T268) |
| Bezier | 15 | groups of 4, passes through 1st and 4th; leftovers unused |
**Measured in Origin** (line graphs, `set %C -l N`, PNG export, curve read pixel by pixel; seven points X 0..6, Y 0 3 1 4 1 5 2):
- **Spline (9)** is a *natural* cubic spline y(x) (zero second derivative at both ends): rms 0.5 px against the picture; not-a-knot, clamped, Akima and PCHIP are 10 to 38 px off. It is drawn only when X is strictly increasing and the plot has at most 900 points (900 splined, 901 straight, also when zoomed in to show 30 points; the count is all points of the plot, not the visible ones). Decreasing X, repeated X or more than 900 points: the whole plot is drawn with straight segments in row order. Two or three points work. Between points the curve is drawn as chords, with no inner points when a segment is shorter than about 4 px on screen (dense data therefore looks straight; at about 7 px a segment gets one inner point).
- **B-spline (8)** is a uniform cubic B-spline over the points with the first and last point repeated three times, so it starts and ends at the end points and winds around the others: rms 0.4 px. It is parametric, so any X order works (loops appear when X goes back).
- **Bezier (15)** is a cubic Bezier over points 0-3, then 3-6, 6-9 and so on (groups share their end point); leftover points are not drawn (5 and 6 points draw only the first group, 8 points end at the 7th point, 10 points at the 10th). Standard cubic Bezier formula, rms 1.5 px, largest error 3.8 px at the cusp where two groups meet (Origin draws fewer chords). Fewer than 4 points: not measured, Sheaf draws straight.
- Raw probes and images: `re-workspace/scratch/spline-connect/` (local). Not measured: SplineStep resolution dependence, 900 limit for B-spline and Bezier (Sheaf applies none), the `.opj` file codes of 8/9/15 (Sheaf reads 8 as B-spline, 15 as Bezier, 5 to 10 else as Spline, writes 5, 8, 15).
- **Centred steps (13, 14)** (measured 2026-11-20, T268, `set %C -l N` and PNG exports of the points
  (0,0), (1,2), (3,1), (4,4), (7,3), `re-workspace/scratch/t268`): 13 steps at the X midpoints 0.5, 2,
  3.5, 5.5, starting at the first point and ending at the last; 14 steps at the Y midpoints the same
  way (vertical first). Codes 16 and 17 drew nothing and a straight line. Sheaf: `Connect::
  StepHorizontalCenter` / `StepVerticalCenter`, read and written as 13 and 14.
- **2 and 3 Point Segment (2, 3)** (measured 2026-11-15, T71): Plot > Line + Symbol > 2 Point Segment and
  3 Point Segment make a line + symbol graph whose plot has line connection 2 (3): points 1-2, 3-4, ...
  (1-2-3, 4-5-6, ...) are joined and the groups are not; with 9 points the 9th stands alone (2) and the
  three groups fill exactly (3). Sheaf: `Connect::Segment2`/`Segment3`, templates `segment2`/`segment3`,
  the Plot Details and plot-settings connect lists, groups counted from the first point of each piece of
  line (missing data starts a new piece); `.opj` codes 2 and 3 (LabTalk's codes; the file field is
  taken to use the same codes, not checked in a file).
Also: connect begin and end (closes; off), connect within subset (on for line plots), Akima/modified Bezier (later versions).

### 5.3 Line options
| Option | Meaning | Default |
|---|---|---|
| style | 0 solid, 1 dash, 2 dot, 3 dash dot, 4 dash dot dot, 5 short dash, 6 short dot, 7 short dash dot | solid |
| width | points | 0.5 (2.2) |
| color | palette index, RGB, automatic (group), none | group |
| transparency | 0 opaque .. 100 clear | 0 |
| fill under curve | enable; base line (auto, bottom, top, left, right axis, value, extra line); fill to another plot; above/below colours; pattern | off |
| symbol/line gap | % of symbol diameter, max 256; global option | off |
| drop lines | horizontal and vertical, with colour, style, width | hidden |

### 5.4 Symbols [W:labtalk list-of-symbol-shapes, pd-dialog-sym-tab]
Shapes: 0 none, 1 square, 2 circle, 3 up triangle, 4 down triangle, 5 diamond, 6 plus, 7 cross, 8 star(*), 9 h-bar, 10 v-bar, 11 numbers, 12 capital letters, 13 lowercase letters, 14 right arrow, 15 left triangle, 16 right triangle, 17 hexagon, 18 star, 19 pentagon, 20 sphere; 56 data markers, 58 X-position marker lines.
Interiors: 0 solid, 1 open, 2 dot centre, 3 hollow, 4 + centre, 5 x centre, 6 - centre, 7 | centre, 8 half up, 9 half right, 10 half down, 11 half left. Hollow lets objects beneath show; open hides them unless fill is none.

**Measured in Origin 2016 (T323, 2026-11-21; `tools/origin-ref/symbols.lines.txt`, PNGs exported with `expGraph`,
plots ungrouped with `layer -gu` because a plot group gives every member the first one's shape and interior).**
- Shapes 1-20 are drawn as the list says: 9 and 10 are a horizontal and a vertical stroke as wide as the symbol;
  11 writes the row number (row 11 shows "11"), 12 and 13 the row's capital and small letter (row 12 "L", row 13
  "m"); 15 and 16 are triangles pointing left and right; 17 a hexagon with a point up; 18 a five-pointed star; 19 a
  pentagon with a point up; 20 a shaded sphere (a lighter spot to the upper left). **14 (right arrow) drew nothing.**
  The legend sample of 11 is "1" and of both 12 and 13 "a".
- Interiors on a circle, square, triangle, diamond and hexagon: Open is filled white and hides a yellow layer behind it,
  Hollow is not filled. Dot centre is a small dot; + centre and - / | centre reach the edge; x centre reaches the
  corners of the symbol's square (beyond a circle or diamond). The halves fill that half of the shape (cut through
  the centre) in the edge colour and the rest white. `set -cf` (fill colour) changed nothing that could be seen.
- The .opj curve record keeps the shape number at 0x17 and the interior number at 0x18 (opj-format 10.5).

**Sheaf (T323).** `SymbolShape` has the 19 drawn shapes (`SymbolShape::ORIGIN` maps Origin's numbers; 14 is read as a
square with a note), `SymbolStyle::interior` one of the 12 `SymbolInterior`s (`None` keeps the older `filled`
switch: solid, or an unfilled outline, so earlier files draw as before) and `SymbolStyle::fill_color` the colour of
the open part (`None` white). Plot Details > Symbol lists every shape and interior and has Fill Color. A file
Origin wrote reads every shape and interior; Sheaf writes them back the same way. The sphere's gradient is drawn as
two lighter discs (the scene has no gradients); the fill colour other than white is Sheaf's reading of the dialog,
not measured. The Symbol Type increment still steps through the first 8 shapes; a first plot with a shape beyond
them keeps it.
Other: size (pt, default 9, shape square, solid), edge thickness (2), colour automatic = line colour, fill colour (global default), size from column with scaling factor and formula (`s`, `s1` min, `s2` max), user bitmap/SVG symbols (P2).

### 5.5 Error bars (P0)
Error column attached as third plot column; types 231 (Y) and 233 (X). Template ERRBAR exists. Options: direction plus/minus/both, cap width, line width/colour/style. Measured defaults and the model are in section 14.13 (cap width 8 pt whatever the symbol size, Y error from the third plot column, the automatic range reaches the ends of the bars).

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
**Measured in Origin 2016 (T339, 2026-11-21, `re-workspace/scratch/t338`):** `string s$ = "%(1,@opt)"` on a graph window
whose plot 1 is column B (long name Volt, units mV, comment cmt; book long name BookLong) and plot 2 column C (no
labels): no option `cmt` / `C` (comment's first line, else long name, else short name); `@LL` and `@L` Volt / empty;
`@LU` mV / empty; `@LC` cmt / empty (not the fallback the table above says); `@LS` and `@C` B / C; `@LG` and `@U`
`Volt (mV)` / C; `@LA` Volt / C; `@LN` `cmt (mV)` / C; `@LM` cmt / C; `@D` Book1_B; `@W` Book1; `@WS` Sheet1; `@WL`
BookLong; `@LP` empty; `@R` `[Book1]Sheet1!Col("Volt")[1:5]` / `[Book1]Sheet1!Col(C)[1:5]`; `@X`, `@LD` (without a
number), `@N` `###`. These are the axis-title answers (T281) plus `@LM` and `@R`. Sheaf draws them in legends and
free labels with `%(n, option)` (`layout::legend_name`); the combined form `%(1,@(...))` is not read.

Combine: `%(1,@(@LL(@LU)))` = long name followed by (units). Other legend kinds: categorical, box-chart components, point-by-point. Legend symbols not tied to a plot: `\L(O Shape:=..., ...)` (P2) [W:legend-substitution-notation].

### 6.2 Which label is used; translation modes
Documented default: the Y column's comments row supplies the text, switchable by the translation mode [W:quick-help/customize-axistitle]. `legendupdate` modes: comment first line (`@LM`, default, index 0), data range (`@R`), long name and units (`@U`), user parameter 1 (`@LD`), long name (`@LA`), short name (`@LS`), parameter 1 (`@LP`), custom format [F:legendupdate.OXF]. For CHI files with empty comments choose long name or short name [INF].

### 6.3 Update modes [F:legendupdate.OXF]
Scope: whole page, active layer, specific page, specific layer. Mode: reconstruct (rebuild from scratch) or update (keep manual text edits). Legend: one per layer or one for whole page. Entry order ascending/descending; plot order within layer ascending/descending. Hide entries for hidden plots (default on); hide entries for fit curves (default off). Which listed item is the default of mode and order: U10. A renamed legend becomes static.

### 6.3a What Sheaf does (2026-11-12)
`Legend::text` holds the text as above (`None` is the automatic `\l(n) %(n)` per plot). Supported: `\l(n)` / `\L(n)` and `%(n)`, `n` a plot number or `l.p`, any literal text with the markup of section 7, one entry per line, `Legend::columns`. `%(n,@...)` options are accepted and ignored. Reconstruct Legend sets the text back to automatic. The text is written to .opj as it is and a text that is not the automatic one is read back; the number of columns has no place in the file and is named in the save notes.

### 6.4 Frame, position
Frame tab (none, line, shadow...), position by anchor corner, size, arrange vertical or horizontal, multi-column, line spacing %, `\sep:nn` gaps. Default frame and position of the stock template: UNKNOWN U11 ([REC] frame none, top-right inside layer). Sheaf's templates keep frame none. Legends that Origin saved in the owner's projects all have a black frame on a white background, so the .opj reader sets `Legend::frame` from the file (opj-format.md 10.8: the frame colour slot is `none` or not) and the legend is drawn with its frame in SVG, PNG, PDF, EPS and EMF; Plot Details > Legend has "Draw a frame". The kind of frame (shadow, marble) and the background colour are not modelled.

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
| `\cN(text)` | indexed or RGB-tagged colour (7.2-7.3) |
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

### 7.1 Sheaf underline and strikethrough rendering (T58 graph text)

The first `T58-graph-text-formatting.md` rendering slice (2026-11-15,
`codex/text-decorations`) draws `\u(text)` and `\s(text)` using shared scene paths. The
[Origin escape reference](https://docs.originlab.com/origin-help/escape-sequences/) defines
these two operations. Nested decorations inherit bold/italic, size and baseline shifts and
end with their own closing parenthesis. In `\u(a\+(2)b)`, the raised, smaller `2` has its own
raised, thinner underline. Titles, free labels, legends and tick-label prefixes/suffixes use
the same placement function. A single decorated run retains the renderer's original anchor.

OriginPro 2016 controls are local in `scratch/text-decorations/origin-verified.lines.txt` and
`origin-facts.json`. A successful 3200 x 2449 PNG export shows six 32 pt Arial cases: plain,
underline, strike, both, a decorated superscript, and 90-degree text. The `ABCdef` underline
covers 456 pixels and is 10 pixels thick; the strike covers 456 pixels and is 7 pixels thick.
The rotated underline is vertical and 10 pixels thick. Nested script underlines move with
their glyphs. Initial invalid/timeout export attempts and the unavailable SVG export are
excluded from these observations. The successful export uses a fresh synthetic data graph.

System-font layout reads each resolved face's OpenType `post` underline and `OS/2` strikeout
metrics, cached by face (including bold/italic). Their positions describe the stroke's top;
the scene converts to a stroke centre and a downward Y axis, then scales and rotates with
the run ([post](https://learn.microsoft.com/en-us/typography/opentype/spec/post),
[OS/2](https://learn.microsoft.com/en-us/typography/opentype/otspec190/os2)). On this machine,
Arial has 2048 units/em: underline top -217, thickness 150; strikeout top 530, thickness 102.
These thicknesses agree with the observed raster widths after rounding. Pixel differences
from native font rasterization, baseline rounding and fallback fonts remain.

For a missing font table or a fixed-metric measurer, Sheaf uses fallback em values:
underline centre 0.14 below baseline/thickness 0.07; strikeout centre 0.23 above/thickness 0.05.
These rounded fallback values are Sheaf choices. Existing text measurers can retain their
width-only implementation through default metric methods. Every backend draws the same
decoration paths: screen/PNG/PDF through SVG, and direct EPS/EMF. Tests cover nesting limits,
anchors, rotation, script scaling, font style/colour, title/legend/label/tick placement and
export. The SVG golden includes text; the PNG golden isolates stroke geometry so font
installation differences do not change it. This slice adds no model fields. Per-run font
switching, combined scripts, editor/toolbar formatting and whole-object underline
remain open in the graph-text brief.

### 7.2 Indexed text colours (T58 graph text)

`codex/text-run-colors` (2026-11-15) adds `\cN(text)` for the original indexed colour list.
For example, `\c2(red \c4(blue) red)` restores red after the nested blue span. A closing
parenthesis restores the surrounding span or label font colour. Explicit indexed colours
are opaque; uncoloured text retains the label's colour/opacity. Bold, italic, Greek, size and
script styles compose with colour; underlines and strikeout strokes take their run's colour.
Single-run glyph anchors and rotations are retained.

The [original Origin list](https://docs.originlab.com/labtalk/ref/list-of-colors/) has 24 entries,
starting black, red, bright green, blue, cyan and magenta. Index 9 is navy (0,0,128), 14 is royal
(0,0,160), 18 is white and 24 is dark gray (64,64,64). This is independent of a graph's plot
colour cycle. OriginPro 2016 measurements in `scratch/text-run-colors/` use 33 palette controls
and 12 separate boundary controls (`palette-vector.lines.txt`, `boundaries.lines.txt`). Exact
EMF text-colour records confirm every RGB triplet, including white. Native label reference
points establish their order; the exporter draws labels in reverse creation order. The PNG
capture is useful for appearance, but its current quantization changes channel values and is
excluded from exact RGB fixtures.

For positive untagged numeric indices, Origin uses the low byte as a one-based palette
position; byte values outside 1-24 give black. Independent vector controls confirm 258 red,
280 dark gray, 9999 orange, and black for 25, 99, 255, 256, 257, 281, 512, 1000 and 65535.
Leading zeros work (`\c0002` is red). Sheaf accepts at most ten decimal digits, checks u32
overflow. Untagged values through 0x01000000 use the palette lookup; the boundary itself
is black. RGB-tagged values follow 7.3; other special-colour encodings and signed codes
remain literal text pending separate support.

Zero retains Sheaf's previous neutral style and preserves its enclosed text. Origin's `\c0`
and `\c00` controls instead displayed only a literal `\c` fragment; that invalid-index
behaviour is excluded from the supported subset. Literal `\v(...)`, malformed escapes and
nesting limits retain their existing rules. No global palette/settings or persistent model
fields are added. The `text_color_tests` cover the native palette and boundary fixtures,
restoration, inherited opacity, style/script combinations, anchors, rotation, decoration
colours, overflow/literal cases, SVG/PNG goldens and EPS/EMF/PDF output. The PNG golden isolates
decoration strokes from installed glyph fonts and checks every RGB value directly.


### 7.3 RGB text colours (T58 graph text)

`codex/text-run-rgb` (2026-11-15) extends `\cN(text)` to RGB codes without adding
model fields or changing global palettes. The [Color function reference](https://docs.originlab.com/labtalk/ref/color-func/)
distinguishes internal colour codes from plain RGB decimals. OriginPro 2016 scalar controls
confirm `Color(r,g,b) = 0x01000000 + r + 256*g + 65536*b`. Text escapes use a **one-based**
colour code: add one to that return value to reproduce exactly the requested RGB. Using the
return value verbatim instead subtracts one from the entire 24-bit colour, including borrows
between channels; it does not merely clamp the red channel.

The independent native captures in `scratch/text-run-rgb/` (`rgb.lines.txt` and
`rgb-boundaries.lines.txt`, their scalar outputs and EMF exports) establish these cases:

| Escape N | EMF text RGB |
|---|---|
| 16777216 | (0,0,0), palette/tag boundary |
| 16777471 | (254,0,0) |
| 16842496 | (255,254,0) |
| 33488896 | (255,255,254) |
| 33554431 | (254,255,255) |
| 25249549 | (12,71,129) |
| 22426642 | (17,52,86) |
| 22352368 | (239,17,85) |
| 16974337 | (0,2,3) |
| 18890880 | (127,64,32) |
| 16777217 / 16777218 | (0,0,0) / (1,0,0) |
| 16842497 / 33488897 | (0,255,0) / (0,0,255) |
| 33554432 | (255,255,255) |
| 25249550 / 22426643 / 22352369 | (13,71,129) / (18,52,86) / (240,17,85) |

Thus the supported RGB range is 0x01000001 through 0x02000000 inclusive: subtract one,
then decode the low three bytes as R, G and B. The preceding 0x01000000 control is black;
0x02000001 produced black natively but belongs to another tag and stays literal in Sheaf.
Signed Auto/None and other special tags remain unsupported. Existing decimal-digit/overflow,
literal-wrapper and nesting limits apply. Native reference coordinates identify each control;
EMF creation order is reversed and PNG quantization is excluded from these numeric fixtures.

The native `\c25249549(a\c2(b)c)d` control returns to (12,71,129) after red `b`, and to
the label's (200,100,20) for `d`. Sheaf uses the same existing run-colour path for glyphs,
underline and strikeout, including nested font/script styles and rotated text. Explicit RGB
is opaque, while text outside the span retains its parent's alpha. Seven focused regressions
cover the measured values, boundaries, nesting, glyph placement and EPS/EMF/PDF/SVG output;
new SVG and font-independent stroke PNG goldens also check the channel values directly.

### 7.4 Whole-object Format toolbar (T58 slice 3)

The shared Format bar now targets a selected free label, legend, X title or Y title in the
active graph, including automatically generated titles. Font family lists installed families;
size, Bold, Italic, Increase/Decrease Font Size and Font Color change the existing object's
`Font`. Text, escapes, worksheet values and other objects are preserved. One action is one
Undo step. Undo/Redo and `.sheaf` / `.opj` with its companion retain all five font fields.
The same scene carries the chosen face, style and colour into the screen and exports.

OriginPro 2016 whole-object controls (2026-11-15, local `scratch/t58-format-toolbar/`): a
single-selected legend shows 22 pt and `Default: Arial`. Selecting Times New Roman changes its
font index to 325 on this installation; clicking the red swatch changes its colour index to 2.
Neither operation rewrites `legend.text$`. Entering 18 in the size box changes `legend.fsize`
to 18. Increase/decrease from 10 gives 12/8; from 14, 17/12; from 18, 22/15; from 30, 36/25.
From 22, Increase gives 26 and Decrease returns 26 to 22. Sheaf uses `round(size * 1.2)` and
`round(size / 1.2)`, matching these controls. Screenshots with selection handles and property
reads establish the target; stale export snapshots and attempts obscured by tooltips or missing
the smaller legend are excluded. The final refreshed Original-theme EMF contains Times New Roman.

The tested native single-selected legend did **not** change style on Bold/Italic clicks;
its text stayed unchanged. Sheaf deliberately exposes whole-object Bold/Italic through its
existing font flags to meet this brief's whole-object goal. Their native whole-object parity
is **unestablished**; this is not evidence of Origin's internal style storage. The current
[Format toolbar reference](https://docs.originlab.com/origin-help/format-toolbar/) documents
font controls on graph annotations; it does not settle that 2016 selection-state difference.

Sheaf choices: the size box accepts finite 1 to 1000 pt values and commits on Enter; Escape or
leaving the box cancels the draft. Font Color keeps eight named basic RGB colours (black, red,
green, blue, cyan, magenta, yellow, white) and adds the other 16 through More Classic colors,
completing the 24 Classic entries measured in 7.2. Origin's Auto, other system/custom lists and
palette management remain open. Family selection is restricted to installed families; fallback rendering
retains its existing behaviour. Text-window selection controls and the T59 worksheet adapter
are separate slices.

Classic graph-font palette follow-up (2026-11-16): all 24 swatch-centre RGB triples from the
accepted selected-legend Font Color screenshot (`scratch/t58-format-toolbar/whole-object/`)
match the independent worksheet palette measurements and the 7.2 EMF controls. Dark Yellow is
128/128/0, Olive 0/128/0 and Royal 0/0/160; the original Green remains 0/255/0. The fresh pilot
refused an existing foreign Origin instance before COM creation; no new native colour-change
result is claimed. The accepted earlier red-swatch mutation and palette pixels remain the
reference. The compact named submenu is Sheaf layout, rather than Origin’s swatch layout.
It shares the existing worksheet-font presets while keeping worksheet Custom... and graph
object Fill/Line palettes separate. Both menu levels close after a choice. All four text targets,
rendered RGB, unchanged raw markup/styles, no-op/stale requests, one-step Undo and project/companion
roundtrips are covered by graph-toolbar regressions; explicit inline colour escapes retain their
existing precedence over the base font. No renderer, model or persistence fields change.

Custom graph-font RGB follow-up (2026-11-16): Font Color > Custom RGB... adds three opaque
RGB channel fields, a colour preview, Apply font color and Cancel font color. The current base
font colour seeds the fields each time the root menu opens. Only whole numbers from 0 to 255
are accepted; invalid text disables Apply and Enter. A valid Enter or Apply uses the existing
captured font request and closes both menu levels. Cancel or dismissal changes no document
state. Unchanged acceptance adds no history; stale target/revision/generation requests retain
the existing rejection rules. Worksheet custom colours keep their separate dialog; graph
object Fill/Line and inline text-editor palettes retain their controls. Explicit inline colour
escapes still override the base font. No model, renderer, request type or IO code changes.

Origin's [Color function](https://docs.originlab.com/labtalk/ref/color-func/) documents RGB
components from 0 to 255, and the Format toolbar reference above covers selected graph text.
A fresh private Origin 2016 control (`scratch/t58-graph-font-rgb/native-clean/`) sets both
`legend.color` and a named label's colour through `color(r,g,b)`: 17/131/213 reads 30769937,
black reads 16777216, and white reads 33554431. The legend's raw substitution text and 22 pt
size stay unchanged; the label's raw text also stays unchanged. Its setup flag text was literal,
so no positioning/layout claim is inferred. The inspected RGB screenshot contains 2,198 exact
17/131/213 pixels in the measured text region. Every clean LabTalk step was checked; the first
pilot used a malformed wait delimiter and is excluded. Both owned instances/helpers closed.
These property/pixel controls establish RGB application, not the native custom-picker geometry;
the compact numeric submenu is Sheaf UI. Other colour lists, Auto and palette storage remain open.

`text_format::{Snapshot, Target, Change, Request}` separates widget changes from target application.
T59 can add its cell target behind the same toolbar. Requests capture graph/layer/object plus
document generation and revision. Dispatch rechecks active selection and dialog/cell editing state;
invalid, unchanged or stale requests add no history. Open family/colour lists retain their original
target rather than following a later selection. No model or file-format fields were added.

Underline, Superscript, Subscript, Greek, Fill Color and Line Color (`object_format`, same
request/staleness rules, own `Action::FormatObject`). Origin's online
[Format toolbar help](https://docs.originlab.com/origin-help/format-toolbar/) [W] describes the first
four as acting on selected characters and does not document whole-object behaviour. Fill and
line/border colour are described on Origin's
[Style toolbar](https://docs.originlab.com/origin-help/style-toolbar) as setting the fill or line
colour of the selected object; Sheaf shows them in the Format row (gui-look.md, Format and Style toolbar description). Sheaf whole-text choices,
**unmeasured** in Origin 2016: the four text buttons wrap the whole
text in one `\u(`, `\+(`, `\-(` or `\g(` group, or remove that group when the text is exactly one
such group (nesting counted as the parser does; `\u(a)b` is wrapped, not unwrapped; a group may
span lines, so multi-line text is one group) and show pressed when wrapped; they apply to free
labels and to X/Y titles with their own non-empty text, not to the legend or automatic titles.
Fill Color (palette: None plus the 24 Classic colours) sets a rectangle's or circle's fill and a
visible legend's background; Line Color sets a drawn object's or straight line's colour. Both are
disabled with a hover reason elsewhere (the legend frame has no colour field). The selection
survives; undo labels are "Format graph text" and "Format graph object". No model fields added.

Classic object-colour follow-up (2026-11-16): a fresh private Origin 2016 graph contained a
named rectangle and a named straight line. For each index 1 through 24, the rectangle's
`fillColor` and `color` and the line's `color` were set to that index. All commands succeeded;
all three getters returned the assigned index. Each independently exported EMF contains at
least two matching pen and two matching solid-brush records with the RGB triples in 7.2,
including Royal 0/0/160, Wine 128/0/0 and Dark Gray 64/64/64. The Royal export was inspected.
The isolated `Origin.Application` instance and watchdog closed; pre-existing process identities
were identical before/after. Evidence: ignored `scratch/t58-object-classic-colors` (24 EMFs,
checked script/log, native-proof.json and native-royal.png). The public properties and creation
syntax are documented in [Graphic Objects](https://docs.originlab.com/labtalk/ref/graphic-objs/)
and [Draw](https://docs.originlab.com/labtalk/ref/draw-cmd/).

Sheaf retains the eight basic choices in the root Fill/Line menu and puts the other 16 in
More Classic colors. This grouping, the visible-legend background mapping and the supported
target restrictions remain Sheaf UI choices; no native toolbar geometry or legend-background
measurement is claimed. Extra choices use the original captured target/revision/generation,
close both menus, preserve the selection, and have the same one-step Undo and unchanged/stale
no-op rules. The public eight-colour palette constant and font/worksheet controls are unchanged.
Auto, other lists and palette storage remain open.

Custom object RGB follow-up (2026-11-16): five fresh private Origin 2016 controls set a named
rectangle's fill/outline and a named straight line's colour with `color(r,g,b)`: 17/131/213,
0/0/0, 255/255/255, 0/255/128 and 1/2/3. All commands and exports succeeded; all three getters
retained the assigned internal RGB code. Every EMF has at least two exact matching pens and
solid brushes. The asymmetric RGB export was inspected. The private `Origin.Application`
instance and watchdog closed, with pre-existing process identities unchanged. Evidence:
ignored `scratch/t58-object-rgb` (checked script/log, five EMFs, native-proof.json, native-rgb.png).
These are property/value/rendering measurements, not a native picker-layout or legend-background
measurement. The [Color function](https://docs.originlab.com/labtalk/ref/color-func/) documents
RGB assignment; the existing target mapping above remains Sheaf policy.

Fill Color and Line Color > Custom RGB... provide three integer fields (0 to 255), a preview
and Apply/Cancel; valid unmodified Enter accepts from a channel field. Invalid, dismissed,
unchanged and stale choices add no history. Reopening seeds the captured object's current
property, independently for fill/line and each new target. An absent fill seeds white as a
draft; opening or cancelling preserves None, while accepting white explicitly adds a fill.
That seed and the compact numeric submenu are Sheaf choices. Presets retain all 24 Classic
colours and Fill retains None; presets replace an unapplied RGB draft. Accepted colours use
the same selection, history and native/companion save paths. Font/worksheet pickers, models,
renderer/backends and the public basic-palette contract are unchanged.


---

### 7.5 Text label selection editor (T58)

Origin's [Text Object Properties, Text tab](https://docs.originlab.com/origin-help/textob-prop-text-tab/)
documents selection/caret formatting buttons above a raw text box and a formatted preview below.
The buttons insert commands around selected text or insert an empty command group at the caret.
This is a documentation reference, not a fresh measurement of Origin's dialog geometry.

Sheaf's free-label Text window now offers Normal, bold, italic, underline, strikethrough,
superscript, subscript and Greek. A button keeps the raw characters outside the selected range
and returns focus to the selected body. With no selection, typing enters the empty group.
Selecting `-2` in `cm-2` and choosing superscript produces `cm\+(-2)`; Unicode cursor offsets
are characters, not UTF-8 bytes. Applying the same format to its complete selected body removes
that wrapper. Normal removes complete inline formatting wrappers from the selection and its
immediate surrounding wrappers, preserving literal `\v(...)`, substitution and Unicode escapes.
These toggle/Normal details are Sheaf choices, not measured Origin rules. The existing whole-label
Bold/Italic checkboxes still change the base font; Normal affects inline markup only.

The preview uses the public graph export renderer with an isolated one-label graph. It therefore
shares the published parser, glyphs, script layout, colours, decorations and rotation with exports.
Its cache changes with text, font, rotation or display scale. It fits large text into a bounded
preview and reports text over 8,192 characters or excessive dimensions without truncating the
stored text. Size and base styles also update the preview. Buttons and preview do not mutate the
project: OK/Apply performs the existing single history operation; Cancel discards the draft.
A new draft containing only empty formatting groups adds no invisible label.

The button labels, compact layout and Ctrl+B/I/U/G and Ctrl+= / Ctrl+Shift+= bindings are
Sheaf choices based on the published command forms and existing shortcut map; their behaviour
inside Origin's properties dialog has not been measured. Inline colour selection is covered
in 7.8/7.9, family selection below, title/legend reuse in 7.7, and paired-script insertion
in 7.10. The shared Format bar remains a separate target-dispatch implementation.
No renderer/model/persistence changes are introduced by this editor slice.

### 7.6 Inline font families (T58 graph text)

`codex/text-run-fonts` (2026-11-15) carries a requested family through each markup run.
For full font names, `\f:Times New Roman(a\f:Courier New(b)c)d` sets `a` and `c` in
Times New Roman, `b` in Courier New, and restores the label's own font for `d`. The
[escape reference](https://docs.originlab.com/origin-help/escape-sequences/) documents the
family switch. Runs with the same family and other style coalesce; differing families
remain separate even when their weight, size and colour agree.

OriginPro 2016 controls in `scratch/text-run-fonts/` use `fonts.lines.txt` (eleven cases)
and `font-names.lines.txt` (eight edge/name cases). Exact selected-font records in EMF,
associated with text reference coordinates, confirm full Arial, Times New Roman and
Courier New names, the four nested `a/b/c/d` families above, and a Times span containing
bold (weight 700), italic and underlined text. At 24 pt these selected faces have height
-100 in the captured EMF coordinate system. Parent text is confirmed Arial by its selected
face; the second probe's quoted `label -f` request did not change it, so no Courier-parent
behaviour is inferred from that request.

The native Symbol controls use the Symbol face and private character U+F06D for `m`.
Sheaf keeps its established Unicode Greek mapping: `\f:Symbol(m\f:Arial(x)m)y` produces
mu, Latin x, mu, Latin y, with the requested Symbol/Arial/Symbol/parent families. Switching
to a non-Symbol family ends Greek mapping inside that span. Unicode or literal wrappers
continue to suppress Greek remapping. Actual glyph coverage and fallback on a machine can
differ from Origin's Symbol encoding; this does not add a new Symbol character map.

The same effective family is used for widths, placed glyphs and font-table decoration
metrics. Mixed-family anchors and rotation therefore use the sum of the measured run
widths. A single unshifted run retains the existing native text anchor. Bold, italic,
script size/baseline and colour compose with family changes. Titles, legends, labels and
tick strings share this placement path. No model, file-format, global font setting or
backend implementation changes are needed; existing font fallback and export warnings
still apply. EPS maps the three standard families to Helvetica, Times-Roman and Courier;
SVG/EMF retain their requested family and PNG/PDF use the existing SVG font resolution.

This slice supports full, nonempty names with no edge whitespace, up to the existing
64-character bound. An empty name keeps the complete `\f:(...)` text literal, matching the
accepted Origin 2016 N6 control (correction 2026-11-16). Edge-whitespace names retain Sheaf's
previous neutral-family behaviour and enclosed text. A whitespace Times name and missing or
abbreviated names selected Arial in the native controls; those whitespace, short-name and
alias/default-name behaviours are not claimed as matching Origin. Missing fonts retain the
existing platform-dependent fallback. Nesting limits and other malformed/literal text rules
remain unchanged.

The empty-name correction uses the already accepted `font-names.emf`, its all-successful
command log and synthetic `font-names.lines.txt`: N6 contains one Arial run with the seven
literal characters `\f:(MM)`, reference (1694, 1047), font height -100 and advance 317.
The two-character `MM` controls advance 166 in the same export. The earlier quoted parent-font
request did not select Courier New, so no Courier-parent inference is made. Three regressions
retain the full literal text, surrounding family/underline state and measured advance, and
compare SVG, EPS, EMF and raster output with an explicit literal span. Valid full-name escapes
and existing goldens are unchanged.

A fresh 21-control Origin 2016 probe (2026-11-16) sets the parent with
`Probe.font=font(Courier New)` and verifies the selected Courier New EMF face. All commands
and exports succeeded in a separate hidden COM server; its exact process identity was closed
and the pre-existing server's identity stayed unchanged. `A\f:(B)C` is one complete Courier
New run (advance 480); inside Times New Roman the text `B\f:(C)D` remains in Times, with the
outer Courier restored afterward. Full Times New Roman, Courier New and Symbol controls
confirm the family switches. Default/default/Default: Arial, e/a/g/s, Swiss, Times, Helvetica,
an unknown name and an edge-whitespace Times name select Arial in this configuration; these
results do not distinguish special aliases from ordinary missing-name fallback. Roman and
Courier select those named faces. No short-name substitution or platform font lookup policy
is changed. Raw exports, selected-font records and identity checks are hashed in ignored
`t58-font-aliases/native-proof.json`.

Modern special defaults require separate version and configuration evidence: Origin's
[2022b release note](https://www.originlab.com/ReleaseNotes/detail.aspx?id=2022b2ORG-23928)
introduces the special-default-font controls and mixed-font `\f:e` / `\f:a` examples; the
[current dialog reference](https://docs.originlab.com/origin-help/special_default_fonts/)
links those names to configurable categories. These references do not establish their
behaviour in Origin 2016. This correction adds no short-name aliases or global font settings.

**Installed-family abbreviations (T58).** The current Origin escape reference documents
abbreviating a name when it identifies a single font family. Sheaf compares inline names with
installed family names without regard to case. An exact family name takes priority. A prefix of
at least two characters expands only when it has one match. Ambiguous and missing names go
through the existing font fallback. One-letter special defaults and names beginning `Default:`
remain outside this rule. Matching uses the font database available at render time, so a prefix
can be ambiguous on another machine. The full name reaches widths, glyphs, decorations and
vector output through the existing text layout. A prefix that resolves to Symbol follows the
same Greek mapping as the full name. Saved markup keeps the original spelling. This follows
the documented abbreviation rule with Sheaf's matching policy. Origin 2016 abbreviation
parity and the configurable 2022b defaults remain unverified.

Eight font regressions cover native family/style restoration, same-family coalescing,
family-sensitive widths and decoration geometry, rotation, Greek/literal/Unicode behaviour,
name bounds, an empty font database and EPS/EMF/PDF/SVG output. New SVG and stroke-only PNG
goldens use deliberately different deterministic test-family metrics; those synthetic
metrics are not Origin measurements or installed-font rasterization fixtures.

**Inline family picker (2026-11-16, `codex/t58-inline-font-family`).** The shared label/title/
legend editor adds Text font, with a case-insensitive search of installed family names. Names
must fit the existing full-name grammar: nonempty, no edge whitespace, at most 64 characters,
no control characters, parentheses or backslashes. It uses the existing font database; no new
font loading, fallback, parser, model or export behaviour is added. The accepted Origin EMF
controls above establish the full-name escapes and Symbol mapping; this picker layout and its
selection/replacement policy are Sheaf choices, without a new Origin GUI-selection measurement.

A choice writes `\f:Family(...)` around the selected raw text, or inserts an empty group at the
caret, restores the selected body and returns keyboard focus to the text box. Inside the chosen
range, existing family wrappers are replaced; other styles, colours, scripts and literal blocks
remain intact. Repeating the same family on its selected group is idempotent. A neutral style
probe reuses the editor's raw-fragment validation before family replacement; the actual family
change retains the renderer's documented start/end of Symbol Greek mapping. Invalid names and
unsafe fragments add no markup. The object's base font stays unchanged.

Search/dismissal alone keeps automatic text automatic and adds no history. A changed family
remains a draft until Apply/OK; Cancel discards only unapplied edits. Legend samples and line
boundaries stay outside text-format groups, while name substitutions remain raw. All four text
targets use the existing preview, rendering, one-step history and project/companion save paths.
The six UI and four private-helper regressions cover these rules, installed-family output,
Unicode/script selection, styles/literals, Symbol transitions, stale generation and persistence.

### 7.7 Axis-title and legend text editors (T58)

The existing X/Y axis titles and legend content now use the selection editor from 7.5.
Format > Axis Titles opens the active layer's X or Y title; each title's context menu has
Edit Text. A legend's context menu has Edit Text, and Graph > Legend > Edit Legend Text
also reaches a hidden or off-page legend. A multi-layer graph names the layer in the window.
These entry points and compact window layout are Sheaf choices, not newly measured Origin UI.

Origin documents axis titles and legends as text objects and shares its Text tab between
common text and legends: [graph annotations](https://docs.originlab.com/user-guide/customizing-graphs/),
[Text tab](https://docs.originlab.com/origin-help/textob-prop-text-tab/) and
[manual legend control](https://docs.originlab.com/origin-help/legend-manualcontrol/).
The editor preserves raw column-metadata escapes, legend sample codes and name references.
Automatic text is retained on unchanged OK, Cancel and base-font-only edits; editing raw text
chooses custom text. Automatic text restores the existing generated title or legend, and
Show on graph controls the existing visibility flag. Base font sizes through 1,000 points
are preserved, matching the published Format toolbar's supported range. Titles/legend have
no invented label-rotation property. OK or Apply writes one history step when values change;
Cancel discards only the unapplied draft. A replaced project closes a stale editor.

Generated titles come from a read-only facade over the renderer's existing resolver, including
histogram, category and transposed-plot rules. Preview uses the actual selected scene object's
glyphs, decorations and legend samples, with the project's data-name substitutions. A read-only
cropped-scene facade reuses the existing SVG/raster path and supports negative off-page bounds;
no glyph, layout or export rules change. Only the selected object is shown on the preview canvas.

A selected literal parenthesis is safely encoded when necessary to preserve the text.
Whole-legend formatting keeps symbol codes and row boundaries outside text-format groups;
name substitutions remain raw, so formatting does not freeze their data links. This is a Sheaf
editing choice using the published renderer, not a measured Origin command-rewrite algorithm.
The existing screenshot hooks accept `--show text:x`, `text:y` and `text:legend` for the first
layer of the first graph, alongside the existing `text:N` label hook.

### 7.8 Selected-text colour palette (T58)

The shared label, axis-title and legend editor has Text color beside its formatting buttons.
Black, Red, Green, Blue, Cyan, Magenta, Yellow and White remain the basic choices (indices
1-7 and 18). More Classic colors adds the other 16, completing the measured list in 7.2.
Dark Yellow is index 8 (128,128,0), Olive index 12 (0,128,0), Royal index 14 (0,0,160) and
White index 18; Green remains bright green. The existing renderer and base font apply.

Origin's [Text tab](https://docs.originlab.com/origin-help/textob-prop-text-tab/) documents
font-colour selection, raw text, selection formatting and preview. The basic-eight/extra-sixteen
menu grouping and rewrite policy are Sheaf choices, not a newly measured Origin dialog layout
or command algorithm. Numeric Classic colours are measured by the EMF controls in 7.2; both
worksheet swatch lists independently agree (GUI 3).

A choice colours the selected characters or inserts a colour group at the caret for typing.
It replaces existing colour groups in that selection while preserving scripts, font/style
groups, literal blocks and raw substitutions. Repeating a choice is idempotent; Normal removes
inline colour together with other inline formatting. Legend samples and row boundaries stay
outside text-format groups, as in 7.7. All 24 choices use their one-based Classic index escapes.
Both menus close after a choice, restore typing focus to the original selection/caret and seed
the custom RGB fields in 7.9.

Opening/dismissing the palette keeps automatic text automatic. A changed colour remains a
draft until Apply/OK, shares the existing Undo step and preview, and preserves the base font.
Native and Origin-companion saves retain the raw markup. Other native colour lists, palette
storage, inline family selection and combined-script insertion remain separate work.

### 7.9 Custom RGB text colours (T58 editor)

Text color > Custom RGB... in the shared label/title/legend editor has Red, Green and Blue
fields, a swatch and Apply color. Each field accepts a whole number from 0 through 255;
invalid or empty values disable the choice and show the permitted range. The fields start
from the object's base font colour. A named palette choice also seeds them; subsequent typed
values stay in the current editor's picker. Alpha is unsupported; the numeric draft
is transient. Explicit reusable Custom wells use the separate preference pool in7.12.

Editing the fields or dismissing the menus does not change the text draft, automatic-text
state or project. Apply color closes both menus, formats the original selection (or inserts
a group at the caret) and restores typing focus to the text. The result is still a draft
until the Text window's Apply/OK. Other styles, script columns, font-family groups, literal
blocks and legend name references keep their existing meaning. Repeating the same RGB
choice on the selection is idempotent; existing history and project round trips apply.

The command reuses the measured opaque RGB text encoding in 7.3:
`0x01000001 + r + 256*g + 65536*b`, written as a decimal colour escape. This includes Black
and White at the existing tag boundaries. The renderer/parser and base font are unchanged.
Origin's [Text tab](https://docs.originlab.com/origin-help/textob-prop-text-tab/) documents
font-colour selection, formatting and preview; this compact numeric submenu, validation
policy and explicit draft choice are Sheaf UI choices, not a newly measured Origin layout.
The 24 named Classic colours from 7.8 remain available. Other editor insertion controls and
persistent native palette lists remain separate work.

### 7.10 Paired subscript and superscript columns (T58 renderer)

Origin's [escape-sequence reference](https://docs.originlab.com/origin-help/escape-sequences/)
defines `\=(sub,sup)` and a literal comma via `\v(,)`. On 2026-11-16, Origin 2016 was
measured with twenty quoted synthetic Arial labels at 24 pt, exported to EMF. Raw commands,
records and extracted runs are in local `scratch/text-script-pairs/pairs-quoted.*`;
four additional 12/24/48/72 pt controls are in `pair-sizes.*`. Unquoted commands containing
`=` were rejected by LabTalk and provide **no paired-script evidence**. The private runner
held the global reference mutex, refused any existing COM Origin, and closed only its own
instance; every native run completed before renderer validation.

Measured relative to the baseline and height of the surrounding run:

- Both branches begin immediately after the preceding glyph at the same left edge.
  The next ordinary glyph advances by the **maximum** of the two branch widths, with no
  extra gap. For `P\=(1234,5)Q` and `P\=(1,2345)Q`, P advances 67 EMF units, the wider
  branch 132, and Q begins 199 units after P in both cases. Equal branches advance once.
- Branches use 60% of the surrounding size. Superscript is raised by 49% and subscript
  lowered by 35%. At base heights 50, 100, 200 and 300 the upper shifts are -25, -49,
  -98 and -147; lower shifts +18, +35, +70 and +105 (integer rounding). The renderer
  retains fractional coordinates. These measurements apply to paired scripts; the
  existing standalone `\+` / `\-` placement constants are unchanged.
- The first comma outside balanced parentheses separates branches. `\v(1,2)` stays
  intact in the lower branch; `\=(1,2,3)` renders lower `1` and upper `2,3`.
  With no comma, all content is lower text. Either branch may be empty; both empty
  produce no glyph and consume no width.
- Each branch inherits its surrounding style and restores it afterwards. Native controls
  cover bold/italic, red/blue, Symbol Greek, Courier New/Times New Roman and underline/
  strikeout. The family test has lower width 72 and upper width 60, so Q follows 72.
- Nested pairs scale again: `P\=(\=(1,2),3)Q` has 36%-size inner scripts, at baseline
  offsets +56 and +6 against P; the other branch is at -49. Native EMF paints upper
  before lower; the shared layout does likewise.

The private parsed layout tree retains each paired column for width and placement;
public `markup::parse` still returns styled runs in logical lower/upper order and
`plain` returns their text without delimiters. Existing single-run native anchors,
font fallback, colour and decoration paths remain in use. Nesting is bounded by the
existing 24-level escape limit; deeper content and malformed unclosed groups keep
Sheaf's existing tolerant parser convention, which was not measured in Origin.
SVG glyph/stroke and font-independent stroke PNG goldens, exact scene geometry,
rotation/anchors, effective-font width, EMF positions, EPS and PDF exercise this path.
The renderer slice adds no project field or font backend; editor insertion follows below.

**Paired-script editor insertion (2026-11-16, `codex/t58-paired-script-editor`).** Paired
scripts in the shared label/title/legend editor opens Subscript text and Superscript text fields.
The selected raw text seeds the lower branch; the upper starts empty. With only a caret, both
start empty. Insert scripts requires at least one nonempty field and writes the existing
`\=(sub,sup)` form into the draft. Either individual branch may be empty. Focus returns to the
raw text box with the caret after the column, so subsequent typing is ordinary text.

Literal top-level commas in either field use the measured `\v(,)` form; commas inside balanced
markup/literal groups stay intact. Unmatched literal parentheses use the existing Unicode
escapes. Incomplete formatting groups and unsafe selection fragments cannot be inserted.
An inline pair cannot span rows or contain legend sample controls: select text within one
legend entry instead. Normal entry text and raw name references remain editable; symbol and
row selections stay unchanged. The existing raw-fragment validation and 24-level parser bound
remain in use. This limited two-field UI, lower-branch seeding and caret policy are Sheaf
choices; the earlier twenty quoted native controls and four size controls above establish
rendering syntax, not a new Origin properties-dialog insertion measurement.

Nested styles, colours, families and paired columns use the unchanged renderer. Only the
selected raw fragment is replaced; base fonts and worksheet values stay intact. Editing these
fields or dismissing the menu leaves automatic text and history unchanged. Insert changes
only the editor draft; Apply/OK makes the existing single history step, and Cancel discards
unapplied changes. Preview and screen/export share the same placement. Six UI and four private
helper regressions cover shared-column glyph positions, Unicode/focus, commas/literal blocks,
styles/fonts/nesting, unsafe fragments, automatic text, cancellation, stale generation and
label saves and title/legend project/companion roundtrips.

### 7.11 Prime escape characters (T58, documented contract)

Origin's [escape reference](https://docs.originlab.com/origin-help/escape-sequences/)
documents the prime and double-prime forms; [FAQ159](https://docs.originlab.com/quick-help/add-prime-symbol/)
lists Origin 2015 SR0 as its minimum version. This is a documentation check on
2026-11-16, not a fresh Native2016 measurement: local COM activation is unavailable.

Sheaf recognizes only the exact closed forms \y(') and \y("), producing Unicode
U+2032 and U+2033 through the existing glyph path. Surrounding family, weight,
italic, colour, underline and strikeout apply and restore normally; Greek spans
do not remap these characters. The escape itself adds no scale or baseline shift.
Existing superscript and paired-script wrappers still use their published layout
rules. This Unicode mapping is Sheaf policy; Native full-height glyph geometry,
combining-prime placement and the configurable @TAPP extra offset are unmeasured,
and that offset is not implemented.

Literal blocks retain the original notation. Other bodies, whitespace, missing
closing parentheses, uppercase codes and the 24-level bound retain existing
unknown/malformed handling. No font aliases, lookup, project fields, editor
controls or backend implementations change. Modern special-default fonts remain
deferred under 7.6; accents, conditional spaces, wrapping and line separation
remain separate work. Regressions compare styles, placement, rotation, vector/
raster output and actual legend/title/tick scenes with explicit Unicode controls.

**Overbar escape (T58).** The [Origin escape reference](https://docs.originlab.com/origin-help/escape-sequences/)
defines `\ab(text)` as a bar above the enclosed text. Sheaf renders one continuous stroke
over the full span, including changes of font or style and shared script columns. The stroke
uses the color at the opening escape. Its position clears the highest nominal glyph top or
nested bar in the span. The gap and thickness are proportional to the span's font size.
The bar follows the text's rotation and anchor and adds no horizontal advance. Empty spans
produce no mark. The shared scene carries the same stroke to vector and raster output.
These placement choices follow Sheaf's layout metrics. Native bar geometry is unmeasured.

The same span layout handles the documented `\ad(text)` dot and `\at(text)` tilde.
The dot is filled, and the tilde uses a short wave that fits the enclosed span. Both are
centered over the measured text and inherit the color and font size at the opening escape.
They stack with other accents, add no advance and rotate with the text. The wave dimensions
and dot size are Sheaf choices based on the surrounding font size. Native geometry remains
unmeasured.

**Legend line separators (T58).** The [Origin 2022b release note](https://www.originlab.com/releasenotes/detail.aspx?id=2022b3ORG-24940)
introduces `\sep:nn` for irregular line spacing. The [escape reference](https://docs.originlab.com/origin-help/escape-sequences/)
shows it on its own line between legend entries and defines the added gap as a percentage
of font height. Standard Sheaf graph legends accept a standalone `\sep:nn` line with an
unsigned 32-bit decimal integer. It adds that percentage of the legend's base font size
before the next row. The separator does not occupy a row or draw ink. Consecutive separators
add together within one column. Separators at the start or end of a column have no effect.
The legend frame covers the tallest column including its added gaps. These rules use Sheaf
layout metrics. Native multi-column geometry is unmeasured.

**Free graph-label line separators (T58).** The
[Origin text properties documentation](https://docs.originlab.com/origin-help/textob-prop-text-tab/)
describes the same escape for multiline text objects. Sheaf free labels lay out CRLF, LF and CR as
line breaks, with the first row attached at the existing anchor. An ordinary row advances by
1.3 times the base font size. A standalone top-level `\sep:nn` adds that percentage of the base
size before the next row. Gaps add together and only affect spacing between rows. Styles can
span line breaks, and each row rotates with the label. Literal spans and malformed separators
render as text. Picking and exports use the same scene rows. The line advance and exact placement
are Sheaf's layout choices. Axis titles, table cells and inline separator placement are outside this slice.

**Protected substitution text (T58).** The Origin escape table shows `\h(...)` displaying
legend notation literally. Sheaf draws the contents of `\h(...)` as literal text with the
surrounding font, color and decorations. In a legend, both `\h(...)` and `\v(...)` protect
plot symbol and name references within the block. Parentheses balance within a row,
and an unclosed block protects the remainder of that row. References outside a block
expand through the ordinary legend rules. Literal handling for the entire body is Sheaf's
reading of the documented example. [Origin escape sequences](https://docs.originlab.com/origin-help/escape-sequences/)

**Plot-colored graph text (T58).** `\cpN(text)` uses the color of the Nth plot in the
text object's layer. Repainting the graph after a style change resolves the color again.
Nested styles, literal blocks and glyph decorations follow the existing text rules.
[Origin LabTalk Label reference](https://docs.originlab.com/labtalk/ref/label-cmd/)

Sheaf counts the layer's saved plots in order, including hidden ones. It uses the same
representative line or symbol color as its axis color-following control. Index zero and
missing plot slots leave the text in its enclosing color. The slot and fallback behavior are
Sheaf choices. This applies to free labels, ordinary legends and Cartesian axis titles.
Graph exports and the legend and title editors' scene previews use the same rendering.
A shared-frame legend resolves the
color from its owning layer even when its text names a plot in a different layer. For example,
`\l(2.1) \cp1(%(2.1))` draws the sample and name of the second layer's first plot and
colors the name with the owning layer's first plot. A title shared by touching Cartesian
panels uses the plot color from the panel that draws the title. Point labels, specialized
axis titles, the parallel coordinates group legend and the standalone free-label editor
preview remain open.

### 7.12 Reusable Custom RGB wells (T58/T59, documented contract)

Origin's [FAQ130](https://docs.originlab.com/quick-help/define-custom-colors/)
documents custom wells in the font-colour chooser and, for 2016 onward, adding
multiple RGB choices without applying them until accepted. It describes local
reuse and preservation of applied colours in a saved project/window. This is a
documentation contract, not a fresh Native2016 measurement. Color Manager starts
in 2021; [FAQ1076](https://docs.originlab.com/quick-help/5-rows-of-colors/) labels
automatic System Color List storage as 2017 onward. Neither establishes a 2016
named-list manager, file format or automatic system-list workflow.

The graph Format toolbar's font/fill/line Custom RGB pickers offer Add to Custom.
A valid opaque RGB draft is added to one shared pool without applying it to the
graph. Cancel discards the document draft but retains an added well. Custom colors
offers the pool in all three graph pickers; choosing a well uses their existing
captured target/revision/generation checks, no-op handling and one-step Undo.
An entry's context menu removes it from the pool without changing any applied
object colour. The 24 Classic presets, None and numeric Apply/Cancel remain.
Worksheet controls use this same pool through their existing draft dialogs (GUI3,
workbook13b). The shared graph text editor
also offers Add to Custom in its numeric picker and the same Custom colors pool.
Adding stores only the preference and leaves the selected raw-text draft, caret,
automatic-text state and document unchanged. Choosing a well maps its channels to
the existing opaque inline RGB colour command: it colours the selection or inserts
a group at the caret, restores typing focus and seeds the numeric RGB fields.
The text remains a draft until the Text window's existing Apply/OK. Cancel discards
unapplied text changes while retaining added wells; removing a well does not rewrite
the draft. Legend samples/name references, base fonts, script groups, no-op history,
project-generation guards and native/companion saves keep the existing7.8/7.9 rules.
This compact inline integration is Sheaf policy under the documented reusable
chooser and [Text-tab](https://docs.originlab.com/origin-help/textob-prop-text-tab/)
selection/caret contract, not a newly measured Native2016 editor workflow.

The following are explicit Sheaf policies: up to 64 unique opaque RGB entries in
insertion order; no names; preference-only removal; a private version1 JSON file
named custom-colors.json beside profile settings; a 64KiB read bound and complete
same-directory atomic replacement. Missing, corrupt, unsupported-version,
over-capacity or nonopaque records load an empty pool without rewriting the file;
duplicate entries retain their first position. Startup loads once. Add/remove
writes the pool, and failure keeps the session's values with a visible message.
Other running instances do not refresh; the last successful writer supplies the
next startup. An explicit absent startup path keeps tests memory-only.

Applied colours use existing project fields and export paths; unused wells are
profile-only and are not embedded as a project palette. Named lists, OTH/PAL
import/export, incremental plot lists and Color Manager remain deferred.
No Core, renderer, font resolution or project/IO schema changes
are part of this slice.

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

**Sheaf's reading (implemented in `crates/plot/src/margin.rs`, CLI `--margin auto|border|tight|page|tight-in-page` and `--border-width`, the export window's Margin list).**
- *Auto (-2)* uses the global Copy Page settings plus offsets. Origin's own Copy Page margin option is not in any file we may read (`Origin.ini` only has `AddEMFInCopyPage=1` and `AddWMFInCopyPage=1`; no margin key), and the program was not started for this. Choice [INF]: Auto means Border with the default border width, because a copy of a page that pastes with a little white round the axes is what the Copy Page default looks like and because a Border box always contains the labels. Origin's `BorderWid` default -1 stands for "default"; its value is not documented, Sheaf uses 5 pt (about 1.8 mm). Re-measure both when Origin can be run: export a PNG with Margin = -2 and compare its size with the page.
- *Border width* is in pixels of the picture (Origin: whole-pixel offsets): at the export DPI for PNG and TIFF, 1 px = 1 pt for the vector formats. A negative or missing value is the default.
- *Border* and *Tight* measure the bounding box from the drawn items: strokes by half their width, text by its measured box, data cut at the layer frame, the page background not counted. *Tight in page* keeps an item when at least half of its box area is inside the page (zero-area items: when their centre is), and the resulting box is cut at the page edge. An empty figure falls back to the page.
- *Size* (9.3) sizes the exported area. A page ratio of 100 % is the area at its own size, 50 % half of it. The library functions and the plain `export_file` keep margin Page, so their outputs did not change; the CLI and the app default to Auto.

### 9.3 Size `tr1`
Unit 0 inch (default), 1 cm, 2 pixel, 3 page ratio %. Rescaling 0 by width (default) or 1 by height; aspect always kept; width/height value 0 = original page size. Pixel size = inches x DPI (raster only). Publisher recipe: Margin = page, size = page ratio 100 %, DPI as requested [W:quick-help/set-expgraph-size-resolution].

### 9.4 Raster options `tr2`
| Format | DPI default (choices) | Colour depth default | Extras |
|---|---|---|---|
| PNG | 300 (72,100,150,300,600,1200) | 256 colours (Monochrome, 16 Colors, 256 Colors, 24-bit Color, 8-bit Gray Scale) | |
| TIF | 300 (+96) | 256 colours (Monochrome, 16 Colors, 256 Colors, 24-bit Color, 8-bit Gray Scale) | colour space RGB (CMYK, YCbCr); compression none (PackBits, LZW) |
| JPG | 300 (+96) | 24-bit (or 8-bit Gray Scale) | quality 0..255 default 0, higher = smaller/lower quality |
| BMP | 300 (+96) | 256 colours (Monochrome, 16 Colors, 256 Colors, 16-bit Color, 24-bit Color, 8-bit Gray Scale) | |
| GIF | 300 (+96) | 256 colours (Monochrome, 16 Colors, 256 Colors, 8-bit Gray Scale) | |
| PCX | 300 (no 96) | 256 colours (Monochrome, 16 Colors, 256 Colors, 24-bit Color, 8-bit Gray Scale) | |
| PSD | 300 (no 96) | 256 colours (Monochrome, 256 Colors, 24-bit Color, 8-bit Gray Scale) | |
| TGA | 300 (no 96) | 256 colours (256 Colors, 16-bit Color, 24-bit Color, 8-bit Gray Scale) | |

The Color Depth lists above were read from Origin 2016's Export Graphs dialog on 2026-11-14 (each format picked in
Image Type, Image Settings expanded; `re-workspace/scratch/t05/d2`). Labels and order are Origin's.

**Monochrome and 16 Colors (measured 2026-11-14, Origin 2016 PNG export through the dialog; scripted `expGraph` and
`OIMG.INI` both ignore the depth, only the dialog sets it; `re-workspace/scratch/t05`, `o_mono.png`, `o_16.png`):**
- Monochrome is a 1-bit indexed PNG with the palette black, white. It is an ordered dither, not a threshold: the
  standard 8 x 8 Bayer matrix anchored at the top-left pixel. Inside a pure red line (grey 76) exactly the 19 cells with
  the smallest matrix entries are white, so a pixel is white where `4 t + 2` is below its grey level (`t` the entry,
  0..63; the `+ 2` places the boundary between entries 18 and 19, other levels unmeasured).
- 16 Colors is a 4-bit indexed PNG with an adaptive palette built from the picture (all 16 entries on a 5-bit-per-
  channel grid, for example white as 247, 247, 247 and red as 247, 0, 0); solid colours are not dithered (the red
  line kept its pixel count). Origin's quantiser is a library one and is not reproduced.
- 16-bit Color (BMP and TGA, measured the same way, `o16bit.bmp`, `o16bit.tga`): BMP is BI_RGB with a 40-byte header,
  16 bits per pixel, no palette, rows bottom to top padded to 4 bytes; TGA is image type 2 (true colour), 16 bits,
  descriptor 0 (no attribute bits, rows bottom to top). Pixels are `0RRRRRGGGGGBBBBB`; Origin sets the unused top bit
  on the page background (0xFFFF) and not inside the layer (0x7FFF). The picture has the same 64 colours as the
  256-colour export, so Origin draws in 15-bit colour first. Sheaf writes the same layout with the top bit always clear
  and each channel reduced by dropping its low three bits (other formats given 16-bit write 24-bit).
- 256 Colors (same probe) also has palette values on that 5-bit grid (levels 0, 57, 99, 140, 181, 214, 255): the
  palette holds exactly the picture's own 64 colours. Sheaf likewise keeps the exact colours when a picture has at
  most 256 (or at most 16 for 16 Colors) and uses NeuQuant only above that; it does not reduce colours to 15 bits
  first, so its entries are the 24-bit colours.

Trap: PNG and TIF default to 256 colours; use 24-bit for colour figures.

Sheaf: `crates/plot/src/raster.rs` writes Monochrome (Origin's 8 x 8 ordered dither; 1 bit per pixel in PNG and BMP, 8-bit indices with a two-colour palette in TIFF, PCX, PSD and GIF) and 16 Colors (a 16-entry NeuQuant palette, nearest colour; 4 bits per pixel in PNG and BMP, 8-bit indices elsewhere) for the formats whose Origin list has them, and PNG (256 colours via a NeuQuant palette, 24-bit with alpha, 8-bit grey), TIFF (24-bit, grey or 256 colours; none, PackBits or LZW), JPEG (quality 1-100, 24-bit or grey), BMP (8-bit palette or 24-bit), GIF (256 colours), TGA, PCX and PSD (24-bit, 8-bit grey or 256 colours). The dialog defaults to 24-bit, not Origin's 256 colours. [W:details-of-treenodes-in-export-graph]

**256-color TIFF:** Export Graphs supports **256 Colors** for TIFF, keeping the selected None, PackBits or LZW compression and the DPI metadata. It uses the shared quantizer on pixels composited onto white. The [TIFF 6.0 specification, section 5](https://www.itu.int/itudoc/itu-t/com16/tiff-fx/docs/tiff6.pdf) defines one 8-bit index per pixel, palette photometric interpretation (3), and ColorMap tag 320 with 256 red entries, then green and blue, each expanded to 16 bits by multiplying by 257. The existing TIFF encoder handles strip writing and compression; the indexed image supplies the palette tags through its public directory API. [Origin’s published TIFF settings](https://docs.originlab.com/origin-help/settings-in-expgraph-dailog/) list this depth and the same three compression choices. This addition uses the published settings, without a new installed-Origin comparison. Mono and 16-color TIFF are written since 2026-11-14 as 8-bit indices into a 2- or 16-entry table padded to 256 (Origin's TIFF bit depth for these modes was not measured).

Validation (2026-11-14): a palette/strip regression checks an odd-width image with transparent pixels, comparing its palette and decompressed index plane with indexed PNG for all three compressions. A UI test selects TIFF, 256 Colors and each compression, then checks the written tags. Pillow independently opened actual graph exports (301 x 231 pixels, 150.5 DPI) with identical index bytes and RGB pixels to the matching PNG; macOS `sips` also decoded each compression at the correct size.

**PSD export:** Export Graphs offers Photoshop Document (`.psd`) and the CLI/API use the same output-extension path. The [published Origin settings](https://docs.originlab.com/origin-help/settings-in-expgraph-dailog/) list PSD raster export and its color-depth choices. Sheaf's original writer follows the [Adobe PSD file-format specification](https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/): version 1, eight bits per channel, RGB/grayscale/indexed modes, raw planar composite pixels, planar 256-entry palettes, and a ResolutionInfo resource containing horizontal/vertical DPI. The graph is flattened onto white, with no layers, masks or alpha channels; the dialog states this. DPI is rounded to signed 16.16 precision and rejected if it rounds to zero or exceeds the positive signed range. The 30000-pixel side limit is checked before allocating a raster, together with the existing 400-million-pixel total limit. PSB and editable graph layers are not implemented; Monochrome is written as an indexed image with a black and white palette (not Photoshop's bitmap mode). Export size, margin and resolution settings apply as for other raster images. This implementation uses published references, without a new installed-Origin or Photoshop comparison.

Validation (2026-11-14): Pillow independently decoded synthetic graph exports in all three modes, with exact pixels matching same-depth PNG exports (RGBA PNG composited on white). Each was 301 x 231 pixels at 150.5 DPI in resource 1005. macOS `sips` also decoded all three to PNG at the correct size; it reported its default 72 DPI, so resolution validation uses the resource payload. UI tests select PSD and export each depth; CLI tests cover pixel sizing, fractional DPI, uppercase extensions, and oversized output refusal.

### 9.5 Vector options `tr2`
- **EPS**: PostScript level 2 (1/3), ASCII data (binary), RGB (CMYK), bitmap handling "individual bitmaps" (ignore, source copy, combine) at 150 dpi, preview none (TIFF/WMF), Adobe font type 1 (3, 42), embed outline fonts (substitution, built-in), vector line thickness 0, semi-transparency as raster at default resolution (opaque, or chosen dpi default 300), raster compression on.
- **PDF**: colour translation normal; line and fill mode device; background rectangle off; bitmap handling individual at 72 dpi (72..600); add text to metadata off; compression Flate for colour images, mono images, and text/graphics; fonts: embed = descriptor only (embedded, outline), stroke width 2, substitution off, TrueType off. For a self-contained publication PDF prefer embedded fonts [INF].
- **SVG, EMF**: no settings documented (U14). **AI, CGM, DXF, WMF**: legacy P2 (WMF placeable header on; DXF units inches).

### 9.6 Copy page / clipboard (P1)
Ctrl+J copies page (OLE), Ctrl+Alt+J copy as image, Ctrl+C copies as image by default. Size factor % of original: 100 from 2021b, 40 before, so 2016 default likely 40 [W:quick-help/export-graph-size-small]. Margin control as 9.2. 2016 ini: `AddEMFInCopyPage=1`, `AddWMFInCopyPage=1` (enhanced and Windows metafile put on clipboard) [F:Origin.ini]. View > Show Copy Page/Export Area (Ctrl+4) outlines the region.

Sheaf (Edit > Copy Page, Ctrl+J, `copy_page` in `crates/app/src/app.rs`): size factor 40 % (`COPY_PAGE_SIZE_FACTOR`), margin Auto (= Border, see 9.2), 300 dpi bitmap. On Windows the bitmap (`CF_DIB`) and an enhanced metafile of the same picture (`CF_ENHMETAFILE`, from the EMF writer, same size factor and margin) go on the clipboard in one session (`crates/app/src/winclip.rs`, through the `clipboard-win` crate that eframe's clipboard already brings, plus two Win32 calls). Not done: the WMF (`AddWMFInCopyPage`), the OLE object of Ctrl+J, and "copy as image" variants. Where the metafile cannot be put (not Windows, clipboard busy) the picture goes alone through egui's image copy.


### 9.7 Multi-page PDF (T32)
Origin 2018b added an "Option to export multiple graphs as a single multi-page PDF file" [W:originlab.com/2018b]. It is the `multipage` variable of `expGraph` (int, default `0`): set, the graph pages chosen by `export` go "to one PDF file with multi-pages" [W:x-function/ref/expgraph]. File names index in the order of the Graph Page list.

Not measured (no Origin on the macOS machine; measure on Windows with `tools/origin-ref/`): the dialog label and place of the check box, the name of the combined file, whether pages keep each graph's size or share one paper size, page order, whether `tr`/`tr1` settings apply to every page.

Sheaf (`sheaf_plot::render_multipage_pdf`, `crates/plot/src/multipage.rs`):
- One page per graph, in the order given (project order for "all graphs"). Each page is the one-page PDF export of that graph with the same size and margin settings (9.2, 9.3): its media box is the exported area, vector, text kept as text. svg2pdf converts each page to its own chunk (`to_chunk`), placed as a form XObject filling the page; fonts are embedded once per page.
- Paper variant (`render_print_multipage_pdf`, `sheaf print`): each sheet placed by Page Setup as in 19.4, one sheet per graph.
- Export Graphs dialog: with PDF and more than one graph, the check box "Export as multi-page PDF file" (Sheaf's label), off by default as Origin's `multipage`. One file, named from the file-name template of the first graph; the overwrite rule applies to it.
- CLI: repeated `--graph NAME` or `--all-graphs` on `sheaf export` (output must be `.pdf` for several graphs) and `sheaf print` (`--output` writes one paper-sized PDF; submission to `lp` sends one job per graph). MCP: `export_graph` with `graphs` or `all_graphs` (docs/spec/mcp.md).

---


### 9.7a Export from a workbook or with no window (T182, 2026-11-19, not measured)

Outside a graph window, File > Export holds Graphs..., Graph as Image... and Multi-Page PDF... (with the
worksheet exports). Origin's behaviour with no graph active was not measured; Sheaf opens the Export
Graphs window on **every graph of the project** (the window's "all graphs" choice), started from the graph
that was active most recently (it names the first file): Graphs... and Graph as Image... in PNG, Multi-Page
PDF... in PDF with "Export as multi-page PDF file" ticked (9.7). Everything else in the window is as from a
graph. With no graph in the project the items only say so in the status line.

### 9.7b Export Graphs from a folder (T250, the order not measured for expGraph)

Select Graph(s) also lists **All in Active Folder** and **All in Active Folder (Recursive)**
(expGraph's `export` "all in folder" and "recursive", 9.1) when the window opens from a graph or from
File > Export. They take the graphs of the Project Explorer folder that was open when the window
opened, the recursive one with its subfolders, in the order measured for `merge_graph` (15.11: the
folder's own graphs in project order, then each subfolder depth first, subfolders by name). If the
folder tree changes while the window is open the choices give no graphs and nothing is written.
File names follow File Name(s) with its substitution (`<long name>`, `<short name>`, `<index>`) and
the Overwrite Existing rule, as for All Graphs in Project; with PDF and two or more graphs, the
multi-page box (9.7) applies. Not built: Open (visible only), embedded graphs and Specified.

### 9.7c Preview tabs (T251, Origin's tab contents not measured)

The preview pane has Origin's two tabs, **Image** and **Graph** (gui-look: the pane shows tabs
"Image" and "Graph"). Image is the picture the export writes (the area of Export Settings' margin
mode). Graph shows the whole page with a red outline around the area the export takes (with Tight
the outline can reach past the page). The tab is part of what makes Auto Preview redraw. Test:
`the_graph_tab_outlines_the_exported_area_on_the_page` (`crates/app/src/dialogs/export.rs`).

### 9.8 Export Dialog Themes (T05)

**Origin 2016, measured on Windows 2026-11-15.** Five isolated synthetic-graph sessions
under the reference lock established the row above Image Type, its theme field and flyout.
The installed profile opened with `<Last used>` selected. The flyout offers Save as
`<default>`, Save, Save As..., System Default, Delete..., Compare..., Generate Script and
the available built-in/named themes. Save and Delete are disabled for a built-in selection.
Save As opens **Theme Save as...**, with Theme Name and OK/Cancel; Cancel returns to the
export window without exporting. Changing settings adds `*` to the selected named theme.
System Default cleared the field on this installation; its current image type was affected
by the installation's preferences, so this is not a fresh factory-format measurement.

A uniquely named PNG theme was saved, the type changed to TIFF and the named theme loaded
again: combo selections and WM_GETTEXT changed PNG -> TIFF -> PNG with the saved name restored.
The last bitmap was stale and is excluded as evidence of the restored type. The one generated
`.ois` was removed after Origin exited. Raw captures and the cleanup manifest remain local in
`scratch/t05-export-dialog-themes/`; no Origin theme file is included in Sheaf.

The [Dialog Themes guide](https://docs.originlab.com/origin-help/dialog-theme/) describes
custom default settings, last-confirmed settings and named reusable sets. The
[export guide](https://docs.originlab.com/user-guide/publishing-and-export/) documents saving
export settings as dialog themes. Newer versions' graph-specific settings (since 2021) are a
separate feature; they were not measured in Origin 2016 here.

**Sheaf.** The export window's Dialog Theme flyout saves and loads named settings, overwrites
a selected named theme with Save, marks modified named settings with `*`, saves a custom
`<default>`, restores System Default and
resets the custom default with Reset. Delete... asks for confirmation. Save As and Delete
prompts disable the export window until answered; Cancel/Escape leave settings files alone.
Named/default settings can be saved without exporting. Loading a theme changes the dialog,
not the graph or its undo history, and clears an obsolete overwrite confirmation and preview.

Themes keep the supported format, graph scope, filename/substitution, output folder, overwrite
rule, multi-page choice, DPI/depth/compression/quality, width/unit and margin/border settings.
They exclude graph/window ids, preview textures and pending operations. `<Last used>` is
written only after a successful export, including Apply, not Cancel, an unanswered replacement,
a failed export or a skip that writes nothing. A settings-write error is reported as a warning;
an image already exported remains a successful export.

The startup order is a saved custom default, otherwise last used, otherwise Sheaf's existing
system settings (PNG/24-bit, deliberately retained as described in T05). The injected dialog-theme
folder holds `graph-export.json` for named/last-used settings and `graph-export-default.json`
for the custom default. These use the existing locked, atomic theme-store API, separate from
importer files. A missing settings folder disables persistence. Unknown settings are ignored;
missing fields use the system settings, and invalid known fields reject the load before any
control changes. Name whitespace normalization, case-insensitive replacement and the 60-character
limit are Sheaf's choices, shared with import themes.

Compare, Generate Script, graph-specific theme attachment and Origin `.ois`
interchange remain open. Graph Theme application and the preview's Graph tab are still separate
T05 leftovers; no exporter or renderer behavior changes here.


## 10. Themes and templates

- **Graph theme `.OTH`** (XML, plain): a tree of numeric node ids with values; lists are base64 int32 arrays. A theme only changes the properties it contains; several can be applied, the topmost wins [W:themeref-themeorganizer]. Built-in: `Origin\Themes\Graph` (50 files, e.g. Black on paper, Color Publication, B&W Publication, FDA portrait/landscape, Physical Review Letters, Times New Roman Font, Ticks All In/Out, Line Style and Symbol variants, Fill Pattern variants, Layer No Element Scaling, Speed Mode ON/OFF, Night Sky, Opposite Lines). User themes live in the user Themes folder. Observed ids: page node 4 (6 colour, 10 with 11/12/13 incl. width and height); axes 1073741862 with X 536936487 and Y 537002023; ticks 1073741878 (57 minor, 58 major); grids 1073741887 (66 colour, 67 style, 68 width); list nodes 521 colour list, 522 line style list, 523 symbol shape list, 524 symbol interior list; 262 font for all text (opaque id; Times New Roman = 1852392883) [F:Themes\Graph\*].
- **Graph template `.OTP`** (binary; not read): "Save as template" keeps layout and style (page, layers, axes, plot types and styles, group rules, text, drawn objects, legend, speed-mode settings) and not the data; a new graph is built by plotting selected columns into it. Registry of user-visible templates and their categories: `Template.ini` (section 8). Base templates: LINE, SCATTER, LINESYMB, ORIGIN, STANDARD, DOUBLEY ... [F:*.otp names; INF for contents].
- **System theme for new graphs**: owner's `[Display] SysTheme=Dark Colors & Light Grids` and `[System Themes] GraphNewGraph=Dark Colors & Light Grids` [F:Origin.ini; User:Origin.ini]. Despite the name this theme contains only the grid line sets (3.5), the colour list (1.5) and one unexplained property (node 139 = 2); fonts/sizes/page come from the template.
- **Colour lists** `Themes\Color`: Q01 Black and RGB, Q02 Basic, Q03 Rainbow7, Q04 Paired Color, Q05-Q08 seasons, S01-S04 sequential, D01-D05 diverging, System Color List. **Palettes** `Palettes\*.pal` (about 60) are colour maps (P2).

Our app: store themes and templates as JSON (own format), with the same partial-override semantics.

---

## 11. UNKNOWNS: measure in running Origin

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

Tags as elsewhere in this document, plus **[COM]** for behaviour observed in Origin through COM/LabTalk on our own synthetic data (scripts and pictures in `re-workspace/scratch/graph-p0/`, git-ignored). The design decision is recorded in ADR 0009.

### 12.1 What Origin does

- **Two layers.** A double-Y graph is one page with two layers. Origin 2023 later added two Y scales inside one layer; Origin has only the two-layer form, template `DOUBLEY.OTP` (the current help calls it "2Ys Y-Y") [W:tutorials/double-y, origin-help/2ys-y-y-graph; F:DOUBLEY.OTP, no DblY.otp].
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

- **Graph scope.** A legend with scope `Graph` lists every visible drawable plot of every layer drawn in its layer's frame (`Graph::same_frame`, section 15.2; in a graph of one panel that is every layer): layer 0's plots in plot order, then layer 1's, and so on. Rows look as before. The box is placed in its own layer's frame. It is drawn after all layers of the graph (at the end of the last layer's drawing), so that no layer's curves cover it.
- **Margins.** A legend in a corner keeps `0.8 * font size` plus the inward length of the major ticks from each frame edge. The tick length counts for every axis of the legend's group that draws ticks pointing `In` or `Both` on that edge (its main edge, or the edge across with `opposite_ticks`). For a single layer this is the rule of figure-rendering.md; on a CV-style graph with a right axis the legend also keeps clear of the right axis's inward ticks.
- **No entries.** A legend with no entries is not drawn.
- **Automatic colours** are handed out per layer from that layer's palette (`Layer::recolor_automatic`). A plot that moves between layers takes the new layer's automatic colour unless its colour was chosen by hand. In the DoubleY template the first left curve is black and the first right curve is the third Sheaf palette colour, blue (30, 80, 220), as in Origin. Unlike Origin, a second curve on the same axis gets the next colour.

### 12.6 Commands

**Project API.**
- `Project::add_right_y_layer(graph, parent) -> Result<usize, ProjectError>`: when the panel that `parent` is drawn in already has a right-Y member, returns it and changes nothing; otherwise appends `right_y_layer` of that panel, linked to the panel's root and placed over the panel, changes the panel as in 12.2 and returns the new index. In a graph of one panel the panel is the root, as before; for the panels of a multi-panel graph see section 15.4.
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
- `--x-*` and `--y-*` set layer 0. Linked layers follow its X. (In a graph of several panels they set every panel of layer 0's group; section 15.6.)
- `--axis-colors plot|fixed` sets `color_follows_plot` on the Y axes of layer 0's group. When it is absent the template decides.
- `--colors`, `--labels` and `--line-width` apply to the plots of layer 0's group in legend order (layer 0's plots first). `--no-legend` hides every legend of that group. Further layers of a graph read from an Origin project (multi-panel graphs) are left alone, as before.

Examples: `sheaf plot cycles.csv -y Capacity --y2 Efficiency -o fig.png`; `sheaf plot it.txt -t cv -y "i/mA" --y2 "Q/C" --y2-title "Charge (C)" -o fig.svg`; `sheaf plot data.csv -t double-y -o fig.pdf`.

**Desktop app** (later, with ADR 0007): the Plot menu's multi-axis group with "Double-Y" (today it appears as a template in the Plot menu); a Graph toolbar button that adds a Right-Y layer; a "Left axis / Right axis" choice per plot that calls `move_plot_to_layer`, creating the right layer the first time; layer icons 1 and 2 that choose the layer the dialogs edit. Done in part: the layer icons choose the active layer, which the graph settings and the Graph and Format menus edit.

### 12.7 Origin project files (for the .opj reader and writer)

Observed on one Origin project [COM; one sample each]:
- **Frame.** Both layer records store the same absolute frame (layer header `0x71`, page units).
- **Linked-layer header bytes.** The linked layer differs from its parent at header `0x0AB` (= 1), `0x12D` (= 1), `0x02E` (1 becomes 9) and bit `0x80` of `0x0FC`. Turning the X link off changed none of them. Writing experiment (opj-format.md 13.12): `0x0AB` is the 1-based number of the parent layer and alone makes Origin report `layer.link`; in a three-layer graph the third layer linked to the second had `0x0AB` = 2, and `0x12D` was the layer's position (1, 2).
- **X link.** A straight X link shows as annotations named `X1` and `X2` in the linked layer; with the X link off, `X1` was gone. Writing experiment: the pair, with a 20-byte style block starting with `*`, gives `layer.x.link` = 1 and without it the value is 0; a custom link has the text `X1` in that block.
- **Axis visibility.** In each axis group, bit `0x40` of byte `0x26` of blocks 2, 3, 4 and 5 means: block 2 primary-side tick labels shown; block 3 primary line and ticks shown; block 4 opposite-side tick labels shown; block 5 opposite line shown. For Y, primary is left and opposite is right. This corrects opj-format.md section 10.3, which calls block 4 "title (not used)".
- **Automatic axis colour.** Stored as `F7 FF` at block 3 offset `0x0F` and as `F7 FF FF FF` at `0x92` and `0x16A`; black is 0 there.
- **Right title and legend.** The right title is the text object `YR` of the linked layer. The DoubleY template names its legend object `legend` in lower case. The writer (13.12) writes the legend as `\l(1.1) %(1.1)`, CR LF, `\l(2.1) %(2.1)` (layer, plot) in the parent, and Origin drew the entries of both layers.

Reader rule (implemented in `crates/io/src/opj/graph.rs`, `link_right_y_layers`):
- the Y axis of a layer is drawn on the right (`y_axis.side = Opposite`, title from `YR`) when its Y group shows the opposite side (block 4 labels or block 5 line) and nothing on the left, or, for any layer after the first, when it shows tick labels on the right (block 4). In the second case a left side the layer also shows is not drawn, and the import notes it;
- such a layer k > 0 with exactly the same rectangle (header `0x71`, eight bytes) as an earlier unlinked layer whose Y axis is on the left becomes a member with `link = {parent, x: Straight, y: None}`, `frame = {0,0,1,1}`; its X axis gets `side = Hidden` (the X scale is the parent's), and if its X group shows the top line, the parent gets `x_axis.show_opposite = true` to close the box. The parent's right Y line is turned off, and its legend lists the plots of both layers (`scope = Graph`); a legend object in the member is dropped;
- automatic axis colours (block 3 `0x0F` = `F7 FF`) set `color_follows_plot` on any axis;
- the legend object is matched without regard to case.

Checked on the research project `dy3.opj` (Origin; graphs G1 to G4 made by script, local only): G1 (the DoubleY command), G2 (X link turned off: still linked by this rule, since the X range stored in both layers is the same), G3 (layer 2 shows its Y axis on both sides) and G4 (layer 2's axis colour set to black) draw like Origin's own PNG of each, except: Origin draws the legend in a frame; Origin draws the top line in the plot's blue (12.8 keeps it black on purpose); and for G4 Origin's PNG still shows a blue right axis although the file stores black, which this reader follows. Whether Origin keeps the colour elsewhere or the PNG was made before the change took effect is unknown.

### 12.8 Deliberate differences from Origin

- The shared X range covers the data of all linked layers, so no curve is cut off by a rescale of the other layer.
- Colours increment within each layer.
- The X axes and the top line of the box stay in the X axis colour (black); only Y axes follow their plot.
- Adding a right axis gives one legend for the graph, not a second legend in the new layer.
- The legend sits inside the frame (TopRight), not above it.
- Every plot is listed (Origin's four-column run showed one fewer).
- The right layer of a CV-style graph copies the CV style: ticks in, symmetric range about zero, black axes, closed box.

Tick positions of the left and right axes are not aligned with each other (Origin does not align them either).

### 12.9 Unknowns

- The meaning of `0x12D`, `0x02E` and bit `0x80` of `0x0FC` of the linked layer's header (12.7; they were not needed for Origin to link the layer), and where Origin keeps the unit "percent of the linked layer" (`layer.unit` is 1 for a written layer, 7 for Origin's own).
- Whether Origin's blue for layer 2 is stored in the template or follows from a colour-list rule.
- Whether Origin offers "title colour follows plot" for axis titles (it appears in current help).
- How Origin splits a selection with two X columns (documented, not run).
- Origin's distance between the right tick labels and the right title, and the right title's size (not measured; Sheaf mirrors its left-side rule).
- Why Origin's DoubleY template uses finer automatic tick steps than the Line template for the same data (X -1..11 step 2 against -2.5..12.5 step 5); related to U4.
- Whether the owner wants coloured Y axes on double-Y figures (the DoubleY template colours them; `--axis-colors fixed` turns it off) and a symmetric right axis on CV-style figures.

---

## 13. Stacked lines by Y offsets and waterfall (P0, GP-OFF-01)

Status: researched on 2026-11-12 with Origin runs through COM on our own synthetic data (scripts and outputs in `re-workspace/scratch/graph-p0/`, git-ignored), plus a read-only look at the settings and counts of the owner's stacked XRD graphs. Tags as in section 12; **[COM]** is behaviour seen in Origin. The model decision is recorded in ADR 0010.

### 13.1 What Origin does

- **The idea.** "Stacked Lines by Y Offsets" (template `OffsetStackY.otp`) draws several curves in one layer, each moved up by its own amount so that they do not overlap. Only the drawing changes; the worksheet data stay as they are [W:stacklineyoffset-graph].
- **Offset modes.** The plot group of a layer has one of these modes, chosen on the Stack tab of Plot Details or with Graph > Offset Grouped Data in Layer [W:pd-dialog-stack-tab, pd-offset-tab]:
  - None.
  - Cumulative: each plot's Y values are added point by point to those of the plots before it. Stacked columns and areas use it; that is GP-COL-01, not this section.
  - Constant: a fixed step per plot; the first plot does not move.
  - Auto: offsets computed from the curve heights and a gap.
  - Individual: each plot has its own X and Y offset (set on the Offset tab, or by dragging a curve after clicking it twice).
- **Only group members move.** A plot added to the layer later outside the group kept offset 0 [COM].
- **The 2016 template uses Auto with a 10 % gap** [COM]. The current help says the template uses Individual offsets [W]; that is probably a later change.
- **The Auto rule in Origin** [COM, checked on seven data sets and on the owner's files]:
  - H is the largest height (largest minus smallest Y) of any curve of the group, and the gap is 10 % of H;
  - the first curve does not move;
  - every later curve is moved up so that its lowest point lies one gap above the highest point of the curve before it, after that curve was moved.

  The current help describes a gap computed pair by pair; for the first check row of 13.11 that rule would give 141 for the fourth curve, while Origin gave 150.
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
    pub x_multiplier: f64,         // default 1; not written when 1
    pub y_multiplier: f64,         // default 1; not written when 1
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
It also gets `x_multiplier` and `y_multiplier` (default 1). Positive finite factors scale the
original values before adding the offsets; zero, negative and nonfinite factors draw as 1
(Origin 2016 measurements in 18.1). Other stack modes ignore these stored factors. Followers
inherit the leader's complete placement, including multipliers.

13.4.4 **None and Waterfall.** Every member gets (0, 0).

13.4.5 **Followers** get the pair (and the member index) of the first member whose Y column is the followed column; without such a member, (0, 0) and no member index. Plots whose Y column holds no numbers get (0, 0) and are not drawn.

`sheaf_plot::stack_offsets(project, graph, layer)` returns these pairs, one per plot in plot order. The app will use it to copy the offsets in effect into the layer when the user switches to Individual, so that the picture does not jump.

### 13.5 Drawing

- Every use of a data point (x, y) uses (mx * x + ox, my * y + oy) instead (multipliers are 1 outside Individual):
  - lines and symbols;
  - error bars, from my * y + oy - e to my * y + oy + e (error magnitudes stay unscaled);
  - column bars, from oy to my * y + oy;
  - the data range for automatic axis limits (figure-rendering.md section 2, step 1).
- When an offset is 0 the value is used exactly as stored, so a layer with `stack` None is drawn exactly as before, with byte-identical SVG.
- Worksheets, analysis tools and fits always use the stored values.
- Limits, ticks and tick labels follow figure-rendering.md section 2, applied to the moved values.
  Tick labels show axis coordinates. Data labels and the Data Reader show source values; the
  reader snaps to the transformed point. The Integrate gadget maps its range and shading through
  the placement and converts pointer X back to source units; its area uses source data.
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

Import rule (implemented): if any curve of a layer has a non-zero, finite offset (Origin's "no value" double counts as 0), set `stack = Individual` and add an entry with each curve's offsets for its Y column (the first curve of a column wins). This redraws the graph exactly as Origin saved it. Mapping `0x13A`/`0x148` to Auto can follow once the mode codes are confirmed. The writer (opj-format.md 13.12) does not need the mode fields: Origin moves every curve by the offset in its record when the layer header byte `0xFD` is `0x50`, whatever the mode was, and without a plot group. For the Auto mode Origin's own files use a plot group (byte `0x06` of the records: `0x29` on the first curve, `0x19` on the others), the no-value double at header `0x135` and the gap fraction at `0x13D`, and then computes the offsets again at load. The same fields with a step at `0x135` give the Constant mode (curve k moved by k times the step). The offsets Origin computed agree with 13.4. No X offset field was found (`0x1B2` is ignored); the f64 at `0x1C2` is a Y factor. Checked on the owner's XRD project: its six-pattern graph stores six increasing offsets (the first 0) and now draws each pattern in the band of its label, as in the owner's figure.

### 13.11 Check values (Origin, Auto, gap 10 %, X = 0, 1, ..., 20)

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

### 13.12 Deliberate differences from Origin

- The `stacked` template hides the Y tick labels and ticks and closes the box (the owner's style); Origin's template shows them.
- The waterfall is a flat 2D drawing without a Z axis, planes or hiding fill, with its own default shifts, and it draws the first curve in front.
- Individual offsets are stored per Y column, not per plot.
- A fitted curve moves with its data (`follows`); Origin's behaviour for fit curves in a stacked layer was not tested.
- The automatic axis range follows Sheaf's rule (figure-rendering.md section 2), not Origin's rescale rule, so the limits differ (0..200 instead of -10..160 for the first check row).

### 13.13 Unknowns

- Origin's codes for the modes other than Auto: None, Constant, Individual and Cumulative cannot be set by script in 2016 (`set -sy` is refused and there is no layer property). The modes were clicked on the Stack tab (13.14); the bytes they write were not compared.
- The storage of X offsets and multipliers (13.10) is inferred; the layer header differences are unexplained.
- The Auto gap percent can be edited on the Stack tab (13.14), and 8 % and 20 % follow the rule; the H = 10 rule for flat curves is confirmed only at 10 %.
- Whether Origin's Auto uses a plot's display row range for the curve heights; Sheaf uses the plotted rows.
- What Origin's Data Reader shows on a moved curve (the stored or the moved value).
- Origin's exact waterfall geometry.
- Owner preferences: whether the stacked template should hide the legend (he deletes it and types the curve names next to the curves); whether he wants an option that writes each curve's name at its right end (a possible follow-up); whether hidden curves should keep their place (Origin's behaviour, adopted here).
- The current online help describes Individual offsets in the template and a pairwise Auto gap; neither matches Origin, and later versions may differ.

### 13.14 Plot Details Stack tab (measured 2026-11-14)

Clicked in Origin with `tools/origin-ref/origin-dialog.ps1` on a layer of three or four grouped lines (`plotxy`
then `layer -g`; Y = x, x+1, x+2, x+3 for X = 0..10, so each curve is 10 high); offsets read after OK with
`layer.plot=k; get %C -sy`; pictures and control lists in `re-workspace/scratch/t13stack`:

- The tab: an Offset group with the choices None, Cumulative, Incremental, Constant (a box), Auto (Gap (%) 8 and
  Keep Proportion of Plot Range) and Individual (X and Y ticks, Y ticked); below it Display column/bar plot by
  sorted order, Use Subgrouping (in Group tab) for Cumulative/Incremental, Normalize to Percent for
  Cumulative/Incremental and Show Connect Line on Stacked Column/Bar (greyed for line plots). The boxes are
  greyed unless their choice is on. Without a plot group Origin shows a hint that the plots must be grouped
  first, and OK moves nothing.
- Auto as it opens (8 %): offsets 0, 9.8, 19.6 (gap 0.8 = 8 % of the height 10). Gap 20: 0, 11, 22. Both follow
  the rule of 13.1, so the dialog's gap is the percent of that rule (the stacked template stores 10).
- Constant: the box starts empty, with a note that it takes numbers separated by spaces, used in a loop.
  "3" gives 0, 3, 6; "2 5" on four curves gives 0, 2, 7, 9 (the steps between neighbours, repeated).

Sheaf (`graph_details.rs` `stack_tab`): the same Offset group of choices. Constant takes one step, not a
list; Auto chosen on this tab starts at 8 %; Cumulative is live for line, scatter and line + symbol plots
(13.15); Incremental, Keep Proportion, the X/Y ticks and the column/bar options are greyed (no stacked columns
yet, GP-COL-01); Waterfall is Sheaf's own extra choice
with its two shifts. The per-plot offsets of Individual stay listed under the group.

### 13.15 Graph > Offset Grouped Data in Layer and Cumulative lines (T62 slice 1)

Origin facts (13.1, 13.14, measured as far as listed there): the submenu has None, Cumulative, Constant, Auto
and Individual, one of them ticked; Cumulative adds each plot's Y values point by point to those of the plots
before it. **Not measured** (Sheaf's choices, labelled so): whether the menu items prompt, what Cumulative does
with missing rows or different X, and the menu's undo text.

Sheaf:

- **Menu.** Graph > Offset Grouped Data in Layer has the five items, live, with a tick on the active layer's
  present mode. A choice changes the active layer's `stack` as one history step (undo text "Offset grouped data
  in layer"); the mode the layer already has is a no-op (no history step, the stored values stay). Plot Details'
  Stack tab edits the same field, so the tick and the radio buttons always agree (Waterfall, Sheaf's own
  mode, has no menu item and ticks none).
- **No prompts.** Constant and Auto use stored values as the Stack tab does: a layer not yet in that mode starts
  at Constant step 1 or Auto gap 8 % (the tab's own start values); individual offsets stay in `plot_offsets`.
  Origin's own prompting (if any) is unmeasured.
- **Cumulative rule** (`core/stack.rs` `cumulative_add`): the members (13.3) are summed in plot order,
  pointwise by **row index**, whatever their X values; plot k is drawn at `Y_k[row] + sum over j<k of Y_j[row]`.
  A previous plot adds 0 at a row where it is missing (not a finite number), outside its row range, or removed
  by a worksheet row filter, or hidden by a mask while Hide Masked Points is on (drawing treats those points as
  gaps, so they add nothing either), so one hole does not break the curves above it; the plot's own missing value stays a
  gap. Hidden plots still add their share (13.3). A fitted curve following a member is moved by that member's
  added amount at the fit's own X (linear interpolation along the leader's X, clamped at its ends).
  Applies to line, scatter and line + symbol plots; other plot types in the layer are drawn as they are and add
  nothing. Places (`places`) carry no offset in this mode (it is not a pair of offsets), so the `.opj` writer and
  Individual are unaffected: the writer warns that Cumulative was written as plain curves and the `.sheaf`
  companion keeps the mode.
- **Everything follows the drawing.** The plot data (`PlotData.y`), hence automatic axis ranges, filters,
  symbols, drop lines, labels and error bars, the Data Reader and the Data menu tools (which take the added share
  back when a point is moved) use the summed Y. The Integrate gadget's shading still follows the source values.
- **Menu and Plot Details only offer Cumulative where it acts:** the Cumulative item (and the Plot Details
  Cumulative radio button) is greyed, with the hover reason "Cumulative needs a line, scatter or line + symbol
  plot in the layer", when the active layer has no such plot (a column, bar or area layer would otherwise tick and
  add an undo step with no visible change). The other modes are unaffected.
- **CLI/MCP:** `--stack cumulative` (and the MCP `stack` string).


### 13.16 Plot > Statistics > Stacked Histograms (T62 slice 2)

Origin: the command makes one graph with one histogram panel per selected Y column, the panels stacked
vertically.

**Measured 2026-11-16 in Origin 2016** (Plot > Statistics > Stacked Histograms on three columns A, B, C of
200, 150 and 100 deterministic values, `re-workspace/scratch/t62-origin`, PNG `stackhist.png`):
- A portrait page (the export was 1600 x 2089 px) with three layers. Layer 1 holds the **first** selected
  column and sits at the **bottom** (top 66.67 %), layer 3 the last one at the top (top 10 %); every layer is
  at left 25 %, width 70 %, height 23.33 %, so the panels are 5 % of the page apart (not touching).
- **Each panel has its own bins and its own X scale** (A: -2.5..2.5 step 2 with bins of 0.5; B: -2..5 step 2,
  bins of 0.5; C: -1.6..-0.4 step 0.2, bins of 0.1) and its own count axis (0..60, 0..34, 0..23). Every
  panel shows its X tick labels; only the bottom panel has the X title (the first column's name); every
  panel has the Y title "Count".
- Every panel is drawn the same way: red outline and a red diagonal hatch on white, not the next colour of a
  list; each panel has its own legend at its top right (overlapping the panel above).

**Differences from Sheaf's provisional rules below (to change in a code slice):** panel order (Sheaf puts the
first column at the top), shared bins and linked X (Origin: per-panel bins and scales), colours (Origin: the
same red hatch everywhere), page orientation and the gaps between panels.

Sheaf (`Project::plot_stacked_histograms`; GUI Plot > Statistics > Stacked Histograms; CLI
`sheaf plot -t stacked-histograms`):

- One panel per Y column that holds numbers (text columns are left out; no numbers at all is an error),
  **top to bottom in selection order**. The panels reuse `PanelLayout::stack` (one column, no gap, X linked to
  the first panel so all share one X scale and the inner panels show no X tick labels) with the order turned
  top-down, and the shared panel helpers `plot_panels`/`panel_layers` of 15.
- **Shared bins** (provisional): the bin width, first edge and last edge are the ones the histogram rule
  (frequency-counts automatic bins, measured for single histograms) gives for the **union of the finite values
  each panel's histogram uses**: the rows its sheet's row filter lets show, and not masked while Hide Masked Points
  is on (`Project::shown_values`), so a hidden outlier does not widen the shared bins; they are stored explicitly on every plot (`hist`), so the panels have identical bin edges
  and later editing one panel's bins does not touch the others. Each panel has its own count axis.
- Each panel gets the next colour of the layer palette (starting at the Histogram template's red).
- `--panels` and `--y2` (a right Y axis, also refused by `plot_columns_double_y`) are refused with this template; the legend of each panel names its column.


### 13.17 Waterfall Y:Color Mapping and Z:Color Mapping (T62 slice 3)

Origin: Plot > Y-offset/Waterfall has Waterfall Y:Color Mapping and Waterfall Z:Color Mapping (3D waterfall is B15).

**Measured 2026-11-16 in Origin 2016** (four Gaussian columns B..E against A, both menu items,
`re-workspace/scratch/t62-origin`, `waterY.png`, `waterZ.png`):
- Both draw Origin's oblique waterfall: a skewed three-sided frame with grey grid lines, the curves set back
  one behind the other along a Z axis whose ticks are labelled with the column names (B at the back, E at the
  front), the Y axis on the right; no legend and no colour scale.
- **The colour runs from red at the low end through orange, yellow, green and cyan to blue at the high end**
  (the reverse of Sheaf's blue-to-red ramp).
- Y mapping colours each curve along its length by its Y value on one scale for the layer (baselines red, the
  tallest peak blue, a peak half as high green-yellow); the colour changes smoothly along the curve.
- Z mapping gives each curve one colour by its place: B red, C yellow, D cyan, E blue.

**Differences from Sheaf's provisional rules below:** the ramp direction (Origin red low, blue high) and the
oblique 3D-looking frame of Origin's waterfall template (Sheaf's waterfall is the flat 2D offset of 13.6).

Sheaf (`Layer::waterfall_color` None/Y/Z, `Template::WaterfallY` = `waterfall-y`, `WaterfallZ` = `waterfall-z`;
GUI Plot > Y-offset/Waterfall; CLI `sheaf plot -t waterfall-y|waterfall-z`): the 2D waterfall of 13.6 (placement
unchanged), the legend hidden, and:

- **Colours** come from the shared ramp (`plots::map_color`: blue, cyan, green, yellow, red) also used by colour
  mapped symbols, parallel plots and sunburst plots.
- **Y mapping:** the line is built exactly as the plain line (the plot's connect mode, axis breaks, missing data
  with the page's "Connect line across Missing Data", symbol-line gaps: one code path, `draw_line`); only the
  colouring differs. The final polyline is split into its straight pieces and each is coloured by the mean Y of its
  two ends (read back off the page in data units, less the curve's stack offset, so it is on the source Y scale; a
  segment colour, not a gradient inside a segment; runs of one colour are drawn as one path). One scale for the
  whole layer: the smallest and largest shown Y of all curves. Fill under the curve is not applied to the mapped
  line; symbols of line + symbol plots keep the plot colour.
- **Z mapping:** each curve is one colour, from its Z value on a scale from the smallest to the largest Z of the
  layer. Z is the number in the long name of the curve's Y column; else the number in its comments; else the curve's
  place in the stack counted from 1 (`core::stack::waterfall_z`).
- **Colour scale:** `hexbin::draw_color_bar` beside the layer's whole frame (the right end of the back curve),
  titled Y or Z, ticks at nice values; an equal-valued scale puts one tick in the middle.
- The mapping is stored in the layer (`waterfall_color`), so it round-trips in `.sheaf`. The `.opj` writer writes
  the curves in their plot colours, without a scale, and warns (the companion keeps the mapping).
- A mapping on a layer that is not drawn as a waterfall (linked layers, other stack modes) is ignored.


---

## 14. Text tick labels and column charts (P0, GP-COL-01)

Use: sample names under the X axis (ICP-OES results per sample) with one or several elements drawn as bars side by side. Status: researched on 2026-11-12 with Origin runs through COM on our own synthetic data, plus exports and byte comparisons of copies of the owner's projects (scripts and pictures in `re-workspace/scratch/graph-p0/`, git-ignored). Tags as in section 12; **[COM]** is behaviour seen in Origin. Bar and label positions were measured on exported PNGs. Where the current online help **[W]** and Origin disagree, Origin decides.

### 14.1 What Origin does

**Plotting against a text X column**
- A Y column whose X column holds text is drawn against the row number: row 1 at x = 1, row 2 at x = 2, and so on [W:quick-help/xcol-text-plotting]. This holds for line, scatter and column plots [COM].
- Origin sets the bottom axis to the tick label type "text from dataset" by itself, with the X column as the dataset (`layer.x.label.type` = 2) [COM]. Under that type the major tick at value v shows the cell of row v [W:axesref-ticklabels].
- These ticks get no label [COM]: ticks at values that are not whole numbers, and whole-number ticks without a row (0, n + 1, ...).
- An empty cell is printed as `--` [COM]. A column that holds mostly text prints its few numbers as written [COM].
- Automatic range [COM, 4 to 120 rows]: from 0.5 to n + 0.5, not rounded to ticks; major step 1 and one minor tick between majors, so the minor ticks mark the borders between rows and both ends.
- With 450 rows Origin fell back to its numeric step: majors every 100, labels from rows 100, 200, ... [COM]. Its options name 400 as the most major ticks on a text axis [W:options-dialog-axis-tab].
- Labels are never thinned and not rotated by default; long or many labels overlap [COM].
- A set rotation (degrees, counter-clockwise, `layer.x.label.rotate`) hangs each label from its highest point, directly below the tick [COM]: with a positive angle the end of the text is at the tick, with a negative angle its start, and at 90 degrees the text is centred on the tick. Origin moved neither the axis title nor the layer, so long turned labels ran over the title.
- A column designated Label is not used for tick labels [COM].
- "Tick-indexed dataset" (row i labels the i-th major tick) and "tick-indexed string" (labels from a typed list) also exist [W:axesref-ticklabels]. P2.

**Column plots**
- Bars rise from Y = 0, and the automatic Y range includes 0; negative values hang below 0 [COM].
- X spacing s: 1 for row numbers. For numeric X it is the mean spacing, (largest X - smallest X) / (n - 1) [COM]. With uneven X, bars can overlap their neighbours. Which rows n counts [COM, two cases]: a row with an empty X but a Y value counts (X = 1, 2, -, 4, 5 gave bars 0.8 wide at gap 20); an empty row (no X and no Y) between two groups of bars does not (the owner's Mn dissolution graph: X = 1, 1.05, 1.1, -, 1.3, 1.35, 1.4 and gap 60 gave bars 0.63 of the neighbour distance, that is 0.4 × 0.4 / 5).
- Gap between bars, g percent (default 20, range 0 to 100) [W:pd-dialog-spacing-tab]. The bars at one X position together take (1 - g/100) × s, centred on X [COM: 0.80 s at g = 20 and 0.50 s at g = 50, for one plot and for three]. The help calls it a percentage of the bar width; the measurements show a share of the spacing.
- Plots made together form a group: side by side in plot order, touching, each (1 - g/100) × s / n wide [COM]. A column plot added to the layer on its own later is drawn at the same place, full width, on top [COM].
- Overlap, o percent (default 0, range -100 to 100, negative values leave a gap; grouped plots only) [W:pd-dialog-spacing-tab]. Not reachable from LabTalk in 2016 (`set -vo` changed nothing) [COM], so its geometry was not measured.
- Automatic X range of column plots on numeric X: first X - s/2 to last X + s/2, not rounded [COM].
- Defaults [COM]: black fill and a 0.5 pt black outline; a group steps the fill colour (black, red, ...); error bars centred on each plot's own bar.
- Exports of copies of the owner's projects match these rules: a single column plot with gap 40 (bars 0.6 of the spacing), a group of two with gap 60 drawn side by side, and a plot whose X values come in groups (so that the mean spacing is larger than the distance between neighbours) drawn as touching bars [COM].

### 14.2 Model

| Where | Field | Default | Meaning |
|---|---|---|---|
| Axis | `tick_label_source` | `Auto` | `Auto`: on the X axis the cells of the first plot's X column when that column holds text, otherwise numbers; a Y axis shows numbers. `Numbers`: always numbers. `Column { column }`: the cells of that column. |
| Axis | `tick_label_angle_deg` | 0 | Counter-clockwise rotation of the X tick labels in degrees. A Y axis ignores it. |
| Layer | `bars.gap_pct` | 20 | Gap between the groups of bars, in percent of the spacing s; clamped to 0..100. |
| Layer | `bars.overlap_pct` | 0 | Overlap of neighbouring bars of the group, in percent of a bar's width; clamped to -100..100; negative values leave a gap. |

```rust
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum TickLabelSource { Auto, Numbers, Column { column: ColumnId } }
pub struct BarSpacing { pub gap_pct: f64, pub overlap_pct: f64 }
```

- Every new field has a serde default and is not written while it has its default value (`{"kind":"auto"}` and `{"gap_pct":20.0,"overlap_pct":0.0}` are left out), so files written before this feature load unchanged, and graphs that do not use it are saved exactly as before. `FORMAT_VERSION` stays 1.
- A label column counts as a column the graph reads (`Graph::referenced_columns`).
- **Groups.** Sheaf has no plot groups, so the bar settings live on the layer and all visible column plots of a layer form one group. An overlap of 100 draws them on top of one another, as Origin does with column plots that are not grouped.
- **Automatic colour.** For a column plot the automatic colour (`Layer::recolor_automatic`) and `--colors` set the fill (`symbol.color`) only; the outline keeps its line colour.

**Template `column`** (`Template::Column`, label "Column"; in `Template::ALL` after `line-symbol`, and in the desktop app's Plot menu as Column/Bar/Pie > Column, where Origin has it):
- page, fonts, legend (top right) and colours (black, red, blue, ...) of the `line` template;
- plot kind Column, outline black 0.5 pt, fill from the palette;
- a closed black box; Y ticks pointing in with one minor tick, as in the owner's house style;
- no X ticks: a tick pointing in would cross the middle bar of each group, and the owner's column figures show none;
- no grid.

### 14.3 Category axis and labels from a column

"First plot" means the first visible plot, whose Y column holds numbers, among the plots that use the axis (for a linked layer, section 12.3, the plots of every layer that shares it).

**Category axis.** A linear X axis is a category axis when its first plot has no X column of numbers and either
- (a) that plot's X column holds text, or
- (b) `tick_label_source` is `Column` with a column that exists.

The rows of a plot without an X column of numbers stand at the row numbers 1, 2, 3, ... as before.

**Label column.**
- `Numbers`: none.
- `Column { c }`: column c, when it exists. Any column type works; its cells are shown as in the worksheet.
- `Auto`, X axis: the first plot's X column when it holds text.
- `Auto`, Y axis: none.

**Range of a category axis.** Let lo and hi be the smallest and largest position of a drawn row (finite X and Y, inside the row range) over the axis' plots; without rows lo = hi = 1.
- The automatic limits are lo - 0.5 and hi + 0.5. They are never rounded, whatever `auto_range` says.
- A fixed `min` or `max` replaces its own side; two fixed limits follow the usual rules, reversal included.
- Major step: `major_step` when set; otherwise 1 when max - min <= 200; otherwise `nice_step((max - min) / 5)` (450 rows give 100).
- Major ticks lie at whole multiples of the step. Minor ticks follow `minor_ticks` as on any linear axis; 1 gives ticks at 0.5, 1.5, ..., n + 0.5.

**Labels.** On any axis with a label column, category or not, for each major tick v let k = round(v):
- if |v - k| <= 1e-9 × max(1, |v|) and 1 <= k <= the column length, the label is the cell of row k;
- otherwise the label is empty, and an empty label draws nothing;
- `tick_format` and `tick_label_divisor` do not apply; labels use the text markup of all other labels.

A category axis whose source is `Numbers` shows ordinary numbers (the row numbers) over the same range.

**Axis title.** When `title` is None, the X title is:
1. the title of the first plot's X column (long name, else short name, plus units), whether that column holds numbers or text;
2. otherwise, when the labels come from a column, that column's title;
3. otherwise `Row`.

Rule 1 also changes graphs read from `.opj` files whose X title is Origin's `%(?X)` over a text X column: they show the column's name, as Origin does, instead of `Row`.

### 14.4 Column plots

**Spacing s of a layer**, taken from its first visible column plot:
- s = 1 when that plot has no X column of numbers (the bars stand at row numbers);
- otherwise s = (xb - xa) / (n - 1), where xa and xb are the smallest and largest X of the drawn rows (finite X and Y) and n counts the rows of the row range that have a finite Y;
- s = 1 when n < 2 or the result is not a number above 0.

**X range.** On a linear X axis that is not a category axis and shows a column plot, the data range [lo, hi] becomes [lo - s/2, hi + s/2] and is used without rounding, as for `Tight` (s from the first column plot among the axis' plots). The step is `nice_step((max - min) / 5)` unless `major_step` is set, and fixed limits replace their side, so graphs read from `.opj` files (which have fixed limits) keep their range. A category axis already has this room.

**Y range.** On a linear Y axis the data range includes the base of every visible column plot: 0, or the plot's offset in a stacked layer (section 13.5).

**Placement.** With n the number of visible column plots of the layer, j the 0-based index of this plot among them in plot order, and g = gap_pct/100 and o = overlap_pct/100 after clamping:
- W = (1 - g) × s
- w = W / (1 + (n - 1) × (1 - o))
- d = w × (1 - o)

Each row with a finite position x and value y draws one rectangle:
- horizontally from left = x - W/2 + j × d to left + w, in data units;
- vertically from the base to y; the base is 0 (the plot's offset in a stacked layer) on a linear Y axis and the lower axis limit on a log axis;
- a bar with a corner that cannot be placed is skipped;
- filled with `symbol.color` and outlined with `line.color` at `line.width_pt` when that is above 0;
- plots in plot order, rows in row order.

Check values, in units of s:

| Case | Bars at one X position, from left to right |
|---|---|
| n = 1, gap 20 | x - 0.4 .. x + 0.4 |
| n = 3, gap 20 | x - 0.4 .. x - 0.1333, x - 0.1333 .. x + 0.1333, x + 0.1333 .. x + 0.4 |
| n = 3, gap 50 | each bar 1/6 wide, the group from x - 0.25 to x + 0.25 |
| n = 3, overlap 100 | all three from x - 0.4 to x + 0.4 |
| n = 2, overlap -50 | w = 0.32: x - 0.4 .. x - 0.08 and x + 0.08 .. x + 0.4 |
| n = 2, gap 150 (clamped to 100) | bars without width at x |

**Error bars** of a column plot are drawn at the middle of the plot's own bar, left + w/2.

**Legend sample.** A rectangle centred on the row, as long as a line sample (2.2 × the legend font size) and one font size high, filled and outlined like the bars.

### 14.5 Rotated X tick labels

- Let a be `tick_label_angle_deg` taken to (-180, 180] (a value that is not a number counts as 0). When a = 0, labels are placed exactly as in figure-rendering.md section 3.
- Otherwise work in label coordinates: u runs along the text, w points down, and the origin is the start of the baseline. The label's box has the corners (0, -0.72 S), (Wt, -0.72 S), (Wt, 0.21 S) and (0, 0.21 S), where Wt is the text width and S the tick font size.
- Turn each corner to page offsets: (u cos a + w sin a, -u sin a + w cos a).
- Let ymin and ymax be the smallest and largest page y of the corners, and xt the mean x of the corners whose y equals ymin (within 1e-9 relative).
- Place the baseline start at (tick x - xt, top - ymin), where top is the tick end plus the label gap, as for unturned labels, with text anchor start and rotation a.
- The highest point of the label then lies directly under the tick: a positive angle puts the end of the text there, a negative angle its start, and at 90 degrees the text is centred.
- The label block reaches down to top + the largest (ymax - ymin) of the labels that are not empty. The X title and the keep-on-page rule use that bottom, so unlike Origin the title never overlaps turned labels.
- An X axis on the top edge mirrors this: the lowest point of the label sits directly over the tick.
- Check values (fixed metrics, "S1" at 12 pt, Wt = 13.2): at 90 degrees the start is at (tick + 3.06, top + 13.2) and the block 13.2 deep; at 45 degrees (tick - 3.224, top + 15.443), block 17.225; at -45 degrees (tick - 6.109, top + 6.109).

### 14.6 Import

Delimited text: when "first column is X" is on (the default), the first column is designated X whether it holds numbers or text, as Origin's text import designates columns by position. Other text columns stay Label. Only new imports change; saved projects keep their designations. Analyses that need numbers already skip an X column of text and use row numbers.

### 14.7 Origin project files

In the 515-byte curve record (opj-format.md section 10.5) [COM, byte comparisons of one graph saved with gaps 20, 40 and 55, and the owner's files]:
- u16 at `0x19`, the symbol size field of other plot types, holds the bar width of a column plot in percent of the spacing, times 50: 4000 is a gap of 20, 3000 of 40, 2250 of 55, 2000 of 60. A copy at `0x11A` holds the same number. The owner's projects use 2000, 2250, 3000, 3600 and 4000.
- The reader sets the layer's `bars.gap_pct` = 100 - width / 50 from the layer's first column plot, when the width lies within 0..100 percent.
- Byte `0x06` of the record marks plot groups: bit `0x20` on the first plot of a group and bit `0x10` on the others (a lone column plot had neither). The reader makes a group of such a run, `Plot::group`, with the members coloured from Origin's colour list in order; the writer writes groups again (opj-format.md 10.9, 13.13).
- Where Origin stores the tick label type and its dataset is not known. The overlap was looked for in 2026-11 (opj-format.md 13.12): `set` options and `layer.overlap` changed no byte, so it was not found. The writer writes the gap at `0x19` and `0x11A` of every column record; Origin drew gaps of 0, 20 and 60 percent as written.

### 14.8 Command line

`sheaf plot` and `sheaf export` take:

| Option | Values | Default |
|---|---|---|
| `--x-labels` | `auto`, `numbers`, or a column's short or long name | the template's (`auto`) |
| `--x-label-angle` | degrees, -90 to 90, counter-clockwise; negative values as `--x-label-angle=-45` | 0 |
| `--bar-gap` | percent, 0 to 100 | 20 |
| `--bar-overlap` | percent, -100 to 100; negative values as `--bar-overlap=-20` | 0 |

- A column name is looked up in the sheet that holds the Y column of the graph's first plot; the words `auto` and `numbers` come first.
- `--x-labels` and `--x-label-angle` set the first layer's X axis (linked layers follow it); `--bar-gap` and `--bar-overlap` set every layer of the first layer's group.
- Errors name the option and the value: "--x-label-angle must be between -90 and 90 (got 100)", "--bar-gap must be between 0 and 100 (got 120)", "--x-labels: no column named 'Nope' in sheet 'icp' (columns: A, B, C, D) ...".
- `-t column` chooses the template.

Examples: `sheaf plot icp.csv -t column -o icp.png`; `sheaf plot icp.csv -t column -y Li,K,Mn --x-label-angle 45 --bar-gap 30 -o icp.pdf`; `sheaf export project.sheaf --graph Graph2 --x-labels Sample -o g.svg` (names in a label column).

### 14.9 Desktop app (later, with ADR 0007)

- The Plot menu lists Column/Bar/Pie > Column today, through `Template::ALL`.
- Graph settings, X axis tab: "Tick labels: Automatic / Numbers / From column" with a column list, and "Rotation".
- Plots tab: kind Column; for a layer with column plots, "Gap between bars %" and "Overlap %".

### 14.10 Deliberate differences from Origin

- An empty cell gives no label (Origin prints `--`). Missing numbers in a numeric label column show as the worksheet shows them.
- One tick per row up to a span of 200; Origin keeps one per row further, up to its limit of about 400.
- One spacing for bar widths and the X range. With an X cell missing in a row that has a Y value, Origin used the valid points for the range (1, 2, -, 4, 5 gave 0.333..5.667) and Sheaf gives 0.5..5.5.
- No automatic rotation and no tick-indexed label types.
- The X axis line stays at the frame bottom when bars go below zero; Origin's column template draws it at Y = 0.
- Plot groups are not modelled: all column plots of a layer form one group.
- The `column` template closes the box, draws Y ticks pointing in and no X ticks (the owner's style); Origin's template has ticks out and open axes.
- The title moves below turned labels instead of being overlapped by them.

### 14.12 Stacked, 100 % stacked, horizontal and floating columns (plot types 213, 215, 207)

Measured with Origin through COM on X = 1..4 and Y = (2,3,4,5), (1,2,1,2), (3,1,2,1) (`re-workspace/scratch/plot-types/p5.lines.txt`, pictures `sc213.png`, `scp213.png`, `sb216.png`, `fb207.png`, `bar.png`, `floatcol.png`). `plotxy` only stacks with the plot type of the stacked kind (`plot:=213` with template `StackColumn`, `plot:=213` with `StackColP`, `plot:=216` with `StackBar`); `plot:=203` with the stacked template draws plain side by side columns.

- **Stacked column.** All plots of the layer stand in one slot per X position, width (1 - g) x s as for one column plot (0.8 s at gap 20; measured 110 of 137 px). Plot order from the bottom: the first plot (black) at the bottom, then red, then blue. Each bar starts where the one below ends; the automatic Y range was 0..9 for pile heights up to 8 (the automatic range rule applies). The legend lists the plots in reverse order (last plot first), so it matches the pile.
- **100 % stacked column.** Every pile is scaled to 100; the Y axis runs exactly from 0 to 100 with no margin. The template adds data labels (the percentage on each bar) and thin lines joining the tops of neighbouring bars; Sheaf draws neither.
- **Horizontal bar (215).** Same geometry as columns with the axes exchanged. In the picture the layer's X axis is the vertical one on the left (the title of the X column is there) and the Y axis is at the bottom; `layer.x.from/to` was 0.5..4.5 and `layer.y.to` 5.6 for values up to 5. The first plot of a group is the lowest bar of each position and the legend is listed in reverse. The BAR template colours the group red, green, blue (the column template: black, red, blue); the template has dotted lines and ticks between the positions. Sheaf draws plain ticks and its own palette.
- **Floating column (207).** Two Y columns B (first) and C (second) gave bars from C to B at each X; the legend names the second column; Y range 0..6 for ends 1..5 (0 is included as for columns). The default fill was blue, outline black.
- **Sheaf.** `PlotKind::{StackedColumn, Stacked100Column, Bar, FloatingColumn}` and templates `stacked-column`, `stacked-column-100`, `bar`, `floating-column` (CLI `-t`). A horizontal-bar layer shows the values on its X axis (bottom) and the positions on its Y axis (left); Origin names the axes the other way round. Stacked plots share the slot of the first of them; negative values are added like the others. A floating column plot takes the lower end from `Plot::y_base`; `plot_columns` chains the selected columns (each plot runs from the column before it to its own). Stacked bars (`PlotKind::StackedBar`, `Stacked100Bar`, templates `stacked-bar`, `stacked-bar-100`) and floating bars (`PlotKind::FloatingBar`, template `floating-bar`) are the horizontal forms of the stacked, 100 % stacked and floating columns and share their code (`is_horizontal()`, `is_stacked_column()`, `is_percent_stacked()`, `Plot::y_base`): piles grow from left to right, the first plot innermost at the value axis' zero, the 100 % value axis is exactly 0..100, a floating bar runs from `y_base` to the Y column along the horizontal axis and `plot_columns` chains columns as for the floating column. Origin draws these with plot type 216 and the stacked bar template (measured above).
- **Measured later** (`re-workspace/scratch/oc-graphs/bars*.lines.txt`, 12 and 13 Y columns, X = 1..3, legend swatches read pixel by pixel): the stacked bar (`StackBar`) and the 100 % stacked bar (`StackBarP`) take the colour list of the stacked column (black, red, blue, magenta, green, navy, violet, purple, wine, dark yellow, then (43,99,162), (30,150,150), (155,100,26)); both list the last plot first in the legend and pile from the left with the first plot innermost. The 100 % bar's value axis runs exactly 0 to 100 in steps of 20 (as the column's) and the template writes a white share on each part ("8%" for 12 equal parts); the stacked bar's value axis ran 0 to 26 for piles of 24 and 0 to 128 for piles up to 117 (the usual rule).
- The floating column (`FloatCol`) and the floating bar (`FloatBar`, plot type 207 for both) chain their columns: with columns B, C, D, E the plots are C (the bar from B to C), D (from C to D) and E (from D to E), each named after its column, all in the same slot at full width and drawn in plot order (the later over the earlier). The colours are blue, cyan, magenta, yellow, dark yellow, navy, purple, wine, green, dark cyan, (0,0,160), orange for 12 plots, which is the base palette from its fourth entry, as the first plot's blue showed. The legend lists the last plot first (G, F, E, D, C for five plots), for the floating column too.
- **Sheaf, after the measurement.** The three colour lists were right; the floating legends were wrong and now list the last plot first (`PlotKind::is_floating`), floating plots share one slot (they overlapped before only by accident of a single plot), and `plot_columns` chains the columns of a floating template instead of pairing them (a pair is still one plot). Fill Area keeps its pairs (not measured). Not drawn: the shares on the 100 % bars. Not done: bar labels and join lines, category (text) axes for horizontal bars. The `.opj` writer stores all three as plain columns (type 203) and the save warns.
- **`.opj`.** The writer stores these plots as plain columns (type 203) and names the loss in the save warnings; the companion keeps the real type.

### 14.13 Error bars

Measured with Origin on a scatter plot of 5 points with a Y error column (`p6`, `p7`, `p8` in the same folder, pictures `y_hi.png`, `y18.png`, `y3.png` at 3200 px, 4.11 px per pt).

- **Cap width.** The cap is 34-36 px wide over all (8.3-8.8 pt including the line) for 9 pt, 18 pt and 3 pt symbols: a fixed width of 8 pt, independent of the symbol size. Sheaf had 1.2 x the symbol size (10.8 pt).
- **Line width.** The bar and the cap are 3 px thick in the 3200 px picture (about 0.5-0.75 pt); Sheaf keeps 0.5 pt.
- **Columns.** A third plot column `(x, y, err)` in `plotxy iy:=(1,2,3)` is always read as the Y error; X error needs a column designated X error, which `plotxy` could not be made to read in 2016 (no X error measurement; the same cap is assumed).
- **Range.** The automatic axis range reaches the ends of the error bars (data 2..5 with ends 1.5 and 6 gave Y 1..6.5).
- **Sheaf.** `Plot::x_error` (X error column after the Y column, found like the Y error), `Plot::error` with direction (both, positive, negative) and cap width (default 8 pt, 0 = no cap); Plot Details > Error Bar edits direction and cap width; Plot > Symbol > XY Error plots with both columns. X error bars are not drawn for bar plots. Positive and negative directions draw the bar from the point to that end only, with one cap; the range widens only on that side. Not stored in `.opj` (save warning).

### 14.13a Graph > Add Error Bars: existing-column chooser (T01 follow-up)

Measured in Origin 2016 on 2026-11-14 through `tools/origin-ref/origin-dialog.ps1`, under the
global Origin lock. Synthetic data: X = 1..4, B = 2,4,3,5, C = 5,3,4,2, D = 0.1..0.4,
E = 0.5..0.8; B and C plotted as scatter curves. Setup and raw control list are local-only at
`re-workspace/scratch/t01-error-bars-dialog/setup.labtalk.txt` and `dialog/first.controls.txt`.

`worksheet -e err;` opens **Error Bars**. **Add Error Bars to** names `[Graph1]1!1"Signal B"`;
**Source of Errors** contains **Percent of Data (%)** (selected), **Standard Deviation of Data**,
**Square Root of Data**, two numeric edits, and **Scaling Factor**, with OK and Cancel. The edits
were empty in this capture. No numeric defaults or generated values were measured. The screenshot
was obscured by another foreground window and is not visual evidence. The earlier T01 assumption
that this dialog simply asks for an error column was incorrect.

**Sheaf's existing-column chooser** is deliberately a smaller scope using `Plot::y_error`: Graph >
Add Error Bars opens **Error Bars**, with **Add Error Bars to** choosing one curve of the active
layer and **Y Error Column** choosing any numeric or Text & Numeric column across all workbooks
and sheets. Fully qualified names and long names identify columns. The selected graph curve is
the initial target, otherwise the first curve; its current Y-error column, or the adjacent
designated Y-error column, is suggested. Otherwise the source starts unselected and OK is disabled.

Opening or changing choices does not edit the project. OK assigns/replaces only the chosen
curve's Y-error column, leaving its row range, X errors, direction, cap width and other curves
unchanged, in one Undo/Redo step. Confirming the current source creates no undo step. Cancel,
Escape and the close button discard choices. Changed/reordered plots and missing/non-numeric
sources cannot be applied; switching projects closes the chooser. No data or designation is
changed. Pure-text columns are excluded. Short, missing or negative error values retain the
renderer’s existing handling.

**Generated errors (T241, not measured: Origin unusable on this computer on 2026-11-20).** OriginLab's
help ("Adding Error Bars to Your Graph", section "Adding error bars to an existing graph by
calculating simple dataset statistics") says the tool computes "a user-specified percentage of each
data point", the "standard deviation with specified scaling factor of the data set" or the "square
root of each data value", and that "the error data is output to a new column on the source
worksheet. The column includes a comment indicating the error bar type. The formula used for error
bar calculation will appear in the Set Column Values dialog." Sheaf's **Source of Errors** now lists
**Percent of Data (%)** (a number, default 5), **Standard Deviation of Data** with **Scaling Factor**
(default 1), **Square Root of Data**, and **Existing Column** (the chooser above). A plot that has, or
sits next to, a Y error column starts on Existing Column, otherwise on Percent of Data (Origin's
first choice, selected in the 14.13a capture). OK with a generated source inserts a new column
right of the plot's Y column (Origin requires error data right of its Y), designation Y Error,
comment `Error bars: Percent of Data 5%` / `Error bars: Standard Deviation of Data, Scaling Factor
1` / `Error bars: Square Root of Data`, filled by Set Column Values with `abs(col(B))*5/100`,
`sd(col(B))*1` (the sample standard deviation on every row) or `sqrt(col(B))` (negative values give
missing errors), the formula kept on the column, run once; the plot's Y errors become that column.
Column, link and values are one undo step ("Add error bars"). Percent and Scaling Factor must be
positive. Not measured: Origin's defaults (the capture showed empty edits), its column name and
long name, whether Percent uses the magnitude, and its rounding.

### 14.14 Colour lists of column and area templates; stacked area and fill area (T12)

Measured with `plotxy` and `expGraph` through COM on synthetic data (scripts and PNGs in `re-workspace/scratch/T12/`, local only), legends read pixel by pixel. The same twelve Y columns plotted from each template:

| Template | Colours of the plots in order |
|---|---|
| `<new>` (no template), Line, Scatter, Line + Symbol | the base palette: black, red, green, blue, cyan, magenta, yellow, dark yellow, navy, purple, wine, olive, ... (section 1.5) |
| Column, Box | the base palette from its second entry: red, green, blue, cyan, magenta, yellow, dark yellow, navy, purple, wine, olive, dark cyan |
| Stacked Column, Area, Stacked Area | the theme list (section 1.5, "Q02 Basic"): black, red, blue, magenta, green, navy, violet, purple, wine, dark yellow, (43,99,162), (30,150,150), ... |

Sheaf copies this in `template.rs`: `from_red` for Column, `themed` for the stacked column, area and fill area templates. Plain Area draws the later plots over the earlier ones (no pile).

- **Stacked Area (214) and 100 % Stacked Area.** Plot i fills from the sum of the plots before it up to the sum including itself, row by row; the legend lists the plots in plot order (stacked columns list them reversed). The Y axis starts at exactly 0; the X axis runs from the first to the last X value with no margin (the same for plain Area, measured: probe X 1 to 8 gave a 1 to 8 axis, Y 0 to 12 for a pile reaching 11).
- **Fill Area.** One plot per pair of Y columns: the first of a pair is the lower edge, the second the upper edge and the plot's name (`Plot::y_base`, as Floating Column). Origin's template could not be told apart from two plain areas drawn over each other, so only the pair form is implemented.
- `.opj`: stacked areas are written as plain areas from zero, fill areas as areas down to zero, and the save warns (the companion keeps them).

### 14.15 Histogram and box chart (T12)

Measured through COM (`plot:=219` with the HIST template, `plot:=206` with BOX; `plotxy iy:=[Book]Sheet1!1:3`: the range without parentheses makes each column a box, with parentheses the first column is X and the boxes are grouped by its values). PNGs in `re-workspace/scratch/T12/`.

- **Histogram.** The bins are those of the Frequency Counts tool (`sheaf_calc::freq`, size by the 1.75 ln n rule, edges at multiples of the size): data 40.7 to 69 (12 values) gave bins of 7 from 35 to 70, data 13.9 to 73.2 (100 values) bins of 10 from 10 to 80, data 38.7 to 63.7 (7 values) bins of 9 from 36 to 72, each time with the X axis exactly from the first to the last edge and the Y axis from 0 (to 4.5 for a highest bin of 4, to 50 for 43). The bars touch. The Y title is "Count", the X title the column's. The first colour is red; Origin hatches the bars and outlines them in that colour. Sheaf outlines the bars in the colour and leaves them unfilled, as it has no pattern fills. `Plot::hist` (bin size, first edge, end) overrides the automatic bins.
- **Box chart.** One box per Y column, at 1, 2, 3, ... with the column names as tick labels and no X title; colours red, green, blue (the Box template, 14.14). Box from the 25th to the 75th percentile by the Hazen rule of the statistics tools (`quantile_origin`), median line across the full box, whiskers to the furthest values within 1.5 times the box height (caps 0.1 of the box spacing wide), a cross at each whisker end, the other values as crosses (outliers), the mean as a small open square; box width 0.8 of the spacing; 0.5 pt lines. Checked: 2..13 gave a box from 4.5 to 10.5, median 7.5, whiskers 2 and 13; a column with 30 among values 4 to 8 gave the outlier 30 and whiskers 4 and 8.
- **Not done:** grouped box charts and the data points beside the box need the Group and Subject designations and categorical columns of T10 (`worksheet-ops`); a worksheet with a grouping column is plotted today as one box per Y column only. The Y axis of the box chart follows the automatic rule of T20 (Origin: -5 to 40 for data 1 to 30; Sheaf -2 to 32); the Y limit of a histogram from 4 is 4.5 as in Origin.
- `.opj`: both are written as plain columns of their data and the save warns.

### 14.16 Bubble and colour mapped symbols (T12)

Measured through COM: `plotxy iy:=[Book]Sheet1!(1,2) plot:=201 ogl:=[<new template:=BUBBLE>]` with the third column set as Z (`wks.col3.type=5`; X is 4 and Y 1) draws the bubbles (the plot type numbers 245 to 247 gave nothing). Probe: X 1..8, Y 2 4 3 6 5 7 6 8, Z 1 4 2 9 3 6 8 5.

- Bubbles are open circles in the first colour (black rings). Their width is linear in Z from the smallest to the largest value: measured on the 800 px export 2.5, 5, 7, 9, 12, 14, 19 and 22 px (about 2.4 to 21 pt) for Z = 1, 2, 3, 4, 5, 6, 8, 9. Sheaf uses 2 pt to 20 pt (the outline adds the rest).
- Origin adds a size legend beside the frame (title = the Z column, four circles labelled 1.00, 3.66, 6.32, 8.98: the smallest value and three equal steps to the largest). Drawn only by Add Bubble Scale (section 27).
- Colour mapped symbols: Origin's default colour map was not measured (no plot number reached the Color Mapped command); Sheaf maps the smallest to the largest Z over blue, cyan, green, yellow and red. Graph > New Color Scale and the legend's Add Color Scale draw one (section 27).
- Model: `Plot::size_from` and `Plot::color_from` name the Z column (found from the Z designation after the Y column by `Project::z_column_for`); `Template::Bubble`, `ColorMapped`, `BubbleColorMapped`; CLI `-t bubble` needs a project file with a Z column. `.opj`: written as plain scatter, the save warns.

### 14.17 Pie; what was seen of polar (T12)

Measured through COM: `plotxy iy:=[Book]Sheet1!1 plot:=225 ogl:=[<new template:=PIE>]` with the column set as Y (a column left as X puts its values into the legend instead). PNGs in `re-workspace/scratch/T12/`.

- The pie template is a 3D pie viewed from above at a slant (an ellipse about half as high as wide, with a side band); the 2D template (`PIEBW`) is a flat black and white pie with hatch patterns. Both: the first slice starts at 12 o'clock and the slices follow clockwise, each outlined in black; every slice is labelled outside with its share to two decimals (`33.33%`); the legend lists one square per slice at the top right, named by the X column's cells or by the row numbers 1, 2, 3, ...; the slices take the base palette in order (black, red, green, blue, cyan, magenta, yellow, dark yellow, navy, purple); the layer has no axes and a black frame around the whole layer; the legend font is the legend font of the layer, the percentages the tick label font.
- Sheaf draws the flat pie in colour (`PlotKind::Pie`, `Template::Pie`, CLI `-t pie`, no longer in the menu because Origin has no "2D Color Pie"): a pie whose radius is 35 % of the frame height, centre at 42 % of the width, drawn by `draw_pie`. The menu items "3D Color Pie" and "2D B&W Pie" draw the 3D pie and the hatched pie: see 14.21. Rows that are not positive numbers are left out. Not done: exploded slices, label positions inside, other label contents.
- `.opj`: written as plain columns, the save warns.
- **Polar** (`POLAR` template, `plotxy iy:=[Book]Sheet1!(1,2) plot:=200`): theta from 0 to 360 degrees in steps of 30 with labels outside the circle (0 on the right, counter-clockwise), grey ring and spoke grid lines, the radius axis drawn twice along the vertical diameter (labels 1.0 to 2.0 up and down, mirrored) and a line curve. Sheaf draws it as a polar layer: see 14.18.

### 14.18 Polar plots (T12)

Origin's Plot > Specialized > "Polar theta(X) r(Y)" and "Polar r(X) theta(Y)", and the 2D Graphs toolbar button of the first one.

**Measured** (spec 14.17, `POLAR` template, `plotxy plot:=200`, one probe): angle 0 to 360 degrees in steps of 30 with the labels outside the circle, 0 on the right and counter-clockwise; grey ring and spoke grid lines; the radius axis drawn twice along the vertical diameter, labels going up and the same going down (the centre is the radial "from" value, the circle edge the "to" value); the data as a line curve.

**Measured later** (`POLAR` template, data 1.13 to 2.37 and other ranges, `re-workspace/scratch/oc-graphs/pol*.lines.txt`, pictures `pol1.png` and `pa1.png`; the axis dialog was read through the program window):

- **Page and layer.** The page is 8.04 by 6.15 in. The layer is a square of 4.55 in whose left edge is at 23.3 % of the page and top at 11.7 %. The angle labels stand outside the circle and the circle touches the layer's frame.
- **Legend.** The default legend is at the top, above the layer's top right: its box spans 66 % to 76 % of the page width and 1 % to 6 % of the page height, so it sits over the right half of the circle's top edge without touching it.
- **Radius axis.** It is drawn twice, to the left of the circle (0.2 radii off its left edge, not on a diameter): the upper copy runs from the circle's top down to the hole (the axis' lower limit) with the numbers increasing upwards, the lower copy mirrors it; with a lower limit of 0 the two join at the centre, which is labelled once. The Y column's name is the axis title, rotated, left of the axis (Sheaf draws no axis title).
- **The centre is radius 0, not the lower limit.** The Radial Axes > Scale page has a check box "Center at Zero" which is on. A point at radius r stands at r / (upper limit) of the circle's radius; the lower limit only leaves a hole round the centre that stays empty: with limits 1.5 to 2.5 the ring for 2.0 stood at the same place as with limits 0 to 2.5, the spokes and rings start at the hole and a line is cut where it enters it. Negative radii are drawn on the opposite side of the centre; the radius axis cannot be given a negative lower limit (`layer.y.from = -1` was refused, 0 stayed). With the box off the lower limit is the centre (what Sheaf drew before).
- **Automatic radius range.** The usual rule with the 8 % rescale margin, the same as for a Y axis (spec 4): data 1 to 2 gave 0.75 to 2.25 (steps of 0.5), 1.13 to 2.37 gave 1 to 2.5, 0.2 to 0.37 gave 0.175 to 0.4 (steps of 0.05), 1.45 to 2.05 gave 1.4 to 2.1 (steps of 0.2), 10 to 10.4 gave 9.9 to 10.5. So the limits are not rounded to a tick; they are the data plus the margin rounded outwards to half a step. A lower limit below 0 is cut at 0 (data 3.2 to 87 gave 0 to 100; data -1 to 2 gave 0 to 2.25 with steps of 0.5, where Sheaf's rule gives steps of 1).
- **Angle.** The angular axis runs 0 to 360 in steps of 30 with one minor tick, 0 on the right and counter-clockwise; its From and To can cut the circle (0 to 180 gave a half circle) but setting 90 to 450 or 360 to 0 changed nothing. Its Units are Degrees, Radians, Gradians or Custom (circle start and end values, greyed for the first three). With Radians the circle is 0 to 2 pi, there is a tick every quarter of pi (eight spokes) and the labels read 0.00, 0.25, 0.50 ... with the pi sign after the number. Gradians and Custom were not measured. No setting for the zero direction or the sense of rotation exists in the angular or radial axis pages, in the layer or in the plot details of this version, and no LabTalk property reached one (the properties tried changed nothing in the picture), so the angle 0 on the right and counter-clockwise are fixed; Sheaf keeps `zero_deg` and `clockwise` as its own options.

**What Sheaf does**

- Model (`sheaf-core`): `Layer::coords` is `Coords::Cartesian` (default, not written to the project file, so old files load unchanged) or `Coords::Polar(PolarCoords)`. `PolarCoords` holds `radius` (`RadiusColumn::Y` for theta(X) r(Y), `X` for r(X) theta(Y)), `zero_deg` (where angle 0 points, counter-clockwise from 3 o'clock; Origin's default 0), `clockwise` (default false) and `angle_unit` (`AngleUnit::Degrees`, default, or `Radians`; written only when it is not degrees). There is no dialog for these yet. The plots of a polar layer are ordinary Line, Scatter and LineSymbol plots. Templates `polar` and `polar-r-theta` (CLI `sheaf plot FILE -t polar`); the layer's frame is a square, the grid rings and spokes are on (grey, 0.5 pt, the grid style of the Grids toggle) and the axis titles are off.
- Data (`data::collect`): for a polar layer the two columns of a plot are read as `(angle, radius)`; with the radius in X the columns are swapped, a missing X column being the row number. Error bars are not drawn.
- Geometry (`LayerGeometry::polar`, a `PolarMap`): the circle is centred in the frame, its radius is half the smaller side. `data_to_page(theta, r)` puts the point at distance `|r| / (radius axis upper limit)` of the circle's radius from the centre in the direction `zero_deg + theta` (minus theta when clockwise; a negative r points the opposite way; the centre is radius 0, `PolarMap::zero_centred`, which is off for radar charts, where the lower limit is the centre); `page_to_data` is its inverse (angle in [0, one turn)); `checked_data_to_page` gives `None` for a radius whose size is outside the radius axis range (so the hole below the lower limit stays empty). So lines, symbols, hit testing and the cursor read-out use the same functions as for rectangular layers. Lines are straight segments between the points on the page (no arcs between them).
- A point whose radius lies outside the radius range (below the lower limit too) is not drawn, and the line breaks there (it is not clipped to the circle). With the automatic range every point is inside.
- Axes: the angle axis is fixed, one turn (0 to 360 in steps of 30, or 0 to 2 pi in steps of a quarter of pi in radians) whatever the data hold; the end of the turn coincides with 0 so only one label stands there. The radius axis uses the layer's Y axis settings and its automatic range rule (`crates/core/src/autorange.rs`: the data span plus the rescale margin, 8 % as in Origin, rounded outwards) on the radius values, with the lower limit cut at 0; its upper limit is the circle's edge and its lower limit the hole round the centre. A fixed `min` and `max` of the Y axis work as usual.
- Drawing (`crates/plot/src/polar.rs`): the grid first (rings at the radius major ticks, spokes at the angle major ticks, in the grid styles of the Y and X axis), then the data, then the outer circle in the angle axis' colour and width with outward ticks, the angle labels outside the circle (the text box is pushed away along its direction so that its nearest point is a fixed gap off the circle), and the radius axis left of the circle (0.2 radii off its edge): line, ticks pointing left and the labels left of the line, on the upper and the lower half, from the circle's top and bottom to the hole, the centre labelled once when there is no hole. The grid's spokes start at the hole. Cartesian axes, axis titles and the rectangular frame are not drawn. The legend and free labels work as for any layer. The labels count towards the room kept inside the page.
- Template (measured above): page 8.04 by 6.15 in, a square layer from 23.3 % and 11.7 % of the page, the legend centred at 0.84 of the layer's width and 0.11 of its height above its top edge, the default Rescale Margin of 8 %.
- Differences left: the radius axis title (Origin writes the Y column's name left of the axis), Gradians and Custom units, a dialog for the units, and Origin's minor ticks (one between the major ones) on both axes.
- `.opj`: written as a plain line plot on rectangular axes; the save warns that the polar layer was not written, and the `.opj.sheaf` companion keeps the layer (round trip tested).
- Golden image: `crates/plot/tests/golden/polar.svg` (angles 0 to 360 in steps of 15, r = 1.5 + 0.5 cos 2 theta).
- Not done: the angle in gradians, the axis dialog for the units, zero direction and sense of rotation, a polar axis dialog (radius labels along other angles, labels at the circle edge only), polar contour and wind rose, zooming the circle, the Plot Details tab of the polar axes. Plot Details shows the plot type of a plot in a polar layer as "Polar Line" and so on.

### 14.19 Radar charts (T12)

Menu Plot > Specialized > Radar, the 2D Graphs button PlotRadar, `sheaf plot -t radar`. Measured with Origin
through COM (`plotxy iy:=... plot:=202 ogl:=[<new template:=Radar>]`, `re-workspace/scratch/oc-graphs/rad*.lines.txt`,
pictures `rad1.png`, `rad2_*.png`, `rad4.png`):

- **First spoke and direction.** The first row is the spoke straight up and the others follow clockwise, evenly spaced.
- **Spoke names.** The X column's cells (text or numbers, e.g. 10, 20, 30) name the spokes, outside the outer polygon; with no X column the row numbers 1, 2, 3, ... do.
- **Scale.** One radial scale with its tick labels on the first spoke, upwards from the centre. The centre is the axis' lower limit, not 0: values 2.1 to 5.2 gave 1.5 to 5.5 with the label 1.5 at the centre and 5.5 at the top vertex, values 0 to 4 gave -1 to 5, values -4 to -1 gave -4.5 to -0.5. The limits are the values plus the 8 % margin rounded outwards to whole steps, and the step is half of the one the usual table gives for that span (0.5 for spans of 3 and 3.1 where a Y axis has 1; 1 for a span of 4 where a Y axis has 2).
- **Grid.** One grey polygon at every radius tick up to the outer polygon, and a spoke to every vertex; the outline is a polygon without ticks.
- **Page and legend.** The page is 8.0 by 6.125 in; the layer is 61.2 % wide and 75.4 % high with its left edge at 11.7 % and top at 10.9 % of the page. The legend (line + symbol swatches, black square then red circle) stands at the layer's top right corner: its box spans 69 % to 79 % of the page width and 4 % to 14 % of its height.
- Each Y column is one plot, a closed polygon (line + symbol by default) joining its values on the spokes.

- The worksheet rows are the spokes, evenly spaced: spoke `i` of `n` is at `360 i / n` degrees. The
  first spoke points up (12 o'clock), the others follow clockwise.
- Sheaf closes each polygon by repeating the first point at 360 degrees (line + symbol colours from the line +
  symbol list).
- The X column's cells, text or numbers, name the spokes, drawn outside the outer polygon; with no X
  column (or an empty cell) the row number is the name.
- One radial scale for all spokes, its tick labels on the first spoke from the centre up. The centre
  is the lower limit; with automatic limits they are the values plus the margin rounded outwards to
  whole steps of half the usual step (`resolve_dim` in `layout.rs`; before the measurement Sheaf started at 0).
  A radius outside the range is not drawn. Template: page 8.0 by 6.125 in, the layer and legend as measured above.
- The grid is concentric polygons at the radius major ticks plus straight spokes (the polar grid
  style); the outline is a polygon, with no ticks.
- Implementation: a polar layer with `PolarCoords::radar = true`. `PolarMap` forces zero at 90
  degrees and clockwise, so `data_to_page`, `page_to_data`, symbols and hit testing are those of the
  polar layer. `data::radar_columns` turns the rows into `(angle, value)`; `polar::radar_axis` gives
  the spoke ticks and names. The radar page is 8 by 6.125 inches with the legend at the layer's top right corner.
- The closing point is a repeated row marked `PlotData::closing`: the line runs to it, but it gets
  no symbol and no mask, and a masked first row also reddens the closing segment. Rows keep their
  place, so the column's mask still counts the right rows. The plot clip of a radar layer is
  widened by one symbol size, so a symbol on the outer polygon is not cut. When all values are equal
  their span is taken as 20 % of the value (1 for 0) to find the range (Origin's range for that case was not measured). Spoke names go through the label markup like every other label column
  (see 14.11).
- The .opj writer stores plain line + symbol plots and warns; the companion keeps the layer.

### 14.20 Ternary plots (T12)

Origin's Plot > Specialized > "Ternary" and the 2D Graphs toolbar button of the same name.

**Measured (2026-11-13, Windows, `worksheet -p 245 ternary` on an X Y Z sheet, pictures and runs in
`re-workspace/scratch/tern`):** the three axes run 0 to 1 in steps of 0.25 with one minor tick between, labels
with two decimals (0.00 to 1.00), or 0 to 100 in steps of 25 with whole numbers when the rows sum to 100;
`layer.x.inc` reads 0.25. X runs along the bottom from left to right, Y up the right side, Z down the left side;
the ticks leave each side along the grid lines. Major grid lines are Gray (128) solid, minor ones LT Gray (192)
dashed. A legend is shown at the top right of the page, the plot named after its Z column. A row with a
negative part is not drawn. Rows that do not sum to 1 or 100 make Origin ask "Ternary plots assume that the
data are normalized (x+y+z=1 or 100)... Would you like your data to be normalized?"; after Yes they are drawn
as fractions (0 to 1). The page is Origin's usual one (the picture is 10.72 by 8.205 in), the triangle the
largest that fits in the usual layer frame.

**Still to measure:** the title rotation and distance, tick lengths.

**What Sheaf does**

- Model (`sheaf-core`): `Coords::Ternary(TernaryCoords)`, `TernaryCoords { scale: TernaryScale }` with `Auto` (default), `Fraction` and `Percent`. Old files load unchanged (`coords` is only written when it is not rectangular). The Z column of a plot is `Plot::size_from` (the field the bubble plot uses for its sizes; a ternary layer draws no bubbles). `Project::plot_columns(Template::Ternary, ys)` fills it from `Project::ternary_z_for`: the Z column after the Y column (`z_column_for`), or, in a sheet without Z designations, the numeric column right after Y, so that a plain three column file works (CLI `sheaf plot soil.csv -t ternary`). `default_plot_columns` for the template gives the Y columns that have a Z column, else the first Y column that has a column after it.
- Template `ternary` (menu item and button live): a scatter plot on the usual page and frame, major grid Gray solid and minor grid LT Gray dashed, outward major and minor ticks (one minor), legend on with the plot named after its Z column (as measured; Sheaf normalises rows without asking).
- Data (`data::collect`, `ternary::columns`): each row is divided by its sum; the plot's `x` and `y` become the normalised X and Y fractions and Z is `1 - x - y`. A row with a missing or negative part, or with a zero sum, becomes NaN and is not drawn (a line breaks there).
- Geometry (`LayerGeometry::ternary`, a `TernaryMap`): the largest equilateral triangle that fits in the frame, centred in it. `data_to_page(x, y)` is the barycentric combination `x * right + y * top + (1 - x - y) * left` and `page_to_data` its inverse, so symbols, lines, hit testing and the cursor read-out use the usual functions; they work in fractions whatever the tick scale. The clip rectangle is the frame widened by 9 pt so that symbols on the sides are whole.
- Ticks: 0 to 1 in steps of 0.25, or 0 to 100 in steps of 25 when the scale is percent (`Auto`: some row of the raw data sums to between 99 and 101, so raw counts such as 1, 2, 3 stay in fractions; unmeasured). Tick font, length, direction and line settings are those of the layer's X axis (bottom and left sides) and Y axis (right side); the major grid styles likewise (X and Z lines from the X axis, Y lines from the Y axis). Labels of the three axes are the same numbers.
- Drawing (`crates/plot/src/ternary.rs`): grid lines first (for each interior tick, the line of constant X, of constant Y and of constant Z: the first parallel to the left side, the second to the bottom, the third to the right side), then the data, then the triangle outline, the ticks, the tick labels outside (each pushed off its tick end along the outward normal of the side until the nearest point of its box is a fixed gap away), and the three titles: X (the X column's name and units) under the bottom side, Y rotated -60 degrees along the right side, Z (the Z column's name) rotated +60 degrees along the left side; titles can be replaced with the X and Y axis' title text (Z has no setting yet). The labels and titles count towards the room kept inside the page. Cartesian axes and the rectangular frame are not drawn.
- `.opj`: written as a plain scatter plot of X and Y; the save warns that the ternary layer was not written, and the `.opj.sheaf` companion keeps it with its Z column (round trip tested). Plot Details shows the plot type as "Ternary Scatter".
- Golden image: `crates/plot/tests/golden/ternary.svg` (eight rows that sum to 1 and two that sum to 0.9 and 1.2).
- Not done: a ternary axis dialog (scale, per-axis ranges and zoom into part of the triangle, which Origin offers), axis directions (clockwise or counter-clockwise), the Z column in Plot Details, ternary contour and colour mapped surfaces, a Z axis title setting, a legend in the ternary style.

### 14.21 3D Color Pie and 2D B&W Pie (T12)

Origin's Plot > Column/Bar/Pie menu has "3D Color Pie" (template `PIE`) and "2D B&W Pie" (template `PIEBW`); it has no "2D Color Pie". What was measured is in 14.17 (3D pie from above at a slant, ellipse about half as high as wide with a side band; flat black and white pie with hatch patterns; slices clockwise from 12 o'clock, black outlines, outside labels with two decimals, legend squares at the top right, palette in order for the 3D pie).

**Measured (2026-11-13, Windows; `plotxy iy:=[Book]Sheet1!1 plot:=225 ogl:=[<new template:=PIE>]` and
`PIEBW`, pictures in `re-workspace/scratch/pies`):** the pie layer is larger than the line template's (7 % from the
left, 86.5 % wide, 8 % from the top, 83 % high) and the pie is centred in it. The 3D pie: an ellipse 0.5455 as
high as wide, radius 0.254 of the frame width; the side band 0.278 of the radius thick in the slice's own colour
(not darker), outlined in black; the slices clockwise, the first starting 32.4 degrees below 3 o'clock (four equal
slices meet at -32.4 and -123.0 degrees), so not at 12 o'clock as 14.17 assumed. The B&W pie: radius 0.349 of the
frame height, first slice at 12 o'clock, clockwise; hatches in this order: lines at 45 degrees about 3.9, 5.6
and 8.3 pt apart, the same at 135 degrees, diagonal cross hatches at the same three spacings, then horizontal lines
4 pt apart (ten slices measured). Shares print with at most two decimals and no trailing zeros (`18.6%`, `25%`).
Legend samples are two font sizes wide and one high, rows 1.15 font sizes apart. A worksheet whose first column is
X gives a one-entry legend named after it instead.

**What Sheaf does**

- Plot kinds `PlotKind::Pie3D` and `PlotKind::PieBw`, templates `pie-3d` ("3D Color Pie") and `pie-bw` ("2D B&W Pie"), both live in the menu; the item "2D Color Pie" is gone from the menu. `Template::Pie` (`-t pie`, `PlotKind::Pie`) stays as a CLI name and for existing projects: the flat colour pie of 14.17, golden `pie.svg` unchanged.
- Drawing (`crates/plot/src/pie.rs`): same frame, labels (share to two decimals outside the slice) and legend placement as the flat pie. 3D (measured values): the ellipse 0.5455 as high as wide (`SLANT`), radius 0.254 of the frame width (0.349 of its height at most), centred; the side band 0.278 of the radius thick (`BAND`) in the slice's own colour, drawn on the front half of the rim, then the tops, all outlined 0.5 pt black; the first slice starts at `START_3D` (122.4 degrees clockwise from 12 o'clock); labels of the front half sit below the band. B&W: white slices with Origin's ten hatch patterns in its order (above; slices past the tenth start the list again), 0.5 pt black lines, centred with radius 0.349 of the frame height; legend samples carry the same pattern. Shares as Origin prints them (`share_text`). The labels sit 1.12 radii out (Origin about 1.26), so a label next to the legend can touch it.
- Pattern fills: the scene has no clip item, so `pie::hatch` cuts parallel lines against the slice polygon (the arc in steps of about 2 degrees; works for concave shapes) and the lines are drawn as one path per slice. Lines sit at multiples of the spacing from the page origin, so patterns continue across shapes. Works in every renderer since it is plain paths. It is not a general pattern fill of the model (`FillStyle` has none yet).
- `.opj`: written as plain columns, the save warns (same note as the flat pie); the `.opj.sheaf` companion keeps the kind.
- Goldens: `pie-3d.svg`, `pie-bw.svg` (values 3, 5, 2, 7, 4).
- Not done: changing the 3D view (tilt, thickness, rotation), exploded slices, label positions inside, the toolbar button "Pie" (still greyed), hatch pattern choice per slice, Plot Details shows the kinds by name.

### 14.22 Violin plot (T37)

Origin's documented facts (from its help, not measured; no Origin was at hand; [W:origin-help/violin-plot],
[W:origin-help/create-violin-plot], [W:origin-help/pd-dialog-distribution-tab], [W:origin-help/pd-dialog-box-tab],
[W:origin-help/pd-dialog-data-tab], [W:origin-help/pd-dialog-spacing-tab], [W:labtalk/ref/ksdensity-func],
[W:labtalk/ref/kernelwidth-func], [W:origin-help/groupedviolin-indexeddata]):

- Origin 2019. A violin plot is "a fancy variation of box chart" that adds a kernel density curve of the observations.
  Menu: "Plot > Statistical: [variant]" in the newer help, "Plot: 2D > Violin Plot" in the older; the two pages disagree.
  Seven variants, with templates: Violin Plot (`Violin.otpu`), Violin with Box (`ViolinBox`), Violin with Point
  (`ViolinData`), Violin with Quartile (`ViolinQuartile`), Violin with Stick (`ViolinStick`), Split Violin
  (`ViolinSplit`), Half Violin (`HalfViolin`). The help never describes the variants one by one; the 2019 announcement
  speaks of a median and summary box, two subgroups split into the halves of one violin, one half with each observation
  in bins, and observation lines inside the violin.
- Data: one or more Y columns, each its own violin; column names or labels are the X tick labels. Split Violin needs two
  or more columns whose last column holds only two categories. Grouped violins (indexed data) and "Inner Split by" a
  two-level column exist on grouped data.
- There is no tab of its own: the curve is on the Plot Details Distribution tab (Curve Type Kernel Smooth by default
  for violins), the box and the data points on the Box and Data tabs, then Percentile, Pattern, Spacing and Lines.
- Kernel: normal, `K(x) = exp(-x^2/2)/sqrt(2 pi)`; `f(x) = 1/(n w) sum K((x - x_i)/w)` (LabTalk `ksdensity`).
  Bandwidth options: Scott `0.9 A n^-1/5`, Silverman `1.059 A n^-1/5` with `A = min(SD, IQR/1.349)` (Scott is
  Origin's `kernelwidth` function), `k x SD`, or a number. The default rule is not stated (Scott is assumed).
- "Extend (% bandwidth)": the curve runs from `min - Extend/100 w` to `max + Extend/100 w`, so it is not cut at the data
  (the help's examples use 300). "Scale": Count (widths by sample size), Width (all violins the same width) or Area
  (the same area); "Scale to Maximum (%)" sets the amplitude. Defaults of these are not stated.
- Curve colour Auto is a darker shade of the fill; the fill transparency Auto follows the Pattern tab. Spacing tab: gap
  between boxes 20 %. Box tab Type options include Box, Data, Box + Data Overlap, Box(Right) + Data(Left), Half Box + Data;
  Half Violin uses Half Box [Left] + Data [Right] with "Distribution Curve Next to Box". Box Range and Whisker Range
  have Origin's usual options.
- Not documented at all: number of grid points, default scale mode, default extend, maximum violin width against the
  spacing, default colours, fill and outline, inner box width, the look of median, quartile and stick marks.

**What Sheaf does** (every item not marked "documented" is Sheaf's choice, unmeasured):

- Kinds `PlotKind::{Violin, ViolinBox, ViolinPoint, ViolinQuartile, ViolinStick, HalfViolin}` (one kind per variant,
  so no new field of `Plot` and old files load unchanged; `PlotKind::violin_inner()` says what is drawn inside) and the
  templates of the same names, in Plot > Statistics after Grouped Box Charts and Interval Plot: "Violin Plot"
  (`violin`), "Violin with Box" (`violin-box`), "Violin with Point" (`violin-point`), "Violin with Quartile"
  (`violin-quartile`), "Violin with Stick" (`violin-stick`), "Half Violin" (`half-violin`). They are box chart
  templates: page, axes and the colours starting at red are the box chart's (the box chart is measured, 14.15).
  Each Y column is one violin standing at 1, 2, 3, ... in plot order with its name as the tick label, as boxes do.
  The box chart's own drawing and goldens are untouched. `plot_columns_double_y`, `--y2` and MCP refuse a violin
  (no right Y axis, like the lollipop). Origin's help has a "Violin with Box" button on the 2D Graphs toolbar ([W:origin-help/violin-plot]); Sheaf has not added it yet (see not done below).
- Density (`crates/calc/src/kde.rs`, documented formulas): Gaussian kernel and the Scott bandwidth `0.9 A n^-1/5`,
  `A = min(SD, IQR/1.349)`; Silverman, `k x SD` and a fixed width are in the library but not reachable from the
  interface yet. Sheaf's choices: the IQR uses the quartiles of the box chart; with IQR 0 (many equal values) `A`
  is the SD; missing and non-finite values are left out; the curve is evaluated on 128 even grid points, plus more points wherever those are more than a quarter of a bandwidth apart (far outliers stretch the range while the Scott bandwidth follows the bulk): within 3 bandwidths of every value the step is at most a quarter of a bandwidth, with at most 8192 added points, so the bulk of the data is always sampled and the widest point is at the mode (the density peak is within 1 % of the sampled maximum); the curve
  extends 200 % of a bandwidth past the smallest and largest value (documented as a setting, default unknown). Fewer than
  two values or no spread give no bandwidth: no silhouette is drawn, the marks inside still are.
  Tested against scipy (`tools/kde-ref.py`, `crates/calc/tests/scipy_kde.rs`: `scipy.stats.gaussian_kde` with the factor
  `w/SD`, 1e-12 relative; scipy's own Scott and Silverman factors as `k x SD`). None of the numbers is from Origin.
- Scale: every violin is scaled to the same maximum width of 0.8 of the spacing (Origin's "Width" mode and the box
  width; Area and Count scaling and Scale to Maximum are not done).
- Drawing (`crates/plot/src/violin.rs`, constants `VIOLIN_*`): the silhouette is a closed polygon mirrored about the
  position, filled and outlined in the plot's colour (outline width = the line width, at least 0.5 pt). While the Fill
  settings are plain the fill is the plot colour 60 % toward white; once the plot has a transparency, pattern or
  gradient it is the plot colour with those settings (`fill::fill_polygon`). The legend sample is a small box in the
  same fill and outline. Marks inside, in the plot colour:
  - Violin Plot: a white dot with a coloured ring at the median (Origin's text mentions a median).
  - Violin with Box: a box 0.12 wide from the first to the third quartile, filled, a white median line, a whisker line
    to the box chart's whisker ends (Origin's 1.5 IQR rule, as 14.15; no outliers).
  - Violin with Quartile: dashed lines at the quartiles and a solid line twice as thick at the median, each across the
    local width of the silhouette.
  - Violin with Point: every value a 3 pt filled dot at its height, spread across 0.8 of the local width by a golden
    ratio sequence (the same picture every time), and the median dot.
  - Violin with Stick: every value a 0.5 pt line across the local width.
  - Half Violin: only the right half of the silhouette; a half box 0.12 wide to the left of the middle line (gap 0.04)
    with whisker and median. (Origin's half violin shows binned data beside the curve; not done.)
  The automatic ranges cover the curve: Y from the first to the last grid point, X from 0.6 to 1.4 for one violin. The Y range also covers the data and the box marks, so that a log axis, which cannot show the curve ends at or below zero, still gets a range around the data. On a log axis the silhouette stops at the lowest grid point above zero and is closed by a flat edge there (no taper; the kernel density is not computed in log space).
  The box chart's X title rule applies (none).
- Rows: filtered rows are left out like the box chart's; unlike the box chart, masked rows are left out too (Origin
  excludes masked points from a plot; the box chart ignoring masks is old behaviour that was not changed).
- `.opj`: written as plain columns like box and pie plots; the save warns ("violin plots were written as plain columns of
  their data"); the `.sheaf` companion keeps the kind.
- Tests: `violin_tests.rs` (silhouette symmetric and widest at the mode, extent, positions and labels, inner box equals
  the box chart's statistics, marks per variant, half violin side, missing/filtered/masked data, degenerate columns,
  legend, fill transparency, centring on the tick, outliers far from the bulk, log axis), `kde.rs` tests for the grid with far outliers, golden `violin.svg`, core template and serde tests, UI menu test, CLI test, `.opj` warning
  test.
- Not done: the "Violin with Box" button of the 2D Graphs toolbar (and the buttons of the other variants, if Origin has them), Split Violin (needs the last-column-with-two-categories data layout and Inner Split by), grouped violins
  from indexed data, Plot Details settings for the violin (bandwidth rule, extend, scale mode, scale to maximum, fill,
  distribution curve types other than kernel smooth, curve next to box), binned data (Data tab bins), a violin tab, and
  everything about Origin's real defaults. Measuring Origin's defaults (grid, extend, scale, width, colours, inner box)
  is left for a session with Origin.

### 14.11 Unknowns

- The overlap geometry: whether Origin keeps the group width (1 - g) s while the overlap changes (assumed here) or each bar's width. Needs one click test (Plot Details > Spacing) or an `.opj` saved with an overlap.
- Whether Origin's tick label dialog offers the automatic 45/90-degree rotation of the current help; the hidden COM exports did not rotate.
- The exact row count where a 2016 text axis stops putting one tick per row (between 121 and 450).
- Whether Origin reads escape sequences (`\+()`, `\g()`, ...) in text-from-dataset cells; Sheaf uses its markup for them like for every other label.
- The error bar cap width on columns (section 14.13 measured scatter plots only).
- Where an `.opj` stores the tick label type and dataset and the overlap.
- Owner preferences for the `column` template: ticks, box, minor ticks, legend frame, colours.

---

### 14.23 Ridgeline chart (T44)

Origin's documented facts (from its help, not measured; no Origin was at hand; [W:origin-help/ridgeline],
[W:origin-help/pd-dialog-display-tab-ridgeline], [W:origin-help/pd-dialog-distribution-tab]):

- Origin 2022. Menu "Plot > Statistical: Ridgeline Chart", template `ridgeline.OTPU`; select at least one Y column.
- Plot Details Display tab, Type: Distribution Curve, Distribution Curve with Data Points, Distribution Curve with Rug,
  Single Block Bar (histograms). Position: by default the column index arranges the ridgelines. Quantile Lines: check
  box, values 0 to 100, line style. Rug: Color, Thickness, Size(%), Gap Between Curve.
- Distribution tab: Curve Type Kernel Smooth by default for ridgelines; bandwidth Scott `0.9 A n^-1/5`, Silverman
  `1.059 A n^-1/5` with `A = min(SD, IQR/1.349)`, k x SD, or Custom. The violin plot (14.22) names and computes them
  the same way (`sheaf_calc::kde::Bandwidth::{Scott, Silverman, Sd, Fixed}`), so nothing differs to document.
- Not documented: default bandwidth, evaluation grid, ridge height and overlap, fill colour and transparency, baseline
  side, axis labels, histogram bins, what the rug percentages are of.

**What Sheaf does** (everything not marked "documented" is Sheaf's choice, unmeasured):

- `PlotKind::Ridgeline` and `Template::Ridgeline` (`ridgeline`, "Ridgeline Chart"), a box chart template (colours from
  red). Per-plot settings `Plot::ridge` (`RidgelineStyle`, skipped when default so old files load): display
  (`RidgeDisplay`: curve, with data points, with rug, single block bar), bandwidth (`RidgeBandwidth`: Scott default,
  Silverman, k x SD, Custom), quantile lines (off; percentiles 25, 50, 75), rug (colour auto, 0.5 pt, size 10 %, gap
  3 %), ridge height 1.5 and fill transparency 30 %. Plot Details edits are copied to every ridgeline plot of the layer.
- One ridge per Y column, the baseline of column k at Y = k, the first column at the bottom. The Y axis ticks at 1, 2, 3, ...
  are labelled with the column names (the plot label) and the axis has no title; the X axis is the data values, titled
  "Value". The Y range runs from 0.5 (lower if a rug reaches below) to the highest peak plus 0.1.
- Density: the shared Gaussian KDE (`density_at`) on a grid that starts as 200 even points from `min - 3 h` to
  `max + 3 h` and, where that step is wider than `h / 4` (a narrow kernel and a far outlier), is refined like the violin's
  (`sheaf_calc::kde::refined_grid`, shared with `kernel_density`, whose results are unchanged): steps of at most `h / 4`
  within 3 h of every value, at most 8192 added points. The curve is smooth and its sampled peak is within 1 % of the true
  one; the common peak scale uses the drawn curve, and a quantile line is as high as the smaller of the exact density and
  the curve drawn at that X, so it never pokes above the curve (the violin uses 128 adaptive points and 2 h). One scale for
  the layer: the tallest peak over all ridges is 1.5 baseline spacings high, so neighbouring ridges overlap. Fewer than two
  values or no spread give no curve; points, rug and the baseline label still draw. Masked and filtered rows are left out.
- Axis scales: the grid stays inside the X scale's domain. Where `min - 3 h` (or `max + 3 h`) cannot be placed (a log or
  reciprocal axis needs X > 0, a percent scale 0 to 100) the curve starts (ends) a tenth of the way from the domain limit
  to the data end (log: at `min / 10`); values the scale cannot place do not set the ends. The X range includes the data
  min and max like the violin's, so it stays valid. The fill polygon closes on the baseline below the first and last
  curve points that could be placed, never at a dropped end.
- Output formats: EPS and EMF have no transparency, so there the overlapping ridge fills are flattened opaque (the 30 %
  transparency is lost and a nearer ridge hides the part of a farther one it overlaps); SVG, PDF and PNG keep it.
- New plots: a plot added to a layer that already has a ridgeline plot (Layer Contents, Plot Setup, Plot Details "add")
  starts with that plot's ridge settings (`Layer::inherited_ridge`), so all ridges keep one bandwidth and display.
- Drawing (`crates/plot/src/ridgeline.rs`): ridges are drawn from the highest baseline down, so nearer ridges overlap the
  farther ones. Each ridge is a polygon from its baseline up to the curve, filled in the plot colour at 30 % transparency
  (the plot's pattern and gradient still apply), and the curve line in the plot colour (line width, at least 0.5 pt).
  Data Points: the plot's symbol on the baseline at every value (no jitter). Rug: ticks 10 % of a spacing long from 3 %
  below the baseline. Quantile lines: 1 pt vertical segments from the baseline to the curve at the box chart's percentile
  rule. Single Block Bar: a step outline of the histogram bins (the Histogram plot's rule, `Plot::hist` for custom bins),
  heights `count / (n x bin size)`, scaled like the curve.
- No right Y axis (`--y2`, MCP and `plot_columns_double_y` refuse). `.opj`: written as plain columns, the save warns
  ("ridgeline charts were written as plain columns of their data"). CLI `-t ridgeline`; MCP template text.
- Tests: `ridgeline_tests.rs` (bandwidth formulas by hand, curve equals the shared KDE, baselines, peak scaling,
  back-to-front order, quantile line height, rug, block bars, degenerate data, outlier grid refinement and peak, log X
  axis range and polygon, inherited ridge settings), core template and serde tests, CLI test,
  UI menu and Plot Details tests, `.opj` warning test.
- Not done: grouped and indexed-data ridgelines, ordering by something other than column index, jitter, quantile line
  style and per-line colours, other curve types than Kernel Smooth, a ridgeline toolbar button, Origin's real defaults.

### 14.24 Hexbin plot (T45)

Origin's documented facts (Origin 2027 help, not measured; no Origin was at hand; [W:Origin 2027 page],
[W:origin-help/hexbin-plot]):

- New graph type "Hexbin Plot" in 2027 (with "Binned Scatter Plot"). Menu "Plot > Contour, Heatmap: Hexbin", template
  `Hexbin.OTPU`; input XY or XYZ columns, a virtual matrix or a matrix. "The color represents the number of data points
  within each bin."
- Bin controls per axis: Min Center, Max Center, Bin Spacing, Number of Bins. "Prefer Regular Hexagons" is checked by
  default and only works for a square (1:1) layer frame. "Derive Y from X" (default) or "Derive X from Y" picks the axis
  computed from the other.
- Not documented (Sheaf's choices, unmeasured): default number of bins, colour map, empty bins, orientation, the exact
  tie rule, colour scale look and placement, what happens to points outside Min and Max Center.

**What Sheaf does** (everything else is Sheaf's choice, unmeasured):

- `PlotKind::Hexbin`, `Template::Hexbin` (`hexbin`, "Hexbin", legend hidden: the colour scale names the colours),
  per-plot `Plot::hexbin` (`HexbinStyle`, skipped when default): per axis `min_center`, `max_center` (None = data
  min or max) and `by` (`Bins(n)` or `Spacing(s)`), `prefer_regular` (true), `derive_y_from_x` (true). Defaults:
  20 bins along X, centres from the data minimum to the maximum. XY input only (the first Y column against X, one
  hexbin plot per layer); XYZ, matrix input and Binned Scatter are not done.
- Binning (`sheaf_calc::hexbin`): pointy-top lattice, row r at `y0 + r dy`, even rows at `x0 + c sx`, odd rows half a
  spacing further. Number of Bins n puts n centres from Min to Max Center (`sx = range / (n - 1)`). Rows run from Min
  Center to the first centre on or beyond Max Center. Each finite point goes to the nearest centre, the distance measured
  on the page (in the space where the lattice is regular); an exact tie goes to the lower row, then the lower column; a
  point outside the closed hexagon of its centre (beyond a narrow Min/Max Center) is not counted. An axis with no width
  (all values equal, one point) gets width 1 around the value. NaN and infinite values are skipped.
- Regular hexagons: the derived axis' spacing is `dy = sx * a * (yrange / xrange) * sqrt(3) / 2` (centre ranges),
  with `a` the frame aspect (width / height) when "Prefer Regular Hexagons" is on and 1 otherwise. The drawing redoes the
  binning with the real frame and the resolved axis spans, taking `a (yrange / xrange) = (W / H)(Y span / X span)`, so
  the hexagons are regular on the page for any frame shape, not only a square one (an extension of Origin's 1:1
  rule); a first pass with the nominal frame sets the axis range. "Derive X from Y" solves the same equation for X.
  The colour scale narrows the real frame, so the drawn hexagons can reach past the first pass' range: when they do,
  `layout::graph_geometry` widens the range to the extent of the hexagons as drawn and resolves the axes again (at
  most six passes, the extent only grows), so every hexagon corner lies inside the layer's clip (fixed Min/Max of an
  axis still clip, as for any plot).
- Non-linear axes (log10, ln, log2, reciprocal, probability, ...): the points are mapped through the axis scale's
  `forward`, the lattice is built and the points assigned in those coordinates, and the corners are mapped back through
  `inverse`; the hexagons are regular on the page (the aspect uses the axes' scale spans), no corner leaves the domain
  and no bin is dropped. A point the scale cannot place (<= 0 on a log axis) is skipped and counted in
  `HexData::excluded`, not in a bin. Min/Max Center are data values (mapped through the scale); a Bin Spacing is in scale
  units (decades on log10); Number of Bins is unchanged.
- Drawing (`crates/plot/src/hexbin.rs`): each non-empty bin is a filled hexagon in data coordinates through the layer
  transform, clipped to the layer; no outline, only a 0.25 pt stroke in the fill colour to hide seams. Colour =
  `ramp(count / max count)`, linear from 0, with the colour mapped scatter's ramp (`plots::map_color`: blue, cyan, green,
  yellow, red, unchanged); empty bins are not drawn. The axis range covers the hexagons.
- Several hexbin plots in one layer (added through Layer Contents, Plot Setup or Plot Details) overlap: that is the
  user's choice, they are not placed side by side. All of them are coloured against one shared maximum (the largest
  count of any bin of any hexbin plot of the layer), and the colour scale shows that maximum.
- Colour scale: a 14 pt wide bar 14 pt right of the frame (gradient of 64 strips, red at the top), title "Count" above
  it in the Y title font, ticks at 0 and nice counts (1, 2 or 5 x 10^k steps, about four intervals) up to the maximum,
  labelled in the Y tick font. The frame leaves room for it at the right (`scale_reserve`).
- Plot Details "Bins" tab: Prefer Regular Hexagons, Derive Y from X / Derive X from Y, per axis Min Center, Max Center
  (Auto or a value) and Number of Bins or Bin Spacing; of the derived axis only the Binning and Number of Bins / Bin
  Spacing controls are greyed (its Min and Max Center still apply and stay editable). Undo works as for other
  settings. Menu: Plot > Contour/Heat Map > Hexbin (the other items of that submenu stay greyed).
- CLI `-t hexbin`, `--hex-bins N` (2 to 1000; both axes); MCP template text and `hex_bins`. No right Y axis and no panels
  (CLI, MCP, `plot_columns_double_y` refuse). `.opj`: written as a scatter plot of the data, the save warns ("hexbin plots
  were written as scatter plots of their data").
- Tests: `crates/calc/tests/mpl_hexbin.rs` (counts cell by cell against matplotlib `Axes.hexbin`, fixture made by
  `tools/hexbin-ref.py`, which also checks a numpy brute force; matplotlib 3.11.2, numpy 2.5.4; these are not Origin
  numbers) and hand cases in `hexbin.rs`; `hexbin_tests.rs` (regular hexagons on a square and a wide frame, colours,
  colour scale, autoscale, corners inside the clip over seeds and bin counts, log axis, shared colour scale); core template/serde test; CLI test; UI menu, Plot Details and derived-axis tests; `.opj` warning test.
- Not done: XYZ and matrix input, Binned Scatter, UpSet, a hexbin toolbar button, colour map choice, a tick count
  setting, Origin's real defaults, outlines, hover read-out of bin counts.

### 14.25 Parallel Plot and Parallel Index (T49)

Origin's documented facts (Origin 2019 help, not measured; no Origin was at hand; [W:origin-help/parallel-plot],
[W:origin-help/parallel-index], [W:origin-help/axes_dialog_parallel_coordinates], [W:origin-help/pd-dialog-parallel-tab]):

- Parallel Plot: menu "Plot > Categorical: Parallel Plot" (also "Plot > 2D: Parallel Plot"), template `Parallel.otpu`;
  at least two columns, all treated as Y; a colour scale legend; the colour is usually mapped from a column (By Points
  tab). Parallel Index: "Plot > Categorical: Parallel Index"; at least three columns plus a grouping column of
  categorical data that creates the legend and assigns the plot colour.
- Axes dialog: "Rescale Each Axis Independently": checked, each axis has its own From and To; cleared, all axes share one
  range, that of the variable with the greatest range. The axis title defaults to `%(?Y)` (Long Name or Short Name plus
  Unit). Dragging a column onto the plot adds a last axis.
- Plot Details Parallel tab: Style, Width, Transparency 0 to 100 (0 opaque); Curvature 0 to 100 (used by the alluvial and
  sets plots, not by these two).
- Not documented (Sheaf's choices, unmeasured, listed below): the default of Rescale, line width and transparency, straight
  lines, the colour map, the tick layout, the default colour column, missing values, where the legend goes. Parallel Sets
  are not done.

**What Sheaf does** (everything else is Sheaf's choice, unmeasured):

- `PlotKind::Parallel` and `PlotKind::ParallelIndex`, `Template::Parallel` (`parallel`, "Parallel Plot", legend hidden:
  the colour scale names the colours) and `Template::ParallelIndex` (`parallel-index`, "Parallel Index", legend position
  Automatic). One plot per axis (its Y column); the settings are per plot, shared by the layer's parallel plots:
  `Plot::parallel` (`ParallelStyle`, skipped when default) with `rescale_each_axis` (true), `width_pt` (0.5),
  `transparency_pct` (0) and `color` (`Option<ColumnId>`). Hidden plots leave their axis out.
- Colour source. Parallel Plot: an optional numeric column; Sheaf's default is the last selected column (a new graph sets
  `color` to the last axis column, so the colour scale shows at once; "None" draws every line in the plot colour, the
  first plot's). Parallel Index: a required group column, by default the first text column of the sheet that is set as
  Group, else the first text column (never one of the axes), or the text column among the selected columns in the app, or
  `--group COLUMN` on the command line (any column; the cells are grouped by their shown text, so a numeric code works).
- Refusals (core `ProjectError`, so the app, CLI and MCP all show them): a Parallel Plot needs two or more numeric
  columns, a Parallel Index three or more plus a group column; a right Y axis is refused (`plot_columns_double_y`, `--y2`)
  and so are panels (`--panels`). A refused plot leaves no graph behind. The numeric columns among the selection are the
  axes; a text column among them is the group.
- Layout (`crates/plot/src/parallel.rs`): the layer's own X and Y axes, grid and ticks are not drawn. Axis `i` of `n`
  stands at `frame.x + frame.w (i + 0.5) / n` and runs from the bottom to the top of the frame: its line, major ticks
  (the layer's Y axis settings: length, direction, colour, width, tick font, number format; a linear scale and automatic
  range, a fixed Y limit of the layer is ignored) with labels left of it. The title is above the frame, centred on the axis,
  in the Y title font, shrunk (never below 5 pt) so that the widest title fits the spacing of the axes; it is the plot's
  label if set, else the column's long name and unit (`Long Name (unit)`, Origin's `%(?Y)`). The frame is fitted on the
  page with room for the titles, the first axis' labels and the colour scale or legend.
- Scaling: rescale on, each axis has the nice range of its own column (the same rule as any automatic Y axis, with the
  layer's range margin); off, every axis has the nice range of the union of all the columns' values (the documented
  "variable with the greatest range", widened so that the smaller columns also fit) and shows the same ticks. A row's
  point on an axis is `(value - min) / (max - min)` of that axis' range.
- Lines: one straight polyline per row through the axes, one scene path per row in worksheet order (later rows on top),
  Round caps and joins, width and transparency (the colour's alpha is scaled by `1 - transparency / 100`) from the Parallel
  tab. A row with a missing cell (NaN, an empty cell, a masked point) breaks there: each run of two or more consecutive
  axes with values is drawn as its own piece of the path; a lone value draws nothing, and a row with no such run draws no
  line. Rows hidden by a worksheet filter are not drawn; the rows come from the first axis' plot.
- Colour: a Parallel Plot with a numeric colour column colours each line with `plots::map_color` (the colour mapped
  scatter's ramp, unchanged) at `(v - min) / (max - min)` of that column's smallest and largest value (0.5 when they are
  equal); a row whose colour cell is missing takes the plot colour. The colour scale is the hexbin plot's bar
  (`hexbin::draw_color_bar`, now shared; the hexbin output is unchanged): a 14 pt bar 14 pt right of the frame with the
  column's name as title and nice ticks inside the data range. A Parallel Index gives each distinct text of the group
  column, in order of first appearance, one colour of the layer's palette (`Palette::color(i, groups)`) and one legend
  entry. Origin's colour map was not measured.
- Group legend (`parallel::draw_legend`): the layer's legend font, frame and background, one row per group with a line
  sample in the group's colour and the group's text ("(blank)" for an empty one). Automatic (the template's) places it
  right of the frame, 14 pt away and top aligned, and the frame leaves room for it; the corner and manual positions place
  it inside the frame as the ordinary legend does. A custom legend text and columns are not used. `draw_legend` of the
  ordinary legend returns at once for parallel plots (it would list the axes). The group legend and the Parallel Plot's
  colour scale are recorded together as one `ObjectKind::Legend` (their drawn rect), so in the app they are picked,
  selected, dragged and double-clicked to the Legend tab like any legend. The layer legend's position places both: the
  drag sets `LegendPosition::Manual` (centre at fractions of the frame); the colour scale (title, bar and labels, as high
  as the frame) is moved by the same rule (`parallel::scale_shift`; Automatic leaves it right of the frame, a corner puts
  it at the frame's left or right inside edge, Manual centres it on the point). There is no separate offset.
- Plot Details "Parallel" tab: Rescale Each Axis Independently, Width, Transparency (%), and Color Column (Parallel Plot) or
  Group Column (Parallel Index), a list of every column of the project (any, for the group) or only the numeric columns
  (for the colour of a Parallel Plot, a text column would be ignored) with "None". The settings are shared by the layer's
  parallel plots. Undo works as for other settings. Menu: Plot > Categorical > Parallel Plot and Parallel Index (a new
  submenu), also in Column > Plot.
- CLI `-t parallel`, `-t parallel-index`, `--group COLUMN` (`none` for the plot colour; with `-t parallel` a numeric
  colour column: a text one is refused with a message that suggests `-t parallel-index`); MCP `template` text and `group`. `.opj`: written as line plots of the columns, the save warns
  ("parallel plots and parallel indexes were written as line plots of their columns; their axes, colours and groups were
  not written").
- Tests: `crates/plot/src/parallel_tests.rs` (axis places, scaling on and off, breaks at missing and masked cells, colour by
  column and by group, titles, colour scale, legend, width and transparency; the legend and scale are hit-test objects
  that a Manual or corner position moves); core template/refusal/serde test; `.opj`
  warning test; `crates/cli/tests/parallel.rs` (both kinds as SVG, `--group`, refusals, a text colour column);
  `graph_details.rs` colour-column choices test; UI menu, group-from-text-column and
  Plot Details tests. The existing hexbin and golden tests pin that other kinds are unchanged.
- Not done: Parallel Sets, a curvature setting, a custom colour map or By Points tab, log axes, dragging a column onto the
  plot, reordering or flipping axes, hover read-out of rows, hit-testing of the axes, a toolbar button, Origin's real defaults.

### 14.26 Treemap (T50)

Origin's documented facts (Origin 2025b help, not measured; no Origin was at hand; [W:originlab.com/2025b],
[W:origin-help/treemap], [W:origin-help/pd-layout-tab-treemap]):

- New graph type "Treemap plot", menu "Plot > Categorical: Treemap", template `treemap.otpu`; data: several columns of
  categories and one column of values.
- Plot Details Layout tab: Layout (Split, Squarified, Squarified+, Pivot by Size, Pivot by Middle, Pivot by Split Size),
  Order (Categorical Order, Alphabetical, by Size, by a column), Depth (1 to the number of levels, or Infinite), Gap
  between rectangles, and an optional separate Layout and Order for each level but the first. Squarified sorts the
  rectangles by size, the largest at the top left, and ignores Order; Squarified+ keeps for each row the direction with the
  better aspect ratio; Pivot by Size takes the largest item as pivot, Pivot by Middle the middle item, Pivot by Split Size
  the item that splits the list into roughly equal areas.
- Not documented (Sheaf's choices, unmeasured, listed below): the default layout, order, depth and gap, the colours, the
  labels, the borders, missing and non-positive values, the algorithms themselves.

**What Sheaf does** (everything else is Sheaf's choice, unmeasured):

- Algorithms (`sheaf_calc::treemap`, pure functions `layout(kind, weights, rect) -> rects`, one rectangle per weight in the
  order of the weights, tiling the rectangle, the area of each `weight / total * area`; a weight that is not finite and
  positive gets a zero-size rectangle):
  - Squarified: Bruls, Huizing and van Wijk (2000): sorted by size, descending; rows along the shorter side of the rest, an
    item joins the row while its worst aspect ratio does not grow; the first row is a column on the left (wide rest) or a
    band along the top (tall rest), so the largest item is at the top left.
  - Squarified+: as Squarified, but for every row both the shorter and the longer side of the rest are tried and the row with
    the smaller worst aspect ratio is kept (a tie: the shorter side). This is Sheaf's reading of Origin's one-line description.
  - Split: the ordered list is cut in two, in order, at the first position whose two total weights differ least; the rectangle
    is cut across its longer side in the ratio of the two weights (left or top gets the first part), each part again.
  - Pivot by Size / Middle / Split Size: the ordered treemaps of Shneiderman and Wattenberg (2001) and Bederson et al.
    (2002): pivot `P`, `L1` the items before it, then the items after it cut into `L2` and `L3`. In a rectangle at least as
    wide as high: `L1` a full-height column on the left, `P` at the top left of the rest with `L2` on its right, `L3` the
    band along the bottom; in a taller rectangle the axes are exchanged. The `L2`/`L3` cut is the one that brings `P`'s aspect
    ratio nearest to 1 (the first on a tie; found with prefix sums, so one step is linear and 30000 sorted weights lay out in well under a second); the lists are laid out again the same way. Pivot choice: the largest item (first
    on a tie); the item at index `n / 2`; the item with the least difference between the total before and after it.
- Hierarchy (`sheaf_calc::treemap_tree`): one level per category column, the first the top level. Rows with a missing, non-finite
  or non-positive value are skipped (not an error); rows with the same category path are one leaf with the summed value; an
  empty category cell is named "(blank)"; a node's value is the sum of its children. Order: Categorical (first appearance,
  the default), Alphabetical (case-insensitive), By Size (largest first), By Column (ascending by the smallest key of the
  node's rows: numbers first, then text alphabetically, missing last; the column is `order_column`). The Squarified layouts
  sort by size themselves, so their Order is ignored (the Order list is greyed out for them).
- Nesting: the children of a node are laid out in the node's rectangle inset by half the gap (below a header band for a
  top-level node) and every laid-out cell is shrunk by half the gap on each side, so a child lies at least one gap inside
  its parent and siblings are one gap apart; the whole treemap lies half a gap inside the layer frame. A rectangle too small for the inset
  becomes empty and is not drawn. Depth N shows N levels; the nodes of the last level shown are leaves with the value of
  everything below. Per level: with the check box on, level 2, 3, ... take their own layout and order from the list (a level
  without an entry takes the general ones).
- Settings: `PlotKind::Treemap`, `Template::Treemap` (`treemap`, "Treemap"; axes hidden, legend hidden), `Plot::treemap`
  (`TreemapStyle`, skipped when default): `categories` (columns, first = top level), `layout` (Squarified), `order`
  (Categorical), `order_column`, `depth` (`None` = Infinite), `gap_pt` (1.5), `per_level_on` and `per_level`. One plot per
  graph: the plot's own Y column is the value column (the last numeric selected column); the category columns are the
  columns of text among the selected columns, in selection order, else every column of text of the value column's sheet
  (core `plot_columns`, `plot_treemap`). Refusals: no category column, no numeric value column, a value column that is
  text, a right Y axis (`--y2`), panels, an overlay. A refused plot leaves no graph behind.
- Drawing (`crates/plot/src/treemap.rs`): the rectangles fill the layer frame; no axes, no frame line, no legend. One colour
  of the layer's palette per top-level category (by first appearance, whatever the order); a node `L` levels deep is that
  colour blended `1 - 0.7^L` of the way to white; every rectangle has a 0.5 pt white border. A node with its
  children shown has a header band 1.55 text heights high (and a padding) holding its name, in the colour of the band; the
  band and its text of level `L` are `0.85^L` of the top level's (Sheaf's choice), and a level-1+ node less than twice the
  band high or wide has none and its name is not drawn (level 0: the height test only). A leaf (the
  last level shown) shows its name and below it its value (up to four significant digits, no exponent) at the top left, in
  the layer's X axis tick font at 0.45 of its size (9.9 pt for the 22 pt default), black or white as reads better on the fill.
  A label is drawn whole or not at all: the name is left out when it is wider than the rectangle (less a 0.3 text size padding
  each side) or the rectangle is not as high as one line, the value when it does not fit under the name; a header name too wide
  for its band is left out. There is no clipping.
- Plot Details "Layout" tab (a new tab, shown for a treemap plot): Layout and Order lists (the Order greyed out for the
  Squarified layouts), Depth (Infinite, 1 to the number of category columns), Gap (pt), the Order column list (shown when an order
  by column is chosen), and the check box "Different Layout and Order for Each Level Below the First" with one Layout and Order
  row per level below the first. Undo works as for other settings. Menu: Plot > Categorical > Treemap (also in Column > Plot).
- CLI `-t treemap` with `--y` (the value; the last numeric one counts), `--group A,B` (category columns, first = top; default the
  text columns among `--y`, else every text column of the sheet), `--treemap-layout`, `--treemap-order`
  (`categorical|alphabetical|size|column:NAME`), `--treemap-depth`, `--treemap-gap`; the same in MCP (`group`, `treemap_*`) with the
  template text. `.opj`: written as column plots of the value column; the save warns ("treemaps were written as column
  plots of their value column; their categories and nested rectangles were not written").
- Tests: `crates/calc/src/treemap.rs` (all six layouts on random weights with fixed seeds: tiling without overlap, areas in
  proportion within 1e-9 relative, hand cases for Bruls' example, Split order, the pivot lists, tall rectangles, degenerate
  input), `treemap_tree.rs` (aggregation, skipped values, duplicates, orders, depth, gap nesting, per-level),
  `crates/plot/src/treemap_tests.rs` (nesting inside parents by the gap, colours per top level and tints, labels hidden when they do
  not fit, depth, every layout with every order, other kinds unchanged), core template/refusal/serde test, `.opj` warning test,
  `crates/cli/tests/treemap.rs` (SVG, defaults, every layout and option, refusals), UI menu and Layout tab tests with undo.
- File > Append renumbers the appended graphs' columns through `Graph::remap_columns` (core `graph.rs`): the plots' X, Y, error,
  base, size and colour columns, label columns, treemap and sunburst `categories`, the treemap `order_column`, the parallel `color` column, plot
  offsets, tick-label columns and the rug `plot` choice of each axis. A new field holding a column id goes there.
- Not done: Origin's real defaults, colours and labels, hover read-out and picking of rectangles, drilling down, a colour-by-value
  option, a toolbar button, a label font setting, reading Origin's treemap from .opj, a size cap on very many rectangles.

### 14.27 Sunburst plot (T51)

Origin's documented facts (Origin 2022 help, not measured; no Origin was at hand; [W:originlab.com/2022],
[W:ReleaseNotes ORG-23725], [W:origin-help/sunburst-plot]):

- New graph type "Sunburst Plot", menu "Plot > Categorical: Sunburst Plot", template `Sunburst.OTP`; data: at least two
  text columns and one numeric column; the last numeric column holds the values and every other column is a category or
  sub-category; each level is a ring.
- A wedge's colour follows its value; a category's colour follows the mean value of its children; a missing text category
  appears unfilled.
- Plot Details: Pie Geometry tab (Rotation, Radius/Center, Doughnut) and Labels tab; the sunburst is a variant of the Doughnut
  plot. The Mini Toolbar button "Collapse Expand Mode" drills into a clicked wedge.
- Not documented (Sheaf's choices, **unmeasured**, listed below): which level is innermost, the start angle and direction, the
  hole, ring widths, borders, the colour map, the label rules, the order, how "mean" and "missing" are read.

**What Sheaf does** (everything else is Sheaf's choice, unmeasured):

- Hierarchy: the one of the treemap (`sheaf_calc::treemap_tree`, 14.26: skipped missing or non-positive values, duplicate paths
  summed, a node's value the sum of its children), generalised only by a flag `Node::blank` that marks a node whose category cell
  was empty (named "(blank)"); the treemap output is unchanged. The angular layout is `sheaf_calc::sunburst::layout`: wedges
  parents before children; a node's angle is its value's share of its parent's angle, the top level sharing the full turn, so
  the angles of the children add up to the parent's. Order: Worksheet (first appearance, the default) or By Size (largest first).
- Colour value: a leaf's own value; a category's is the unweighted mean of the values of the leaves below it (Sheaf's reading of
  "the mean value of its children"). The colour is `plots::map_color` (the shared blue-cyan-green-yellow-red ramp) at
  `(value - lo) / (hi - lo)` over the range `lo..hi` of the leaf values (0.5 when the range is empty). A wedge whose category cell
  was empty is outline only (no fill; a dark outline of the tick font colour, at least 0.5 pt).
- Geometry: the circle is centred in the layer frame with a diameter equal to the shorter side of the frame, so it stays round on
  a non-square frame; no axes, no frame line, no legend (template: axes and legend hidden). The first category column is the
  innermost ring; every level has the same width from the hole radius to the outer radius; a ring gap (pt) takes half from each
  side of a ring where it has a neighbour (not at the hole or the rim). The first wedge starts at the start angle in degrees
  counter-clockwise from 3 o'clock (default 90, 12 o'clock, as Origin's pie rotation is counted) and the wedges follow clockwise.
  Arcs are polylines of steps of at most 2 degrees; a wedge round the whole turn is two circles (outer and the hole).
  Every wedge has a white border (0.5 pt by default).
- Labels: the name (and, when asked, the value, with up to four significant digits) at the wedge's centre (middle radius, middle
  angle), along the radial direction and turned half a turn when it would read upside down, in the layer's X axis tick font at 0.45
  of its size, black or white as reads better on the fill. A line is drawn whole or not at all: it is left out when its text (plus
  0.3 text sizes each side) is wider than the ring, and the lines together (their height plus 0.3 text sizes each side) must fit
  the chord of the wedge at the inner end of the widest line (middle radius minus half its width: a radial label is widest
  across the wedge where the wedge is narrowest), else the second line (the value) goes first; a name that does not fit hides the whole label, so a value is never drawn without its name
  (unless names are off). Names default on, values off.
- Colour scale: a vertical bar right of the frame (the hexbin and Parallel Plot bar, `hexbin::draw_color_bar`, with its reserve in
  the layer's edges), titled with the value column's name and ticked at nice values over the leaf range.
- Settings: `PlotKind::Sunburst`, `Template::Sunburst` (`sunburst`, "Sunburst Plot"), `Plot::sunburst` (`SunburstStyle`, skipped when
  default): `categories` (columns, first = innermost), `start_angle_deg` (90), `hole` (fraction of the radius, 0.3, 0 to 0.9),
  `ring_gap_pt` (0), `border_pt` (0.5), `show_names` (true), `show_values` (false), `order` (`worksheet`|`by_size`). One plot per graph:
  the plot's Y is the value column (the last numeric selected column); the categories are the columns of text among the selected
  columns in selection order, else every text column of the value column's sheet (`plot_columns`, `plot_sunburst`). Refusals: no
  category column, no numeric value column, a text value column, a right Y axis, panels (CLI also: overlay, styling a graph). A
  refused plot leaves no graph behind.
- Plot Details "Pie Geometry" tab (shown for a sunburst plot; Origin's tab name, with the Labels tab's two check boxes folded in):
  Start Angle, Hole (%), Ring Gap, Border Width, Order, and Labels Names and Values. Undo works as for other settings. Menu: Plot >
  Categorical > Sunburst Plot (also in Column > Plot).
- CLI `-t sunburst` with `--y`, `--group A,B,C` as for the treemap, `--sunburst-start`, `--sunburst-hole` (percent), `--sunburst-gap`,
  `--sunburst-labels names|values|both|none`, `--sunburst-order worksheet|size`; the same in MCP (`group`, `sunburst_*`) with the
  template text. `.opj`: written as column plots of the value column; the save warns ("sunburst plots were written as column plots
  of their value column; their categories and rings were not written").
- File > Append renumbers the sunburst `categories` through `Graph::remap_columns` (test in `project.rs`).
- Tests: `crates/calc/src/sunburst.rs` (angles as shares of the parent, colour values and means, order),
  `crates/plot/src/sunburst_tests.rs` (angles drawn, start angle and direction, hole and ring radii with the gap, arcs of at most
  2 degrees inside a round circle on a wide frame, leaf and category colours, unfilled missing category, labels hidden when too
  small and upright, scale title and ticks, full-turn ring, other kinds unchanged), core template/refusal/serde and append tests,
  `.opj` warning test, `crates/cli/tests/sunburst.rs` (SVG, defaults, options, refusals), UI menu and Pie Geometry tab tests with undo.
- Not done: Origin's real defaults, colours and labels, hover read-out and picking, Collapse/Expand drilling, a depth setting,
  the Radius/Center settings, tangential labels for thin rings, a colour-by-category-palette option, a label font setting, reading
  Origin's sunburst from .opj.

### 14.28 Grouped Columns (indexed data) and Grouped Box Charts (T76)

Origin's documented facts (documentation only, **not measured**; no Origin was at hand; paraphrased;
[W:docs.originlab.com/x-function/ref/plot_gindexed], [W:docs.originlab.com/origin-help/grouped-column-index-data],
[W:docs.originlab.com/x-function/ref/plot_gboxindexed], [W:docs.originlab.com/origin-help/groupedboxcharts-indexeddata],
[W:docs.originlab.com/tutorials/grouped-box-plot]):

- The two items run the X-Functions `plot_gindexed` and `plot_gboxindexed`, available since Origin 9.1 SR0 (so in 2016). Both
  open a dialog: the data form (summarized or raw), the bar and error-bar statistics, stacking, themes, panels or pages
  split by a group column. Plot > Column/Bar/Pie > Grouped Columns (indexed data) is the greyed name in Sheaf's menu; in
  newer help the same function sits under Plot > Categorical.
- Indexed data: one or more group columns label the rows of the data column. The order of the group columns is the
  hierarchy: the first is the main group on the axis, the next one nested inside. Groups sort alphabetically by default.
- The result is one grouped plot (not one plot per group). With two or more group columns the X tick labels are a table
  of rows, one per level. The fill colour is indexed by the subgroup by default.
- Not documented on the pages read: the default bar statistic and error bar, the spacing and gap numbers, the legend,
  the maximum number of group levels.

**What Sheaf does** (everything below that is not in the list above is Sheaf's choice, **unmeasured**):

- Items: Plot > Column/Bar/Pie > Grouped Columns (indexed data) (`Template::GroupedColumns`, CLI `grouped-columns`) and
  Plot > Statistics > Grouped Box Charts (`Template::GroupedBox`, `grouped-box`). They plot straight from the selection
  with no dialog (Origin's dialog is a leftover).
- Selection: the data is the one selected numeric column designated Y; every other selected column is a group column, in
  selection order, one or two (a numeric group column such as a scan number must be designated X or Group first). Anything else
  is a status-bar warning, "Select one or two group columns and one data column (a numeric group column must be designated X or Group)", and no graph. More than two group
  columns and a right Y axis are refused with a message. CLI: `sheaf plot -t grouped-columns|grouped-box -y DATA --group
  G1[,G2]` (`--group` required; the data is the last numeric `--y` column, designation is not checked) and `--group-error none|sd|se` for the columns; panels, overlay, graph styling and
  `--y2` are refused. MCP's `plot` takes the same names and `group_error`.
- Model: the plots stay `PlotKind::Column` and `PlotKind::Box`; the new per-plot field `Plot.indexed` holds the group column,
  for two group columns the second column and this plot's level, and the error bar. One group column gives one plot; two
  give one plot per level of the second column, labelled with the level, so the legend names the subgroups and the colours
  follow the layer's palette. The X axis shows the levels of the main group only (single-row labels) and takes the group
  column's title. `.sheaf` round-trips the field; `.opj` export writes plain plots of the data column with a save warning;
  user templates refuse these graphs.
- Levels: the sorted distinct values of a group column (numbers numerically when every level is a number, otherwise plain
  case-sensitive text order). Rows with a missing data value or an empty group cell are dropped. The main-group levels are
  those of all rows with data, so every subgroup plot shares the positions 1..K.
- Columns: the bar is the mean of the rows of the category (and subgroup); a category with no rows in a subgroup has no
  bar. The error bar is None by default, or the SD (n-1) or the SE of those rows (a single row has none); Plot Details >
  Error Bar sets it for every plot of the layer, one undo step. Bars side by side come from the layer's gap and overlap.
- Boxes: the existing box statistics (Hazen quartiles, 1.5 IQR whiskers, outliers, mean) of the rows of the category and
  subgroup. S subgroups share the 0.8 box band, each box 0.8/S wide; one subgroup keeps the plain box width. The Y range
  covers whiskers, outliers and error bar ends. No significance brackets on grouped boxes.
- Tests: `crates/core/src/template.rs`, `crates/core/src/indexed.rs`, `crates/plot/src/grouped_tests.rs` (goldens
  `grouped-box.svg`, `grouped-columns.svg`), `crates/app/src/app/ui_tests/grouped_indexed.rs`,
  `crates/cli/tests/grouped_indexed.rs`, `crates/io/src/opj/write/graph_feature_tests.rs`.

**Arrow at Begin / Arrow at End (T245, not measured: Origin 2016 has no LabTalk property for them, `layer.x.arrowEnd` reads 0 and setting it changes nothing, T266).** The Line and Ticks tab's **Arrow** row has
**Arrow at Begin** and **Arrow at End** (`Axis::arrow_begin`, `arrow_end`). A filled arrowhead in
the axis line's colour sits with its base on that end of the axis line and its tip outward: the
beginning is the left end of an X axis and the bottom end of a Y axis, the end is the right or top
end, as the page shows them (a reversed axis keeps its arrows on the page ends). The head is 5 times
the line width long and 3.5 times wide (at least 6 by 4 points), Sheaf's own sizes. It moves with
the axis line (`% From` offsets and At Position =), belongs to the axis' click area, and is drawn on
the main side only. The `.opj` writer names it; the companion keeps it. Test:
`axis_arrows_point_out_of_the_chosen_ends_in_the_line_colour` (`crates/plot/src/offset_axis_tests.rs`).

**Leftovers:** Origin's dialog and data forms, raw-data bar statistics and the other error-bar statistics (CI, percentile
ranges, MAD), stacking, two-row table tick labels, more than two group levels, Grouped Scatter, panels or pages split by a
group, themes (half box, violin, ...), colour indexed by the main group, and a measurement of all of it in Origin 2016.

### 14.29 Interval Plot and Pareto chart (T83)

Use: Plot > Statistics > Interval Plot and Pareto. Status: written from Origin's documentation on a machine without Origin;
nothing here was measured, so every choice not listed as documented is Sheaf's and **unmeasured**.

**Documented** [W: docs.originlab.com/origin-help/interval-plot, .../create-pareto-chart, .../paretochart-rawdata,
docs.originlab.com/x-function/ref/plot_paretoraw, .../plot_paretobin]:

- An interval plot is a box chart with the box hidden: only the mean shows, as a solid circle (template BoxInterval). The
  default error statistic, confidence level, labels and colours are not given.
- Pareto: one command for binned data (category and count columns) and one for raw data (cells counted per distinct value;
  template ParetoRaw). Bars from most to least significant. The cumulative percent is a line with symbols in a new layer
  whose right Y axis always runs 0 to 100 (option `cum`, default on); drop lines at 80, a count table, combining small
  categories into "Other" and groups are options. Fill, gap, colours, tie order and missing values are not given.
- Pareto, raw data, option `symr` (default 1): the cumulative-percent symbols sit at the right edge of each bar, and the
  cumulative line starts at (0, 0), the left edge of the first bar at 0 %. Sheaf copies both (placed from the drawn bar
  width, so they follow the layer's gap); the first and the last point carry no data labels.

**Measured 2026-11-16 in Origin 2016** (Plot > Statistics > Interval Plot and Pareto Chart - Raw Data through the
menu, PNG exports `re-workspace/scratch/t83-origin/interval.png`, `pareto.png`):

- Interval Plot of A = 1..10, B = 2, 4, ..., 16, C = {3, 5, 4, 8, 10}: one filled circle per column at the mean
  (5.5, 9, 6) with a capped bar of the Student-t 95 % confidence interval (3.33..7.67, 4.90..13.10, 2.38..9.62),
  so Sheaf's default statistic and level match. Colours black, red, blue (the column order through Origin's
  list), bars in the symbol's colour; X at 1, 2, 3 labelled with the column names, range 0.5..3.5, no X title; Y
  title "Range"; Y axis -2..20 (wider than the intervals); **one legend with two entries**, an error-bar sample
  "Mean ± 95% CI" and a circle "Mean", not one entry per column.
- Pareto Chart - Raw Data of one text column (b ×5, a ×4, c ×2, d, e, f ×1; 14 cells): two layers. Bars red with a
  black outline, most frequent first, ties in order of first occurrence (d, e, f) as Sheaf; X tick labels the
  categories, no X title. Left Y "Count" from 0 to **1.1 × the total** (15.4); right Y "Cumulative Frequency"
  from 0 to 110 with ticks every 20 labelled **0% ... 100%**, so 100 % stands level with the total count.
  The cumulative percent is a thin blue line with blue squares at the **bar centres** (no (0, 0) point), each
  labelled with its value to 7 significant digits (35.71429, 64.28571, ..., 100) above it. No legend.

**Copied by T316 (2026-11-21):** the interval plot's Y title "Range"; the Pareto chart's red bars with a black
outline, left axis "Count" from 0 to 1.1 × the total and no X title, and the cumulative percent as a thin blue line
with blue squares at the bar centres (no (0, 0) start), each labelled above, on a right axis "Cumulative Frequency"
from 0 to 110 with a tick every 20 labelled 0% to 100%. **Still different:** the interval plot's two-entry legend
(Sheaf lists one entry per column) and the labels' 7 significant digits (Sheaf writes the full value).

**Pareto Chart - Binned Data (measured 2026-11-21 in Origin 2016, T319):** `plot_paretobin` takes a Data Input Range
(the categories) and a Count Input Range, with Recalculate (Auto), Combine Smaller Values (off; By Percent or By Count,
tick label "Other") and Output Data. On dent 4, scratch 9, crack 2, chip 4, stain 1 it writes a sheet ParetoBin1 to the
book with the columns Data, Count and Cumulative Frequency (comments: the category column's name), sorted by count
with equal counts in sheet order (scratch 9, dent 4, chip 4, crack 2, stain 1; 45, 65, 85, 95, 100 %), and draws it
exactly as the raw-data chart (Count 0..22 = 1.1 x the total 20, right axis 0..110 %). Sheaf: Plot > Statistics >
Pareto with two columns selected (the categories, then the counts) does the same (`Project::plot_pareto_binned`);
rows without a category or a count are left out; Combine Smaller Values is not built.

**What Sheaf does** (unmeasured unless listed above):

- Pareto counts every cell of the column: row filters and Hide Masked Points are not applied to it (Sheaf's choice; the
  interval plot does honour them). A column with no usable values is skipped and does not use up a category slot.
- Items: Plot > Statistics > Interval Plot (`Template::IntervalPlot`, CLI `interval-plot`) and Pareto (`Template::Pareto`,
  `pareto`), no dialog. Model: the new per-plot field `Plot.stat` (`StatPlot::{Interval{error, level}, Pareto,
  ParetoCumulative}`), skipped in `.sheaf` while empty. `.opj` export writes plain plots of the data column with a save
  warning; user templates refuse these graphs; a right Y axis is refused for both.
- Interval Plot: one `Scatter` plot per selected numeric Y column (symbol: a filled circle, as documented; colours and
  size are Sheaf's), at X = 1, 2, ... in plot order, named by the column on
  the X axis (like box charts; no X title). The symbol is the mean of the finite numbers; the error bar is symmetric:
  the Student-t confidence interval of the mean (default, level 95 %, the Graphical Summary's `mean_confidence_limits`,
  statistics 22), or the SD (n - 1) or SE. A column of one number has no bar. Plot Details > Error Bar chooses
  CI/SD/SE and the level for every interval plot of the layer. CLI `--interval-error ci|sd|se`, `--interval-level`.
  Selection: one or more numeric columns, else the status warning "Nothing to plot".
- Pareto: select one column (text or numbers), or a category and a count column (binned, above); anything else is a
  status warning. The left layer holds `Column` bars of the count per distinct value, most frequent first
  (ties in the order of first occurrence; empty cells are not counted; numbers compare as text without a trailing ".0");
  the categories are the tick labels at 1..K, with no X title. The right layer (the usual linked right-Y layer)
  holds a `LineSymbol` plot of the running percent of the counted cells at the bar centres, styled and scaled as
  Origin 2016's (above). Sheaf adds no drop lines, count table, "Other" merge, binned-data input or groups.
- Tests: `crates/core/src/template.rs`, `crates/plot/src/grouped_tests.rs` (hand-computed mean, SD, SE, t quantiles and
  Pareto percentages; goldens `interval-plot.svg`, `pareto.svg`), `crates/app/src/app/ui_tests/grouped_indexed.rs`,
  `crates/cli/tests/interval_pareto.rs`, `crates/io/src/opj/write/graph_feature_tests.rs`.

**Leftovers:** Origin's dialogs, measured defaults (error bar, symbol, colours, gaps), box-chart based interval themes and
grouped intervals, Scatter Interval, Pareto binned data, groups, drop lines at 80, count table, combine small categories,
and a measurement of all of it in Origin 2016.

### 14.30 Run chart (T270)

Use: Plot > Statistics > Run Chart... with a numeric column selected (else the sheet's first numeric Y
column). Origin added it in 2025b (`plot_runchart`, also in 2026's Graph Maker); OriginLab's help page
[W: docs.originlab.com/origin-help/run-chart] was the source, and Origin 2016 has no run chart, so
nothing here is measured.

- **Documented:** subgroups of one column by a size (or by a reference column, or each row of several
  columns: not built); the mean or median of each subgroup as a Line+Scatter plot over the sequence;
  optional individual data points at their subgroup; a horizontal dashed line at the mean or median of
  the whole input as the reference. Template Runchart; the statistics table on the graph is not built.
- **Sheaf's choices** (`crates/core/src/runchart.rs`, window `dialogs/run_chart.rs`): Subgroup Size 1
  by default (each value is its own subgroup), Connect Mean or Median (Mean), Show Individual Data
  Points on. Subgroups are consecutive rows; missing values are left out and an empty subgroup has no
  point; the last subgroup may be shorter. A new workbook `RunChart<n>` gets Subgroup (X), Mean or
  Median (Y) and, with the points, Point Subgroup (X) and Data (Y); the graph `RunChart<n>` shows the
  statistic as black squares joined by a black line, the values as small grey circles, X from 0 to
  the number of subgroups + 1 (a tick per subgroup up to 20), X title "Subgroup", Y title the source
  column, and a blue dashed straight line at the statistic of all values, labelled
  "Mean = 5.5" (four significant digits) right of the frame. One undo step ("Run Chart").

### 14.31 Grouped histogram (T271)

Use: Plot > Statistics > Grouped Histogram... on a worksheet. Origin 2026 added it as Plot > Categorical:
Grouped Histogram (`plot_ghist`); OriginLab's help page [W: docs.originlab.com/origin-help/grouped-histogram]
was the source, and Origin 2016 has no such plot, so nothing here is measured.

- **Documented:** a Y column (optionally with Y error) and at least one group column; Multiple Data
  "Overlay on Same Graph", "Separate Layers on Same Graph" or "Separate Graphs"; Split Data by a label
  row; panels by group columns; Split Pages; Share Y; Share X and Bins; a distribution curve (with
  Hide Bins); Data Height; the Hist template; output to graphs or a report sheet.
- **Sheaf's choices** (`crates/core/src/grouped_hist.rs`, window `dialogs/grouped_hist.rs`): one group
  column; the window offers Data (the first selected numeric column, else the first numeric Y column),
  Group (the first other selected column, else the first text column, else the first X column) and
  Overlay on Same Graph (default) or Separate Layers on Same Graph. The group's levels are sorted as for
  the grouped box charts (numbers numerically, else text order; empty cells and missing values left
  out). Each level's values go to a column of a new workbook `GroupedHist<n>` whose long name is the
  level (so the legend names the groups). Overlay is the Histogram graph of those columns in one layer;
  Separate Layers is their Stacked Histograms graph. The bins are shared (the usual rule for all the
  values together). One undo step ("Grouped Histogram"). Separate Graphs, panels, pages, distribution
  curves and data height are not built.

## 15. Multi-panel graphs (P0, GP-PAN-01)

Use: several panels on one page, most often a vertical stack of cyclic voltammograms or XRD patterns that share the potential or 2-theta axis, with the X tick labels shown only under the bottom panel. Status: researched on 2026-11-12 from the definition files of Origin's plotting functions (variable names and defaults only), the public help pages on stacked and multi-panel graphs, and three Origin runs through COM on our own synthetic data (scripts and pictures in `re-workspace/scratch/graph-p0/`, git-ignored). Tags as in section 12; **[COM]** is behaviour seen in Origin. Where the current online help **[W]** and Origin disagree, Origin decides.

A multi-panel graph is one page with several layers, one per panel. Each panel has its own frame, axes, plots and legend. Panels that share an axis are linked layers, the model of ADR 0009 (section 12): no second link model is needed.

### 15.1 What Origin does

**Pages** [COM]: the plain Line template and the panel templates (PAN2VERT, PAN2HORZ, PAN4, PAN9) have a landscape page of 10.72 x 8.205 in. The STACK template's page is 11.413 x 7.973 in, and the Stack command turns it upright (7.973 x 11.413 in). A merged graph gets a landscape page of 11.41 x 7.973 in.

**Panel templates** [COM]. Layer rectangles in percent of the page (left, top, width, height):

| Template | Layers |
|---|---|
| PAN2VERT (Vertical 2 Panel) | layer 1 is the lower panel, (15.00, 50.01, 75.05, 35.06); layer 2 is the upper one, top 4.81, same size |
| PAN2HORZ (Horizontal 2 Panel) | layer 1 on the left, (15.02, 9.99, 34.98, 75.04); layer 2 on the right, left 59.13, same size |
| PAN4 (4 Panel) | row by row from the top left; lefts 15.02 and 53.08, tops 9.99 and 49.56, each 34.98 x 35.04 |
| PAN9 (9 Panel) | row by row from the top left; lefts 15.02, 40.78, 65.50, tops 9.99, 36.38, 63.70, each 21.70 x 21.71 |

The gaps are uneven, so these rectangles are set by hand in the templates, not computed. In these templates:
- layers 2 and up follow layer 1's position only (unit "% of linked layer"); their scales are not linked;
- every panel shows its own tick labels, titles and legend, with ticks pointing out;
- text is drawn smaller in small panels: by 0.5 in PAN4 and 0.3 in PAN9;
- the selected Y columns are dealt out in turn: column k goes to layer ((k - 1) mod n) + 1, so PAN2VERT with four columns gives the lower layer Y1 and Y3, and PAN4 with six gives layer 1 Y1 and Y5, layer 2 Y2 and Y6, layer 3 Y3 and layer 4 Y4 (the help speaks of splitting the columns into groups; 2016 alternates them);
- layers without a column stay empty, with axes from 0 to 10;
- colours start again in every layer.

**Stack command** (defaults from its definition file, results [COM]):
- portrait page, line plots, one panel per Y column, layer 1 at the bottom; a Y column that follows a second X column keeps that X column;
- margins 15 (left) and 10 (right, top, bottom) percent of the page, no gap; each panel is (100 - 10 - 10 - (n - 1) x gap) / n high and 75 wide;
- layers 2 and up are linked to layer 1 with the X axis linked one to one ("Straight") and Y not linked; the shared X range covers the data of every panel (X from -1 to 1 in one panel and 0 to 2 in another gave -1.5 to 2.5 in both);
- with no gap, only the bottom panel shows X tick labels and the X title; inner panels show neither; the top panel shows tick labels on its top axis but no title;
- with a gap of 1 or more, every panel shows its X tick labels and title (the current help gives a threshold of 2);
- every panel keeps its left Y tick labels, its Y title, a Y line on both sides and its own framed legend at the top left; ticks point in;
- at a joint, the top Y label of the lower panel touches the bottom Y label of the panel above; with a gap, an X title can run into the panel below; Origin prevents neither;
- the 2016 command has no Y link, no single Y title and no common scale option; later versions added them [W].

Check values [COM]: 3 panels gave (14.99, 63.33, 75, 26.68) for layer 1 and tops 36.67 and 9.99 for layers 2 and 3; 2 panels with a gap of 5 gave height 37.5 and tops 52.51 and 9.99.

**Arrange Layers** (`layarrange`): 2016 defaults are gaps of 5 and margins of 15 on every side. Width = (100 - left - right - (cols - 1) x h_gap) / cols and height likewise; layers fill the grid row by row from the top left. Check: 4 layers in 4 x 1 gave tops 15, 33.76, 52.51 and 71.26, height 13.75, width 69.99 [COM].

**Merge Graph** (`merge_graph`): 2016 defaults are gaps 5 and 5 and margins 15 (left), 10 (right), 10 (top) and 15 (bottom). 2 graphs gave 2 x 1, 3 gave 2 x 2 with one empty cell and no layer in it, 4 gave 2 x 2 and 5 gave 3 x 2 (fits rows = ceil(sqrt(n)) and cols = ceil(n / rows) [INF]). The rectangles follow the Arrange Layers formula exactly; merged layers are not linked and text is scaled (0.46 for 2 x 2, 0.36 for 3 x 2) [COM]. Measured in full in 15.7c.

### 15.2 Model

Panels use the fields of section 12 and one new axis field.

| Where | Field | Default | Meaning |
|---|---|---|---|
| Layer | `link` (section 12.2) | `None` | Panels 1 and up of a graph whose panels share an axis are linked to panel 0, with `x` or `y` (or both) `Straight`. |
| Layer | `frame` | | For a linked panel, as for every linked layer, fractions of panel 0's frame (Origin's "% of linked layer"): the panel right above a panel of the same size is `{0, -1, 1, 1}`. |
| Axis | `hide_end_label` | false | The tick label at unit position 1 (the top end of a Y axis, the right end of an X axis) is not drawn. |

- `hide_end_label` has a serde default and is not written while false, so older files load unchanged and other graphs are saved as before. `FORMAT_VERSION` stays 1; an older Sheaf ignores the field and draws every end label.
- A link to panel 0 is valid only while panel 0 is not linked itself (section 12.2); an invalid link draws the panel as an independent layer, with its frame read as page fractions.

**Places and panels** (functions of `Graph`, no stored data):
- `place(i)`: the frame of layer i as fractions of its root's frame: `{0, 0, 1, 1}` (`Frame::WHOLE`) for a root, the layer's own `frame` for a validly linked layer.
- `same_frame(i)`: the layers of i's group with the same place, in layer order (a layer and its right Y layer, but not the other panels of a stack).
- `panels()`: every layer that is the first of its `same_frame` list. A one-layer graph and a double-Y graph have one panel.
- `page_frame(i)`: the frame of layer i in page fractions.
- `right_y_member(i)` (section 12.2) looks only at the layers of `same_frame(i)`, so each panel can have its own right Y axis.

### 15.3 Panel layout

A `PanelLayout` (`sheaf_core::panels`) is an instruction, not stored in the graph. Lengths are in percent of the page: the margins left and right and the horizontal gap of its width, the others of its height.

| Field | Meaning |
|---|---|
| `rows`, `cols` | grid size, each at least 1, at most 64 cells |
| `h_gap`, `v_gap` | gaps between neighbouring columns and rows, 0 to 50 |
| `margin_left`, `margin_right`, `margin_top`, `margin_bottom` | 0 to 90 |
| `order` | `TopDown`: panel k fills the cells row by row from the top left. `BottomUp`: the rows are counted from the bottom, so panel 0 is at the bottom left. |
| `link_x`, `link_y` | panels 1 and up are linked to panel 0 on that axis |
| `page` | `Keep` (the template's page), `Upright` (the template's page turned upright: width and height swapped when it is wider than tall) or `Tiled` (`cols` times the template's width by `rows` times its height) |

Constructors:
- `stack(n)`: Origin's Stack command: n x 1, gaps 0, margins 15, 10, 10, 10 (left, right, top, bottom), `BottomUp`, X linked, `Upright`.
- `grid(rows, cols, frame)`: Sheaf's grid for a template whose layer sits at `frame` on its page. The page is `Tiled`, and each panel sits in its cell where the template puts its layer, so every panel has the room of a single graph of the template and its labels fit as they do there: with the template's margins a (left), b (right), c (top) and d (bottom) in percent, the margins are a / cols, b / cols, c / rows and d / rows, and the gaps (a + b) / cols and (c + d) / rows. One column fills `BottomUp`, as Origin's Vertical 2 Panel; several columns fill `TopDown`, as its 4 Panel and 9 Panel. No links.
- `arrange(rows, cols)`: Origin's Arrange Layers defaults: gaps 5, margins 15 on every side, `TopDown`, no links, `Keep`.

**Frames.** With all values in percent:
- cell width w = (100 - margin_left - margin_right - (cols - 1) x h_gap) / cols;
- cell height h = (100 - margin_top - margin_bottom - (rows - 1) x v_gap) / rows;
- panel k is in column c = k mod cols and row r = k div cols (`TopDown`) or rows - 1 - k div cols (`BottomUp`), rows counted from the top;
- its page frame is left = (margin_left + c (w + h_gap)) / 100, top = (margin_top + r (h + v_gap)) / 100, width = w / 100, height = h / 100.

**Validation**, in this order: a grid size outside the limits ("a panel grid needs at least 1 row and 1 column and at most 64 panels (got R x C)"); a gap that is not a number from 0 to 50; a margin that is not a number from 0 to 90; w or h below 1 ("the margins and gaps leave no room for the panels").

Check values (fractions of the page; the first four agree with Origin's numbers in 15.1 within 0.0002):
- `stack(3)`: width 0.75, height 0.266667, left 0.15; panel 0 at top 0.633333, panel 1 at 0.366667, panel 2 at 0.10.
- `stack(2)` with `v_gap` 5: height 0.375, tops 0.525 and 0.10.
- `arrange(4, 1)`: width 0.70, height 0.1375, tops 0.15, 0.3375, 0.525, 0.7125.
- Merge Graph's defaults as an `arrange(2, 2)` with margins 15, 10, 10, 15: (0.15, 0.10), (0.55, 0.10), (0.15, 0.50), (0.55, 0.50), each 0.35 x 0.35; with 3 rows, height 0.216667 and panel 4 at (0.15, 0.633333).
- `grid(2, 2, frame of the line template)`: margins 8, 3, 3.5, 9, gaps 11 and 12.5; each panel 0.39 x 0.375, panel 1 at (0.58, 0.035).

**Arranging an existing graph** (`Graph::arrange`, Origin's Arrange Layers): the panels of the graph (`panels()`, in layer order) go into the cells in order. A root gets the cell as its frame; a linked panel gets the cell as fractions of its root's new frame, and so do the layers drawn in its frame (its right Y layer). Links, axes and the page are not changed. More panels than cells is an error ("the layout has N cell(s) but the graph has M panels").

### 15.4 Building a panel graph

`Project::plot_panels(template, groups, layout)` takes a template, a list of groups (each a list of Y columns, in order) and a layout:

1. Checks: the layout is valid (15.3); there is at least one group ("there is nothing to put in the panels"); every column exists; the template has one layer ("the 'double-y' template has more than one layer and cannot be used for panels"). Nothing is added to the project when a check fails.
2. Panels: P = min(number of groups, rows x cols). Group g goes to panel g mod (rows x cols), so extra groups are dealt out in turn, as Origin's panel templates do. Cells without a group get no layer, as in Merge Graph. An empty group gives a panel without plots.
3. Page: from the template's page by `layout.page`.
4. Layers: P copies of the template's layer. With `link_x` or `link_y`, panels 1 and up get `link = {parent: 0, x, y}` with `Straight` on the linked axes. The frames follow 15.3, linked panels as fractions of panel 0's frame.
5. Joined rows: when `v_gap` is 0 and `link_x` is on,
   - a panel with another panel directly below it (same column, next row) gets `x_axis.show_tick_labels = false` and `x_axis.show_title = false`, and its X ticks (major and minor) that point `Out` or `Both` point `In`, so that they do not reach into the panel below;
   - a panel with another panel directly above it gets `y_axis.hide_end_label = true`.
6. Joined columns: when `h_gap` is 0 and `link_y` is on,
   - a panel with another panel directly to its left gets `y_axis.show_tick_labels = false` and `y_axis.show_title = false`, and its Y ticks that point `Out` or `Both` point `In`;
   - a panel with another panel directly to its right gets `x_axis.hide_end_label = true`.
7. Plots: the Y columns of each group are added to their panel with `add_plot` (X and error columns from the sheet's designations). Then every layer's palette is applied from its first colour, as Origin starts the colours again in every layer.
8. Everything else comes from the template: the Y axis rules (for example symmetric about zero in the cyclic voltammetry template), ticks, box, legend and fonts.
9. The graph's long name is the long name of the workbook of the first column.

`Project::add_right_y_layer(graph, i)` (section 12.6) gives the panel that layer i is drawn in a right Y axis: the new layer is linked to the panel's root with the X axis `Straight`, its frame is the panel's place, and the panel's legend lists the plots of both layers. A linked panel whose X axis is not linked to its root cannot get one ("this panel does not share the X axis of the first panel, so it cannot get a right Y axis"), because links do not chain.

### 15.5 Drawing

Panels use the group drawing of section 12.3 with these additions:

1. **Shared axes.** A linked panel's axis on a `Straight` link is its root's resolved axis, whose automatic range covers the data of every panel linked on that axis (section 12.3, step 2). Its own fonts, tick directions and lengths, visibility flags, `hide_end_label` and title still apply.
2. **Keeping labels on the page.** The group's label edges are measured as in section 12.3, step 4, and the shifts are applied to the rectangle that holds the nominal frames of all members (for a stack, the whole stack). Every member's frame is then moved and scaled with that rectangle, so the panels stay aligned and touching, and the largest label of any panel decides for all. A group whose members all lie inside the root's frame (every double-Y graph) gives exactly the frames of section 12.3, so existing figures are unchanged.
3. **End labels.** An axis with `hide_end_label` skips the tick label whose tick has unit position at least 1 - 1e-9, also in the label measurement of rule 2.
4. **One title for touching panels.** Within a linked group, the panels whose axis (X or Y) is on its normal side and shows its title are put in order along that axis. A run of panels that touch along the axis (for Y titles: the same left side and width, each one's bottom equal to the next one's top, within 1e-6 relative) and whose title texts are equal draws the title once: by the run's panel with the lowest layer index, centred on the whole run, beyond the outermost tick label edge of the run, in that panel's title font and colour. The other panels of the run draw no title. So `--y-title "Intensity (a.u.)"` on a stack gives one Y title for the stack. Panels with different titles keep their own, as in Origin.
5. **Legend margins** (section 12.5) count the inward ticks of the layers in the legend's frame only (`same_frame`), and a legend with scope `Graph` lists the plots of the layers in its frame only: a layer and its right Y layer, not the other panels.
6. Layers are drawn in layer order, each as before. A graph without panels, and every existing golden image, draws byte for byte as before.

Check values (fixed text metrics, golden images `cv-stack.svg` and `grid-2x2.svg` in `crates/plot/tests/golden/`):
- A two-panel Line stack with X from -1 to 1 in panel 0 and 0 to 2 in panel 1 gives both panels the X axis -1..2 with major ticks -1, 0, 1, 2; with the X link set to `None` the panels get -1..1 and 0..2.
- A two-panel cyclic voltammetry stack with currents reaching ±0.9 in both panels gives each panel the Y axis -1.2..1.2 with step 0.4; the lower panel draws six Y tick labels (no 1.2), the upper panel seven, and the upper panel draws no X tick labels and no X title.

### 15.6 Command line (`sheaf plot`)

| Option | Values | Default | Meaning |
|---|---|---|---|
| `--panels` | `stack`, or `RxC` such as `2x1`, `1x2`, `2x2` | off | make a panel graph: `stack(n)` with n = the number of groups, or `grid(R, C, frame of the template)` |
| `--panel-y` | comma-separated column names; repeat the option once per panel | | the Y columns of each panel, in panel order |
| `--per-panel` | `N,N,...`, each 1 or more | | how many consecutive curves go into each panel |
| `--panel-gap` | `PCT` (both) or `H,V`, each 0 to 50 | from the layout | gaps in percent of the page |
| `--panel-margins` | `L,R,T,B`, each 0 to 90 | from the layout | margins in percent of the page |
| `--panel-order` | `bottom-up`, `top-down` | from the layout | where the first panel goes |
| `--panel-link` | `x`, `y`, `xy`, `none` | stack `x`, grid `none` | the axes the panels share |

How the groups are made:
- One input (a data file, or a project with `--book` and `--sheet`): each plotted Y column (`-y`, or the default columns) is a group; with `--panel-y`, each occurrence is a group of the named columns.
- Several data files: each file is a group (its `-y` or default columns), and the legend names the file (plus the column when the file gives several curves), as for an overlay; with `--panel-y`, each group takes the named columns of every file, in file order.
- `--per-panel` then cuts the list of all curves, in order, into panels of the given sizes; the counts must add up to the number of curves ("--per-panel counts add up to S but there are N curves").

Rules and messages:
- The `--panel-*` options and `--per-panel` need `--panels`; `--panels` cannot be combined with `--graph` or `--y2`; `--panel-y` cannot be combined with `-y` or `--per-panel` (clap's messages).
- `--panels` must be "stack" or `RxC` ("--panels must be 'stack' or ROWSxCOLUMNS such as 2x2 (got 'X')"); R x C must be 1 to 64 ("--panels 0x2: rows x columns must be from 1 to 64 panels (got 0 x 2)"); a stack holds at most 64 panels.
- `--panel-gap` and `--panel-margins` give "--panel-gap must be from 0 to 50 percent of the page (got 60)" and "--panel-margins takes four numbers, L,R,T,B (got 2 numbers)".
- The double-y and waterfall templates are refused ("the double-y template cannot be used with --panels; choose another template").
- Style options act on every panel: `--x-*`, `--y-*`, `--x-labels`, `--x-label-angle`, `--stack`, `--no-y-tick-labels` and `--no-y-ticks` set each panel (a linked X axis follows panel 0's); `--colors`, `--labels` and `--line-width` count the plots across the graph in layer order, so `--colors black,red,blue` colours a three-panel stack panel by panel; `--no-legend`, `--axis-colors`, `--bar-gap` and `--bar-overlap` act on every layer. `--offsets` and `--waterfall` are refused on a graph of several panels ("--offsets and --waterfall work on a graph of one panel; this graph has 3 panels").
- `sheaf export` keeps its rule of section 12.6: the options act on layer 0's group. For a saved stack that group is every panel; for a saved grid without links it is panel 0 only.

Examples:

```
sheaf plot before.txt after.txt -t cv --panels stack -o cv-stack.png
sheaf plot xrd.csv -t stacked --panels stack --panel-order top-down --y-title "Intensity (a.u.)" --no-legend -o xrd.pdf
sheaf plot cycling.csv --panels 2x1 --panel-y "Q LMO,Q Al-LMO" --panel-y "CE LMO,CE Al-LMO" --panel-gap 0 --panel-link x -o cycling.png
sheaf plot xrd.csv --panels 2x2 -o grid.svg
```

### 15.7 Desktop app (later, with ADR 0007)

- Plot > Multi-Panel with Origin's names: Vertical 2 Panel (`grid(2, 1, ...)`), Horizontal 2 Panel (`grid(1, 2, ...)`), 4 Panel (`grid(2, 2, ...)`), 9 Panel (`grid(3, 3, ...)`) and Stack... (a dialog with the options of 15.6), each through `Project::plot_panels`.
- Graph > Arrange Layers... calls `Graph::arrange` with `PanelLayout::arrange` as the starting values.
- A layer selector in the graph settings; a change to the scale of a linked axis is written to its root. A double-click picks the panel under the pointer (`layout_geometry` gives each panel's frame).

### 15.7a Stack dialog (2026-11-14)

Origin 2016's `plotstack -d;` opens **Stack: plotstack**. Its measured defaults are Line,
Portrait, Vertical, Bottom to Top, Legend for Each Layer, Link Layers checked and X Axis Link
Straight (1 to 1). Plot Type offers Line, Scatter, Line+Symbol, Column, Use Template and Custom;
this installed version has no Histogram entry. Plot Assignment contains an automatic Number of
Layers and Number of Plots in Each Layer. Spacing contains Vertical Gap and four margins. The
control dumps are in `re-workspace/scratch/t01-stack-panels/dialog/`; the captured screen pictures
were obscured by another window and are not visual evidence.

Headless Origin measurements on five Y columns sharing one X (`assignment4.lines.txt` and
`assignment4.out.txt` in the same scratch folder) used `plotstack`'s `number` argument and counted
plots with `layer -c;`. The final requested count repeats and the final layer clips to the remaining
columns: `1` gives `1,1,1,1,1`, `2` gives `2,2,1`, and `1 2` gives `1,2,2`. Earlier diagnostic logs
used unsupported count properties; only the `assignment4` counts are fixtures.

The desktop **Plot > Multi-Panel > Stack...** dialog now calls the existing `plot_panels` builder.
It starts from the measured defaults and section 15.2's stack geometry (margins 15/10/10/10,
zero gap). Users can choose the four built-in plot types, layer order, portrait/landscape page,
per-layer/no legend, automatic layer assignment, page-percent vertical gap and margins. Every
layer starts its own palette. Link Layers with Straight shares X scales; with None it links only
positions; unchecked produces independent layers. Selected worksheet row bounds are retained.
Units display the existing model's linked-layer/page convention; spacing always uses page percent.
Layer legends are framed at the top left and ticks point inward. Existing template typography,
page size and section 15.9's deliberate layout differences remain.

OK creates one graph in one undo step; opening, Cancel and Escape do not edit the project.
Positive whole assignment counts, the 64-layer bound, current numeric source columns and usable
geometry are validated before OK. Horizontal stacks, external/custom templates, a whole-page
legend, manual layer count, other units and live preview remain disabled. Multiple Panels by Label
is still disabled. UI regressions cover defaults, source order, grouping, Scatter, legends, layer
order, independent axes, row bounds, orientation, spacing, invalid values, Cancel/Escape and undo/redo;
parser regressions use the three measured assignment cases.

### 15.7b Multiple Panels by Label (2026-11-14)

Origin 2016's `plotbylabel -d;` shows **Multiple Panels by Label: plotbylabel**, Group Identifier
(Long Name by default, Units, Comments), Plot Type (Line by default, Scatter, Line+symbol, Column),
Number of Rows and Columns (both Auto), Spacing, Preview and Auto Preview. This version has no graph
template or X-column/group-count selector. The controls and synthetic probes are in
`re-workspace/scratch/t01-label-panels/`. The current
[Origin reference](https://docs.originlab.com/x-function/ref/plotbylabel/) documents user header
identifiers and dates external template selection to 2021b; those newer controls are not inferred
from this installed dialog.

Measured with X in A and Y in B:F (`grouping.lines.txt` / `grouping.out.txt`): Long Names
`Beta, Alpha, Beta, empty, alpha` produce three groups, in first-appearance order: B/D, C/F, E.
The corresponding Y titles are `Beta`, `Alpha`, empty. Units `mA,mA,uA,uA,uA` produce B/C and
D/E/F. Five empty Comments produce one five-plot layer. Thus group matching ignores ASCII case,
keeps source order within each group and retains the first label's spelling. A second probe
(`grouping2`) shows `Beta`, ` beta ` and `beta` combine, and empty and one-space labels combine.
Only ASCII case and these whitespace cases were measured; Unicode case folding is not implemented.

The page stays landscape. Origin's automatic grids for 1/2/3/5 groups are 1x1, 2x1, 2x2, 3x2
(rows x columns), filling row by row from the top left, with no layers in unused cells. Defaults
are gaps 5/5 and margins left/right/top/bottom 12/10/10/12 percent. Three groups give frames at
(12,9.992), (53.5,9.992), (12,51.51), width 36.51 and height 36.5 percent. Five groups give a
3x2 grid with height 22.66 percent. Explicit rows=2, cols=3, gaps=8/7 give width 20.66 and height
35.49 percent. These agree with `PanelLayout::arrange` after applying the measured margins/gaps
within 0.02 percentage points (Origin's page-unit rounding).

The desktop dialog now reads selected columns' Long Name, Units, full Comments or existing named
parameters, trims surrounding whitespace and groups with ASCII-insensitive matching. It uses the
existing `plot_panels` builder with four built-in types, source row bounds and group identifiers as
Y titles (including an empty title for an empty group). Parameter names are listed alphabetically
from the existing model; their grouping uses the same rule, but was not measured separately in
Origin. Parameters, formulas and worksheet data are never edited by this dialog. Axis scales are
independent. The automatic grid has ceil(sqrt(groups)) rows and enough columns; either dimension
can be made manual, or both, provided all groups fit in at most 64 cells. Gap/margin validation
uses the existing panel model. Too many groups are refused before building an unbounded list.

Opening, Cancel and Escape do not edit the document; OK creates one graph in one undo step and
focuses it. Missing/nonnumeric sources or an unusable grid disable OK. Sheaf retains its built-in
template's page size, typefaces and legends instead of Origin's MGROUPS template. Live preview is
disabled. UI tests cover the measured grouping/frames, source immutability, parameters/full
comments, selected rows, Column/Scatter, manual dimensions/spacing, invalid geometry, Cancel/Escape
and one-step Undo/Redo; grid regression cases include the measured 1/2/3/5-group arrangements.

### 15.7c Merge Graph Windows (T60, measured 2026-11-15)

Graph > Merge Graph Windows copies finished graphs into the cells of one new graph. Measured in
Origin 2016 through `merge_graph` (its interface names and defaults, then 40 scripted merges of stock
Line graphs and its dialog; probes and outputs in the ignored `re-workspace/scratch/t60/`):

- **Page:** 11.41 x 7.973 in (6846 x 4784 dots at 600 dpi), landscape whatever the sources' pages;
  Portrait turns it; Width/Height/Unit set another size (a 30 x 20 in page kept every percentage
  and factor).
- **Cells:** the Arrange Layers formula (15.1) with gaps 5 and 5 and margins 15 (left), 10 (right),
  10 (top), 15 (bottom), filled row by row from the top left. 3 x 1 gave (15, 9.992, 75.01, 21.68),
  tops 36.66 and 63.34; 1 x 3 lefts 15, 41.67, 68.33 (21.66 x 75); ygap 0 heights 25. Rows and
  columns default to `ceil(sqrt(n))` x `ceil(n / rows)` (2: 2 x 1, 3 and 4: 2 x 2, 5: 3 x 2); an
  empty cell gets no layer ("Add Extra Layer(s) for Grid" off).
- **Layers:** a source's layers stay together in its cell; a double-Y source kept its right Y
  layer linked to its left one (link index remapped, `x.link` 1, frame 100 % of the parent).
  Merged layers are not linked. The sources are kept ("Keep Source Graphs" on); off deletes them.
  "Rearrange Layout" off leaves every layer at its source frame (they overlap) with factor 1.
- **Link Source Graphs:** layers 2 and up get `layer.link` 1 in "% of Linked Layer" units (layer 2
  of a 3 x 1 at (0, 123, 100, 100)) with `x.link` and `y.link` 0: position only.
- **Show Axes Frame** (with a gap of 0): the panels with a panel below them get no X tick labels
  (`x.showlabels` 0), no ticks (`x.ticks` 0) and no X title (`xb.show` 0); the bottom panel keeps
  all three. In a row (xgap 0) the panels with one on their left lose their Y title, tick labels and
  ticks. The dialog notes "Overlapping axes/ticks are hidden".
- **Keep Layer Aspect Ratio:** the layer keeps its width-to-height ratio in percent of each page,
  as large as fits, at the top left of its cell (3 x 1: 20.57 x 21.68 for a 68.16 x 71.83 source).
- **Text factor** (`layer.factor`; Scale Mode Auto and "Scale with Layer Frame" gave the same in
  every case; Fixed Factor sets it): 1 when the cell is at least 68.16 % wide or 71.83 % tall (the
  stock template layer's size), otherwise `sqrt(w h) / 75.6` (w, h the cell in percent), shown to two
  decimals: 2 x 2 0.46, 3 x 2 and 2 x 3 0.36, 3 x 3 0.29, 4 x 4 0.20, no gaps and margins 0.66,
  margins 30 0.23, a 65 x 21.68 cell 0.50 but 70 x 21.68 gives 1, 21.66 x 70 gives 0.52 but 21.66
  x 74 gives 1. Neither the page nor the source's own layer size (30 x 30, 50 x 50, 80 x 40 %)
  changed it. 75.6 is fitted (between 75.4 and 75.9); Origin's own formula is not known.
- **Dialog** (our description): Merge (Active Page, All in Active Folder, ... (Open) by default,
  All in Project, Specified), Graphs list, Keep Source Graphs, Rearrange Layout, then collapsible
  Arrange Settings (rows, columns, Add Extra Layer(s) for Grid, Keep Layer Aspect Ratio, Treat Each
  Source Graph as a Unit (on), Link Source Graphs, Set Layer Width/Height by Common Scale, Show
  Axes Frame), Spacing (in % of Page Dimension), Page Setup (Orientation, Original Size, Width,
  Height, Unit), Scale Elements (Scale Mode, Fixed Factor), Add Label (None, A, a, Custom; Top
  Left/Right Outside/Inside), a preview, OK and Cancel.

**Sheaf** (`sheaf_core::merge`): `MergeOptions` holds these settings with Origin's defaults and
`Project::merge_graphs` makes the graph (fresh name `Graph<n>`, first source's page style, the
measured page and cells; `auto_text_factor` multiplies the fonts of axis titles, tick labels,
legends and text labels). A source whose layers stand side by side on its page is fitted into its
cell by the box round its unlinked layers (Sheaf's choice: Origin's placement of several unlinked
layers of one source was not measured); validly linked layers keep their frames, so double-Y and
stacked sources stay intact. With Link Source Graphs, an unlinked layer that other layers follow (the
left layer of a double-Y source) stays unlinked, because a linked layer cannot be a parent. Sheaf
adds **Share X** (`share_x`, `--share-x`): the linked layers also follow the first one's X scale one
to one, the vertical shared-X figure of the brief. Linking needs Rearrange Layout (the dialog greys
it with Arrange Settings). A typed page size is used as given. The text factor also scales the
plots' data labels (unmeasured) and a Fixed Factor applies without rearranging (unmeasured; Auto then
gives 1 as measured). The app's Merge choices include the project-wide list, a Specified
checklist with Up and Down, and the source choices below. All in Project is the
Sheaf default for projects with multiple graphs. Duplicate, missing or non-graph
sources, too few sources or cells, a factor outside 0.05 to 20 and a page side
outside 0.5 to 200 in are refused without changing the project.
Add Extra Layer(s) for Grid, Set Layer Width/Height by Common Scale and turning
off Treat Each Source Graph as a Unit remain open.

**Panel labels** (T60, Origin 2016 measured 2026-11-22). Three source graphs
arranged in a 2 × 2 grid received one editable text object per source on its
first layer. This held with both a double-Y source and a source with two
separated unlinked layers. The fourth, empty cell had no label.
The letter choices produced A/B/C and a/b/c in source order. Custom
`Graph##` produced Graph01/Graph02/Graph03 and `Panel $(i)` produced
Panel 1/Panel 2/Panel 3. Plain custom text was repeated on each source.
The four corner positions put the label on the chosen side of both the
horizontal and vertical edges of the first layer with about 2% of its extent as
padding in each direction. The label used a nominal 30 pt font, scaled with
the layer. Labels were also created with Rearrange Layout off.
[Origin merge command](https://docs.originlab.com/x-function/ref/merge_graph/) ·
[Merge Graph dialog](https://docs.originlab.com/origin-help/graphref-mergegraph/)

Sheaf exposes these controls in Add Label. None is the default. A and a use
alphabetical labels and extend past Z/z with AA/aa. Custom accepts literal
text and the measured numbering forms, with a run of hashes specifying the
padding width. The dialog measures the text in the figure's default font.
The merge engine creates regular graph labels at the selected corner using
Sheaf's existing text scale for the source layers. The
draft uses the same settings for Preview and OK, and Undo removes the
merged graph with its labels. Further Origin custom substitution codes
remain open.

**Folder source choices** (Origin 2016, T60 follow-up measured 2026-11-16): fresh
standard Line graphs were placed using public Project Explorer commands. RootZ/RootA remain
at the root, LocalZ/LocalA are direct members of Scope, ChildZ is in Scope/Zulu, OtherZ is in
Outside, ChildA is in Scope/Alpha and DeepA is in Scope/Alpha/Deep. Graphs were created in that
order; Zulu was created before Alpha, and LocalA was active for the Scope controls.
`merge_graph option:=folder keep:=1` selected LocalZ then LocalA; `recursive` selected LocalZ,
LocalA, ChildA, DeepA, ChildZ; root `folder` selected RootZ then RootA. Every source was
validated as a one-layer graph and its folder path read back independently. Layer-by-layer
plot dataset names establish membership/order; checked 800-pixel PNG exports establish two,
five and two panels. The earlier three-member recursive control agrees. These direct-graph controls agree with creation order, and recursive traversal visits named
child folders depth first;
the measured ASCII folder names sort Alpha before Zulu despite folder creation order.

The project-wide Native control selected root graphs, Outside, then Scope and its descendants.
Sheaf keeps its existing All in Project/manual-list order; changing that separate contract is
outside this source-filter slice. Current documentation describes the folder/recursive modes:
[merge_graph](https://docs.originlab.com/x-function/ref/merge_graph/).
Accepted command/readback/image hashes and owned-instance cleanup are in ignored scratch
`t60-merge-folder-choices/native-proof.json`. Two initial shared-workbook probes used an
unusable short-name reference and are excluded before any merge result. The accepted runs
used independently validated active-workbook plots; all owned instances/guards closed,
with foreign process identities preserved. Additional reverse-arrival controls could not start
because COM reported Class Not Registered in both client architectures; no registration was
changed. Native order after reversing folder arrival remains unestablished.

Sheaf captures the Explorer folder when the dialog opens through either the menu or `--show`.
Folder lists exclude workbook windows, respect implicit root membership, and show graphs in
the order that will be merged. Direct graphs use existing Project creation order, matching the
accepted fixture. Recursive child-folder sorting uses case-folded names (Unicode
and equal-name tie behavior are Sheaf policy; Native controls covered distinct ASCII names).
Changing only the Explorer selection keeps the captured source folder. Any folder-tree edit
requires reopening before a folder-mode merge, preventing index paths from silently targeting
another folder. All in Project/Specified keep their existing live graph-list behavior, ticks
and manual ordering. Empty/one-graph choices are refused without history; source removal,
multilayer grouping, layout, Undo and native/companion saves reuse the existing core merge.
New-graph destination placement is outside these source choices.

**Active Page, Open and Include Embedded** (T60, measured 2026-11-21).
On Origin 2016, the scripted `merge_graph` Active Page mode made a new graph
from the active window, with one or two layers in the measured examples.
Scripted Open selected the active folder's two visible single-layer windows
and two visible multilayer windows, in creation order, for six layers total.
It excluded a minimized window, a hidden window and the graph in a child
folder. This was a command probe. The current guide describes a one-frame
preference for the dialog's initial list. An actual cell embedding was verified
with `page.isEmbedded` (1). On that fixture, direct-folder mode returned eight
layers. Include Embedded returned nine, with the embedded graph first.
[Origin merge command](https://docs.originlab.com/x-function/ref/merge_graph/) ·
[Merge Graph dialog](https://docs.originlab.com/origin-help/graphref-mergegraph/) ·
[Insert Graph](https://docs.originlab.com/x-function/ref/insertgraph/)

In Sheaf, Active Page uses the graph active when the dialog opens and is the
initial choice for a project with only that graph. Open uses the captured
folder and the current workspace visibility, excluding minimized windows.
Include Embedded starts with graphs referenced by graph sheets and by cells
of workbooks in that folder. It follows workbook order, sheet order and the
stored order of a sheet's cell references, then appends the folder's ordinary
graphs in creation order. Sheaf stores these worksheet items as links to
standalone graph windows, so it resolves every link to its live graph and
includes each window at most once. The three new choices can merge a single
graph. The original choices and CLI still require two. Preview and OK use
the same selection rules, and Keep Source Graphs applies to the selected
graph windows, including the ones found through worksheet references.

The Merge dialog's vertical scroll viewport fills the available dialog width. Short source
choices no longer shrink its clipping rectangle around the narrow top controls, so longer
option labels remain readable. The existing height limit, source choices and footer remain.
This is a Sheaf layout correction, checked against desktop captures, not a new Origin measurement.

**Merge preview** (T60 follow-up, 2026-11-16): Origin's current
[FAQ139](https://docs.originlab.com/quick-help/mergegraphs/) documents a Preview button and
an Auto Preview checkbox, and notes that previewing many graphs can be slow. Its
[multilayer tutorial](https://docs.originlab.com/tutorials/multilayer-graph-customization/)
shows an approximation of the merged page to the right of the settings. These are documentation
contracts; no fresh Origin 2016 timing, default checkbox state or exact geometry was measured
because COM activation is currently unavailable. No registry or installation changes were made.

Sheaf shows the full draft page to the right of the scrollable settings. **Preview** renders
on demand; **Auto Preview** updates after a draft or source change. Auto starts off (Sheaf policy),
avoiding an initial expensive render on large projects. Both use the exact same `MergeOptions`
and existing core operation as OK, applied to a disposable project clone. The normal screen/export
renderer draws the resulting plots, axes, linked layers, labels and page margins; source-removal
options affect only that clone. Preview does not allocate IDs in the real project or change its
revision, dirty flag, Undo or Redo. OK retains the existing one-step edit; Cancel/Escape discard
the preview.

The cached image is invalidated by any option, document revision/generation, folder validity or
pixel-density change. With Auto off, a changed draft shows the Preview prompt instead of an
outdated image. Invalid merges show the core validation error instead of the previous image;
folder-tree changes require reopening before any preview computation. The whole page is fitted
inside a 640 by 480 pixel raster at normal density (up to 1280 by 960 at high density), preserving
its aspect ratio. The right-hand image is fitted to the available width and at most 240 screen
points high. These bounds, responsive geometry, invalidation and error rules are Sheaf policies,
not measured Origin behavior. No model, renderer, file-format, CLI or MCP behavior changes.

**Command line:** `sheaf merge-graphs PROJECT (--graph NAME ... | --all-graphs) [--rows N]
[--cols N] [--gap H,V] [--margins L,R,T,B] [--portrait] [--page-size W,H] [--keep-aspect]
[--link] [--share-x] [--hide-inner] [--text-scale auto|keep|FACTOR] [--remove-sources] [-o FIGURE]
[--dpi N] [--save PROJECT]`; at least one of `-o` and `--save`.

### 15.7d Layer Management and fitting the page (T60, measured 2026-11-15)

**Origin's Layer Management window** (opened by `laymanage`; our description): a "Layer Selection"
grid (#, Name, Link to) on the left; tabs Add, Arrange, Size/Position, Link, Axes and Display; a
preview on the right; the note "Use the Apply buttons on each tab to update your preview graph. Use
the OK button to apply the changes in preview graph to source graph."; Undo, OK and Cancel.
- Arrange: Arrange Order (Layer Index Order, Current Position Order), Number of Rows and Columns
  (1, 1), Add Extra Layer(s) for Grid (on), Keep Layer Aspect Ratio, Treat Linked Layers as a
  group, Link Layers, Set Layer Width/Height by Common Scale, Show Axes Frame; spacing gaps 5 and
  5, margins 15, 10, 10, 15 (the merge values, not `layarrange`'s 15 on every side).
- Size/Position: Option Resize, Move, Swap, Align. Resize and Move take Units (% of Page, inch, cm,
  mm, pixel, point, % of Reference Layer (default)), a Reference Layer and Width/Height (100, 100)
  or Left/Top (0, 0); Swap takes two layers and a Reorder tick; Align a Reference Layer and a
  Direction (Left, Top, Right, Bottom).
- Link: Link To (None or a layer), X and Y Axis Link (None, Straight(1 to 1) default for X of a
  linked layer, Align, Custom with X1/X2 or Y1/Y2), Units (% of Linked Layer default) and "When
  dragged to reposition, also reposition parent layer".
- Axes (scales; per side Axis, Tick In & Out/In/Out/None, Title, Tick Label, Formula) and Display
  (Drawing Order; Color, Border Dimensions (% of Layer: 16.6, 10, 12.4, 15.2), Scale Elements:
  Scale with Layer Frame or Fixed Factor).

**Sheaf's Layer Management** (`sheaf_core::layer_manage`, `dialogs/layer_management.rs`; Graph menu,
page right-click and layer icon menu, the icon's layer selected): the layer list allows several
layers; Arrange, Size/Position and Link are built and work on a copy shown as numbered frames on a
page-shaped preview; Undo takes back the last Apply; OK writes the copy in one undo step; Cancel and
Escape discard it. Display > Color > Background Color is implemented as described below. Add (T61)
and Axes are shown disabled, as are Add Extra Layer(s) for Grid
(Sheaf leaves surplus cells empty), the Common Scale ticks, the Align/Custom links, the Units combo of
Link and the reposition-parent tick. Arrange with no selection takes every layer; "Treat Linked
Layers as a group" places a layer with the layers linked to it as one unit; Link Layers links the
later units to the first in position only. Sheaf choices (not measured): Move in "% of Reference
Layer" counts from the reference layer's top left corner, a pixel is 1/96 in, the reference layer
starts at layer 1, and a reference layer that follows a selected layer is refused (it would move
with it). The Link tab shows the first selected layer's link; Swap with Reorder refuses an order that
puts a parent after its child.

**Display > Color > Background Color** (T60 follow-up, measured in Origin 2016 on
2026-11-16): the stock layer starts at None (`layer.color=0`). The palette has None,
Classic colours and custom colours. Two independent layers initially green (3) and blue (4)
were used to check scope. Selecting layer 1, choosing Red and Apply, then Cancel left the
source at (3, 4); repeating with OK gave (2, 4), changing only layer 1. Selecting both layers
showed a mixed-value marker. Explicitly choosing None, Apply and OK gave (0, 0).
The preview is separate from the source until OK. Seven isolated native runs completed;
accepted settled captures, control lists and readbacks are hashed in ignored scratch
`t60-layer-display/native-proof.json`. Earlier delayed-paint captures and a misdirected
palette click are excluded as colour-change evidence.

Sheaf now exposes this background control for one or more selected layers, using the existing
`Layer.background`, export and persistence paths. None clears that field. The eight shared
basic presets and editable opaque RGB bytes are the current Sheaf palette; the full native
palette layout is not copied. Mixed layers keep their individual colours until an explicit
choice; switching the selection reloads its draft. Apply changes only the working preview,
Undo restores the previous preview, OK makes one document history entry, and Cancel/Escape
leave the source alone. No selection, an unchanged colour and a final preview equal to the
source add no history. OK refuses to overwrite layer edits made behind the window. Frames,
links, axes, plots, legends, gradient settings and border settings are preserved. The small
numbered-frame preview shows base colours; it does not render gradients or full plots.
Native `.sheaf` saves and `.opj` companions retain the colour; bare `.opj` still omits it with
the existing loss warning. Drawing Order, Border Fill Color, Border Color, Border Dimensions
and Scale Elements remain disabled in this dialog. Native mixed-value interaction and the
existing Sheaf layer drawing order are not changed by this slice. Supporting current docs:
[Display tab](https://docs.originlab.com/origin-help/laymanage-displaytab/) and
[Layer properties](https://docs.originlab.com/labtalk/ref/layer-obj/).

**Display > Color > Border Fill Color and Border Color** (T60 follow-up, Origin 2016,
2026-11-16): these use a rectangle outside the plot frame. A fresh standard Origin-template
three-point Line graph starts with Background Color, Border Fill Color and Border Color at
None. Its frame is left 17.91, top 11.58, width 68.16, height 71.83 percent of the page.
Border Dimensions shows left 16.6, top 10, right 12.4, bottom 15.2 percent of the layer.
These are measured fixture values, not defaults established for every template.

Separate fresh graphs were used for a red outline and a red fill, chosen through the settled
Display palette, Apply and OK. Both checked PNG exports are 800 x 612 pixels. In each,
the red rectangle's inclusive bounds are (53, 27) to (756, 577), matching the rounded bounds
from expanding the layer frame by those four percentages of its width/height. The plot frame
itself is approximately (143, 71) to (689, 510). The outline surrounds the plot and its margins;
the fill colors both the plot interior and the expanded margins. In the fill export, pixels
(80, 300), (400, 300) and (400, 50) are red while (10, 10) stays white; those four samples
are white in the outline control. `layer.color` stays 0 (None), `layer.border` stays 0 and
the frame percentages do not change. `layer.border` is the separate legacy ornament setting
(off/shadow/marble/whiteout/blackout); it is not either of these colour controls.

Accepted settled control captures, property readbacks, exports and hashes are recorded in
ignored scratch `t60-layer-border-display/native-border-proof.json`. Export calls returning
False and the interrupted first fill run are excluded; the accepted replacement fill completed
with its own Origin instance and helpers closed. Other sessions' instances were preserved.
These controls establish the geometry and independence from Background Color; multi-layer
mixed/selection behavior, overlap drawing order and changes to Border Dimensions remain open.

**Backend prerequisite:** Sheaf's existing `Layer.background` fills the plot frame, and
`BackgroundStyle.border` strokes that same frame (`plot/fill.rs`, called from `plot/layout.rs`).
There is no separately stored border fill or four border outsets. Reusing those fields would
change the wrong rectangle or the independent background colour. Keep both Display colour
controls disabled until a separately represented layer-relative rectangle, independent fill
and outline, rendering and persistence are implemented with preview/history/roundtrip tests.
Preserve the existing plot-frame border semantics and the published Background Color control.
This measurement slice changes no Rust code, model, renderer, file format or enabled inventory.

**Fit Page to Layers** (`pfit2l`; Margin Control Border (default) or Tight, Border Width 5, Adjust
Direction Height and Width (default), Height Only, Width Only), measured on the stock page: the
layers keep their size and distance from each other in dots; the page becomes the box round the
content plus, with Border, the border width in percent of the **old** page on each side (5: 321 of
6432 and 246 of 4923 dots; 10 twice that); Tight adds about 13 dots. Height Only keeps the page width
and every left; Width Only the reverse. The content box is the layer frames and the page's
objects, **not** the tick labels or axis titles: with a border of 0 the layer's left edge became the
page's (its Y tick labels and title fall off the page) while the box reached 370 dots above the
frame, where the stock legend stands. A graph with data and an empty one gave the same numbers.

**Fit Layer to Graph** did nothing measurable in Origin 2016: on a one-layer graph (layer moved to
30, 20, 40, 40 percent, or 130 x 120 at -10, -10) and a two-layer graph, the page size, every layer
position and the picture were the same before and after, from the menu and by its accelerator, with
no dialog. What it acts on (a selected object, perhaps) is unknown, so Sheaf leaves it grey (tag `N`).

**Sheaf's Fit Page to Layers** (`Graph::fit_page_to_content`, `sheaf_plot::fit_content_bounds`): Graph
> Fit Page to Layers... opens Margin Control (Border, Tight), Border Width (5) and Adjust Direction
(Height and Width, Height Only, Width Only); Format > Fit Page To Layers... opens the same window, as
Origin's Format item does (measured 2026-11-15, T70). The content box is the layer
frames and the boxes of legends, text labels, drawn shapes and slope annotations; Tight leaves
1.56 pt (13 dots at 600 dpi). One undo step.

**Fit Page To Printer** (Format menu, T70, measured 2026-11-15): acts at once; the page becomes the
default printer's printable area for the Page Setup paper in the page's orientation (the stock 10.72 x
8.205 in page became 10.667 x 8.0 in on landscape Letter: 1/6 in less on the left and right, 1/4 in on
top and bottom); the layers keep their frames in percent of the page. Sheaf asks the default printer
(Windows) and otherwise uses the paper less those measured edges (12 pt, 18 pt). One undo step.

**Change X, Change Y, Ungroup** (plot right-click menu, T73, measured 2026-11-15): Change X and Change Y
list the plot's sheet's columns (`[Book1]Sheet1!A(X)`, ...) in column order, the current one ticked and
the one the plot uses on the other axis greyed; choosing one replaces that column, the legend and axis
title follow, and the axis keeps the range it was drawn with (Sheaf fixes an automatic range at its
drawn values). On a plot of a group Change X/Y are greyed and Ungroup ends the group with every plot
keeping its drawn colour (Sheaf makes those colours explicit); on a plot in no group Ungroup is greyed.
One undo step each.

**Axis Center** (axis right-click menu, after Axis Zoom In/Out; T70, measured): the axis keeps its span
and its range moves so that the value under the right-click becomes the middle (X 0..11 at 5.42 gave
-0.08..10.92); only that axis changes and its range becomes fixed. Sheaf works in the axis' own scale
(decades on a log axis) and keeps an axis reversed by its limits reversed (Axis Zoom too).

### 15.8 Origin project files

The `.opj` reader reads the layers of a panel graph as independent layers with the limits stored in the file, so the owner's panel graphs already draw at Origin's ranges without links (only the second layer of a double-Y graph is linked, by the rule of section 12.7). Where Origin stores `layer.link`, `x.link` and `y.link` was not found in the layer header (section 12.7 lists bytes that differ for the linked layer of a double-Y graph); reading panel links, and writing panel graphs, needs that first.

### 15.9 Deliberate differences from Origin

- Text keeps its size. Grids get a larger page (the template's page per cell) instead of Origin's text scale factor, and their gaps are the room the template leaves around its frame, not Merge Graph's 5 percent, which is too narrow for full-size labels.
- At a joint without a gap, the lower panel leaves out its top Y label (and a panel with a neighbour on the right its right X label); Origin lets the labels touch.
- At such a joint, X (or Y) ticks that would point into the neighbouring panel point into their own panel instead.
- Touching panels of one group with the same title draw it once, centred on them. Origin has no single title option.
- The top panel of a stack has no tick labels on its top axis; Sheaf cannot draw top or right tick labels on a normal axis yet.
- Empty grid cells get no layer, as in Merge Graph; Origin's panel templates keep empty layers.
- Grids with one column fill from the bottom (as PAN2VERT), grids with several columns from the top left (as PAN4 and PAN9); the order can be changed with `--panel-order`.
- The shared X range always covers every linked panel's data (Origin: the layer rescaled last decides, section 12.8).

### 15.10 Unknowns

- Whether the shared X range of Origin's Stack is a property of the link or set once by the command; it matters after the data change.
- The `.opj` bytes of layer links (15.8).
- Owner choices: the page size of stacks with many panels (Origin keeps the page; a 6-panel stack on the upright Line page has panels of about 61 pt), one Y title by default instead of one per panel, panel labels such as (a) and (b) (Merge Graph can add them; not measured), and one colour per panel across the graph instead of Origin's colours per layer (with the cyclic voltammetry palette, every one-curve panel gets the first light blue).
- Merge Graph's automatic rows and columns were seen only for 2 to 5 graphs.
- Not tested in Origin: the column assignment of PAN2VERT and PAN4 with several X columns and horizontal stacks. The Layer Management Arrange defaults were measured on 2026-11-15 (15.7d).
- Merge Graph Windows: Origin's own text-factor formula (15.7c fits it), where it places several unlinked layers of one source, what "Treat Each Source Graph as a Unit" off and the Common Scale options change, and its panel labels.

## 16. Free text labels (P0, GP-TXT-01)

### 16.1 What Origin does

- A free text object has its text (with the escapes of section 7), a font, a size, a colour, a rotation and a place. Its attachment is *page*, *layer frame* or *layer and scales* (section 1.6).
- It is made with the Text tool of the Tools toolbar (click on the page, type in a box on the graph) and with the page's right-click command "Add Text...". A double click on it opens its text and format settings; it is moved by dragging, removed with Delete [W:attachment-scaling-object]. [INF] The tool ends after one label.
- Pictures of the owner's figures show labels of two kinds: curve names such as "Zn-LMO" and "After Delithiation", and peak indices such as "311".

### 16.2 Sheaf's model and anchor rule (Sheaf choice)

- A label belongs to one layer (`Layer::labels`) and is attached to the *layer frame*: its anchor is `(x, y)`, fractions of the layer's frame, `(0, 0)` the top left corner, `(1, 1)` the bottom right. Values outside 0 to 1 are allowed and put the label outside the frame. The anchor follows the frame when the layer moves or the page is resized and ignores the axis range.
- The anchor is the start of the text, at the vertical middle of its first character, so a label with rotation 0 grows to the right of the point, and a rotated one grows along its angle (counter-clockwise degrees).
- A new label's font is the layer's legend font, upright and black; the text window changes size, bold, italic and rotation. Bold and italic of single words are written with `\b(..)` and `\i(..)`.
- Sheaf has no "page" or "layer and scales" attachment; labels read from `.opj` files are stored in the same frame fractions by the reader.

### 16.3 Commands

**Desktop app.**
- Graph > Add Text... and the page's right-click "Add Text...": open the Text window for a new label in the active layer, placed in the middle of its frame (`0.5, 0.5`).
- The Text tool of the Tools toolbar: a click on the page places the label at that point in the layer under the click (a click outside every frame uses the active layer) and opens the Text window. The tool ends after one label, or with Esc.
- The Text window has inline formatting buttons, text (several lines; escapes allowed), Size, Bold, Italic, Rotation and a rendered preview (7.5), and OK, Apply and Cancel. A new label is in the project only after OK or Apply; Cancel, or OK with no text, adds none. OK or Apply on an existing label with no text is refused with a message (Delete removes a label).
- Double click on a label, or its right-click "Properties...", opens the window for it. A click selects it; dragging it moves it (the selection handles follow the pointer; the label moves when the button is let go). Delete removes it.
- Each of Add text, Edit text, Move text and Delete text is one undo step.
- `sheaf-app --show text:N` opens the Text window of the N-th label of the first graph (for pictures).

**Command line** (`sheaf plot` and `sheaf export`, style option):
- `--label "TEXT@X,Y"`, repeatable: adds a label to layer 0 at the fractions `X`, `Y` of its frame. The text is everything before the last `@`, so it may hold `@`; escapes are kept as written (`--label "\b(Zn-LMO)@0.85,0.1"`).
- A value without `@`, with an empty text, or whose position is not two finite numbers is an error that quotes the value and shows the form `TEXT@X,Y`; no file is written.
- The labels are added after the other style options, in the order given, and are saved with `--save`.

### 16.4 Not done

- Colour, font family, attachment choice, resizing and a background or frame for a label; copy and paste of labels; "Bring to the front" and "Push to the back"; editing text in place on the graph.

## 17. Drawn objects (P1, T24)

Lines, arrows, rectangles and ellipses drawn on a graph with the Line, Arrow, Rectangle and Circle buttons of the Tools toolbar. Pictures and the inset graph are not part of this section.

### 17.1 What Origin does

Nothing in this section was measured: the work was done on a machine without Origin. Every line below is from Origin's public documentation, paraphrased, and must be checked on the Windows machine (`tools/origin-ref`) before it is called Origin's behaviour. Tag: [DOC].

- [DOC] The Tools toolbar has Line, Arrow, Rectangle and Circle. One drag makes one object, and the tool returns to the Pointer.
- [DOC] Shift while dragging: a line or arrow snaps to horizontal, vertical or 45 degrees; a rectangle becomes a square; the Circle tool makes a circle instead of an ellipse.
- [DOC] An object is attached to the page, the layer frame, or the layer and its scales.
- [DOC] A double click opens a window with the line (style, width, colour), the arrow (shape, length, width, at the begin and/or end) and the fill (pattern, colour) of the object. Delete removes it; dragging moves it.

Unmeasured, so chosen by Sheaf: the default line width and colour, the arrow head size and shape, whether the tool stays on after one object, the stacking order of objects against labels and the legend, the cursor, what a click without a drag does, and the exact snapping rule near the angle boundaries.

### 17.2 Sheaf's model (Sheaf choice)

- `Layer::shapes` holds `Shape { kind, x1, y1, x2, y2, color, width_pt, dash, fill, head_pt, head_at_start }`, saved only when not empty, so files written before this section load unchanged. Kinds: `line`, `arrow`, `rectangle`, `ellipse`.
- The points are fractions of the layer's frame, as for a text label (section 16.2): `(0, 0)` is the top left corner, `(1, 1)` the bottom right, outside 0 to 1 is allowed. The objects are attached to the layer frame and do not follow the axis scales; a "layer and scales" or "page" attachment does not exist yet.
- A line or arrow runs from the first to the second point. An arrow has a filled triangular head at the second point (`head_at_start` adds one at the first); the head is `head_pt` long (default 9 pt) and 0.7 of that wide, and is shortened so that it never takes more than the line. A rectangle has the two points as opposite corners; an ellipse is inscribed in that rectangle. `fill` (rectangle and ellipse) is a solid colour; no fill is the default, so the data shows through.
- Drawing: the objects are drawn after the legend and the labels, in list order, so they are above the data and above the text. Every renderer (SVG, PNG, PDF, EPS, EMF, TIFF and the window) takes them from the same scene.
- Picking: a line or arrow is hit near its line, an unfilled shape on its outline, a filled one inside it as well (a point in the empty corner of an ellipse's box is not a hit). The tolerance adds half the stroke width, and the bounds of an object include it. Where objects overlap the one drawn last (on top) is picked: a shape over a label, or a later shape over an earlier one.
- Shift constrains in page points, not in fractions: a line or arrow snaps to the nearest of 0, 45, 90 degrees and so on with its length kept; a rectangle or ellipse takes the longer side on both axes.

### 17.3 Commands

**Desktop app.**
- Tools > Line, Arrow, Rectangle, Circle: the tool is on (pressed) until one object is drawn, a click on Pointer, or Esc. A drag draws it in the layer under the pointer's start (the active layer if that is outside every frame), with a blue outline while dragging. A drag shorter than 3 page points draws nothing and leaves the tool on. The new object is selected and the tool is the Pointer again.
- A click selects an object, dragging it moves it (it moves when the button is let go), Delete removes it, a double click or right-click Properties... opens the window with the line colour, width and style, the fill of a rectangle or ellipse, and the head length and a second head of an arrow. OK and Apply are one undo step each.
- The window closes when Delete, Undo or Redo changes the objects of its layer, or when its object is gone or another kind of object has its place, so it never edits another object.
- Add, Move, Edit and Delete of a drawn object are one undo step each ("Add drawn object", "Move drawn object", "Edit drawn object", "Delete drawn object").
- The right-click menu of a drawn shape has Delete, Properties..., "Bring to the front" and "Push to the back". The ordering commands move that shape to the end or beginning of its layer's shape list, retaining all geometry and style. Cut and Copy remain greyed. This within-layer shape ordering is Sheaf's choice and has not been measured in Origin.
- Ordering changes the renderer and topmost hit together. The moved shape's selection follows its new index; undo/redo clears a selected shape when its layer's shape list changed, so later Delete or Properties cannot silently target another object. An open shape-properties window closes when the layer's shapes change, as for other shape edits. An edge move or stale index does nothing and adds no undo step. Labels, legends and other layers retain their existing stacking behavior.
- A selected line or arrow has two endpoint resize handles; a rectangle or ellipse has four corner handles. Their positions come from the shape's coordinates, not the stroke/arrowhead bounding box. In Pointer mode, dragging a handle fixes the opposite endpoint/corner and changes only the chosen coordinates. Arrow endpoint identities and all styles are retained; inverted/crossed corners and off-frame coordinates are allowed. This resize interaction is Sheaf's choice, not measured Origin behavior.
- A resize previews without changing the project and commits one "Resize drawn object" undo step on release. A no-op, Escape or stale/cancelled drag leaves the project unchanged. Collapsed line/arrow geometry and zero-span rectangle/ellipse geometry are rejected with a 0.001 UI-point tolerance to absorb screen-coordinate roundoff. Resizing has no Shift aspect/snap constraint; the separate drawing-tool constraints above still apply.
- `sheaf-app --show shape:N` opens the window of the N-th object of the first graph (for pictures).

**Command line** (`sheaf plot`, `sheaf export`, style option):
- `--draw "KIND@X1,Y1,X2,Y2"`, repeatable: KIND is `line`, `arrow`, `rectangle` (or `rect`) or `ellipse` (or `circle`); the four numbers are fractions of the first layer's frame. The object is black, 1 pt, solid, unfilled. A value without `@`, with another kind or without four finite numbers is an error that quotes the value; no file is written. The objects are saved with `--save`.

### 17.4 Origin project files

An `.opj` file written here does not hold drawn objects. The writer says so in a warning that counts them per layer ("layer 1 has drawn objects (2 arrow, 1 ellipse) that an Origin project file written here cannot hold") and the `.opj.sheaf` companion keeps them. Reading drawn objects from `.opj` files is not done.

### 17.5 Not done

Colour of the head apart from the line, head width, dash of the head, rounded rectangles, pattern fills, page and scale attachment, ordering across object kinds/layers, snapping to a grid, polylines, curves, pictures and the inset graph.


Measured with LabTalk's `draw` command (2026-11-13, `re-workspace/scratch/draw17/draw1.out.txt`), which
may not use the defaults of the graph's Line and Arrow tools: `draw -l` makes a line attached to the axis
scales (`attach` 2), 0.5 pt wide, colour 1 (black), line type 1, arrow head width 5 and length 10; `-a`
did not change the arrow shapes; `draw -r` was refused and `draw -c` gave an object without these
properties.

The tools themselves, measured the same day by dragging them on a graph in Origin's window and opening the
new object's Object Properties (`origin-dialog.ps1 -MenuClicks` with a `G:` drag and a `DC:` double click,
`re-workspace/scratch/draw17/line*`, `arrow1`, `rect*`): the Line tool draws Black, Width 0.5, Solid, no
heads, attached to Layer and Scale; the Arrow tool the same with no head at the start and a filled head at
the end, Width 5 and Length 10; the Rectangle tool a Black 0.5 Solid border filled LT Gray without
pattern, attached to Layer Frame. The windows have the tabs Line, Arrow, Coordinates (units inch, cm, mm,
pixel (default), point, scale), Control (Name, Attach to Page / Layer Frame / Layer and Scale, Visible,
Selectable, movement, resizing, rotating, skewing and Edit locks) for lines, and Border, Fill Pattern,
Dimensions, Control for rectangles. Sheaf now uses 0.5 pt, a 10 pt head and the LT Gray (192, 192, 192)
fill of a new rectangle; it still attaches every object to the frame (lines and arrows do not follow the
axis scale), and the ellipse tool was not measured.

### 17.6 Slope annotation (T52)

A slope marked between two data points of a plot, linked to the data (Origin 2026b's Slope Annotation tool).

**What Origin does** (documentation only, not measured; this was written on a machine without Origin):

- [W:originlab.com/2026b] "Graph Annotation Improvements": "New Slope Annotation tool".
- [W:origin-help/tools-toolbar] The Tools toolbar entry reads "Add a slope Annotation object".
- [W:2026b release notes, ORG-32493, summary only] The slope is computed between two selected points from their coordinate values.

Everything below is Sheaf's own choice, unmeasured, and must be checked against Origin before it is called Origin's behaviour.

**Model.** `Layer::slopes: Vec<SlopeAnnotation>` (saved only when not empty). `SlopeAnnotation { plot, x, row_a, row_b, show_legs, label_offset_pt, color, width_pt }`:
`plot` is the Y `ColumnId` of the plot and `x` its X `ColumnId` (stable when plots are reordered; `Graph::remap_columns` renumbers both on File > Append, as for the rugs). The annotation belongs to the first plot of the layer with that Y and, when `x` is stored, that X, so two plots that share a Y column (other X, offsets or multipliers) are told apart; `x` of `None` (saved without it, or a plot with no X column) matches any X, and if several plots still match the first one is used. The tool, `--slope` and MCP store both columns. Moving a plot to another layer (Move Plot to Right/Left Y, `move_plot_to_layer`) moves its annotations with it in the same undo step, unless another plot left behind is also theirs; the right Y layer a new right axis creates starts without annotations. the rows count from 0 (the worksheet shows them from 1), `show_legs` defaults to true, `color` `None` is the plot's colour, `width_pt` is 1.

**What is drawn.** The two points are the plot's rows as drawn: multipliers, offsets and stacking applied, masked, filtered and missing rows excluded. A segment joins them (`width_pt`), the optional legs run horizontally from the first point to the second point's X and then vertically to the second point (dashed, 0.75 of the width), and the label sits at the middle of the segment, 8 pt to the upper side of it (plus `label_offset_pt`); on a steep segment (the normal pointing more than 0.1 sideways) the label is anchored beside it, starting to the right of the segment or ending to its left, so it never covers the segment, in the legend's font and the annotation's colour. Nothing is drawn, and no label, when a row is missing, not a number, masked, filtered out or outside the plot's range, when the plot is not in the layer or has no one-point-per-row data (pie, box, violin, ridgeline, hexbin, histogram, parallel, treemap, sunburst), when a value cannot be placed on its axis (not positive on a log axis), when the two X values are equal (dx = 0), when either point is outside the frame, in a waterfall layer, or in a polar or ternary layer. The drawn objects of section 17 keep their order; the annotations are drawn after the data and before the legend and labels.

**The value.** `slope = dy / dx` in the axes' own coordinates: data units on a linear axis; on a log10 axis the logarithm to base 10 of the values, so a slope is per decade of that axis; ln and log2 axes use their base (per e-fold, per octave); other scales (reciprocal, probability, ...) use the data values. A Tafel plot of potential (mV) against current on a log10 X axis therefore reads in mV per decade, whichever two rows are picked. Four significant digits (`sheaf_calc::trend::sig_str`, the trendlines' format). With the axes' units the label is `slope = 118.3 mV/dec`: the numerator is the unit of the column on the drawn Y axis or `dec`, `e-fold` or `oct` on a log Y axis; the denominator is the unit of the column on the drawn X axis (horizontal bars swap the axes: their drawn X axis shows the Y column) or `dec`, `e-fold`, `oct` on a log X axis or `dec`, `e-fold`, `oct` on a log X axis; the unit text is appended only when both parts are known (so a log X axis needs only the Y unit; a linear X axis needs both columns' units). Both axes log10: `dec/dec`.

**Commands.**

- Desktop app: Tools toolbar > Slope (after Circle; the status bar says what to click). A click near a data point (the Data Reader's snapping, 30 screen points, nearest point of the nearest plot) picks the first point; a click near a point of the same plot (the nearest point of that plot only) adds the annotation, selects it and the tool is the Pointer again. A click on the same row, or too far from a point, only repeats the hint. A click on a plot without one point per row (histogram, box, violin, ridgeline, hexbin, parallel, treemap, sunburst, pie) is ignored and the status bar says "Slopes need a plot with one point per row"; a masked, filtered or missing point is refused the same way ("Row N is masked, filtered out or not a number"), the first point staying picked. Esc cancels. The new annotation is one undo step ("Add slope annotation"). A click on the segment or its label selects it; Delete or the right-click menu removes it ("Delete slope annotation"); a double click or Properties... opens a window with Show Legs, colour (Auto is the plot's) and width; OK and Apply are one undo step each ("Edit slope annotation"), and the window closes when its annotation is deleted or undone.
- `sheaf plot ... --slope A,B` (also `sheaf export`): worksheet rows from 1, repeatable; every `--slope` applies to the first plot of the first layer. A value that is not two different whole numbers from 1, a row beyond the first plot's data, a first plot without one point per row, or a row that is masked, filtered out or not a number, is an error that quotes the value and writes nothing. The annotations are saved with `--save`. MCP: `style.slope`, a list of `"A,B"` strings or `{row_a, row_b}` objects.
- `.opj`: not written; the save warns ("layer 1 has 2 slope annotations that an Origin project file written here cannot hold ...") and the `.opj.sheaf` companion keeps them.

**Not done.** Dragging the label, a free decimals setting, a unit-less "per decade" switch, attaching an annotation to a plot by position instead of by column, annotations across plots, slope annotations in the Graph Slicer or copied with the layer's other objects.

**Object Edit Bring to Front / Send to Back (T63 G).** In Sheaf the two Object Edit toolbar buttons
dispatch the same `OrderShape` operation as the shape context menu (one undo step, selection follows
the moved shape). They are enabled only when a drawn shape is selected and the move is possible
(already frontmost / backmost greys the matching button); text, legend and axis-title selections keep
them disabled (broader object ordering is B2). Not measured in Origin 2016 yet; the greyed-without-
selection behaviour follows Origin's convention for Object Edit buttons.

### 17.7 Straight lines (T67, measured 2026-11-15)

Origin 2016's Graph > Add Straight Line (`addline`; probes `tools/origin-ref/addline-*.lines.txt`):
Type Vertical (default) or Horizontal, At Value (0), Color (black), Style (Solid, Dash, Dot, Dash
Dot, Dash Dot Dot, Short Dash, Short Dot, Short Dash Dot), Label Display Format (auto: the axis'
tick label format; ".2" gives two decimals), Label Location Top (default), Bottom, Left or Right,
Selectable and Moveable (on), Name. It makes the line (`vline`, `vline1`, ..., `hline`, ... or the
name) and its label (`<name>Text`), 0.5 pt wide, attached to the layer's scales; a vertical line
runs from the Y axis' from to its to at X = value, and these ends stay where they were after a
rescale. The label is the value, in the line's colour and the layer's text size, its centre 5 % of
the axis span from the line: a vertical line's Top label above the frame, Bottom below it, Left and
Right beside the line half-way up; a horizontal line's Top label above the line at the middle of
the X range, Bottom below it, Left and Right outside the frame's edges at the line's Y.

**Sheaf** (`sheaf_core::straight_line::StraightLine` in `Layer::straight_lines`; drawn by
`plot/src/straight_line.rs`): the same options and names, label placement and formats (the 5 % is
taken of the frame, which is the axis span on linear axes; on log axes in the frame too, unmeasured);
a line whose value is outside the axis range is not drawn; `span` keeps the ends the app's dialog saw
(None, as the command line makes them, runs across the frame). Graph > Add Straight Line... opens the
window; OK adds the line in one undo step; a click on the line or its label selects it (after axes
and plots, which a line across the frame would otherwise hide), Delete removes it, a double click or
Properties edits it ("Straight Line"). A line that is not Selectable lets clicks through. Moveable is
stored but dragging a line is not built (shown disabled). `sheaf plot --vline X[,top|bottom|left|right]`
and `--hline Y` add lines to the first layer. `.sheaf` keeps them; `.opj` leaves them out with a
warning and the companion keeps them.

### 17.8 Drawn-shape clipboard audit (T125, documentation only)

The [official drawing-object guide](https://docs.originlab.com/origin-help/create-draw-objects/)
was read on 2026-11-17. It documents single-object Copy through context/Edit menus
or Ctrl+C, and ordinary Paste at a clicked graph destination. Same-location
Ctrl+Shift+V is explicitly a 2025 addition. The default placement behind the last
pasted object changed to placement in front in 2018b. This version split prevents
claiming a same-coordinate append as ordinary Origin2016 Paste. These are [DOC]
facts; no fresh Native measurement, COM retry or installation change was made.

The first implementation is scoped in [T125](../tasks/T125-drawn-shape-clipboard.md)
to one existing line/arrow/rectangle/ellipse. It remains open. A clicked anchor,
physical dimensions across unequal layer frames, source/target attachment, exact
2016 insertion order, Cut/history, focus precedence and clipboard lifetime still
need an explicit tested contract. Current Shape coordinates are frame fractions;
test-only graph-view coordinate helpers do not supply a production paste-point
route. That route borders the active resize/drag/geometry ownership and must be
coordinated before implementation. No Rust, model, renderer, IO or menu behavior
changes in this audit; Cut/Copy remain grey and no new Paste shortcut is enabled.
Multiple selection, Ctrl-drag, worksheet XY extraction, OLE/Copy Page and format
copying are distinct routes outside this slice.

**Implemented 2026-11-19 (T125, Claude Windows "t30-origin"; not measured).** The contract
(`crates/app/src/shape_clip.rs`):

- **Copy and Cut:** a drawn object's right-click Copy keeps a snapshot of it, with its two points
  in page points from the top-left corner of its bounding box. Cut does the same and then deletes the
  object, one undo step ("Delete drawn object"). Copy changes nothing in the project.
- **Lifetime:** the snapshot is taken when copied. Later edits to the source or deleting it do not
  change it, and it stays across projects until the program closes. It is Sheaf's own clipboard, not
  the system one, and it holds one object.
- **Paste:** while an object is copied, the right-click menu of the page or of a layer's empty area
  starts with Paste.
  - Paste puts the top-left corner of the bounding box at the clicked point: in the clicked layer, or
    in layer 1 on the bare page.
  - The copy keeps its size in points in a frame of another size, its endpoint order (an arrow keeps
    its direction) and its style.
  - The copy goes in front of the layer's other drawn objects (Origin's 2018b order; 2016 put it
    behind the last pasted object, which Sheaf does not track). One undo step ("Paste drawn object").
- **Text labels (T147, 2026-11-19):** a text label's right-click Cut and Copy use the same
  clipboard. A label holds the clipboard instead of a drawn object, and the other way round. Paste
  puts the label's anchor (a table's top-left corner) at the clicked point, with its text, font and
  rotation unchanged ("Paste text"). Cut deletes it in one undo step ("Delete text").
- **Axis titles (T148, 2026-11-19):** an axis title's right-click Copy puts a text label on the
  clipboard. It has the title as shown (the automatic one built from the column too), the axis's
  title font, and a 90-degree turn for a vertical axis. Paste makes it a free text label. Cut also
  deletes the title in one undo step ("Delete axis title"). The title's Bring to the front and Push
  to the back stay grey.
- **Edit menu and keys (T149, 2026-11-19):** on a graph, Edit > Cut, Copy, Paste and Clear act on
  the selected object, and so do Ctrl+X, Ctrl+C and Ctrl+V (while no text field has the keyboard).
  - Cut and Copy work on a drawn object, a text label or an axis title. Clear deletes whatever the
    Delete key deletes (also a legend, a slope, a straight line or a scale).
  - Paste goes to the active layer, at the page point clicked last with either button, or the
    middle of the layer before any click.
  - Each item is greyed while it has nothing to work on.
- **Not done:** attachment to data or page, several objects at once, and Ctrl+drag copies.


### 17.9 Pictures on a graph page (T216, 2026-11-20, not measured)

Origin's graph page right-click menu has Insert Images From Files... (section 26.2 lists it). Sheaf's
version: the item opens the system file picker for a PNG file; the picture becomes a drawn object of the
clicked layer (`ShapeKind::Image`, its PNG kept in the project as base64 text) with its top left corner
at the clicked point, 96 pixels to the inch (0.75 point a pixel), made smaller with its proportions kept
to fit two thirds of the page; one undo step ("Insert Image"). A file that is not a readable PNG is
refused. Like the other drawn objects it is selected, moved, resized, aligned, grouped, cut, copied,
pasted and deleted; it has no border unless its line width is set. It is drawn on screen and into SVG,
PNG/TIFF/JPEG/BMP and PDF exports (an SVG `<image>` with a data URL) and into EMF as a 24-bit
`EMR_STRETCHDIBITS` bitmap, transparency blended onto the page colour (T237 slice 9); EPS leaves it out,
and an `.opj` written here leaves it out with the other drawn objects (the `.opj.sheaf` companion keeps
it). Only PNG files; Origin's link-to-file and other formats were not measured.

### 17.10 Plot by group from the command line (T224, 2026-11-20, not measured)

Origin's `plotgroup` X-Function plots worksheet data grouped by the values of a column. Sheaf's command line
does the data-grouping part: `sheaf plot FILE -t line -y Current --group Cell` (any ordinary XY template:
line, scatter, line + symbol, column, ...) splits the rows of the Y columns (and of the X column they are
plotted against) by the values of the group column, in order of first appearance, into a new sheet
`Grouped` (per value an X column and the Y columns, named after the value; rows with an empty group value
are left out; `Project::group_columns`), and plots its Y columns: one curve per value, the legend naming the
values. The group column is never plotted. Not with `--y2`, `--graph` or `--panels`. Origin's graph and
layer grouping (a graph or layer per value) and the dialog are not offered.

### 17.11 Send Graphs to PowerPoint (T232, 2026-11-20, Sheaf's file, not measured)

Origin's graph page and window title-bar menus have **Send Graphs to PowerPoint...** (`sendppt`), which hands
graphs to a running PowerPoint. Sheaf writes a PowerPoint file instead (`sheaf_io::pptx`): the graph page's
menu item (after Export Graphs...) opens a small window, **Active graph (name)** (the default) or **All graphs
in the project (N)** in project order, OK then asks for a `.pptx` file (Graphs.pptx). The file has one slide per
graph, 16:9 (13.333 x 7.5 in, PowerPoint's Widescreen), white, with nothing on it but the graph: a PNG of the
page at 300 dpi (Export Graphs' default) made as large as fits inside a quarter-inch margin, proportions kept,
centred, named after the graph. The command line does the same with `sheaf export project -o slides.pptx`
(`--graph` repeated or `--all-graphs`; `--dpi`, `--image-width`, `--image-height` and `--margin` apply to the
pictures). Checked by opening the file in PowerPoint (Microsoft 365, through COM: 2 slides, 960 x 540 pt, one
picture each, no repair prompt). Origin's slide template, placement and title options are not offered.

**Word (T242, 2026-11-20).** The same window has **PowerPoint (.pptx)** (the default) and **Word (.docx)**; with
Word, OK asks for a `.docx` (Graphs.docx) and writes one graph per page (`sheaf_io::docx`): a US Letter page with
one-inch margins, the graph's 300 dpi PNG as an inline picture as large as fits the 6.5 in text width (and 8.5 in
of height, leaving room for the caption), centred, with the graph's name in a centred paragraph under it and a
page break before each graph but the first. The page menu keeps Origin's one item; Origin's own "Send Graphs to
Word" (later versions) was not compared. `sheaf export project -o graphs.docx` does the same (`--graph` repeated
or `--all-graphs`). Checked by opening the file in Word (Microsoft 365, through COM: 2 pages, 2 inline pictures
468 pt wide, the captions Graph1 and Graph2).
## 18. Plot Details: drop lines, fill under the curve, data labels, layer background (T13)

Measured on 2026-11-13 by opening Origin's Plot Details on a line plot (`tools/origin-ref/origin-dialog.ps1`,
a double-click on the graph; pictures and control lists in `re-workspace/scratch/t13/open1`..`open4`):

- A line plot's node has the tabs Line, Drop Lines and Label; ticking "Fill Area Under Curve" on the
  Line tab adds a Pattern tab. The tree names the plot `[Book]Sheet! A(X), B(Y) [1*:11*]`.
- Line tab: Connect, Style, Width, Color, Transparency (slider, 0 %), "Fill Area Under Curve" with the
  modes Normal (default), Inclusive and Exclusive broken by missing values, Sidelines only, Span
  vertically broken by missing values, Sidelines and Base, Fill to Base, Fill to next data plot (one
  colour, or above/below colours).
- Normal fills the area between the curve and the bottom of the layer, not down to Y = 0 (a curve from
  -1 up to 3 and back on a -2 to 4 axis was filled down to -2). The fill starts in the line's colour,
  "Follow line transparency" is ticked, and the legend sample becomes a filled box.
- Drop Lines tab: Horizontal and Vertical check boxes (both off), each with Style (Solid), Width (0.5)
  and Color (Auto, the plot's colour); Skip Points (the display frequency described below).
- Label tab: Enable (off), Font "Default: Arial", Size 22, Color Black, White Out, bold/italic/
  underline, Frame (None, Box, Shadow), Margin, Rotate 0, Horizontal and Vertical Offset 0, Position
  Right (Center, Left, Right, Above, Below), Label Form Y (X, Y, Row Indices, (X,Y), Custom, a
  column), Numeric Display Format, Attach to Data, leader lines.
- Layer node: tabs Background (Color None, Border Style None, gradient fill), Size/Speed (layer area
  in % of page, link axis length, image caching, speed mode 3000 points), Display (scale elements,
  show X/Y axes, labels, data, frame, clip data to frame, data on top of axes, grid on top of data,
  clipping margins), Stack.

Sheaf (`Plot::decor`, `Layer::background`, drawn in `crates/plot/src/plots.rs` and `layout.rs`):
Fill Area Under Curve uses its own colour in Normal mode (Origin puts the colour on the Pattern
tab; Sheaf shows it under the check box), with the colour's alpha as transparency. The line's
Transparency slider sets the alpha of the line colour. Both drop lines offer style, width and an
Auto or chosen colour, drawing from each point to the left and bottom edges of the frame.

Fill Area Under Curve offers **Normal**, **Inclusive broken by missing values**, **Sidelines only**
and **Sidelines and Base**.
Normal remains the old default and keeps its previous filled rendering. Sidelines only leaves
an unfilled curve and adds vertical boundaries from its first and last finite displayed points
to the bottom of the frame. Sidelines and Base also connects those bottom endpoints. These
outlines use the connecting line's colour, width, dash and transparency; the stored fill colour,
hatch and gradient are inactive and their controls are hidden in these modes. The mode selector
is disabled while Fill Area Under Curve is off; its selection is retained. Dependent group edits
copy it. Other fill modes and user-selected baselines remain open.

Measured in Origin 2016 on 2026-11-15, `re-workspace/scratch/t13-fill-sidelines/`: blue fill colour,
red 6 pt line, points (2,1), (6,1.5), (10,1), Y range -1..2, 900-pixel exports.
`verified_sides.png` and `verified_base.png` have no blue area: the red boundaries reach the
frame bottom at -1 rather than the zero line inside the frame. The legend uses a red line sample
rather than Normal's blue filled box. Controls confirm the two selected modes. In
`verified_missing.png`, rows 3/5 are explicitly missing (`0/0`, both `ISNA` results 1), leaving
first/last runs and an isolated middle point. Only the OUTER endpoints get sidelines; missing-run
ends and the isolated interior point get none, and the connecting-line gap remains open.
The original `normal.png` is a valid bottom-baseline/blue-fill control with its line inadvertently
changed to 0.2 pt; initial `sides`/`sides_base` captures selected line width instead of fill mode
and are excluded. Initial `sides_missing` has seven finite points and is excluded from missing-data
evidence. The corrected controls and raw worksheet reads are retained locally. See also the
[published Line-tab description](https://docs.originlab.com/origin-help/pd-dialog-line-tab/) and
[LabTalk missing-value syntax](https://docs.originlab.com/labtalk/ref/missing-val-constant/).

The shared scene keeps one stroke across the curve and its added boundaries, preserving opacity
at the joins, and changes the legend sample to a line. Screen, raster and vector exports share
that scene; existing connection styles, clipping, skipped symbols and row selection still apply.
Source values and axes are unchanged. Hidden masked rows follow the existing missing-data
view policy, so a hidden endpoint leaves no dangling sideline. Native and companion files preserve
the mode, including while disabled; the existing `.opj` save warning covers the unsupported fill settings. Only
Cartesian straight-line examples were Origin-measured; other connections use Sheaf's existing
curve geometry, and specialised coordinates, alternate baselines and unusual stroke preferences
remain unmeasured. Normal's older hatch/gradient/border fidelity remains covered by section 18.6.

**Inclusive broken by missing values** fills each finite run by directly joining its last point
back to its first. The connecting stroke also closes that chord, with no added boundary down to
the frame bottom. Missing rows separate the polygons. A one-point run has no filled area but
keeps its round connecting-stroke cap. Fill colour, hatch, gradient and transparency remain active;
the legend keeps a filled box. The same finite-run closure uses the existing connection geometry,
clipping, row selection and symbol-gap handling. Each closed outline is one stroke, so its joins
do not receive a second coat of translucent line colour. Normal remains the old default and its
filled geometry is unchanged. Group editing, Apply/Cancel/history and native/companion saves
preserve the new mode; `.opj` retains the unsupported-fill warning.

Measured in Origin 2016 on 2026-11-15, `re-workspace/scratch/t13-fill-inclusive/`: the corrected
`verified_sloping` control selects Inclusive (combo 4, item 1) and applies OK, with blue fill and
a red 6 pt line through (1,1), (5,3), (10,2), on Y=-1..4. Only the endpoint-chord triangle is blue,
with a red closing chord and blue filled legend. In `verified_missing`, rows 4/8/10 of a 13-row
series are explicitly missing (`0/0`, each `ISNA` result 1): three finite runs become three closed
filled triangles, without bridging the gaps; the isolated ninth row remains a red round cap.
The `verified_closed` control repeats X=(2,2,6,6,2), Y=(1,3,3,1,1): the closed rectangle is
blue with a red outline and no fill down to the frame bottom.
The initial `sloping`/`missing`/`closed` attempts failed to open Plot Details (setup/command false)
and are excluded as mode evidence. The verified controls, actions, raw worksheet reads and
exports remain local. The [published Line-tab reference](https://docs.originlab.com/origin-help/pd-dialog-line-tab/)
describes the endpoint closure and missing-run behavior. Only Cartesian straight connections
were Origin-measured; other connections reuse Sheaf's existing geometry. Specialised-coordinate,
self-intersection and remaining fill-mode parity are still unmeasured.

Data Points Display Control on Drop Lines now offers Skip Points and a frequency. This uses the
existing `SymbolStyle::every` (the former Symbol tab's Show every field), so old projects keep their
stored frequency. It selects the first of every n points for drop lines, symbols (including
bubbles/colour mapped symbols), X/Y error bars and data labels. Source row indices remain intact;
the plotted row range anchors the phase. Mask overlays do not restore skipped symbols. Zero and
one mean no skipping; an empty range draws nothing. Bars and areas do not offer the control and
keep every point. The full-resolution connecting line, underlying data and automatic axes are
unchanged. Worksheet filters retain the existing compacted display order; their interaction with
the frequency has not been measured in Origin.

Measured in Origin 2016 on 2026-11-14 through `set %C -skip n` and PNG exports of an eleven-point
scatter plot: n=0 and 1 show all eleven points, n=2 shows rows 1,3,5,7,9,11, n=3 shows 1,4,7,10,
and n=20 shows only row 1. A plot limited to rows 3..10 with n=3 shows rows 3,6,9. A missing Y at
row 4 does not shift the phase: n=3 shows rows 1,7,10 (`missing.lines.txt`, `missing4_verified.png`). Origin 2016
does **not** force the last point to appear, unlike newer versions described in the
[current display-control documentation](https://docs.originlab.com/origin-help/pd-dialog-plotlevel-display-tab/).
Raw inputs/outputs and exported pictures are in `re-workspace/scratch/t13-skip-points/png2.*` and
`skip*.png`; the original dialog controls (unchecked Skip Points and No Line Break for Skipped
Point(s)) are in `re-workspace/scratch/t13/open3/droplines.controls.txt`. Native and `.opj` companion files
preserve the frequency. `.opj` alone warns and shows every point, including for line plots with
skipped errors but no symbols. The symbol-gap interaction is described below. Smart Skip is not offered.

Line + Symbol now offers **Gap to Symbol** on Line and **No Line Break for Skipped Point(s)**
under Drop Lines. Both start unchecked in Origin 2016. No Line Break is enabled only when Gap to
Symbol and Skip Points are enabled. With Gap to Symbol checked, the connecting line has clearance
around every data point, including points whose symbols are skipped. Checking No Line Break keeps
the line continuous at those skipped positions; gaps remain around displayed markers. The line
still follows all source values, including skipped peaks and the final point without a marker;
missing data retains its existing break. Other plot kinds ignore these saved switches.

Reference: Origin 2016 GUI controls and 900-pixel PNG exports in
`re-workspace/scratch/t13-symbol-line-gaps/`: eleven horizontal points, solid blue 20 pt square
markers, red 3 pt line, frequency 3. `gaps_skip3.png` has ten short red fragments between the eleven
positions; `gaps_no_break.png` instead has long continuous fragments between markers at rows
1,4,7,10, with the line continuing to row 11. These observations agree with the published
[Line tab](https://docs.originlab.com/origin-help/pd-dialog-line-tab/) and
[display controls](https://docs.originlab.com/origin-help/pd-dialog-plotlevel-display-tab/).
`gaps_zigzag.png` retains all skipped peaks. A corrected three-row diagonal control at
(2,0.25), (6,1), (10,1.75), `clean_square.png` and `clean_circle.png`, changes the marker shape
at the same nominal size without changing any of the 2,031 red line pixels in the plot frame.
This supports using circular holes independent of marker shape. The earlier diagonal circle
capture failed to activate Gap to Symbol and is excluded; the earlier square capture had extra
rows, replaced by the verified three-row control.

Sheaf removes line geometry rather than painting an opaque cover, so backgrounds, grids and fills
remain visible through gaps, and picking uses the broken path. Mask overlays retain the same
holes. The shared scene carries the gaps through screen, raster and vector exports. The current
clearance uses a fixed 30% of the nominal symbol diameter, consistent with the horizontal exports.
Origin's global symbol-diameter preference is described under
[Options > Graph](https://docs.originlab.com/origin-help/options-dialog-graph-tab/); changing that
global preference is not offered. Exact clearance for other preference values and unusual
connector/marker styles is not measured. Native and companion saves retain both gap switches;
`.opj` alone warns that symbol/line gaps were omitted.

Line + Symbol also offers **Draw Line in Front** and **Draw Line Behind**. Behind is the default.
With gaps off, Front paints the connecting line after symbols in both the data plot and legend
sample. Gap to Symbol disables both radios and overrides the stored Front choice; the selection
is retained while disabled. Other plot kinds ignore the saved choice. The same scene order feeds
screen, raster and vector output. Native and companion saves retain the setting, including while
gaps are enabled; `.opj` alone warns that draw order was omitted.

Measured in Origin 2016 on 2026-11-15: three points (2,1), (6,1), (10,1), solid blue 20 pt squares
and a red 6 pt line, 900-pixel exports. `behind_verified.png` has blue at the data marker centre
(468,327) and legend marker centre (694,42); `front_verified.png` has red at both positions.
The short verified probe returned success and closed with OK. The initial longer probe's valid
`front`/`gap` control captures show Front selected and then greyed while still selected after
enabling Gap to Symbol; its interrupted restore/OK sequence canceled, so its final `front.png`
is excluded from Front raster evidence. Raw controls, steps and exports are under
`re-workspace/scratch/t13-symbol-line-order/`. This agrees with the
[documented Symbol/Line interface](https://docs.originlab.com/origin-help/pd-dialog-line-tab/).

Data labels have font, size, colour, rotation, position and the forms X, Y, Row Indices and (X,Y).
They are written a quarter of the font size from the point, numbers as the worksheet shows them (up to 15
significant digits). White Out on the Label tab draws an opaque white rectangle immediately behind
each displayed nonempty label, independent of page/layer colour. It follows the label's anchor,
offsets and rotation and uses the same clipping and row/Skip Points selection as the text. Older
projects omit `DataLabels::white_out` and load with it off. Native/companion saves preserve it;
the `.opj` writer names the unsupported data labels in its save warning.

White Out reference (2026-11-14): Origin 2016, `set %C -q 1; set %C -qw 1;`, Arial 30,
label "12345", Center, gray layer behind a red line. The 900-pixel PNG has
three 100 x 40 pixel opaque white label boxes; off has none. Rotation 45 degrees rotates the boxes
with the text. Raw commands, outputs, pictures and region bounds are in
`re-workspace/scratch/t13-label-whiteout/`. The existing Label control capture also has White Out
unchecked (`t13/open3/label.controls.txt`). Published controls:
[Label tab](https://docs.originlab.com/origin-help/pd-dialog-label-tab/),
[LabTalk label switches](https://docs.originlab.com/labtalk/ref/options_for_data_plot_labels/).
Sheaf approximates the font box using the measured text advance plus 0.05 em on each side and a
baseline band from -0.92 to +0.23 em. Exact padding for other fonts is unmeasured.

**Label Frame and Margin (Origin 2016, 2026-11-14):** the Label tab offers None (default),
Box and Shadow, with border/fill color controls enabled for Box/Shadow and a shared margin
in percent of font height ([documented controls](https://docs.originlab.com/origin-help/pd-dialog-label2-tab/)).
Synthetic Arial 30 labels "12345", Center, on a gray layer/red line were exported at 900 pixels:
Box with margin 0 is a black rectangle about 98 x 40 pixels; the initial border and fill are
both black. At margin 20 it grows to 117 x 59 pixels. White Out is painted **after** the frame,
clearing only the smaller white text box, so the black margin remains around it. None/margin 20
and Box/margin 0 with White Out produce byte-identical PNGs. Shadow adds a lower-right black
rectangle, displaced about 6-7 pixels at both Arial 15 and 30. At 45 degrees the frame is an
axis-aligned rectangle enclosing the rotated, padded text; the shadow stays to the page's
lower right, while White Out and text rotate together. Raw GUI steps, control dumps and PNGs:
`re-workspace/scratch/t13-label-frame-margin/` (`box0`, `box20`, `shadow0`, `shadow20white`,
`box0white`, `none20white`, `shadow20rot45`, `smallshadow0`).

Sheaf stores `DataLabels::frame`, `margin_percent`, `frame_border` and `frame_fill` with
backward-compatible defaults None, 0, black and black, omitted when default. The Label tab
supports these controls, group-dependent editing and the existing Apply/Cancel/history behavior.
Frame colors use the existing alpha-capable picker. The shared scene draws shadow, frame,
White Out, then text; clipping, empty text, row selection and Skip Points apply to the whole label.
The existing approximate font box is retained: margin uses 4/3 of the font size as the font height,
calibrated to the measured Arial case; shadow is approximated by a fixed 6 pt page offset and
the border by 0.5 pt. Exact metrics for other fonts remain unmeasured. Negative/nonfinite margin
uses zero for drawing; margin is limited to 500%. Native and companion saves preserve all frame
settings, including disabled labels; `.opj` alone retains its existing data-label omission warning.
Leader lines and automatic overlap avoidance remain open.

The layer Background colour fills the frame behind the grid. Sheaf's own additions
beyond what was measured (hatch and gradient fills, bar width, label column, rows, decimals and offsets,
background gradient and border, X/Y axes switches, page border, layer icons, comments, subgroups) are in
section 18.6. Not done: the other fill modes, custom forms and display formats
beyond decimals and leader lines. The `.opj` writer
does not store these settings; the save names them and the companion file keeps them.

### 18.1 Group tab (measured 2026-11-13)

Origin's Plot Details for one plot of a group made by one plot command (`plotxy iy:=(1,2:4)`,
`re-workspace/scratch/t13/group1..3`) opens on the Group tab: Edit Mode Independent or Dependent
(default Dependent); an increment table with a row per property and the choices None, By One, Stretch and
Binned for colours (None and By One for the others): a line plot has Line Color (By One, Origin's colour
list) and Line Style (None); a line + symbol plot has Line Color (By One), Symbol Edge Color (By One),
Line Style, Symbol Type and Symbol Interior (None); a Group Members list; Subgrouping None, By Size or By
Column Label. Grouped plots also get an Offset tab.

Sheaf treats the plots of a layer as one group (`Layer::group_style`): Edit Mode (Dependent by
default: a change on the Line, Symbol, Drop Lines or Label tab of one plot is made to every plot of
the layer except the incremented properties, `Layer::follow_leader`); Line Color with None, By One and
Stretch (the palette modes Same, Cycle and Spread over the layer's colour list), and Line Style, Symbol
Type and Symbol Interior with None and By One, running on from the first plot (`apply_group_increments`;
Symbol Interior alternates filled and open); Group Members lists the plots. Not done: Binned, several
groups in one layer, subgrouping by column label. Subgrouping by size is Sheaf's own (section 18.6).

Offset tab: rows X and Y with Multiplier and Offset boxes. Origin requires Individual on the
layer's Stack tab with the corresponding axes enabled. Sheaf shows these fields on each plot
of a layer with several plots, enabled in Individual mode; both axes apply, as with the existing
offsets (the Stack tab's separate X/Y switches remain unavailable).

**Multiplier reference (Origin 2016, 2026-11-14):** synthetic X = [1, 2, 3], Y = [2, 4, 6],
`layer -b s 4 1 1`, X/Y offsets 5/7 and factors 2/3 (`set ... -sxs`, `-sys`) export symbols at
(7, 13), (9, 19), (11, 25). This matches the
[documented affine transform](https://docs.originlab.com/origin-help/pd-offset-tab/), applied
in data units before axis scaling. Factors 0.5 give (5.5, 8), (6, 9), (6.5, 10).
Factors -2/-3 and 0/0 are retained by `get ... -sxs/-sys`, but both render with factor 1 at
(6, 9), (7, 11), (8, 13); those exported PNGs are byte-identical. None mode returns to
(1, 2), (2, 4), (3, 6). Y labels still read 2, 4, 6. With error columns 0.5 (X) and 1 (Y),
the first scaled point has endpoints X = 6.5/7.5 and Y = 12/14: error magnitudes are not
multiplied. Worksheet values remain unchanged. Raw scripts and PNGs are local in
`scratch/t13-offset-multipliers/probe4*` and `probe5*`; earlier probes used an incorrect layer
command or error-plot assignment and are not evidence for these rules.

Sheaf stores factors on `PlotOffset` with a backward-compatible default of 1, omitted when 1.
Nonfinite factors also use 1 for drawing. Native and `.opj.sheaf` saves preserve the settings;
the `.opj` writer leaves the data unchanged, writes its existing offsets and explicitly warns
that multipliers are omitted. Screen and all exports share the transformed scene. Specialized
plot/coordinate combinations beyond Cartesian line/scatter are not Origin-measured here.

Display tab (measured, section 18 list): Sheaf offers Labels (the tick labels of both axes), Data
(`LayerDisplay::show_data`), Frame (the plotting rectangle, see 22.10), Clip Data to Frame (off: the
data may run over the whole page), Grid on Top of Data (the grid lines are drawn after the plots), and
X Axes and Y Axes (off hides that axis: line, ticks, tick labels and title; the grid stays; what Origin
hides is not measured). Data on Top of Axes (`LayerDisplay::data_on_top`, default off) draws the
axes, ticks and axis text before the data; legends, free labels and drawn annotations remain above
the data. The axes remain unclipped; Clip Data to Frame still applies only to the plots. Grid on
Top of Data remains independent. Picking a data/axis overlap follows the chosen order. The same
scene order is used on screen and in SVG, raster, PDF, EPS and EMF exports.

Reference for Data on Top of Axes (2026-11-14): Origin 2016's Display tab starts unchecked.
Clicking the control changes its state from 0 to 1 and back to 0, without changing Grid on Top of
Data (0 throughout). Setup, steps, screenshots and control lists are in
`re-workspace/scratch/t13-data-on-top/dialog/{display,enabled,disabled}.controls.txt`.
Origin's [Display/Speed documentation](https://docs.originlab.com/origin-help/pd-dialog-display-speed-tab/)
describes data and symbols superimposed on the axes; its
[axis/grid overlap example](https://docs.originlab.com/quick-help/axis-ontop/) explains the independent
grid switch. Its general default statement differs from the captured template; Sheaf keeps the
measured template's unchecked default. Scale Elements and Clipping Margins are not offered.
The `.opj` writer names a changed Display tab in its save warnings; the companion preserves the
setting. Older projects that omit the new field load with data behind the axes.

Size/Speed tab (measured): Layer Area Left, Top, Width, Height with Units % of Page (default), inch, cm,
mm, pixel and point; Link Axis Length to Scale with X:Y Ratio; Graphic Image Caching None or Raster;
Speed Mode with worksheet data at most 3000 points per curve and matrix data 32 per dimension. Sheaf's
tab (named Size/Speed) has the Layer Area in every unit but pixel (the unit is only how the numbers are
shown; the frame stays fractions of the page, and a linked layer shows % only); the rest is not offered.

Page node (measured, `re-workspace/scratch/t13/page1..2`): tabs Print/Dimensions (Width 10.72 and Height
8.205 in Units Windows(inch), also inch, cm, mm, pixel, point; printer default dimensions; Print Cropmarks;
Keep Aspect Ratio in Full Screen View), Miscellaneous (View Mode, redraw, drawing order, layer icon, gap
across layers, label colour follows symbol, independent layer control, modifiers), Display (page Color,
Gradient Fill, connect lines across axis breaks and missing data, Master Items) and Legends/Titles (the
%(n) translation mode @LM by default, units in auto axis titles, data plot index, Legend Update Mode
Update when Adding, Include data plots from all Layers (off), Legend Symbol Width, Indicate Active Dataset).
Sheaf shows Print/Dimensions (size in inch, cm, mm or point, and Sheaf's page title), Display (page
colour, and a border line around the page, Sheaf's own), Legends/Titles (Include data plots from all Layers,
which sets every layer's legend scope; what the window title shows, Sheaf's own) and its own Text tab.
Miscellaneous holds only Sheaf's own settings (layer icons, comments); Origin's controls on it are not
offered. Print/Dimensions can keep the layers the same size in points when the page is resized (Sheaf's own).

Buttons (measured): Origin's Plot Details has `>>` (left of the buttons), Workbook (on a plot node), OK,
Cancel and Apply. Sheaf has OK, Cancel and Apply, and Workbook on a plot node, which brings the
worksheet of the plot's Y column to the front while the dialog stays open; `>>` is not offered.

### 18.2 Layer Contents (measured 2026-11-13)

Origin's Graph > Layer Contents... (F12; `origin-dialog.ps1 -MenuClicks "700,500;K:{F12}"`,
`re-workspace/scratch/t13lc/lc1`) is its own window, "Layer Contents: Add, Remove, Group, Order Plots -
Layer1" (about 680 x 300): at the left a source list (Worksheets in Folder by default; also recursive,
project, favourites, matrices and others) with Include Shortcuts (on) over a table Book Name, Sheet, Long
Name, Short Name (`B(Y)`); in the middle a plot type button and `=>`, `<=`; at the right Layer (1),
Group, Ungroup, up and down arrows, Rescale on Apply (off), and the table of the layer's plots: group
(`g1`), Short Name, Long Name, Legend, Plot Type; at the bottom `>>`, Layer Properties..., Plot Setup...,
Apply, Close, Cancel.

Sheaf (`dialogs/layer_contents.rs`, F12 and the layer's right-click menu): the same window and title; the
left list holds the numeric Y columns of the whole project (the only source offered); the plot type is
Line, Scatter, Line + Symbol or Column; `=>` adds the chosen columns with their X and error columns at
the end, `<=` removes the chosen plots; Group gives the chosen plots one group number, Ungroup clears it;
Up and Down move one plot; Rescale on Apply makes the axes automatic again; Layer Properties... opens Plot
Details on the layer; Plot Setup... and `>>` are greyed. Edits show at once and are one undo step; Cancel
takes them back, Close keeps them. Plot Setup... in the Graph menu still opens Plot Details' Layer
Contents tab.

### 18.3 Plot Setup (measured 2026-11-14)

Origin's Plot Setup (Layer Contents > Plot Setup..., which closes Layer Contents and opens this window;
`re-workspace/scratch/plotsetup/p1`, `p3`) is titled "Plot Setup: Configure Data Plots in Layer". It opens with
the Plot List only: a tree of Layer 1 (Rescale, Show), its Group and each plot by its data name, with Range
(`[1*:11*] 0 < X < 10 , 5 < Y < 5`), Show, Plot Type and Legend; Preview, OK, Cancel, Apply. Its chevron
shows the upper panels: Plot Type (Line, Scatter, Line + Symbol, Column / Bar, Bubble, Color Mapped, Bubble +
Color Mapped, ...), Show(S) and the chosen sheet, the sheet's columns with X, Y, yEr and L ticks, Column, Long
Name, Comments, Sampling Interval and Position (an `<autoX>` row first), Replace and Add.

Sheaf (`dialogs/plot_setup.rs`, Graph > Plot Setup..., the layer's right-click menu and Layer Contents'
button): the window always shows both parts: Plot Type (the first four; Bubble and the colour mapped kinds
greyed), a sheet list, the sheet's columns with one X, any number of Y and one yEr tick (L greyed), Add (a
plot per ticked Y with the ticked X and error column) and Replace (the chosen plot takes the ticked columns);
the Plot List with Rescale, Range, a Show tick, Plot Type and Legend; Preview greyed, OK, Cancel, Apply. Edits
are one undo step; Cancel takes them back. Not offered: groups in the list, drag to reorder, Sampling
Interval, Position, `<autoX>`.

### 18.4 Link Axes Scales (measured 2026-11-14)

Measured with `tools/origin-ref/origin-dialog.ps1` on a graph whose second layer was made by `layer -n y`
(a right Y layer); pictures and control lists in `re-workspace/scratch/t13link`:

- The layer node's tabs are Background, Size/Speed, Display, Link Axes Scales and Stack for the second
  layer; the first layer has no Link Axes Scales tab.
- The tab: Link to (a list of None and the other layer, here None and Layer1), the line "Link Axes in Child
  Layer to Parent", then two groups side by side, X Axis Link and Y Axis Link, each with the choices None,
  Straight (1 to 1), Align (with an "X =" box) and Custom (with "X1 =" and "X2 =" boxes). The boxes are
  greyed unless their choice is on. `layer -n y` gives X Straight and Y None.
- Link to None turns both groups to None and greys all their choices; choosing Layer1 again leaves both on
  None (a new link starts unlinked).

Sheaf (`graph_details.rs` `link_tab`, `Graph::set_link`): the same tab for every layer after the first, with
None and Straight; Align and Custom are drawn greyed with their boxes (Sheaf has no formula links). Link to
lists None and the earlier layers that are not linked themselves (Sheaf follows one level of links); a layer
that others are linked to cannot be linked. Changing the link keeps the layer where it is on the page: its
frame is converted between fractions of the page and of the parent's frame.

### 18.5 The ">>" button (measured 2026-11-14)

Origin's Plot Details has a ">>" button at the bottom left. It hides the tree: the window shrinks to the tabs of
the chosen node and the button becomes "<<", which brings the tree back (`re-workspace/scratch/t13link/open5`).
Sheaf: the same button; the window keeps its size and the tabs take the room; the choice lasts while Sheaf runs.

### 18.6 Look settings beyond the measured tabs (T13, Sheaf's own, not measured)

Written on a machine without Origin. Each setting below is a Sheaf choice for something Origin's tab has
(or a close neighbour of it) and has to be compared with Origin on the Windows machine with
`tools/origin-ref`. The settings are in `crates/core/src/graph_style.rs` and `Plot::decor`, written to the
project file only when they differ from the default (a file written before loads unchanged), drawn by
`sheaf-plot` in SVG, PNG, PDF, EPS and EMF, and edited in Plot Details (`dialogs/details_look.rs`).

- **Fill style** (`PlotDecor::fill`): the Pattern tab of columns, bars, histograms and areas, and the
  fill under a curve on the Line tab. Transparency (on top of the alpha of the colour), a hatch (forward or
  backward diagonal, horizontal, vertical, cross, diagonal cross; colour, line width 0.5 pt and spacing
  3 pt by default) cut to the shape (and drawn only within 5000 pt of the page origin, so that a bar far
  outside a fixed axis range does not build millions of lines), and a linear gradient from the fill colour to a second colour in one
  of four directions, drawn as bands that meet edge to edge when the fill is see-through. Not measured:
  Origin's list of patterns, its default hatch width and spacing, how a gradient and a pattern combine.
- **Drop lines** are drawn only for the kinds whose tab offers them (line and symbol plots, not bars and
  areas); the settings of a plot that changes kind are kept.
- **Spacing, Width (in %)**: the bars at one X position together take this share of the X spacing, which
  replaces the gap while it is set; 0 and 100 are the ends, other values are clamped; histogram bars
  always touch.
- **Label**: besides Origin's forms, a column of the project (the cell of the same row, as the worksheet shows
  it); a list of the rows that are labelled (`1,3,5-8`); decimal places; a horizontal and vertical offset in
  points. Deleting the column puts the form back to Y.
- **Group**: Subgrouping None or By Size (n plots per subgroup, in plot order); "Subgroup on Increment" names the
  property that steps from one subgroup to the next (colour, line style or symbol type) while the others step
  inside the subgroup. The dash and symbol lists are Origin's default lists (section 1.5, "Default line style
  list": 8 dash styles; the first 8 of the 16 default symbol shapes).
- **Layer Background**: a gradient from the colour and a border line (colour, style, width) around the frame.
  The right Y layer made by Double-Y draws no background of its own.
- **Display**: X Axes and Y Axes switches (section 18.1).
- **Page**: a border line inside the page edge; the layer icons can be hidden (they are not part of exported
  pictures); comments, shown as the graph's hover text in the Object Manager; what the window title shows
  (both names, the short or the long one); Print/Dimensions can keep the layers the same size in points when the
  page is resized (linked layers are fractions of their parent and need no change).

### 18.7a Link Axis Length to Scale with X:Y Ratio (measured)

Measured 2026-11-15 in Origin 2016 (layer Plot Details opened by a double click inside the frame,
`tools/origin-ref/origin-dialog.ps1`; raw files in `re-workspace/scratch/xy-ratio`). Graph of 0..10 against
0..20, axes X -2.5..12.5 and Y -2..22, layer 17.91 / 11.58 / 68.16 / 71.83 % of a 10.72 x 8.205 in page.
- Size/Speed: the tick "Link Axis Length to Scale with X:Y Ratio" (off) with a value box (1, grey until
  ticked) below Width and Height. The same tab's Speed Mode had "Worksheet data, maximum points per curve" ticked
  with 3000 and "Matrix data, maximum points per dimension" unticked with 32 and 32.
- The ratio is the length on the page of one X unit over that of one Y unit. Ticking it (ratio 1) left the
  width at 34.36 % (height, left and top unchanged); typing 2 then left the height at 35.91 % (width 34.36);
  typing 0.5 instead left the width at 17.18 % (height 71.83). So turning it on or changing the ratio shrinks
  one side, never grows one.
- The link stays: after OK, setting X to -2.5..27.5 by script kept the width and made the height 35.91 %;
  Y -2..46 then made it 71.83 % again; X -2.5..7.5 made it 215.5 % of the page (the layer runs past the page).
  The width, left and top stayed throughout.
- Not measured: log and other non-linear scales (Sheaf compares the spans in the axes' own scale, decades on
  log10), linked child layers, how the option shows with a break (the axis-dialog notes say it is hidden).

Sheaf: `Layer::xy_ratio` (`None` = off). The Size/Speed tab offers the tick and ratio; ticking or setting the
ratio calls `Layer::fit_xy_ratio` with the current spans (Origin's shrink), and the layout then keeps the
frame's left, top and width and sets its height to `width x span_y / (span_x x ratio)`, without the
page-fitting that otherwise keeps labels on the page. Not offered for a layer that follows another, polar
and ternary layers, or while an axis has breaks (then the link is ignored). Undo restores both. `.opj`
saves name it among the settings not written.

### 18.7b Connect line across Missing Data (measured)

Origin 2016's page Display tab has "Connect line across Missing Data", unticked by default. Measured 2026-11-15
(`re-workspace/scratch/missing-lines`, a line + symbol plot of 2x for x = 0..10 with rows 5, 6 and 9 (x = 4, 5, 8)
set missing): unticked, the line stops at x = 3 and 7 and restarts at 6 and 9 (three pieces); ticked, it runs
straight from the point at 3 to the one at 6 and from 7 to 9 (one line). Symbols are the same either way.
Sheaf: `PageStyle::connect_across_missing` (page Display tab check box); `draw_line` skips rows whose X or Y
value is missing instead of ending the piece. A value that cannot be placed (zero or negative on a log
axis, inside an axis break) still breaks the line; fill areas under the curve are unchanged (unmeasured).

### 18.7 Not done

Speed mode of Size/Speed, attach-to,
automatic overlap avoidance and leader lines of Label, the other fill modes and
Origin's list of fill patterns, subgrouping By Column Label, Binned increments, several groups in one layer,
writing these settings to `.opj` (the writer names what it cannot hold in its save warnings).

### 18.8 Lollipop plot (T33)

Origin's documented facts (from its help, not measured; [W:origin-help/lollipop-plot],
[W:origin-help/pd-dialog-dropline-tab]):

- Plot > Basic 2D : Lollipop Plot (template `Lollipop.OTPU`, Origin 2021). It needs two or more Y
  columns (or a range spanning two); X comes from the associated X column, else the row number or
  sampling interval.
- It is a variation of the vertical drop-line scatter with X and Y exchanged, with drop lines set to
  "Next Data Plot": each plot's drop line goes to the next plot at the same X, one by one, and the
  last Y connects back to the first.
- Plot Details > Drop Lines tab: "Drop To" offers "Next Data Plot" (selected by default for Lollipop
  in the Vertical group). The colour list has "Auto" (the symbol colour) and "Y Value: Plus-Minus"
  (only for Lollipop), which colours drop lines by the sign of the Y value.

Sheaf's own choices, all unmeasured (no Origin at hand):

- `Template::Lollipop` ("lollipop" on the command line and in MCP, menu Plot > Symbol > Lollipop
  Plot): scatter with the standard scatter symbol and size, vertical drop lines on, Drop To = Next
  Data Plot. Fewer than two Y columns are refused in `Project::plot_columns` (menu message, CLI and
  MCP error "a lollipop plot needs two or more Y columns"). `--panels` and `--y2` refuse it (a lollipop has no right Y axis; the app and MCP refuse it too).
- Exchange X-Y: each plot's X and Y columns and the layer's two axes trade places (as Graph > Exchange
  X-Y Axes does), so the drop lines run sideways along the value axis. Plots without an X column
  (row number) cannot be exchanged in the model; they stay upright. Each plot keeps its Y column's
  name in the legend; the axis titles follow the traded columns (the first plot's name on the value axis).
- Drop To has two settings, Axis (default, the older behaviour) and Next Data Plot, one per group
  (Horizontal, Vertical); both groups use the same segment rule when set to Next Data Plot.
- Next Data Plot: plot k draws from each of its points to the point of plot k+1 of the layer at the same
  row; the last plot draws back to the first. With exactly two plots only the first draws (no segment twice);
  one plot draws none. A row where either end is missing, NaN, masked or outside the axis range gives no segment.
- "Y Value: Plus-Minus": two fixed colours, blue (0, 114, 178) for zero and above and vermilion (213, 94, 0)
  for negative, per segment, only when the colour is Auto. For a drop to the axis the sign is that of the plotted Y;
  for a drop to the next plot it is the sign of the start point's value along the direction the segment runs
  (the horizontal value in an exchanged lollipop). The list offers it when Drop To is Next Data Plot or it is set.
- Stroke width and style are the Drop Lines defaults (0.5 pt, solid). `.opj` cannot hold either new setting; a save
  warning names them and the `.opj.sheaf` companion keeps them. Files without the new fields load as Axis / Auto and
  are written back without them.

### 18.9 Plot-only X sorting (T39)

Origin's documented facts (from its help, not measured; [W:originlab.com/2023]): Origin 2023's
"Line-Symbol Plot with automatic X value sorting" sorts the data of line and line-symbol plots
by X "without having to sort the worksheet columns". It is a per-data-plot LabTalk property,
`layer.plot1.line.sortx=1;`, and the graph can then be saved as a template. The Plot Details Line
tab page lists no such option [W:origin-help/pd-dialog-line-tab], and no default is documented
(taken as off).

Sheaf's choices (all unmeasured; Origin's help does not say):

- Field: `LineStyle::sort_x` (serde default false, omitted from `.sheaf` when false, so old files and
  flag-off output are unchanged). "Apply to all" copies it with the rest of the line style and
  Dependent group edits copy it from the leader.
- UI: a check box "Sort X for line connection" at the top of the Line tab, offered only for Line and
  Line + Symbol plots (Sheaf's own place and label). CLI `--sort-x` and the MCP `style.sort_x` set it on
  the plots a command makes or restyles.
- Order: the line joins the plotted rows by increasing drawn X (after multiplier and offset), computed
  once per plot per draw (`PlotData::line_rows`). The sort is stable, so equal X keep worksheet order.
  A row whose X is missing or not finite is left out of the path altogether (it neither connects
  nor breaks it). A row with a missing Y keeps its sorted place and breaks the line between its
  sorted neighbours, as a missing Y does in worksheet order.
- Everything that follows the line path uses that order: straight, step, spline, B-spline and Bezier
  connections (a spline needs strictly increasing X, which sorted data without ties meets), fill under
  the curve in every mode, Gap to Symbol clearances, No Line Break for skipped points, the outer
  endpoints of the Sidelines modes.
- Per-point items (symbols, data labels, error bars, drop lines, masking) stay on their points and keep
  worksheet painting order; the Data Reader and hit testing read worksheet rows. The worksheet is never
  sorted.
- Area, column, bar and other kinds ignore the flag. Sheaf has no exchanged X-Y layer for line
  plots; horizontal bars are their own kinds.
- `.opj`: the writer has no field for it; a save warning says so and the companion `.opj.sheaf` keeps it.

## 19. Printing (File > Print, Print Preview, Page Setup; T27)

Measured in OriginPro 2016 (64-bit) through `tools/origin-ref/origin-screenshot.ps1 -FullScreen`
with clicks (pictures in the local scratch folder `re-workspace/scratch/t27/s4/`, not committed), one
line graph active. Everything not listed here is unmeasured.

### 19.1 Menu and toolbar (measured)

- Origin's File menu is collapsible (a short list with a double-chevron at the bottom); the full
  list has, after Save Project without Data and Duplicate Project without Data, a separator and then
  three items in this order: **Print...** (Ctrl+P), **Print Preview**, **Page Setup...**, a
  separator, then Import. Sheaf's menu has the same three in the same place.
- The Standard toolbar has a Print button (tooltip "Print the active window"). The icon next to
  it opened Slide Show of Graphs, not a preview; no Print Preview toolbar button was measured
  (unmeasured). Clicking the Print button by script only showed the tooltip, so the system dialog
  behind it was not captured.

Sheaf's Standard toolbar Print button uses the same command resolver and print action as
File > Print. It is available for the same active-window contexts as the menu and uses the
measured tooltip "Print the active window". Tests switch between workbook and graph focus and
check menu/toolbar availability and click dispatch; the print backend is skipped by the UI harness.

### 19.2 Page Setup (measured, window captured)

A small modal window titled "Page Setup", about 360 x 415 px:

- a picture of one sheet with a dashed rectangle for the margins and a drop shadow, at the top;
- group **Paper**: **Size** (a drop-down, "Letter" on the measuring machine) and **Source** (a
  drop-down, "Printer auto select");
- group **Orientation**: radio buttons **Portrait** and **Landscape**; Landscape was selected for the
  graph (a page wider than tall; only this one landscape graph was captured, so whether the orientation
  follows the page is unmeasured);
- group **Margins (inches)**: Left, Right, Top, Bottom fields, shown as 1, 1, 1, 1 and greyed out
  while the graph was active (so Origin does not let the user change them there; whether Origin
  applies 1 inch is unmeasured);
- buttons **Help**, **OK**, **Cancel**.

Sheaf follows this: Paper Size (Letter, Legal, Tabloid, A3, A4, A5), Orientation with a third choice
**Automatic** that is the default (landscape for a page wider than tall: Sheaf's own rule,
inferred from the single landscape sample, not measured), Margins in inches (editable, default 1 inch: Origin's displayed value), OK and Cancel. There is
no Source (the tray belongs to the printer's own dialog) and no Help button. Defaults: Letter,
Automatic, 1 inch. The setting belongs to the project (`Project::page_setup`), saved in `.sheaf` and,
because an `.opj` has no place for it, in the `.opj.sheaf` companion (the save warns when it is not
the default). Changing it is an undo step "Page Setup".

### 19.3 Print and Print Preview dialogs (unmeasured)

The system print dialog, Origin's own Print dialog (if it has one: copies, range, scaling, "Print
to file") and the Print Preview window were not captured: the menu clicks that open them did not
produce a dialog or a window in the screenshots after several tries (the first menu path of the
captured Page Setup worked once; the same keystroke path and the toolbar buttons did not). The
brief's statement that Origin prints a graph as vector output scaled to the paper and page-centred
is therefore taken as given, not measured. Do not read the layout of Sheaf's Print Preview as
Origin's; it is Sheaf's own (paper with drop shadow, dashed margin rectangle, the page drawn where it
prints, a line naming paper, orientation and margins, buttons Print..., Page Setup..., Close).

### 19.4 Placement (Sheaf's rule)

The graph page is scaled uniformly (up as well as down, aspect ratio kept) to the largest size that
fits inside the paper minus the margins and inside what the printer can reach, and is centred in that
area (`sheaf_core::page_setup::placement`; Print Preview and the printer path call the same
function). The paper is Letter etc. turned landscape or not by the orientation. Under Windows the
page's enhanced metafile (`sheaf_plot::render_emf`) is played into the printer device context
rectangle computed from `GetDeviceCaps` (`PHYSICALOFFSETX/Y`, `HORZRES/VERTRES`), so the driver
receives vector drawing, not a bitmap. Elsewhere the page's PDF goes to `lp` (paper and orientation as
options, `fit-to-page`; not tested on this machine).

### 19.5 Check with "Microsoft Print to PDF" (measured on Sheaf's side)

A synthetic 5-point line graph (`sheaf plot data.csv`, page 600.06 x 500.79 pt) printed with the
headless function (`print_to_printer`, `DOCINFOW.lpszOutput` set, no dialog; the driver wrote the
file without asking where to save it, in about one second): the PDF has one page of 792 x 612 pt
(Letter landscape, from Automatic). The page background rectangle in the content stream is
560.9 x 468.0 pt at x = 115.6 pt, y = 72 pt: the height limits (612 - 2 x 72 = 468), the width is
468 x 600.06 / 500.79 = 560.8, the left edge is 72 + (648 - 560.8) / 2 = 115.6. That equals the
placement function's result, so the printer got the page centred inside the 1 inch margins. The
page of `sheaf plot ... -o x.pdf` is the page size itself (600.06 x 500.79 pt, drawn at scale 1);
printing differs from it by the paper and the scale 0.935. The driver reported no unprintable edge.
The check is the ignored test `winprint::tests::prints_a_page_to_a_named_printer_without_a_dialog`
(environment variables `SHEAF_PRINT_EMF`, `SHEAF_PRINT_OUT`, `SHEAF_PRINTER`, `SHEAF_PRINT_PAGE`).
Not checked: a physical printer (the HP on this machine was not used), copies, collation, several
pages, other paper sizes through the driver.

### 19.6 CLI printing (`sheaf print`, Sheaf's interface)

```
sheaf print experiment.sheaf --graph Graph1 --output print.pdf
sheaf print experiment.opj --graph Graph1 --printer LabPrinter
```

The input is an existing project (`.sheaf`, `.opj` with its companion, or `.opju`); it must contain
a graph. With one graph its name may be omitted; with several, `--graph` is required. The command
uses the saved Page Setup: paper, orientation and margins. The graph is uniformly scaled and
centred by `PageSetup::place` inside a paper-sized vector PDF. This differs from graph export,
whose PDF has the graph's own page size. Non-finite dimensions/margins, negative margins and
margins leaving no printable area are refused.

`--output FILE.pdf` requires a `.pdf` extension and writes the print-ready PDF without submitting a job. Without it, macOS/Linux
submit to the default destination, or the `--printer NAME` destination. The two flags are mutually
exclusive. Windows currently requires `--output`; the desktop app retains its native Windows
print dialog. This command does not modify the source project or its saved Page Setup.

Submission invokes `lp` directly, with a unique temporary PDF, the paper and orientation, and
`print-scaling=none` plus `fit-to-page=Off` to request the computed placement without fitting it again. The temporary copy is removed after submission succeeds
or fails. Destination and title are process arguments, without shell interpretation. See the
[CUPS lp manual](https://www.cups.org/doc/man-lp.html) for destination, job options and spool-copy
semantics and the [OpenPrinting filter documentation](https://github.com/OpenPrinting/cups-filters)
for scaling controls. Automated tests use a fake spooler: a physical printer, hardware printable edges and
printer-specific option handling remain unverified. The output PDF assumes the whole paper is
printable; a real printer may clip marks outside its hardware limits.

This is Sheaf's own command; no Origin CLI or dialog behaviour is claimed.

### 19.7 Native macOS graph print panel (T27)

File > Print... on an active graph uses PDFKit's native print operation on the main thread.
The input is `sheaf_plot::render_print_pdf`: a vector PDF already laid out on the saved paper by
`PageSetup::place`. The native operation receives the saved paper and orientation, zero additional
margins, scale-none and no auto-rotation, so the operation starts with that placement instead of
applying Sheaf's margins a second time. Changes in the system panel apply to that print job and do
not change the saved project settings.

The prepared operation retains its PDF document for the operation's full lifetime. A successful
native run is reported as printed; a false return has no success message because AppKit uses that
return for both cancellation and failure. Failures to load the PDF or construct an operation are
reported as errors. This is the system's panel, not an imitation of Origin's Windows Print window.

References: [PDFKit print-operation factory](https://developer.apple.com/documentation/pdfkit/pdfdocument/printoperation(for:scalingmode:autorotate:)),
[AppKit run](https://developer.apple.com/documentation/appkit/nsprintoperation/run()), and the
[Rust PDFDocument bindings](https://docs.rs/objc2-pdf-kit/0.3.2/objc2_pdf_kit/struct.PDFDocument.html).
Automated checks prepare the real native operation on the process main thread without running it;
standard UI tests never send a print job. Run the no-spool native check on macOS with
`cargo run -p sheaf-app --example macos_print_check`; it verifies Letter/A4 portrait/landscape,
PDF media boxes, native paper/orientation, margins, scaling and title, and rejects invalid PDF or
mismatched paper. Opening and cancelling the actual panel was not manually verified in this
session. Physical-printer output remains unverified. Changing paper in the system panel does not regenerate the graph layout; use Sheaf's
Page Setup to change that layout.

### 19.8 Printing a workbook (T27, measured dialog, Sheaf's own layout)

**Measured in OriginPro 2016** (File > Print with a worksheet active, picture in the local scratch
folder `re-workspace/scratch/t27/wb/dlg1.png`, not committed): the Print window is the system's
common print dialog with Origin's extra controls for a worksheet. Group **Printer**: Name (a
drop-down of the installed printers), a **Properties...** button, the status lines (Status, Type,
Where, Comment), a **Print to file** check box. Group **Print**: **Columns From / To** and **Rows
From / To** (two boxes each, empty by default), a **Selection** check box and an **Options...**
button. Group **Copies**: **Number of copies** (1) and **Collate** (greyed at 1 copy). Buttons OK and
Cancel. So Origin prints a worksheet by column and row range or by selection, and has a worksheet
printing options window behind Options... that was not opened (the click on it by script did not
show a window; **unmeasured**).

**Unmeasured**: the printed layout of an Origin worksheet. Printing through the Origin automation
interface (`doc -p;`) printed nothing while the machine had no default printer; with one, only the
dialog above could be reached, and the run that was to answer the driver's Save dialog produced no
file. Therefore the following are not Origin facts: whether the label rows, row numbers, grid lines
and column heads print, the font, the row height, the page numbers, the page order, and what
Options... offers. Do not read Sheaf's layout below as Origin's.

**Sheaf's rule** (`sheaf_core::sheet_print`, drawn by `sheaf_plot::sheet`):

- File > Print... and File > Print Preview act on the active window: a graph as in 19.4, a workbook
  prints its **active sheet** as a table. The Page Setup of the project applies (paper, orientation,
  margins); Automatic orientation is portrait for a sheet. Tables are not scaled: the printed area is
  the paper inside the margins and every page is exactly that area, so a table page is placed 1:1
  (the printer path plays the page's metafile into the margin rectangle, as for a graph; the
  printer's unprintable edge can only shrink it when the margin is smaller than the edge).
- Content: a grey row of column names as the sheet's head shows them (`A(X1)`, `B(Y1)`), then the
  label rows that have text in a printed column (Long Name, Units, Comments, F(x)=, user parameters;
  a row without any text in the printed columns is left out), then the data rows. Columns hidden in
  the sheet and rows hidden by the sheet's filter are not printed; the row numbers are the sheet's
  own, so a filtered sheet shows its gaps in the numbering. Cell texts are the column's display
  format as the grid shows it (`Column::shown_text`: General shows up to 5 decimals, date/time and
  other formats as set); numbers are right-aligned, text left-aligned. Text longer than its cell is
  cut (not wrapped). Grid lines, a grey head and row-number column, the label rows pale yellow as on
  screen.
- Measures (points): font 8, row 12, column width = characters x 50/8 (the sheet's own column width
  in characters, 8 by default), row-number column 30, footer line 16. On screen rows are 16 px and 8
  characters 67 px; the print values are Sheaf's choice, not measured.
- Page breaking (`paginate`, pure and tested): rows go down first, each page takes
  `floor((area height - footer - head) / row height)` rows (at least one); then the next band of
  columns starts at the first row again, the row-number column repeated on every page. A band takes
  columns while they fit the area width (at least one, so a column wider than the page gets a page
  of its own and its text is cut). Page order is down, then across. An empty sheet is one page.
- Page numbers: "Page n of m" at the bottom left under each page's table (Sheaf's choice; Origin's
  behaviour is unmeasured).
- Output: Windows plays one enhanced metafile per page (`render_sheet_emf_pages`) with
  `StartPage`/`EndPage` per page inside one document; the Print dialog offers a page range (from/to)
  when there are several pages. Origin's column/row ranges and selection are not offered. Elsewhere
  `render_sheet_pdf` writes one paper-sized vector page per table page (the multi-page writer of
  9.7), text kept as text, and `lp` prints it.
- Print Preview of a workbook is the same window as for a graph with **< Prev**, "Page n of m" and
  **Next >** between the Page Setup and Close buttons; the page picture is the table page drawn
  inside the margin rectangle of the paper.

**Check** (Sheaf's side, headless, no dialog, no physical printer): the ignored test
`winprint::tests::prints_a_three_page_sheet_to_a_named_printer_without_a_dialog` builds a synthetic
130-row, 3-column sheet, prints it to "Microsoft Print to PDF" with `DOCINFOW.lpszOutput` set (no
dialog, about 7 s including the build) and writes Sheaf's own PDF of the same sheet beside it
(`re-workspace/scratch/t27/wb/sheaf-print3.pdf` and `sheaf-print3.sheaf.pdf`, local). Both files
have 3 pages with a 612 x 792 pt media box (Letter portrait). Not checked: a physical printer,
copies, collation, page ranges through the driver, other paper sizes.

### 19.9 Not done

View > Print View, Origin's Source (tray), native CLI printer submission on Windows, Origin's workbook
Options... window, the printed layout of an Origin worksheet (19.8), printing a selection or a row/column
range of a sheet, CLI workbook printing, the PDFKit panel for a sheet on macOS (a sheet goes to `lp`).

### 18.10 Rug plot (T41)

Origin facts (documentation only, nothing measured; no Origin on the machine that built this):

- Origin 2019b added "Rug Plot - Add rug marks to axes", to show data density. [W:originlab.com/2019b]
- Plot > Basic 2D > Scatter + Rug (template `ScatterRug.OTP`): each point is projected onto the axis as a thin
  line beside it; equal values overlap. A rug is not a density plot. [W:origin-help/scatter-rug-graph]
- Axis dialog, Rug tab (2D XY Cartesian layers): Show (a rug panel for the selected axis); Data (`<All>` or
  one plot); Color (Auto = the plot's colour); Thickness; Transparency (%) 0 to 100; Layout: Size (%) (panel
  height as a percentage of the average of the layer's width and height), Overlap (checked for built-in rug
  templates such as Scatter Rug), Gap Between Rugs (%) (only when Overlap is cleared), Margin (%) (below 50),
  Show Perpendicular Border, Show Parallel Border. [W:origin-help/axesref-rug]
- Not stated by Origin: default Size, Thickness, Transparency, Margin, inside or outside placement, and
  whether tick labels move.

Sheaf's choices (all unmeasured):

- Model: `Axis::rug` for the axis' own edge (bottom of X, left of Y; the edge the axis line is on) and
  `Axis::rug_opposite` for the edge across (top, right), each a `Rug`. The default is "no rug" and is not
  written, so graphs without rugs save and export exactly as before. The Axis dialog's Rug tab lists the four
  edges and maps them to the right field, also for an axis drawn on the far edge.
- Defaults when shown: Size 5 %, Thickness 0.5 pt, Transparency 0, Margin 0 %, Overlap on, Gap 10 %, both
  borders off, Data `<All>`, Color Auto.
- Placement: outside the frame, in a panel beside the axis starting where the outward tick marks end, between
  the ticks and the tick labels; the tick labels and the title move outward by the panel size, and the page
  fit counts the panel. Panel size = Size % of the average of the frame's width and height.
- Marks: one segment per plotted row across the panel, at the value of the row on that axis (bottom/top: X as
  drawn; left/right: Y as drawn, so horizontal bars and exchanged axes work), through the axis' own transform
  (log, reversed). Rows with a missing X or Y, masked rows, filtered rows and values outside the axis range
  get no mark. Pie, box and violin plots have none. Rugs mark the plotted positions: stacked and offset
  plots, and bars or columns with a categorical X, are marked where they are drawn. A histogram is the
  exception: its plot data is bins, so its rug marks the raw observations (shown, unmasked rows of the Y
  column) along the value axis (X) and nothing beside the count axis.
- Data choice: `Rug::plot` is the Y column id of the chosen plot, not a position; the rug marks the first plot
  of the layer with that Y column, so removing, reordering or moving other plots keeps the choice. When no
  plot of the layer has that column any more the rug falls back to `<All>` (the dialog shows `<All>`). The
  dialog lists one entry per Y column.
- Waterfall: a waterfall layer draws no ticks on the opposite edges, so an opposite rug starts at the frame
  there and the page fit reserves room for the rug only, the same as drawn.
- Margin: space left empty at both ends of each segment across the panel, as a percentage of the panel
  (clamped below 50). Gap: with Overlap cleared each selected plot gets a sub-panel, the first plot nearest the
  axis, separated by Gap % of the panel size (the whole panel keeps its Size). Overlap on: all plots share
  the whole panel.
- Colour Auto is each plot's colour (line colour for line and area kinds, else symbol colour); Transparency
  scales the colour's alpha. Borders are drawn in the axis colour: Parallel = the two lines along the axis
  at the panel's inner and outer sides; Perpendicular = the two lines across the panel at its ends.
- Template: Scatter + Rug (`scatter-rug`, Plot > Symbol and the CLI/MCP template name) is the scatter template
  with the rug shown on the bottom and left axes. Polar and ternary layers have no rug.
- `.opj` save: the rug is not written; a save warning names it, the companion file keeps it.
- Not done: rug hit testing / selection, a rug on the right axis of a double-Y layer is
  possible (it is the opposite edge of that layer's Y axis) but not tuned.

### 18.11 Trendlines (T43)

Origin facts (documentation only, nothing measured; no Origin on the machine that built this):

- Origin 2025b: "Mini toolbar to add trendline for single or multiple plots". Types: Linear, Polynomial,
  Power, Logarithmic, Exponential, Moving Average. [W:originlab.com/2025b]
- Mini Toolbar, Add Trend Line (single-plot and group-plot tabs): Polynomial order 2 to 10; Moving Average
  "Points of Window" 2 to 10; the fitted equation and/or the R-Square value (Pearson's R for linear) can be
  shown, not for Moving Average; a Label Position button; line colour, thickness (pt) and style. A hidden
  trendline cannot be brought back (it is added again). [W:origin-help/mini-toolbar; quick-help/add-trendlines]
- Not documented: default type, order and window, how Power, Logarithmic and Exponential are fitted, how
  R-square is defined for them, trailing or centred averaging, the label's default place and style.

Sheaf's choices (all unmeasured):

- Model: `PlotDecor::trendlines: Vec<Trendline>` (so on each plot; empty by default and not written, which
  keeps graphs without trendlines identical in files and exports). A `Trendline` is the kind (`TrendKind`:
  Linear, Polynomial{order 2..=10}, Power, Logarithmic, Exponential, MovingAverage{window 2..=10}), Show
  Equation, Show R-squared (both on), colour (`None` = Auto, the plot's colour: line colour for line and area
  kinds, else symbol colour), width (1.5 pt), dash (solid) and a label offset in points. Computed when drawn,
  so a trendline follows the data and its edits; it makes no worksheet output. Dependent plot groups do not
  copy trendlines.
- Points: the plotted ones as drawn (offsets, multipliers and stacking applied, before the axis transform),
  skipping rows that are masked, filtered, or have a missing or non-finite X or Y. Pie, box, violin and
  histogram plots and polar, ternary and waterfall layers get no trendline.
- Fits (`sheaf_calc::trend`): Linear and Polynomial are ordinary least squares on (x, y) (the centred and
  scaled solver of the smoothing tools; coefficients are reported as powers of x). Power `y = a x^b`,
  Exponential `y = a e^(bx)` and Logarithmic `y = a + b ln x` are least squares on the transformed data
  (`ln y` on `ln x`, `ln y` on `x`, `y` on `ln x`), as spreadsheets do, not nonlinear fits of y. Points that
  cannot be transformed are skipped: Power needs x > 0 and y > 0, Exponential y > 0, Logarithmic x > 0.
  A fit needs more than `order` usable points (more than 1 for the others) and at least `order + 1`
  distinct finite X among them (counted after the same filtering, on the transformed X); with fewer
  (for example `x = 1,1,1,2,2` for order 2) there is no curve and no label.
- Power and Exponential are evaluated in log space from the centred fit, `exp(c0 + c1 (ln x - lx0))` and
  `exp(c0 + c1 (x - x0))`, never as `a * exp(bx)`, so x on a large offset (years 2000 to 2030) does not
  under- or overflow. The label prints `a = e^intercept` in the usual form when it is a usable number
  (1e-290 to 1e290); otherwise it prints the same curve in centred form: `y = 1e^(0.5(x - 2015))`
  (A is the curve at the mean x) for Exponential and `y = A(x/G)^b` (G the geometric mean of x) for Power.
- R-squared is `1 - SSres/SStot` in the space that was fitted: on y for Linear, Polynomial and Logarithmic,
  on `ln y` for Power and Exponential (the Excel convention). The label says "R2" for Linear too (the square
  of Pearson's r; Origin's Pearson's R is not shown). It is absent when SStot is 0 or only float noise
  (at most 1e-24 of the sum of squares of the fitted values, in the fitted space): such a constant y is
  fitted as the exact constant line (all other coefficients 0), so the label reads `y = 0.3` (a zero
  term is left out of the equation; a zero exponent prints `y = a`) rather than noise slopes like
  `-5.45e-18x`, and the curve is flat.
- Moving average: the trailing mean of `window` consecutive points taken in increasing X (stable for equal X),
  drawn through the point at the last X of each window, so the curve starts at the window-th point. Origin's
  choice (trailing or centred) is unknown. No equation or R-squared.
- Drawing: after the layer's data, in the trendline colour, clipped to the layer. A fit is evaluated at 200
  places from the smallest to the largest plotted X, evenly spaced along the X axis' own scale (so log and
  reciprocal axes get an even curve; reversed axes work), through the layer transform; a value that cannot
  be placed (ln of a non-positive number) breaks the curve, and page coordinates are limited to
  +-100000 pt. The moving average joins its points with straight segments.
- Label: one line per trendline in the layer, from the top-left corner of the frame (6 pt in) down (layers
  that share a frame, a layer and its right Y layer, continue the line count of the earlier layers, so
  the right layer's labels sit below the left layer's and never overlap them), in the
  legend's font at its size, coloured like the curve, plus the trendline's label offset. Text is
  `y = 1.234x + 0.56` (4 significant digits, trailing zeros dropped, scientific outside 1e-3 .. 1e6 as
  `1.235x10^-5`; polynomial terms highest power first with `\+(n)` superscripts; power `y = 2.566x^0.9951`;
  exponential `y = 2.12e^(0.3152x)`; logarithmic `y = 8.888ln(x) + 1.167`), then `, R2 = 0.9876` (superscript
  2). Show Equation and Show R-squared choose the parts; a line with neither is not written. A trendline
  whose fit cannot be made draws nothing.
- App: the plot's right-click menu has Add Trendline... (always), and Edit Trendline... and Remove Trendline
  (only when the plot has one). With several, Edit and Remove act on the last one. The dialog has Type, Order
  or Points of Window, Show Equation, Show R2, Color (Auto or fixed), Width, Dash, OK, Cancel; OK is one undo
  step ("Add trendline", "Edit trendline"), Remove is one ("Remove trendline").
- CLI and MCP: `sheaf plot ... --trendline linear|poly:N|power|log|exp|ma:N` (repeatable; also `poly` = order
  2 and `ma` = window 2) and `--no-trendline-label`; MCP `style.trendline` (array of the same strings) and
  `style.no_trendline_label`. The option replaces the trendlines of every curve it styles (so repeating the
  command does not pile them up), except curves that follow another (a fit curve over its data); equation
  and R-squared show by default.
- `.opj` save: trendlines are not written; a save warning names them, the companion file keeps them.
- Reference values: the calc tests compare with numpy 2.5.4 (`np.polyfit`, with the transformed data for
  power, exponential and logarithmic).
- Not done: a Mini Toolbar, a label dragged with the mouse (the offset is in the model only), a trendline per
  group of plots, a trendline hit test, Pearson's R for linear, confidence bands, fit parameters to a sheet.

## 20. Data menu tools: Mask, Remove Bad and Move Data Points (T48)

Origin's graph Data menu has three tools that act on the points of a plot: Mask Data Points, Move Data
Points and Remove Bad Data Points (Sheaf's menu already lists them in that order, after the display-range
items). Measured elsewhere (workbook-model 13.1a): worksheet masks, Mask Change Color, Disable Masking, Hide
Masked Points. Nothing of the graph tools themselves has been measured. Every choice below is labelled
**[Sheaf choice]** (made without Origin, change it when measured) or **[?]** (not known). The mechanism, the
core edits, is independent of those choices.

### 18.12 Vertical Drop Line and Column + Label (T68, measured)

Origin 2016's Plot > Symbol > Vertical Drop Line and Plot > Column/Bar/Pie > Column + Label are its `DROPLINE`
and `ColumnLabel` templates, measured 2026-11-15 by plotting through them and exporting 1600 px PNGs
(`re-workspace/scratch/t68`): DROPLINE draws the scatter's symbols and a 0.5 pt vertical drop line in each plot's
colour from every point to the bottom of the frame (the axis, not zero); with two plots the symbols go square,
circle (the plain scatter keeps squares). ColumnLabel draws the column plot (red, green, ...) with each value
written above its own column in Arial 22 pt, about 5.8 pt above the top, centred on the bar also when columns are
grouped. Sheaf: `Template::VerticalDropLine` (scatter, `decor.drop.vertical` on, symbol type By One) and
`Template::ColumnLabel` (column, data labels on, position Above, a quarter of the font size, 5.5 pt, above);
data labels of grouped columns and bars are now centred on their own bar. CLI `-t vertical-drop-line`, `-t
column-label`.

### 18.15 Line Series (T82, measured 2026-11-16)

Origin 2016's Plot > Line + Symbol > Line Series makes the graph at once: every worksheet row of the
selected Y columns is one line through its values at X = 1, 2, ..., k (k columns, column order); rows are
not joined. It is one plot with the k-point segment connection (`get %C -l` 2 for two columns, 3 for
three), filled black squares of size 8, the X axis labelled with the column names and titled "Series",
the Y axis "Values" (X 0.75..2.25 for two columns, 0.5..3.5 for three); Origin keeps the rearranged
data out of sight (lock icon). Sheaf: `Project::plot_line_series` adds a `LineSeries<n>` sheet to the
workbook (positions and values row by row, and the column names), one Line + Symbol plot with
`Connect::Segment2`/`Segment3`/`Segments(k)` (k >= 4, saved as `{"segments":k}`; `.opj` writes it
straight with a note), all rows black, the names as X tick labels with a tick at every position, titles
Series and Values; one undo step. Segment connections group the plot's rows by their index (rows
1..n, n+1..2n, ...): a missing value breaks its group's line and never shifts later groups.

### 18.14 Column Scatter (T79, measured 2026-11-16)

Origin 2016's Plot > Symbol > Column Scatter draws every selected Y column as points standing at its
position 1, 2, ... in plot order: filled diamonds of size 8 coloured by the group (black, red, ...), no
line, the points of a column spread sideways by a jitter (within about +-0.15), the X axis from 0.5 to
n + 0.5 with one tick per column labelled with its name, the Y title "Range", the usual legend. Sheaf:
`PlotKind::ColumnScatter` and template `column-scatter` (menu, `sheaf plot -t column-scatter`), placed
by the box chart's category axis; the jitter is a fixed golden-ratio sequence within +-0.15 (Sheaf's
choice: Origin's rule was not measured). Trendlines, rugs and slopes do not use the jittered X. `.opj`
writes a scatter of the data with a warning.

### 18.13 Scatter Central (T75, measured 2026-11-15)

Origin 2016's Plot > Symbol > Scatter Central is the scatter graph with both axes crossing in the middle
of a boxed frame: `layer.x.position` and `layer.y.position` -50 with `posType` 1 (percent), against 0
for Scatter. The Y title stands horizontally above the frame at the Y axis' x, the X title rotated left
of the frame at the X axis' height. Sheaf: template `scatter-central` (menu, `sheaf plot -t
scatter-central`) with `Axis::offset_pct` -50 on both axes (the X offset is new here: percent of the
frame height, "% From Bottom/Top" in the axis window), the frame drawn as a box, and the titles of any
axis moved into the frame placed at its end as above; offsets 0 and outward (T61) are unchanged. New
layers start at offset 0. `.opj` notes a lost X offset as it does a Y one.

### 20.1 Sheaf's rules

**Superseded in part by 20.4 and 20.6** (Origin measured, Sheaf changed to follow it): the click in Mask mode,
Remove Bad (pick, then Enter), the undo labels, the reach, the plots a mask acts on (Tab) and the ending of a mode
are as 20.6 says; the paragraphs below keep their [Sheaf choice] text where 20.6 does not replace it (Move
entirely, hidden masked points, clipping, Skip Points, waterfall).

- **Any shown plot of any layer [Sheaf choice].** A tool acts on the points of the plots the pointer picks, the
  nearest point that the graph draws within the Data Reader's reach (30 screen points), in any layer; a
  rectangle masks the drawn points of every plot inside it. Sheaf has no active plot (only an active layer), so
  the brief's "active plot only" is not what is built: the pointer decides, which is what "active plot" comes
  to when the plots do not overlap. Origin may act on the active plot only, or on all plots of the layer [?].
  A point that is not drawn cannot be picked: a hidden plot, a layer with Show Elements Data off, a row that
  Skip Points leaves without a symbol (the rows of `PlotData::point_rows`: the first of every n shown rows,
  missing values counting), a masked point under Hide Masked Points, and a point outside the clip rectangle
  (the axis range; with Clip Data to Frame off the point is drawn and so can be picked).
- **Which plots [Sheaf choice].** Line, Scatter and Line + Symbol plots. Other kinds ignore the tools (bars,
  areas, boxes and so on have no single point to pick), and so do polar and ternary layers.
- **Mask Data Points [Sheaf choice].** While the tool is on, a drag draws a rectangle (screen rectangle,
  corner to corner); when the button is let go, the Y cell of every picked-eligible point inside it is masked.
  A point is inside when its drawn position (after multipliers, offsets and the axis scale, so on a log axis
  the rectangle is a rectangle on the screen) is inside the rectangle, edges included. A drag that covers no
  point does nothing. A click without a drag on a masked point unmasks it; a click on an unmasked point does
  nothing. Only the plot's Y cell is masked, not its X cell, as Column > Mask on the Y column does. Origin
  may select by clicking, by a region of another shape, or add Ctrl/Shift modifiers [?].
- **Masked points that are hidden.** With Hide Masked Points on a masked point is not drawn, so it cannot be
  picked and cannot be unmasked from the graph (switch Hide Masked Points off first). With Disable Masking on,
  masks are not applied: a rectangle still masks (the flag is kept, as in the worksheet) but nothing changes
  on the screen [Sheaf choice].
- **Remove Bad Data Points [Sheaf choice].** A click on a point sets the plot's Y cell to a missing value.
  The row is kept: the X cell, the other columns and the row numbers do not change, and the line breaks at the
  gap as it does for a missing value typed in the worksheet. The tool stays on for the next click. Whether
  Origin deletes the row or only the Y cell [?]; the brief of T48 asks for rows to be kept.
- **Move Data Points [Sheaf choice].** A drag starting on a point moves it; the pointer's final position, taken
  back through the axis scale (log axes), the plot's offset and multiplier (Individual stack and waterfall,
  spec 13) and the Y axis of the plot's layer, becomes the point's new X and Y: both cells change, and for a
  plot without an X column (drawn against the row number) only the Y cell. Origin may move Y only, or ask for
  a modifier to move in one direction [?]. The point moves by the distance the pointer is dragged: it is
  dropped at the pointer's position less the offset at which it was grabbed (the grab may be up to 30 points
  from the point), so a drag that returns to where it began (within half a pixel) writes nothing and makes
  no undo step. In a waterfall the pointer is taken back by the plot's shift first, so the point lands where
  it is dropped. Two cases re-lay the graph out after the move, so other points may appear to jump: in an Auto
  stack the offsets come from each curve's extent, so dropping a point below the curve's minimum or above its
  maximum changes the offsets of that curve and of the ones above it; and curves that share an X column
  move that row's X for all of them (the cell is the shared one).
  The new numbers are the exact converted values, not rounded to the grid or to the screen pixel. A point
  dropped outside the frame is moved there too (the axes do not rescale at once, as for any edit).
- **Ending a mode [Sheaf choice].** Esc or the Pointer button of the Tools toolbar leaves the mode (the same
  as for the Data Reader and the drawing tools). Origin's double-click or right-click to end [?]. The Tools
  toolbar shows no tool pressed while a Data menu mode is on (Origin has no toolbar button for them), and the
  status bar names the mode and how to end it.
- **Cursor [?].** Sheaf shows a crosshair for Mask and Remove Bad and a move cursor (a four-way arrow) for
  Move; Origin's shapes were not captured.
- **Nothing to act on.** On a graph without a plot, or when the pointer is not within reach of a point, the
  tools do nothing (no message, no undo step).

### 20.2 What a change does to the worksheet (core, `graph_data.rs`) [Sheaf choice]

Each action is one `Document::edit` step, so one Undo reverts it and Redo brings it back; analysis and plots
follow as they do for worksheet edits and masks. The steps are labelled by the Y column's short name and
designation and the row counted from 1 (`Document::mask_points`, `remove_bad_points`, `move_point`):

| Tool | Undo label | Cells changed |
|---|---|---|
| Mask a rectangle | `Mask 4 points of B(Y)` | the masks of the Y cells (`Unmask 1 point of B(Y)` for a click on a masked point) |
| Remove Bad | `Remove bad point B(Y) row 12` | the Y cell becomes missing |
| Move | `Move point B(Y) row 12` | the Y cell and, when the plot has one, the X cell |

- Nothing is recorded (no undo step, no dirty mark) when no cell would change: a mask that is already set,
  a missing value removed again, a point dropped where it was. A row beyond the data of a column is
  skipped for a mask and for Remove Bad, and is an error for Move (nothing changes). Several cells in the
  same column and row count once.
- **Formula columns follow the worksheet's typing rule.** A column whose values come from a formula that keeps
  running (Recalculate Auto or Manual, `Column::is_locked`; workbook-model 10a rule 8) refuses typed values:
  the grid, Fill, Paste, Clear, Replace, Insert Rows and Delete Rows leave it alone and the app says "That
  column is calculated from a formula; change the formula in Set Column Values". Remove Bad and Move do the same
  (nothing changes, `DataEditReport::locked` says how many columns refused, and the app shows that message).
  Move is all or nothing: if either the X or the Y column is calculated, neither cell changes. A mask is
  not a typed value; the worksheet's Column > Mask works on such columns too, so the graph's mask does.
  Columns that an analysis tool wrote (`Column::analysis`) are not locked in the worksheet and are not here;
  a later recalculation of the link overwrites an edit, as for a typed edit.
- **Recalculating columns and analysis links.** Auto formulas that read a changed cell recalculate inside the
  same undo step, so Undo restores source and results together (workbook-model 10a rule 3). Auto analysis links
  (Smooth, Simulate Curve and the other linked tools) run again when a source column's values **or its mask**
  change, so masking a point from the graph updates a linked result as masking in the worksheet should (before
  T48 a mask change did not touch links; this is a fix of `update_analysis_links`). A Manual link is marked
  pending. Formulas do not read masks, so a mask changes no formula column.
- **Text and Numeric columns.** Remove Bad also clears the text of a Text & Numeric cell. A moved value is a
  number, so it replaces a text cell. A column that holds only text cannot be moved (error).
- **Not finite values.** A point cannot be moved to NaN or infinity (an error; the app's mapping gives NaN for a
  pointer outside the range of a log axis, in which case it does nothing).

### 20.3 Hit testing, modes and overlay (app, `data_tools.rs`, `reader.rs`, `graph_view.rs`) [Sheaf choice]

- **Menu.** Data > Mask Data Points, Move Data Points and Remove Bad Data Points are enabled on a graph and
  start the mode (`GraphTool::Data`); the other Data items stay grey (they need a column display range). No
  check mark is shown on the item [?]. Choosing another item, the Data Reader or another tool replaces the
  mode; Esc or the Pointer button ends it. The Tools toolbar shows no button pressed. The status bar names
  the mode and how to end it ("Mask Data Points: drag a rectangle over the points; click a masked point to
  unmask it; Esc ends").
- **Picking is the Data Reader's code** (`reader::each_point`, shared; `Pick::Edit` is the stricter variant):
  plot data are the worksheet columns as they are (`plot_xy`), the plot's row range, rows that fail an
  active worksheet filter, non-visible plots and non-finite values are not pickable; the multiplier/offset
  place of the plot (`stack_places`), the layer's axes (`data_to_page`) and, in a waterfall, the plot's shift
  (`sheaf_plot::waterfall_shift`: member k is `k * (dx% * w, -dy% * h)` of the front frame) give the drawn
  position, for the Data Reader as well; error bars are not points. `Pick::Edit` also skips plots that are
  not Line, Scatter or Line + Symbol, polar and ternary layers, layers with Show Elements Data off, rows that
  Skip Points does not draw, masked points while Hide Masked Points is on, and points whose page position is
  outside the layer's clip rectangle (`LayerGeometry::clip`, 0.5 pt tolerance, unless Clip Data to Frame is
  off) because the renderer does not draw them. The reach is 30 screen points, the nearest point wins, in any
  layer.
- **Mask.** A drag (egui's drag threshold) draws a translucent rectangle with a ring on each point that
  would be masked; letting go sends one action with the Y cells of those points. A click masks nothing:
  it unmasks the nearest *masked* point within reach (so an unmasked neighbour is never the target). A point
  counts as masked as drawn, so with Disable Masking on a click does nothing.
- **Move.** A drag that starts within reach of a point picks it; while dragging, a ring follows the pointer
  and a line joins it to the point's place. On release the pointer goes screen, page, `page_to_data` (the
  axis scales, so log and reciprocal axes are handled), then the plot's inverse placement (`Place::source_x`
  and `source_y`: subtract the offset, divide by the multiplier) and the two numbers are written to the X
  and Y cells (Y only when the plot has no numeric X column); in a waterfall the plot's shift is taken off the
  page position first. The pointer position used is the final one less the grab offset (see 20.1). A position
  that is not finite (outside a log axis) writes nothing. A click or a drag that does not start on a point does nothing.
- **Remove Bad.** A click on the nearest point within reach sends its Y cell.
- **Esc** pressed during a drag drops the drag without writing anything and ends the mode.
- **Hover.** The point that a click would act on (for Mask, the nearest masked one) gets a ring.
- **Nothing to act on.** No plot, no point within reach or a column that a formula keeps up to date (message
  "That column is calculated from a formula; change the formula in Set Column Values") records no undo
  step. Plot-only X sorting (18.9) changes only the order the line joins the points, so a picked point is
  still a worksheet row.
- `--show data-tool:mask|move|remove` and `--show mask-points:4,5` (rows from 1 of the first plot) put the
  first graph in a mode or mask rows, for screenshots (the Data Reader's `--show reader` does the same).

### 20.4 Measured in Origin 2016 (`tools/origin-ref`, 1024x768 window, `origin-dialog.ps1 -NoDialog`)

Setup of every run: a workbook with X = 1..10 (A), Yb = 2X+1 (B), Yc = 20-X (C, plain numbers unless said),
a Line + Symbol graph (or Scatter) of Yb and Yc against X in a 1024x768 Origin window, mouse and keys sent to
it, the worksheet read afterwards by LabTalk (`stats` per cell with masking off gives n=0 for a masked or
missing cell; the cell text with masking on gives `--` only for a missing cell; `wks.nRows`). Screenshots and raw
runs are local (`re-workspace/scratch/t48`). Everything below is what Origin did; where Sheaf differs it is in 20.5.

**Status of the nine items.** 1 cursor, check mark, ending: measured (Move's cursor not obtainable). 2 Mask: measured.
3 Remove Bad: measured (what a formula column does: one run, see M3). 4 Move: **unmeasured** (M4). 5 undo: measured
for Mask and Remove Bad, not for Move. 6 other plot kinds, Hide Masked Points, filter: **unmeasured**. 7 hit radius:
measured roughly (M7), highlight before the click: none seen. 8 clipped, Skip Points, Show Elements Data, waterfall:
**unmeasured**. 9 follow vs jump: **unmeasured** (Move). Origin runs are slow (about a minute per ten-row dump)
and the last session was stopped by its time limit, so these were not reached.

**M1. Menu, cursors, ending a mode.**
- The graph's Data menu lists, under the three tools, one line per plot of the active layer ("1 g1 [Book1]Sheet1!
  "X"(X), "Yb"(Y) [1*:10*]", numbered) with a check mark on the **active plot**. The three tool items show no check
  mark while their mode is on. Origin has an active plot (the first one until another is picked; the Data Reader
  click on a point of a plot makes that plot active, see M3).
- Mask Data Points: a custom cursor (an arrow with a small icon at its lower right); the status bar reads that a
  drag selects a region, Tab toggles the mode and Space toggles the shape; a yellow hint with the same words appears
  the first time (it has a "do not show again" box). The green box in the toolbar row repeats the text.
  The Tools toolbar has a mask button (own flyout, M2); the Pointer button is not pressed while Mask is on.
- Remove Bad Data Points: the graph goes into the **Data Reader** (crosshair cursor, a Data Info window with
  `[Book1]Sheet1[row]`, X and Y, a green readout `[Book1]Sheet1![5]: X = 5, Y = 11`, status text "Data Reader: Click
  to read data value. Left/right keys go from point to point ..."); the Data Reader button of the Tools toolbar is
  pressed. The point is then removed by pressing **Enter** (M3).
- Move Data Points: the mode shows no change that was visible (no status text, no cursor change, Pointer button
  stays pressed). The hollow squares seen after a click on a plot are the ordinary **plot selection** (a click on a
  point selects the plots of the layer, a second click one point with a bold square), not the Move mode.
  While the mode was on a click on the menu bar did not open the menu (the first click was swallowed) and clicks on
  points selected nothing.
- Ending: Esc ends the Mask mode; **right-click** ends it too and opens the graph's context menu (which has Mask,
  Analysis Markers and Plot Details); a double-click does **not** end it (a drag after it still masks); a click on the
  Pointer or the Data Reader button ends it (a drag after that changes nothing).
- Opening the menu with the keyboard (Alt+D, Down, Enter) works as the mouse path.

**M2. Mask Data Points.**
- Default region mode: a **rectangle** drag. When the button is let go every point of **every plot of the layer**
  inside the rectangle has its **Y cell masked** (Yb and Yc both, same rows); X cells are not masked. Masked points
  are drawn in the mask colour (red).
- **Tab** toggles between "all plots" and "active plot only" (a drag over points of both plots then masked only the
  active plot's). **Space** toggles the shape between the rectangle and a **freehand closed path** (a drag that
  traces a path; the points inside the path are masked; a triangle around one point masked just that point and a big
  loop masked every point inside in both plots). The re-entry state after the menu is chosen again was not measured
  (every run started fresh with all plots, rectangle).
- A **click** (no drag) on an unmasked point **masks** it (the point must be within the hit radius, M7); a click on
  a masked point leaves it masked; Ctrl, Shift or double-click make no difference to a drag or click.
- The Tools toolbar mask button has a flyout of four modes: Mask Points on Active Plot, Mask Points on All Plots,
  Unmask Points on Active Plot, Unmask Points on All Plots. A rectangle drag with an Unmask mode **unmasks** the
  points inside (all plots or the active one). That is the only way to unmask from the graph: the Data menu's tool
  masks only. The flyout choice reached by the menu path was not kept: choosing Data > Mask Data Points after the
  flyout's Active Plot mode still masked both plots.
- Esc, right click and the Pointer or Data Reader button end the mode before a drag, then the drag does nothing.
- One Undo step per drag or click; Edit shows **"Undo Set Mask Data"** after a mask drag (a second Undo step below
  it is the graph's creation). The Tools toolbar mask button's tip reads "Mask Points on Active Plot"; the Data
  Reader tip is "Get data point".

**M3. Remove Bad Data Points** (Data Reader plus Enter).
- A click on a point (the reader picks it, shows its Data Info) changes nothing by itself. **Enter** then removes
  the point. The arrow keys Left/Right move the reader from point to point and Enter removes the one reached (after
  menu choice, no click, two Right keys reached row 7 and Enter removed Yb row 7). Delete, Backspace, Ctrl+Delete,
  double-click and Ctrl+click do nothing; Enter in the plain Data Reader (button of the toolbar, not via the menu)
  removes nothing.
- The system variable `@YPR` (default 1) decides what is removed: with 1 the plot's **Y cell becomes missing**
  (the row stays: X cell, other columns and `wks.nRows` unchanged, the text `--`); with 0 the **whole worksheet row**
  is deleted (every column below shifts up, the last row of the data becomes missing).
- The mode stays on after a removal (a second click and Enter removed a second point; one of the other plot works).
- One Undo (Ctrl+Z) restores all of it; Edit shows **"Undo Remove Data Point"** (for `@YPR` 1 and 0 and for a
  formula column).
- A column set by a Set Column Values formula with Auto recalculation (Yc = 20 - X): Enter on its point: **inconclusive**.
  The worksheet dump after the run showed no missing cell in the formula column (only the earlier Yb cell), so it is
  not known whether Origin clears or refuses a formula column; unmeasured.

**M7. Hit radius.** Clicking at a measured pixel distance from a plot's point (below, above, right or left)
masked it at 2, 4, 6 and 8 pixels and not at 9 and more pixels for the default symbol size; for symbol size 24
masked at 6, 10 and 14 and not at 15 or more, for symbol size 3 masked up to at least 9. So the radius is about
the half symbol size plus a margin and not below 8 to 9 pixels. [Re-measured below if better pinned.]

**M4. Move Data Points: not measured.** In about thirty Origin runs (menu by mouse and by Alt+D with the key
count, with and without a plot or a single point selected first, with the Data Reader point set first, `@MDS=20`
to enlarge the points, scatter and line + symbol, one and two plots, drags with `SetCursorPos` moves, with relative
and with absolute `mouse_event` moves, slow and fast, from points and from offsets of 2 to 6 pixels around them, with
Ctrl, Shift and Alt held, double-click first, click-move-click, arrow keys and Enter) no cell ever changed and the
screen showed no drag feedback. Whether Move needs input that synthetic mouse events do not give, or a state not
found, is unknown: what Move writes (X and Y or Y only), snapping, log axes, formula columns, Esc during a drag,
modifiers and the undo text are all **unmeasured**. Origin's system variable `@MDS` is the size of the points of the
Move tool (default -1, size 6), which is the only fact found about it.

**M5. Undo.** One step each for a mask drag/click ("Undo Set Mask Data") and for a removal ("Undo Remove Data Point",
`@YPR` 0 or 1); the worksheet shows the change at once (read right after the action).

### 20.5 Where Sheaf differed from Origin (before part 2; what was done is in 20.6)

1. **Active plot.** Origin has one; Sheaf has none. Origin's Mask default acts on all plots of the layer by region;
   Tab switches to the active plot, and the toolbar flyout offers active-plot and all-plots modes. Sheaf: the pointer
   decides, a rectangle masks every plot (same as Origin's default).
2. **Click in Mask mode.** Origin: a click on an unmasked point masks it, a click on a masked point does nothing,
   unmasking is only through the Unmask modes (rectangle). Sheaf: a click on a masked point unmasks it, a click on an
   unmasked point does nothing. Sheaf has no Unmask modes, no Tab (active plot) toggle, no Space (freehand shape).
3. **Remove Bad.** Origin: pick with the Data Reader (click or Left/Right) and confirm with Enter; Y cell missing
   (rows kept) with `@YPR` 1, whole row deleted with 0; undo text "Undo Remove Data Point". Sheaf: a click removes at
   once, always only the Y cell; no row-delete variant; no formula-column difference was measured (the one Origin run
   was inconclusive; Sheaf refuses).
4. **Ending.** Origin: right-click ends and opens the context menu, double-click does not end. Sheaf: Esc or Pointer
   ends (Esc matches; right-click and double-click not defined).
5. **Menu.** Origin lists the plots under the tools with the active plot checked; Sheaf has no such list. No check
   on the tool items in either.
6. **Undo labels.** Origin "Undo Set Mask Data" and "Undo Remove Data Point"; Sheaf "Mask 4 points of B(Y)" and
   "Remove bad point B(Y) row 12" (Sheaf's own wording may stay).
7. **Hit radius.** Origin about 8 to 9 pixels for the default symbol (more for larger symbols); Sheaf 30 points.
8. **Move.** Unmeasured; Sheaf's design stays [Sheaf choice].

### 20.6 What Sheaf does now (part 2, following 20.4)

Changed to match Origin (core unit tests in `graph_data.rs`, UI tests in `ui_tests/graph_data_tools.rs`):

1. **Mask click.** A click on an unmasked point masks it (that plot's Y cell); a click on a masked point leaves it
   masked and records no undo step. The old rule (click unmasks) was wrong and its test was replaced.
2. **Plots.** The drag masks the Y cell of every plot of the layer in the region (X cells never). Tab toggles
   "active plot only" (the label in the corner of the graph names the mode: "Mask rectangle on all plots"). Sheaf's
   active plot is the first plot until a click picks a point of another plot (Origin: the same, M1). The state is
   reset when the mode is left, so re-entering starts with all plots; Origin's re-entry is unmeasured.
3. **Shapes and unmask.** Space switches the rectangle to a freehand closed path (even-odd rule, the points inside
   the traced path). Origin unmasks only from its Tools toolbar flyout; Sheaf has no such button, so **U** toggles an
   unmask mode (rectangle, path and click) [Sheaf choice].
4. **Remove Bad.** A click (or Left/Right, `reader::step`) picks a point, drawn with a heavy ring; **Enter** makes its
   Y cell missing and the mode stays on; Delete, Backspace and double-click do nothing; Enter with nothing picked
   does nothing; nothing is picked after a removal. Not built: the row-delete variant of `@YPR` 0 (Sheaf has no
   system variables) and the starting point of Left/Right without a click (Origin's was not measured; Sheaf needs a
   pick first). A formula-kept column is still refused with the message (Origin's behaviour was not obtained: one inconclusive run).
5. **Undo text.** "Set Mask Data" (mask and unmask; Origin's unmask text was not measured) and "Remove Data Point"
   (Edit shows "Undo Set Mask Data", "Undo Remove Data Point"). Move keeps Sheaf's "Move point B(Y) row 12".
6. **Reach.** The Mask click picks within max(9, half the symbol size + 2) screen points (default symbol about 9,
   size 24 about 14: M7, measured for the Mask click only); Remove Bad's click and Move's grab keep 30, like the Data
   Reader, since no reach was measured for them [Sheaf choice]. The hover ring was dropped (Origin showed no highlight).
7. **Ending.** Esc or a right-click ends the mode (the right-click also reaches the context menu); a double click does
   not end it. The Pointer and Data Reader buttons end it as before.

Still different: the Data menu has no plot list with a check on the active plot; no Tools-toolbar mask flyout (U
instead); Move is Sheaf's design, unmeasured (M4): the reach for the grab (30), X-and-Y, follow-by-distance, undo text
stay [Sheaf choice]. Unmeasured and left: items 6 and 8 (other plot kinds, Hide Masked Points, filters, clipped
points, Skip Points, Show Elements Data, waterfall), the exact hit radius, log axes, column plots.

## 21. Layers, insets and three/four Y axes (T61, measured 2026-11-15, Origin 2016)

Everything here was measured in OriginPro 2016 on our own synthetic data (21 rows, X = 0 to 10, five Y columns of very different size) through COM/LabTalk and by driving the menus (`tools/origin-ref`). Raw outputs, pictures and the generator scripts stay in the git-ignored `re-workspace/scratch/t61/`. Anything not listed was **unmeasured**. Layer numbers are 1-based as in LabTalk; frames are in the layer's own unit (`layer.unit` 1 = % of page, 7 = % of linked layer); `showAxes` of an axis is 1 = bottom/left only, 2 = top/right only, 3 = both, 0 = none (`showLabels` follows it).

### 21.1 Graph > New Layer(Axes) and the Graph toolbar

**Menu** (Graph menu of Origin 2016; the short menu shows Add Plot to Layer, Layer Management..., Add Straight Line..., New Layer(Axes), Merge Graph Windows..., Legend, Fit Page to Layers..., Exchange X-Y Axes; the full menu adds Layer Contents... F12, Plot Setup..., Add Error Bars..., Add Function Graph..., Rescale to Show All Ctrl+R, Add Axis Scrollbar, Extract to Graphs..., Apply Palette to Color Map..., Add Graphs to Layout..., Speed Mode..., New Table..., New Color Scale, New Bubble Scale, New XY Scaler, Set Active Layer By Layer Icon Only, Fit Layer to Graph, Offset Grouped Data in Layer, Convert to Standard Font Size). **New Layer(Axes)** has nine items and then a separator and "Open Dialog..." (the `layadd` dialog):

1. Bottom-X Left-Y
2. Top-X (Linked Y Scale and Dimension)
3. Right-Y (Linked X Scale and Dimension)
4. Left-Y (Linked X Scale and Dimension)
5. Top-X Right-Y (Linked Dimension)
6. Bottom-X Right-Y (Linked Dimension)
7. Inset (Linked Dimension)
8. Inset With Data (Linked Dimension)
9. No Axes (Linked XY Scale and Dimension)

**There is no "Horizontal Axis Layer..." or "Vertical Axis Layer..." item in Origin 2016** (neither in the menu nor in `layadd`'s `type` list). The two greyed Sheaf items have no reference behaviour and are reassigned (21.8).

**Toolbar.** The vertical Graph toolbar has six layer buttons, top to bottom, with these tooltips: "Add Bottom-X Left-Y Layer" (tooltip text "Add a new layer (X axis at the bottom and Y axis on the left)"), "Add Top-X Layer", "Add Right-Y Layer", "Add Top-X Right-Y Layer", "Add Inset Graph" ("Add a linked inset layer to the active layer") and "Add Inset Graph With Data" ("Add a linked inset layer and include the data from the active layer"). They add the layers of menu items 1, 2, 3, 5, 7 and 8 (the same result in the picture; Bottom-X Right-Y, Left-Y and No Axes have no button). Left-Y and Bottom-X Right-Y exist only in the menu and in `layadd`.

**LabTalk.** `layadd type:=NAME;` with `NAME` = `normal`, `topx`, `righty`, `lefty`, `txry`, `bxry`, `inset`, `insetdata`, `noxy`. The layer is appended as the last layer, becomes the active layer (`page.active` = new index) and is named by `layer.name$` as in the table. `layadd userdef:=1 bottom:=B left:=L top:=T right:=R linkto:=P xaxis:=none|straight|custom yaxis:=none|straight|custom;` makes any combination (measured: `bottom:=1 left:=0 top:=1 right:=1 linkto:=1 xaxis:=straight yaxis:=custom` gave link 1, x.link 1, y.link 2 = custom, both X sides shown (`x.showAxes` 3) and the right Y side (`y.showAxes` 2); `layadd`'s `offset` and `activate` options were not measured).

**Parent.** The parent of the new linked layer is the **active layer** (`layer.link` = its number): with three layers and layer 2 active, `type:=righty` gave link 2; `righty` then `topx` gave the top-X layer link 2. A linked layer's `unit` is 7 (% of linked layer) and its frame is the parent's, `0 0 100 100`.

Measured on a plain Line graph (parent frame left 17.91, top 11.58, width 68.16, height 71.83 % of page; X -2.5 to 12.5, Y -25 to 125; the new layer is empty, so its own scales are the empty defaults 0 to 10, step 2):

| type | `layer.name$` | link | x.link / y.link | frame | axes shown X / Y | scales |
|---|---|---|---|---|---|---|
| normal | BottomXLeftY | none (0) | 0 / 0 | unit 1, 15.59 / 8.531 / 68.17 / 71.83 (first run; 20.69 / 7.963 in a second run after data was plotted; the offset from the parent was **not** constant) | 1 / 1 | own |
| topx | TopX | parent | 0 / 1 (straight) | 0 / 0 / 100 / 100 | 2 / 0 (top line, ticks and labels) | X own, Y = parent's |
| righty | RightY | parent | 1 (straight) / 0 | 0 / 0 / 100 / 100 | 0 / 2 | X = parent's, Y own |
| lefty | LeftY | parent | 1 / 0 | 0 / 0 / 100 / 100 | 0 / 1 (a second left axis over the first) | X = parent's, Y own |
| txry | TopXRightY | parent | 0 / 0 | 0 / 0 / 100 / 100 | 2 / 2 | both own |
| bxry | BottomXRightY | parent | 0 / 0 | 0 / 0 / 100 / 100 | 1 / 2 | both own |
| inset | Inset | parent | 0 / 0 | 75 / 0 / 25 / 25 (left / top / width / height, % of linked layer) | 1 / 1 | own (empty 0 to 10) |
| insetdata | InsetWithData | parent | 0 / 0 | 75 / 0 / 25 / 25 | 1 / 1 | **copied** from the parent (-2.5 to 12.5, -25 to 125) but not linked; the parent's first plot is plotted again in it |
| noxy | NoAxes | parent | 1 / 1 | 0 / 0 / 100 / 100 | 0 / 0 | both = parent's |

Pictures (PNG exports of each type are in the scratch folder): top-X, right-Y and top-X right-Y layers show ticks pointing out, tick labels and the placeholder titles "X Axis Title" / "Y Axis Title" until data is plotted in them; an empty layer has no curve. The inset is a small box in the parent's top-right corner (left 75 %, top 0 %, 25 % by 25 % of the parent's frame, so it overlaps the parent's own data and legend) with only a bottom X and a left Y axis, no top or right line, and tick label, title and legend fonts scaled down with the layer. Plotting data into any new layer gives it its own legend; with data, right-Y and inset layers show the data's long name and units as the axis title (for example "Sine (mA)"), rotated like the left title, to the right of the right axis. The inset with data keeps the parent's X and Y title text.

Plotting into a new layer: `plotxy iy:=[Book]Sheet!(1,3) plot:=200 ogl:=[Graph1]2!;` (layer 2). Autoscale of an unlinked scale fits that layer's data (an inset's Y became -0.075 to 0.075 for the sine column); a linked scale follows the parent.

### 21.2 Toolbar and menu facts that the app copies

- Undo text after adding any of these layers (button or menu): "Undo Create".
- The graph shows layer icons "1 2 3 ..." in its top-left corner; the active layer's icon is dark, the others grey (see 21.3).

### 21.3 Layer icon: right-click menu, Hide, Layer Properties

**Icons.** A graph with several layers shows numbered icons in the top-left corner of the window; clicking one makes that layer active. Right-clicking an icon (the same menu for the first and for any other layer) shows:

1. Hide Layer (a check mark when the layer is hidden)
2. Hide Other Layers
3. Show Active Layer Only
4. Set Active Layer By Layer Icon Only
5. Delete Layer
6. separator; Layer Contents... (F12); Plot Setup...; Double click to open Plot Setup
7. separator; Axis...; Layer Properties...; Layer Management...
8. separator; one row per plot of the layer ("1  [DQ]Sheet1! A(X), B(Y) [1*:21*]", ticked)

(The effects of "Set Active Layer By Layer Icon Only", "Show Active Layer Only" and "Hide Other Layers" were **unmeasured**; only their presence in the icon menu, and for the first also in the Graph menu, is known.)

**Hide Layer** hides everything the layer draws: its curves and its axes, ticks and labels disappear from the window (the other layers keep their positions); it is a toggle on the same menu item. It does not delete anything. It did not add an Undo entry (the Edit menu still said "Undo Create" from the layer added before; weak evidence).

**Layer Properties...** opens the Plot Details dialog (title "Plot Details - Layer Properties") with the layer's node selected in the tree on the left (the tree lists the graph, then Layer1, Layer2, ... each with a check box, and each layer expands to its plots). The right side has five tabs for a layer:

- **Background**: Color (None), Border Style (None), Gradient Fill: Mode (None), Color (Black), Direction (Top Bottom).
- **Size/Speed**: Layer Area Left / Top / Width / Height with a Units combo (measured for a linked layer: 0, 0, 100, 100, "% of Linked Layer"); "Link Axis Length to Scale with X:Y Ratio" (off, value 1); "When dragged to reposition, also reposition parent layer" (on); Graphic Image Caching (None); Speed Mode, Skip Points if needed: Worksheet data, maximum points per curve (on, 3000), Matrix data (off, 32 by 32).
- **Display**: Scale Elements: Scale With Layer Frame (selected) or Fixed Factor (0.87 greyed); Show Elements: X Axes, Y Axes, Labels, Data (all on), Frame (off), Z Axes (grey); Data Drawing Options: Clip Data to Frame (on), Data on Top of Axes (off), Grid on Top of Data (off); Clipping Margins (%) Horizontal 0, Vertical 0.
- **Link Axes Scales**: "Link to" combo (the parent layer, "Layer1"); "Link Axes in Child Layer to Parent"; X Axis Link and Y Axis Link groups each with None / Straight (1 to 1) / Align (X = or Y =) / Custom (X1 =, X2 = or Y1 =, Y2 =). For a right-Y layer: X Straight, Y None.
- **Stack**: Offset None / Cumulative / Incremental / Constant / Auto (Gap 8, Keep Proportion of Plot Range) / Individual, and the stacked-column options (as section 13.14).

The dialog has OK, Cancel and Apply and a ">>" button; Apply is disabled until something changes. The axis dialog opened by Axis... or by double-clicking an axis ("Y Axis - Layer 3") has a left strip with Bottom, Top, Left, Right (or Horizontal, Vertical when the layer shows one side per direction) and the tabs Scale, Tick Labels, Title, Grids, Line and Ticks, Special Ticks, Breaks. **Line and Ticks** has, per side: Show Line and Ticks, "Use Same Options for Left and Right", Line: Show, Color (Auto), Thickness (0.2, 0.5, 1, 1.5, 2, 3, 4, 5; default 1.5), **Axis Position** with three items and **Percent/Value**; Major Ticks and Minor Ticks: Style (In & Out, In, Out, None; Out), Length (3, 5, 8, 10, 15; major 8), Color, Thickness.

**Axis Position** items: for the left axis "Left", "% From Left", "At Position ="; for the right axis "Right", "% From Right", "At Position =". "% From Right" with 12 draws the right axis 12 % of the layer's width **outside** the frame; "% From Left" with 11 draws the left axis 11 % outside; a negative value moves the axis inside (-50 puts a left axis at 50 % of the layer's width). In LabTalk the left/bottom axis reads as `layer.y.position`, `layer.y.posType`, `layer.x.position`, `layer.x.posType` and the right/top axis as **`layer.y2.position`, `layer.y2.posType`, `layer.x2.position`, `layer.x2.posType`** (`layer.y.opposite` is something else; names such as `axisPos` or `offset` do not exist). `posType` 0 = "Left"/"Right" (at the frame), 1 = "% From ..." (the value is the offset in % of the **X axis length**, outward positive), the "At Position =" type was not read. A hidden side of an axis keeps stale values (for example `posType` 1 with 0 on the left of a right-only layer), so read only the shown side.

**"At Position =" (T243; measured in Origin 2016 on 2026-11-20, T266).** In LabTalk it is
`layer.x.posType = 2` (`layer.y.posType` for the Y axis) with the value in `layer.x.position`
(probes and PNG exports in `re-workspace/scratch/t266`, local). The Axis window's Line and Ticks tab
lists Left/Right (Bottom/Top) and **At Position =**; choosing it shows **X =** (for a Y axis) or
**Y =** (for an X axis), starting at 0, and stores `Axis::at_position`. Measured: the axis line, its
ticks, tick labels and title stand where the other axis has that value (an X axis at Y = 0 on a
-4..8 axis crosses at 0, labels and title below the line; a Y axis at X = 5 stands in the middle,
labels and title on its left), and the frame edge it left is not drawn. At Y = 8, the top of the
range, the X axis lies on the top edge with its labels and title below it, inside the frame. A
value outside the range is **not** clamped: Y = -4.5 draws the X axis below the frame and Y = 8.5
above it, where they would be (Y = 20 is above the page and not seen). Sheaf does all of this as an
offset computed when the layer is drawn, following the other axis' range, scale, reversal and
breaks; a value the other axis cannot place (non-positive on a log axis, inside a break) leaves the
axis at its edge (not measured). Choosing Left/Right (Bottom/Top) again clears the value. The
`.opj` writer does not write `posType` 2 (the axis reopens at the frame in Origin) and names it, so
the `.opj.sheaf` companion keeps it. Tests: `crates/plot/src/offset_axis_tests.rs`
(`an_axis_at_a_position_...`), `crates/app/src/app/ui_tests.rs`
(`the_axis_position_can_be_at_a_value_of_the_other_axis`),
`crates/io/src/opj/write/graph_feature_tests.rs`.

### 21.4 Deleting a layer

Right-click icon > Delete Layer deletes the layer at once (no question). LabTalk: `layer -d;` deletes the **active** layer; `layer -d N;` deletes layer N. Undo text in the Edit menu afterwards: **"Undelete frame"** (Ctrl+Z). The icons renumber (4 layers: deleting layer 2 left icons 1 2 3).

Measured on the 3Ys templates (layers linked to each other, all `x.link` straight):

| state | deleted | result |
|---|---|---|
| Y-YY: L2 linked to L1, L3 linked to L2 | L2 (a parent) | 2 layers; the old L3 is now L2, **unlinked** (`layer.link` 0), its frame became the parent's absolute rectangle (page % 9.593 / 19.89 / 75.73 / 66.52, same as L1), its own scales, axes and `x.link` = 1 setting stay |
| same | L1 (the root, and the parent of L2) | 2 layers; the old L2 becomes L1 and is unlinked with the old L1's absolute frame (and keeps its right-only axis, so the graph now has no left axis); the old L3, linked to the old L2, keeps its link to it (now `layer.link` 1, frame 0 0 100 100) |
| Y-Y-Y: L2 and L3 both linked to L1 | L1 | 2 layers, **both children unlinked** (`layer.link` 0), each with L1's absolute frame, so they sit on top of each other; no axes are added back |
| Y-YY | L3 (last, no children) | 2 layers, nothing else changes |

So Origin has no refusal and no re-parenting: deleting a parent turns every child into an independent layer that keeps the parent's rectangle, and links of other layers are renumbered. The legend text is **not** rewritten: a four-layer graph whose legend lists layer 1 to 4 plots showed "B, D, E, ???" after deleting layer 2 (the entries are references by layer index, so the one pointing at the removed layer index 4 shows "???").

Not measured: deleting through the icon menu a layer that is a parent (the LabTalk `layer -d` behaviour above is taken as the same command), deleting the only layer, and the effect on `layer.y.link`.

### 21.5 The multi-Y templates (Plot > Multi-Y)

**Menu.** Plot > Multi-Y has: Double-Y, 3Ys Y-YY, 3Ys Y-Y-Y, 4Ys Y-YYY, 4Ys YY-YY, Multiple Y Axes... (21.6). The templates are `3Ys_Y-YY.otp`, `3Ys_Y-Y-Y.otp`, `4Ys_Y-YYY.otp`, `4Ys_YY-YY.otp` (and `DOUBLEY.OTP`) in the program folder. LabTalk: select the Y columns, `worksheet -s 2 0 4 0; worksheet -p 200 3Ys_Y-YY;` (the Plot > Multi-Y item is this call; columns 2 to 4 = three Y columns, column 1 is X). `plotxy ... template:=` was not used.

**Layer count and Y assignment.** The number of layers is fixed by the template (3, 3, 4, 4) **whatever the number of columns**: one Y column still makes all three layers (the two empty layers show a 0 to 1 scale and their axes); the GUI menu item behaved the same for one selected column (no refusal, no message). Y column *i* of the selection (1-based, in column order) goes to layer ((*i* - 1) mod *N*) + 1, so the first *N* columns go one per layer and the next ones go round again: with five Y columns in a 3-layer template layers 1 and 2 hold two plots each and layer 3 one (measured for both 3Ys and 4Ys, 1 to 5 columns). Plots added to a layer that already has one are drawn in the next colour of that layer's sequence (a second curve in layer 1 came out red after a black one; legend line colours in the PNG did not always agree with the curve colours, so legend colours here are **unmeasured**).

**Common to all four templates** (measured with exactly *N* Y columns):
- Layer 1 is the root (`layer.link` 0), shows the bottom X axis with labels and one left Y axis. Every other layer is linked with `x.link` = 1 (straight), `y.link` = 0 (own scale), frame `0 0 100 100` in % of the linked layer (unit 7), shows no X axis (`x.showAxes` 0) and one Y axis. The X range is shared (-1 to 11 for X = 0..10, the template's autoscale margin 8 %), each Y scale fits its own layer's data.
- The box is open: no top line and no right line on layer 1 (the right-hand axes belong to the other layers).
- Every axis (line, ticks and tick labels) uses the "automatic" colour = the colour of its layer's first plot (`layer.y.color` / `y2.color` read -9), thickness 1.5, ticks Out, major length 8. Axis titles: none (the Y title text objects hold " "), so the Y axes are identified by colour and by the legend only; the X title is layer 1's (here "Time (s)").
- One legend in layer 1 lists every plot of every layer, one row per plot as "long name (units)", in layer order; it sits above the frame (outside it). No other layer has a legend. Approximate positions in the 800 px PNG: 3Ys at the top-left (left edge about 19 % of the width), 4Ys Y-YYY at the top-right corner, 4Ys YY-YY at the top-left (about 18 %).
- Plot colours are fixed by the template (below), not by the default colour list. The PNG exporter quantises to a palette, so the greens and oranges read as (0, 132, 0) and (255, 132, 0); they are taken as Origin's (0, 128, 0) and (255, 128, 0).
- Frames are template values, not computed: the frame of layer 1 as % of the page (left / top / width / height) is given per template.

| template | layers | layer 1 frame | layer 1 | layer 2 | layer 3 | layer 4 |
|---|---|---|---|---|---|---|
| 3Ys Y-YY | 3 | 9.593 / 19.89 / 75.73 / 66.52 | left axis at the frame, black (0, 0, 0) | right axis at the frame (posType 0, offset 0), red (255, 0, 0); link 1 | right axis `% From Right` **10**, blue (0, 0, 255); **linked to layer 2** (`layer.link` 2) | none |
| 3Ys Y-Y-Y | 3 | 9.764 / 15.7 / 81.69 / 68.84 | left axis, black | **left** axis `% From Left` **-50** (the axis stands in the middle of the plot, tick labels on its left), red; link 1 | right axis at the frame, blue; link 1 | none |
| 4Ys Y-YYY | 4 | 6.856 / 21.47 / 67.72 / 64.07 | left, blue (0, 0, 255) | right at the frame, green (0, 128, 0) | right `% From Right` **12**, orange (255, 128, 0) | right `% From Right` **23**, magenta (255, 0, 255) |
| 4Ys YY-YY | 4 | 14.91 / 21.25 / 68.5 / 63.48 | left, blue | **left** `% From Left` **11**, green | right at the frame, magenta | right `% From Right` **11**, orange |

All of layers 2 to 4 are linked to layer 1 (link 1) except layer 3 of Y-YY. The offsets are in % of the X axis length as in 21.3; in the 800 px pictures the outer axes stand 62 px (Y-YY, 10.2 % of the frame width), 66 and 128 px (Y-YYY, 12.2 and 23.6 %), 59 px (YY-YY left, 10.8 %) from the frame edge, which agrees with the stored 10, 12, 23 and 11 % within the picture's resolution.

**Colour of the data in the templates.** The plot of layer *k* has colour index 1, 2, 4 (3Ys: black, red, blue) or 4, 12, 15, 6 (4Ys Y-YYY: blue, green, orange, magenta) or 4, 12, 6, 15 (YY-YY: blue, green, magenta, orange) in LabTalk's `get %C -c`, which are 1-based entries of Origin's colour list (1 black, 2 red, 4 blue are certain; 12 green, 15 orange and 6 magenta read from the pixels, not from the list). Layer 1's axes and plot in 3Ys are black (the colour is the plot's).

**PNG per template**: `re-workspace/scratch/t61/png/C_3Ys_Y_YY_3.png`, `C_3Ys_Y_Y_Y_3.png`, `C_4Ys_Y_YYY_4.png`, `C_4Ys_YY_YY_4.png` (800 px wide, local only).

**Unmeasured**: the number of columns the GUI warns about (none observed), the effect of selecting columns with two X columns, grouped column selections of several sheets, legend position values in page units, the exact text of any message for too many columns.

### 21.6 Plot > Multi-Y > Multiple Y Axes... (`plotmyaxes`)

**Dialog** ("Plotting: plotmyaxes", description "Customize Multi-Axes plot", with a Dialog Theme row, an Auto Preview check box, Preview, OK, Cancel and a preview pane on the right; the controls, top to bottom):

- **Input**: the selected columns as a range, for example `[Book1]Sheet1!((A,B),(A,C),(A,D),(A,E))` (one X/Y pair per Y column); a range picker button.
- **Plot Type**: combo Line (default), Scatter, Line+Symbol, Custom.
- **Axis and Plot Assignment**: "Number of Left Y Axes" (a number with an Auto check box, ticked), "Number of Right Y Axes" (same), "Add Middle Y Axis" (check box, off), a blue summary "4 Y axes and 4 plots in total", "Number of Plots in Each Layer" (text, default 1, hint "Separate numbers by space. Example: 1 2 3") and a blue line "Actual number of plots in each layer : 1 1 1 1".
- **Options**: "Link Axis Color to Plot" (on), "Show Y Title" (off), "Show Top X Axis" (off), "Y Axes Gap (%)" (a number, shown 10, with an Auto check box, ticked).

LabTalk: `plotmyaxes iy:=[Book]Sheet!(1,2:5) plottype:=line|scatter|linesymb|custom ly:=N ry:=N my:=0|1 number:="2 1 1" axiscolor:=0|1 ytitle:=0|1 topx:=0|1 gap:=G hide:=0|1;` (`plotmyaxes` is an X-Function; `iy` takes several ranges; `ly`, `ry` and `gap` default to `<auto>`).

**What it builds** (Origin 2016, Line template for the frame; layer 1's frame is 17.91 / 11.58 / 68.16 / 71.83 % of the page for a plain Line graph; the new layers are linked as in 21.5: `link` 1, X straight, Y own, frame 0 0 100 100 in % of linked layer):

- With *N* Y columns it makes *N* layers, one plot each, and the default split of axes is `ly` = floor(*N* / 2) left axes and `ry` = *N* - `ly` right axes (measured *N* = 2: 1 and 1; 3: 1 and 2; 4: 2 and 2; 5: 2 and 3; 6: 3 and 3). *N* = 1 makes an ordinary one-layer graph (no extra layer).
- Layer 1 is the first left axis (at the frame). Layers 2 to `ly` have **left** axes at `% From Left` = gap, 2 gap, ... (10 % steps by default). Layers `ly` + 1 to *N* have **right** axes at `% From Right` = 0, gap, 2 gap, ... (the first right axis is at the frame). With "Add Middle Y Axis" the layer after the left ones is a left axis at `% From Left` **-50** (the middle of the plot), then the right axes follow; the total of axes is still the number of columns (3 columns, middle on: 1 left, the middle, 1 right).
- `ly:=0` makes layer 1's own axis a right one (right axes at 0, 10, 20 for three columns); `ry:=0` makes all axes left axes (0, 10, 20).
- Default gap **10** (% of the X axis length); `gap:=20` gave right offsets 0, 20. With `ytitle:=1` and `topx:=1` (and `axiscolor:=0`) the automatic gap was **14** (which of the three options needs the room was not separated); an explicit gap is used as given.
- The **frame of layer 1 moves** so that the frame plus the outward offsets is centred on the page: left = (100 - W - L - R) / 2 + L, with W = 68.16, L = (number of left offsets) * gap * W / 100 and R = (number of right offsets) * gap * W / 100 (checked on six layouts, for example 12.52 for one right offset, 9.111 for two, 15.92 for one left and one right, 22.73 for two left offsets and none right, 11.15 for one right offset at gap 14); width and top do not change.
- Plot colours are Origin's default colour list in order, not the template colours of 21.5: black (0, 0, 0), red (255, 0, 0), green (0, 255, 0), blue (0, 0, 255), cyan (0, 255, 255); `plottype:=scatter` draws squares, circles and up-triangles in the same colours. With `axiscolor:=1` (default) each axis line, tick and label takes its plot's colour; `axiscolor:=0` keeps them black (the picture showed black axes). `ytitle:=1` shows "long name (units)" next to each Y axis, rotated like the left title; `topx:=1` shows the top X axis line with ticks and labels on layer 1 (`x.showAxes` 3).
- A legend lists every plot of every layer (one legend, in layer 1, top-right above the frame).
- `number:="2 1 1"` with four columns made three layers (the number of layers is the number of numbers) with 2, 1 and 1 plots in them.

**Unmeasured**: `hide:=1`, the `custom` plot type's template choice, the preview, how the dialog refuses a column count that does not fit `ly + ry (+ 1)` (the OK path was not run), the legend text format, the axis title placement values.

### 21.7 Exact LabTalk used (Origin 2016, through COM, one line at a time; setup `newbook name:=DQ sheet:=1 option:=lsname;` and a worksheet with X in column 1)

```
plotxy iy:=[DQ]1!(1,2) plot:=200 ogl:=[<new>]; win -r %H G1; win -a G1;
layadd type:=righty;                       // also normal, topx, lefty, txry, bxry, inset, insetdata, noxy
page.active = 2; layadd type:=righty;      // the parent is the active layer
layadd userdef:=1 bottom:=1 left:=0 top:=1 right:=1 linkto:=1 xaxis:=straight yaxis:=custom;
plotxy iy:=[DQ]1!(1,3) plot:=200 ogl:=[G1]2!;
page.active = 2; layer -d;                 // delete the active layer; "layer -d 1;" deletes layer 1
double pv0 = layer.link;  pv0 = layer.x.link;  pv0 = layer.y.link;  pv0 = layer.unit;  pv0 = layer.left;
pv0 = layer.top;  pv0 = layer.width;  pv0 = layer.height;  pv0 = layer.x.from;  pv0 = layer.x.to;
pv0 = layer.y.from;  pv0 = layer.y.to;  pv0 = layer.x.showAxes;  pv0 = layer.y.showAxes;
pv0 = layer.y.position;  pv0 = layer.y.posType;  pv0 = layer.y2.position;  pv0 = layer.y2.posType;
pv0 = layer.y.color;  pv0 = layer.y2.color;
string ts$ = layer.name$;  ts$ = xb.text$;  ts$ = yl.text$;  ts$ = %C;   // %C after layer.plot = n;
get %C -c cc;                              // plot colour index
win -a DQ; worksheet -s 2 0 4 0; worksheet -p 200 3Ys_Y-YY;     // 3Ys_Y-Y-Y, 4Ys_Y-YYY, 4Ys_YY-YY
plotmyaxes iy:=[DQ]1!(1,2:5) plottype:=line ly:=2 ry:=2 my:=0 number:="1 1 1 1" axiscolor:=1 ytitle:=0 topx:=0 gap:=10;
expGraph type:=png path:="DIR" filename:=NAME overwrite:=1 tr1.unit:=2 tr1.width:=800;
```

A LabTalk variable named `v1` to `v9` is not usable in `double v1 = ...;` (the call returned false; the names `pv1` or `q` work). Properties that do not exist read as -1.23456789E-300. `layer.nplots` does not exist here; count a layer's plots by stepping `layer.plot = n;` and watching `%C` repeat (the index clamps at the last plot).

### 21.8 What Sheaf takes from this

- Add Layer buttons and menu: the nine types of 21.1 (the six toolbar ones as buttons). "Horizontal Axis Layer..." and "Vertical Axis Layer..." do not exist in Origin 2016: the greyed items are reassigned to a later task (not implemented).
- The new layer's parent is the active layer; a linked layer is placed over its parent (% of linked layer 0 0 100 100); the inset is 75 / 0 / 25 / 25 of the parent.
- Delete: children of the deleted layer become unlinked and keep its absolute rectangle; other links are renumbered; Undo text "Undelete frame"; no confirmation.
- 3Ys / 4Ys: build as in 21.5; Multiple Y Axes with the defaults of 21.6; an axis position is a percentage of the X axis length, outward positive, one value per shown side.

### 21.9 How Sheaf builds layers (slice 1)

Implemented in `crates/core/src/layers.rs` (`Graph::add_layer`, `Graph::delete_layer`), the Graph > New Layer(Axes) menu, the Graph toolbar (six buttons) and the layer icon menu (Delete Layer, Layer Properties...).

- The nine kinds are as in 21.1. The parent is the active layer; Sheaf's model allows a link only to an unlinked earlier layer, so when the active layer is itself linked the new layer follows that layer's root (Origin chains links; **provisional deviation**). A Bottom-X Left-Y layer is unlinked, has the root's width and height and starts 2.3 % of the page width left and 3.05 % of the page height above it (from one measurement; Origin's offset varied, so the value is Sheaf's own). New layers hide their legend (unmeasured in Origin). Inset With Data copies the parent's limits and major step, its axis title text and its first plot, not its link. Both insets scale their tick label and title fonts by 0.25 (the inset's share of the parent; Origin's Scale With Layer Frame factor is **unmeasured**, provisional). Deleting a layer freezes the X/Y range that a child took from a straight link (the app passes the drawn ranges), as Origin keeps x.from and x.to.
- Delete follows 21.4: children are unlinked and keep their place on the page (the parent's rectangle for a layer that covers it; for an inset the inset's own place, **unmeasured**), later links are renumbered, no confirmation, undo text "Undelete frame". Deleting the **only** layer is refused (Origin's behaviour unmeasured). After a delete the active layer is the one before the deleted layer (or the first), a selection in a later layer moves up, a selection in the deleted layer is cleared, and Plot Details, Layer Contents, Plot Setup and the drawn-object window of that graph close.
- Add makes the new layer active and clears the selection; one undo step each ("Create", "Undelete frame").
- Layer Properties... (Format menu and layer icon) opens Plot Details on the layer's node with the Background tab. Sheaf's layer node keeps its Layer Contents and Legend tabs as well.
- Still grey: Show Active Layer Only and Set Active Layer By Layer Icon Only (effects unmeasured). Hide Layer is a reversible whole-layer toggle (21.13); Hide Other Layers toggles whole-layer isolation/show-all from the published contract in 21.14; its live 2016 behavior remains unmeasured.

### 21.10 How Sheaf draws a displaced Y axis (slice 2)

`Axis::offset_pct` (Y axis, default 0, kept in `.sheaf` files only when not 0) is Origin's axis position "% From Left" / "% From Right" (21.3): the distance in percent of the frame's **width** (the X axis length), outward positive, negative inward (-50 puts a left axis in the middle). Implemented in `crates/plot/src/axis_offset.rs` and used by the layout.

- The axis line, its ticks, its tick labels and its title move together; the frame, the grid, the data and the line on the opposite edge stay. The left axis of a layer moves left, a right axis moves right.
- Automatic margins: the labels of the displaced axis count in the label edges of the group, so the frame narrows exactly as it does for a right axis at the frame (the existing "Keeping labels on the page" rule). The offset is a share of the frame as finally fitted.
- Clipping: data stay clipped to the frame; the axis is drawn outside the clip, in the overlay.
- Hit testing: the displaced line, ticks and labels are the Y axis object of the layer that owns them; the line at the edge stays the object of the layer whose axis is there.
- Linked ranges: unchanged (`ScaleLink::Straight` shares the parent's limits, offset or not).
- Layer Properties/Axis dialog: Line and Ticks tab, Axis Position, a "% From Left" / "% From Right" field (vertical axes). Plot context menu: "Move to Layer > Layer N" (**Sheaf's own design, not measured in Origin**: the item, its label and its place in the menu) moves a plot to another layer with that layer's axes (the existing Move to Right Y Axis stays).
- The offset is not written to .opj yet (slice 3). A displaced Y title is placed beyond the displaced labels and can collide with the titles of the axes between; the multi-Y templates show no Y titles (21.5) and Origin raises the automatic gap when titles are on (21.6, 14 %); that rule is **unmeasured** in Sheaf.
- Origin's offset of the X axis (bottom/top) is **unmeasured** and not implemented.

### 21.11 Sheaf rules for the 3Ys / 4Ys templates and Multiple Y Axes (slice 3)

Implemented in `crates/core/src/multi_y.rs` (plans, options), `Template::Multi3YYy`, `Multi3YYY`, `Multi4YYyy` and `Multi4YyYy` (names `3ys-y-yy`, `3ys-y-y-y`, `4ys-y-yyy`, `4ys-yy-yy`; labels "3Ys Y-YY" and so on), `Project::plot_multi_y_template` and `Project::plot_multi_y`, the Plot > Multi-Y menu and `dialogs/multi_y.rs`. The same names work in `sheaf plot -t` and the MCP `plot` tool.

- **Templates** take the frames, sides, offsets and colours of the table in 21.5 (`MultiYKind::plan`). Layer 1 is the root with the bottom X axis; every other layer is linked to it with a straight X scale and its own Y scale, covers it and shows only its Y axis. The box is open, the Y axes have no titles, each axis takes its plot's colour, one legend in layer 1 lists every plot. **Provisional deviation:** layer 3 of Y-YY is linked to layer 1, not to layer 2 (Sheaf links only to an unlinked layer; the X range is the same either way).
- **Columns** go to layer *i* mod *N* (21.5), N = 3 or 4. Fewer columns than layers still make every layer (Origin did the same, no message); the empty layers draw their axis with a 0 to 1 scale. No column at all is refused ("a multiple Y axes graph needs at least one Y column"). `--y2` and `--panels` are refused with these templates.
- **Colours**: the layer's colour first, then Origin's colour list without it, so a second plot in the layer takes the next colour. The stock legend anchors, logical font sizes and background are measured and implemented in 21.12; shared spacing and extra-column membership remain provisional.
- **Multiple Y Axes...** follows 21.6: Plot Type (Line, Scatter, Line + Symbol; Custom is not offered), Number of Left/Right Y Axes with Auto (floor(N/2) left, rest right), Add Middle Y Axis (-50 %), Number of Plots in Each Layer (the sum must equal the number of columns, a layer cannot be empty), Link Axis Color to Plot, Show Y Title, Show Top X Axis (line and ticks only; tick labels on the top axis are not drawn), Y Axes Gap (Auto 10, and 14 when Y titles or the top X axis are on: which of the options causes it is **unmeasured**, Sheaf uses titles or top X; `axiscolor:=0` alone is unmeasured). The first frame is centred with the outward offsets by the formula of 21.6; one Y column makes an ordinary graph with a left axis and the frame at 17.91 %. A count of axes that does not fit the number of layers is reported in the dialog and nothing is made. The Preview button and `hide` are not built.
- **Files**: `.sheaf` keeps `offset_pct` and the layer links. The `.opj` writer writes one ordinary layer per Y axis but no axis position; the save names "the position of a Y axis away from the frame" so the `.opj.sheaf` companion keeps it (ADR 0008). CLI and MCP have the four template names only; the Multiple Y Axes options are in the app.
- Goldens: `crates/plot/tests/golden/multi-y-3ys-y-yy.svg`, `-3ys-y-y-y`, `-4ys-y-yyy`, `-4ys-yy-yy` and `multi-y-multiple-axes.svg`.

### 21.12 Stock multi-Y legend anchors (T61 follow-up)

Measured in Origin 2016 on 2026-11-16, using our own X = 0,1,2 and synthetic Y columns
named Alpha, Beta, Gamma, Delta and Extra. Four normal controls, twelve long-name/one-Y/
five-Y controls and eight post-export readbacks are in local
`scratch/multi-y-template-legends/{templates,vary,post}.*`; EMF records and property
readbacks are retained there. The private runner held the reference mutex, refused existing
COM instances and closed its own instance; all three runs completed normally.

Origin's [graphical object guide](https://docs.originlab.com/labtalk/guide/creating-and-accessing-graphical-objs/)
defines `x1`/`y1` as the top-left in axis units. The table converts their post-export
readbacks to fractions of the layer frame: `(x1-x.from)/(x.to-x.from)` and
`(y.to-y1)/(y.to-y.from)`. This avoids depending on the data scale or text width.

| Stock template | Left / frame width | Top / frame height | Legend font size |
|---|---:|---:|---:|
| 3Ys Y-YY | 0.1248203654 | -0.2112977099 | 22 pt |
| 3Ys Y-Y-Y | 0.1507422916 | -0.1956329301 | 22 pt |
| 4Ys Y-YYY | 1.0027548209 | -0.3123018389 | 22 pt |
| 4Ys YY-YY | 0.0583295506 | -0.2771200000 | 20 pt |

The values were identical for one and five Y columns despite their different Y ranges.
Longer names expand rightward from the same left edge, rather than moving the left edge
around a fixed centre. Native properties report layer attachment, automatic text anchor,
black text, and no background (`attach=0`, `anchor=0`, `color=1`, `background=0`);
EMF controls identify Arial. A blank `font$` and experimental unsupported property reads
were excluded from conclusions. Negative top fractions place the legend above the frame;
the Y-YYY legend also starts just beyond its right edge.

Sheaf's compatible `LegendPosition::ManualTopLeft { x, y }` retains this anchor as labels
grow, while the existing `Manual` remains centre-based. The new anchor aligns the visible
sample/glyph/frame bounds used for picking, excluding unpainted internal padding. A measured
first pass followed by centre placement keeps the last row above the plot rather than
allowing a padded-box offset to collide with a top tick label. The four named template plans use
the measured points, font sizes, no frame and no background. Independently constructed Multiple Y
Axes dialog plans keep their existing defaults. The existing Legend properties page labels
the coordinates as top-left and allows values outside the frame without clamping on Apply.
The Cartesian and parallel legend/scale paths handle the new anchor consistently.

`.sheaf` and Origin companions preserve the anchor variant. The `.opj` writer places the
annotation rectangle using its top-left plus half its width/height, and warns that the
anchor was written as a fixed position. A bare native reopen therefore has the existing
centre-based representation; the companion restores dynamic top-left placement.

This closes the provisional **anchor/font/background** defaults in 21.11. It does not
establish exact native row spacing, padding or symbol lengths: those still use Sheaf's
shared legend metrics. Native raw template text references the first plot of each layer,
but the five-column export showed additional labels not explained by the raw text readback;
extra-column legend membership and empty-layer row translation remain follow-ups. The
existing automatic graph-wide legend contents are unchanged. No layer assignment, axes,
links, dialog-plan defaults or worksheet behavior is changed here.

### 21.13 Reversible layer visibility (T61 follow-up, 2026-11-16)

Fresh Origin 2016 controls use our own X = 0, 0.5, ..., 10 and three Y columns
(X squared, 0.05 sin(X), 10+X), plotted with the stock 3Ys Y-YY template. The
private COM runner held the reference mutex, verified a newly created process,
and closed its own instance and watchdog; pre-existing process identities were
preserved. Five accepted 800 x 612 PNG exports and per-layer readbacks are local
in `re-workspace/scratch/t61-hide-layer-axes/native-1/`.

Origin's [layer command reference](https://docs.originlab.com/labtalk/ref/layer_options_for_layer/)
provides `layer -h 1` and `layer -h 0`; its
[layer properties reference](https://docs.originlab.com/labtalk/ref/layer-obj/)
also exposes `layer.show`. The installed 2016 controls confirm:

- Hiding layer 2 changes only its `layer.show` from 1 to 0. Its curve and right
  axis disappear; the other two curves and axes retain their positions. The
  graph legend, owned by layer 1, keeps the hidden curve's entry.
- Hiding layer 1 through `layer.show=0` removes its curve, X/left-Y axes, titles
  and graph legend. Layers 2 and 3 still draw their curves and right axes.
- `layer.showData`, `layer.showx`, `layer.showy`, frames, links and both axis
  ranges remain unchanged in all three layers. Hiding layers 2 and 1 changes
  3,985 and 8,347 pixels respectively; each corresponding restore export has
  zero different pixels from the original.

Sheaf stores a compatible `LayerDisplay.hidden` flag (false when absent),
independent of individual plot visibility and Display-tab switches. Newly added
layers start visible even when their parent is hidden (Sheaf creation policy;
the follow-up native creation probe failed before COM activation and is excluded). The icon
menu checks Hide Layer while hidden and uses the same entry to show it again.
Hidden layers contribute no drawing or pickable frame; reader/edit tools ignore
their points. Geometry, scale resolution and layer indices remain intact so
linked visible layers do not move. A graph-scoped legend follows its owner's
visibility and retains sibling entries, matching the controls above. Other
layer-owned annotations are suppressed too; specific annotation types beyond
the native legend/title controls have not been measured separately.

Each toggle uses one existing "Hide layer" Undo step (Sheaf policy; Origin's
Undo behavior remains weakly established in 21.3). `.sheaf` and `.opj.sheaf`
companions retain visibility; the native `.opj` hidden bit remains unmeasured,
so saves explicitly warn that bare Origin files show the layer. Hide Other
Layers follows the published contract in 21.14; the disabled icon-mode commands remain separate follow-ups.

### 21.14 Hide Other Layers (T61 follow-up, documented contract)

The published [Origin guide](https://docs.originlab.com/origin-help/labels-data-layer-hide/)
describes isolating the active layer through Hide Other Layers and displaying all layers
by choosing the entry again. Layer visibility affects graph output as well as its window.
This is a published-document check on 2026-11-16, not a new Origin2016 measurement:
local COM activation is currently refused before creating a server instance.

Sheaf's existing icon-menu command now uses `LayerDisplay.hidden`, preserving each
plot's own visibility and all axis/display settings. One invocation shows the requested
layer and hides every other whole layer; when it is already the only shown layer, the
same entry shows all layers. Previously hidden siblings are shown by this second action,
consistent with the documented show-all contract. Linked geometry stays fixed and the
existing renderer, readers and native/companion save paths apply the same visibility.
Each effective invocation is one existing "Hide other layers" Undo step.

The check mark is derived from a visible target and all siblings being hidden. This
avoids introducing a second mode state. Isolating a hidden target first shows that target;
an all-hidden graph therefore isolates the requested layer on the first click. These
edge choices are explicit Sheaf policies, unmeasured in Origin2016. A single-layer or
stale/invalid request changes no document, history or revision. Selecting another icon
does not automatically transfer isolation; the separate Show Active Layer Only and
Set Active Layer By Layer Icon Only modes remain disabled pending verification.

### 21.15 New Layer (Axes) preset dialog (T61 follow-up, 2026-11-16)

Origin's [layer-add guide](https://docs.originlab.com/origin-help/addlayers-insertlayer/) and
[layadd reference](https://docs.originlab.com/x-function/ref/layadd/) document Open Dialog...,
the nine layer presets, a normal-layer default and Activate New Layer enabled by default.
They also document custom axis definitions/scale formulas and an automatic overlap-offset
option, disabled by default. These are primary-documentation references, not fresh Native2016
measurements; live COM was unavailable and no registration/installation repair was attempted.

Sheaf's Graph > New Layer(Axes) > Open Dialog... and the corresponding graph-page menu open
New Layer (Axes). The first slice selects among the same nine preset helpers as the existing
menu and toolbar, retaining all measured/provisional rules in 21.9. Activate New Layer chooses
whether the new child or its captured source remains active. The source graph is fixed when
opened and the source layer is shown; changing the active layer later does not retarget it.
OK adds exactly one layer through the existing Create history operation, clears object selection
and focuses that graph; Cancel, Escape and closing the dialog make no edit. Any intervening
project revision or replacement closes a stale draft instead of applying it. Source capture,
conservative stale rejection, this compact layout and post-confirmation focus are Sheaf choices.

User Defined and Auto Offset Overlapped Axes remain disabled with reasons. Arbitrary axis-side
combinations, custom scale expressions and choosing another input graph within the dialog are
not implemented by this preset facade. No model, layer-helper, renderer, file or CLI/MCP
contract changes. The existing preset links, geometry, inset-data copying and native/companion
save behavior are reused. The developer capture route is `--show add-layer`.

---


### 21.16 Layer icon menu switches (T183, 2026-11-19, not measured)

The layer icon's right-click menu (Origin 2016 names in 21.x) now runs its three switches. **Show Active
Layer Only** is View > Show > Active Layer Only (22.7, the 21.14 engine), ticked the same way, grey with
one layer. **Set Active Layer By Layer Icon Only** (ticked when on) makes a click, double click or right
click inside the page leave the active layer as it is; only a layer icon changes it (selection and the
other click results are unchanged). **Double click to open Plot Setup** (ticked when on) makes a double
click on a layer icon open Plot Setup for that layer instead of Plot Details. Both are page settings saved
in the `.sheaf` file (`PageStyle::activate_by_icon_only`, `icon_double_click_plot_setup`; off by default,
not written when off), one undo step each, not written to `.opj`. Origin's exact scope and saving were not
measured.
## 22. Graph view modes, Show toggles and Full Screen (T66, measured 2026-11-15, Origin 2016)

Measured in OriginPro 2016 on our own synthetic data (a workbook with X = 1 to 10, Y = 2X+1 and 20-X, a Line + Symbol graph, once with an extra right-Y layer), a 1024x768 Origin window, menus and keys driven through `tools/origin-ref/origin-dialog.ps1 -NoDialog` with `-AfterFile` (`@CLICK`, `@KEY`, `@SHOT`) and `page.viewmode` read through LabTalk. Pictures and raw outputs are local (`re-workspace/scratch/t66/`). Anything not listed is **unmeasured**.

### 22.1 View menu of a graph window

Origin collapses its menus: the short form of View shows Toolbars..., Apps, Project Explorer, Object Manager, Zoom In, Zoom Out, Whole Page and Full Screen, and a chevron; the short form changes after items are used (a ticked or used mode joins it), so the chevron moves. The full form is: Menu >, | Toolbars... (Ctrl+T), Status Bar (ticked), | Command Window (Alt+3), Code Builder (Alt+4), Quick Help (Alt+5), | Apps (Alt+9, ticked), Project Explorer (Alt+1), Object Manager (Alt+8), View Windows >, | Results Log (Alt+2), View Mode >, | Messages Log (Alt+6), Smart Hint Log (Alt+7), | **Print View (Ctrl+2), Page View (Ctrl+1), Window View, Draft View (Ctrl+0)**, | Zoom In (Ctrl+I), Zoom Out (Ctrl+M), Whole Page (Ctrl+W), Zoom All, | Show >, Show Data Info, Full Screen (Ctrl+Shift+J). Page View is ticked on a new graph; the four modes are mutually exclusive (one tick).

### 22.2 The four modes

LabTalk `page.viewmode` is 1 Print, 2 Page (default of a new graph), 3 Window, 4 Draft; writing 0 or 5 left the value unchanged. Ctrl+2, Ctrl+1 and Ctrl+0 set 1, 2 and 4 (read back 1, 4, 2 in order). Evidence: the step script and the screenshots of each value are in the local scratch folder (`re-workspace/scratch/t66/a5.txt`, `a5-m0.png` to `a5-m5.png`; 0 and 5 show a `PAGE.VIEWMODE: object property not found!` toast), but the raw stdout of the `?page.viewmode` queries was not saved, so the read-back numbers (and the .opj reopen read-back in the last bullet) come from the session notes and should be re-measured before anything depends on them.
- **Print and Page View** looked the same in both window sizes tried (a maximized graph window and a small one): the page is drawn at its own size from the top-left with the grey window colour to the right where the page is narrower than the window. A difference (print-size fonts? margins?) was not found: **unmeasured**.
- **Window View and Draft View** stretch the page over the whole window (no grey area; in the small window the layer grew with the window and the axes moved). Window View draws the text with different smoothing from Print and Page View (the rotated axis title looks coarse); Draft View draws the plot like Page View with crisper, unsmoothed text and the same lines and symbols. Whether Draft hides other items (fills, images, patterns) **unmeasured**.
- `page.width` and `layer.width` were not compared between modes: **unmeasured**.
- **Saved with the project:** set to 3, saved as `.opj` (`save` with an unquoted path; a quoted path produced an error dialog), set to 1, reopened: the graph reported 3 again.

### 22.3 Show submenu

In order, with separators after the third, fifth and ninth item: Layer Icons, Active Layer Indicator, Axis Layer Icons | Object Grid, Layer Grid | Frame, Labels, Data, Active Layer Only | Master Items on Screen. Ticks of a new graph: **Layer Icons, Labels and Data**; the other seven are off. Show Data Info (below the submenu) is ticked on a new graph.
- **Layer Icons** unticked: the numbered layer icons ("1", "2" in the top-left corner of the page) disappear; no dialog.
- Frame was later measured in 22.10. Switching Active Layer Indicator, Axis Layer Icons, Object Grid, Layer Grid, Labels, Data, Active Layer Only, Master Items on Screen and Show Data Info remained unmeasured in this run. The collapsed View menu changed between attempts.

### 22.4 Full Screen

Ctrl+Shift+J (View > Full Screen; the page right-click item "View Full Screen" has the same key, as listed in gui-look.md; its own click was not run in Origin: **unmeasured**, Sheaf wires it to the same Full Screen): the graph page covers the whole screen, with no menu bar, toolbars, status bar or window frame, and fits its aspect to the screen height (white page, no grey). A "Reminder Message" dialog appears in the middle when it starts (text: "Hit ESC to exit", "Arrow Keys to shift to different graphs.", "Home and End key to first and last graph.", a "Do not show this message in the future" tick and OK; Enter confirms it). **Esc ends it** and the normal window comes back as it was. With a single graph a Right arrow did not leave a visible change before Esc (the window was already back in the next picture: unclear whether Right ended it; **unmeasured**). Switching between several graphs with the arrow keys, Home and End: **unmeasured**.

### 22.5 What Sheaf does (T66, built)

- The mode is per graph window and kept in `Project::view_modes` (Page View, the default, is not stored); it is saved in the `.sheaf` file, is not an undo step (undo and redo keep the mode now in effect), and is not written to `.opj` (the `page.viewmode` block of the OPJ format was not located: unmeasured).
- Print View and Page View draw the same picture (as measured). Window View and Draft View give the page the window's aspect with its own width (layers grow with the window, text keeps its size, no grey border; the zoom layout works over the window grown by its 12 px margin). Draft View rasterises without anti-aliasing of the text (the lines and symbols stay as they were), as measured. **Not reproduced:** Origin's Window View smooths text differently from Page View (coarser, 22.2); Sheaf's Window View uses the normal rasteriser, so its text looks like Page View's. `page.width`/`layer.width` per mode is unmeasured, so Sheaf does not change the stored page size.
- Show > Layer Icons toggles the existing page setting (`Plot Details > Miscellaneous > Show the layer icons`; one undo step). Active Layer Only later shares the existing Hide Other Layers engine under22.7; the remaining Show items and Show Data Info are grey.
- Full Screen: Sheaf's window goes to the whole screen and shows the page only (white ground, page fitted into the screen, no menu, toolbars, panels or status bar), the Reminder Message first (Enter or OK ends it; the tick is kept for the run only, Sheaf has no settings file yet), Esc ends it; Left/Right/Home/End switch between the graphs in window order as the reminder says (not measured beyond the reminder text; Right on the last graph does nothing).

### 22.6 What Sheaf took from this (T66, plan)

Page View as Sheaf's current picture and the default; Window View (page stretched to the window, aspect not kept) and Draft View (the same with plain text) as in 22.2; modes saved in the `.sheaf` file; the Layer Icons tick; Full Screen with Esc and the reminder; leave the unmeasured Show items and Show Data Info grey until they are measured.

### 22.7 Remaining Show contract audit and Active Layer Only wire-up (2026-11-17)

The current official [hiding guide](https://docs.originlab.com/origin-help/labels-data-layer-hide/)
explicitly gives View > Show > Active Layer Only and layer-icon Hide Other Layers
as equivalent isolation/show-all routes. Hidden elements are excluded from output.
The [hotkey reference](https://docs.originlab.com/origin-help/hotkey-in-origin/)
assigns Ctrl+Alt+D with a multi-layer graph active. This supports wiring the menu
and shortcut to the already-published21.14 engine without a second visibility model.
The2016 menu label/default check observations in22.3 remain separate from this
documentation contract; no new live2016 effect, shortcut or edge measurement is claimed.

Sheaf resolves the current graph and active layer at command activation and calls
the existing HideOtherLayers action. The check is derived from exactly that layer
being visible and every other layer hidden. Toggling off shows all layers. A hidden
target is shown when isolated; all-hidden is unchecked; fewer than two layers or
an invalid active index disables the menu/shortcut. These are existing Sheaf21.14
policies, including one history step, no-op guards and companion-only native layer
visibility. Individual plot/axis flags, data, geometry and zoom/view mode are kept.
Changing which layer is active does not introduce a persistent follow-active mode;
that Native edge remains unmeasured. Only menu dispatch/check and read-only context
data change. Two context-menu constructors supply an inert absent value; their
graph/grid behavior and the visibility/render/IO engines are unchanged.

The other nine entries remain disabled. The audit records these distinctions for
their future implementation rather than treating them as equivalent toggles:

- The [layer reference](https://docs.originlab.com/labtalk/ref/layer-obj/) defines
  showData separately from showLabel. Hiding text/graphic labels through showLabel
  retains their selectability, unlike deleting objects or hiding a whole layer.
  Exact2016 menu scope, hit behavior, history and persistence remain unmeasured.
- [Data Info](https://docs.originlab.com/origin-help/data-info-tool/) is a window
  associated with reader/cursor data and worksheet metadata. The guide distinguishes
  it from configurable hover tooltips introduced in2019. A generic hover tooltip
  does not establish the2016 Show Data Info window/lifecycle contract.
- The [system-variable reference](https://docs.originlab.com/labtalk/ref/sys-var-list/)
  notes that Object Grid became Layer Grid in2020. Modern naming does not establish
  which of the two distinct2016 menu entries controls the same overlay or settings.
- [Master Page Layout](https://docs.originlab.com/origin-help/master-page-layout/)
  describes a saved workspace-display toggle whose hidden master items still appear
  in copy/print/export. The option depends on Use Master Items. This differs from
  ordinary layer hiding; master-page state/rendering is not supplied in this slice.

### 22.8 View > Show > Labels and Data (T177, 2026-11-19, documentation contract, not measured)

Origin's [layer reference](https://docs.originlab.com/labtalk/ref/layer-obj/) has `layer.showLabels`
and `layer.showData`, separate switches of a layer; the menu items were not measured (22.3: both are
ticked on a new graph). Sheaf reads them as page-wide switches of every layer:

- **Labels** turns `LayerDisplay::show_labels` on or off on every layer of the graph: off draws no
  legend, free text labels (tables included) or drawn objects (lines, arrows, rectangles, ellipses) in
  the layer, on screen and in exports. Axis titles, tick labels, plots, slope annotations, straight
  lines and colour scales stay. The hidden objects cannot be picked (Origin keeps them selectable:
  not followed).
- **Data** turns the existing Show Elements: Data (`LayerDisplay::show_data`) on or off on every
  layer.
- Each is ticked when every layer shows its labels (or data), and each click is one undo step
  ("Show Labels", "Show Data"). Older files read `show_labels` as on; the `.opj` written here does
  not hold it (its save warns that the layer's Display tab was not written).
- At the end of T177, seven items were still grey. The Active Layer Indicator follows in 22.9.

### 22.9 View > Show > Active Layer Indicator (T66, 2026-11-21)

Origin's guide describes a per-graph highlight of the active layer's axes. The
original Origin 2016 menu inspection in 22.3 found the control unchecked in a
new graph. A synthetic OriginPro 2016 probe returned an undefined value for
`page.ali`, a setting listed in current LabTalk documentation. Live activation
of the 2016 menu item and its persistence remain unverified.
[Origin layer guide](https://docs.originlab.com/origin-help/customize-layer-element/) ·
[LabTalk page reference](https://docs.originlab.com/labtalk/ref/page-obj/)

Sheaf stores the graph-window setting in its project and toggles it as one undo
step. The graph view highlights the active layer's rendered primary Cartesian
axis positions with the app's selection color. Hidden axes and hidden layers
produce no highlight, and gaps at axis breaks stay open. The overlay follows
layer activation, axis placement, pan and zoom. It belongs to the graph editor,
so exports and printed output use the same graph scene. Indicators for
non-Cartesian axes and for opposite axes are outside this implementation.

### 22.10 View > Show > Frame (T66, Origin 2016, measured 2026-11-22)

In a graph with two separate Cartesian layers, invoking the View > Show > Frame
command (menu ID 33092 via LabTalk) added a thin black rectangle to the active
layer. The rectangle followed the plot bounds with both axes hidden and with the
X axis moved inward by 25%. The colored axes and opposite-line settings kept
their values. Switching the command off on the same layer restored the original
PNG pixels. Saving and reopening kept the frame. Invoking the command in the
other layer added its frame independently. The Origin EPS used a hairline for
the frame.
[Origin 2019b release note](https://www.originlab.com/ReleaseNotes/detail.aspx?id=2019b3ORG-19899)

Sheaf's Plot Details > Display controls the frame of a Cartesian layer. The
graph scene uses a 0.5 pt black outline under the axes, including when clipping
to the frame is off. Screen, print and exports share the scene. The setting
defaults to off in older Sheaf projects and persists in `.sheaf` and the
`.opj` graph layer header (opj-format 10.2). Existing opposite-edge line
settings keep their effect. The View menu shortcut remains pending.
[Origin Plot Details guide](https://docs.originlab.com/origin-help/pd-dialog-display-speed-tab/)

## 23. View > Zoom All and the Tools-bar Scale In / Scale Out (T69, measured 2026-11-15, Origin 2016)

Measured in OriginPro 2016 on our own synthetic data (X = 1 to 10, Y = 2X+1 and 20-X, Line + Symbol graph; one run with a right-Y layer from `layadd type:=righty`; one with both axes logarithmic; a 4-graph project), a 1024x768 Origin window, drags and clicks sent by `tools/origin-ref/origin-dialog.ps1 -NoDialog` with `-AfterFile` (`@DRAG`, `@CLICK`, `@DCLICK`, `@KEYDOWN`, `@SHOT`, `@CURSOR`) and the axis ends read after each action with `?layer.x.from` / `.to` / `layer.y.from` / `.to` (raw outputs and pictures are local: `re-workspace/scratch/t69/`, `*.out.txt`, `*.png`). Anything not listed is **unmeasured**.

### 23.1 Correction: Zoom All is not a presentation

gui-look 4.x and T65 recorded "Zoom All = a presentation of all graphs with a reminder". That was wrong: the reminder text ("Hit ESC to exit", arrow keys, Home and End) is the **Full Screen** dialog (22.4), which probably came from a click that landed on Full Screen in the collapsed View menu (in this study the same mis-click happened once: the short form of View changes between runs). **View > Zoom All is a zoom of the active graph window.**

### 23.2 View > Zoom All

- It sits in the View menu between Whole Page and Show (full form: Zoom In, Zoom Out, Whole Page, **Zoom All**). No key is listed. It was not in the collapsed form of View until it had been used once (then it joined the short form, as 22.1 says of other items).
- **It zooms the active graph window so that everything fits, including objects that lie outside the page.** With a text label placed far outside the page (x = 30 on an axis that ends at 11) the view went to 48 % (zoom combo of the Standard toolbar), the page drawn small at the top-left corner of the window and the outside label visible near the right edge of the window. The same click again changed nothing (idempotent). **Ctrl+W (Whole Page) returned to 100 %** with the page fitted to the window as before; the outside label is out of view again.
- On a graph with nothing outside the page the zoom combo read **115 %** and the page was drawn smaller than Whole Page (about 0.87 of its size, 347 x 276 px against 401 x 307 px in the 1024x768 window) at the top-left corner with a small gap, and a horizontal scroll bar appeared under the page. How the combo's percentage relates to the drawn size (it does not follow a single ratio: 115 % for 0.87 and 48 % for 0.56) is **unmeasured**, as is the exact margin around the bounding box (about 10 px was seen).
- **It does not touch other windows and is not a mode.** With four graph windows in the project, Zoom All changed only the active graph (Graph4); `%H` still named Graph4. No dialog or reminder appeared, and Enter, Right, Home and Esc afterwards changed nothing (the 115 % view stayed). `page.viewmode` stayed 2.
- Unmeasured: Zoom All in a maximized window, from a zoomed (200 %) view (Whole Page undoes a zoom, so Zoom All probably replaces it), whether it counts hidden objects, whether it is saved, the page context menu (no such item was looked for), the Undo text (none looked at).

### 23.3 The Tools-bar magnifiers are axis Scale In and Scale Out

- Tools toolbar, second and third buttons (the first is the Pointer). Tool tip of the first: "**Scale In**: Axes magnification, double-click to restore, ctrl-drag to create zoom graph"; the second is **Scale Out** (its tip was not read separately).
- **Scale In is a rectangle tool.** After choosing it (the button stays pressed and the pointer over the page is a custom magnifier cursor), a left-button drag draws a dotted rectangle (a plain rubber band) and on release the active layer's axes are rescaled **exactly to the rectangle**: x from the left to the right edge of the rectangle, y from its bottom to its top edge, both axes at once. Example (layer shown from x 0 to 11, y 1 to 23, plot frame 165 to 439 px by 206 to 407 px): a drag from (250,260) to (350,340) gave x 3.402 to 7.437 and y 8.727 to 16.735, which is the rectangle in axis units. Ticks were recalculated for the new range, and the data are clipped to the layer frame.
- **The tool ends after one drag**: the Pointer tool is selected again (the Pointer button pressed, the cursor an arrow). To zoom again, choose Scale In again. Choosing a button that is already active unselects it.
- A rectangle can run outside the layer frame: it is not clipped, the range is extended by the same scale (a drag starting at x = 100 px, left of the 165 px frame edge, gave x from -2.650). A drag of 3 x 2 px changed nothing. A plain click with the tool changed nothing and appears to end the tool (the next drag acted as a Pointer drag).
- Scale In steps nest: three rectangles in a row gave three successively smaller ranges; each rectangle is read against the current view (x 3.402 to 7.437, then 3.910 to 6.870, then 4.826 to 5.911).
- **Scale Out does not zoom out by a factor. It steps back**: with the tool chosen, **one click** (anywhere on the page) or **one drag of any rectangle** restores the axis ends of the layer from before the last Scale In, and the tool ends. Three Scale In steps then three Scale Out clicks passed through 3.910 to 6.870, then 3.402 to 7.437, then 0 to 11 (the original range); a further Scale Out at the original range changed nothing. So Origin keeps a stack of previous scales for Scale In and Scale Out takes one off per use.
- **Which axes change:** only the **active layer** (the layer chosen with `page.active`, shown highlighted among the layer icons). With a right-Y layer linked to layer 1 in X: Scale In on layer 1 changed layer 1's x and y and layer 2's x (linked) but **not layer 2's y** (9 to 20 stayed); Scale In with layer 2 active mapped the rectangle through layer 2's own right y axis (12.86 to 16.87 for the same rectangle) and rescaled layer 1's x (linked) but not layer 1's y (1 to 23 stayed). Scale Out restored both layers.
- **Log axes:** the rectangle is applied in the axis's scale: with x logarithmic 0.5 to 20 and y logarithmic 2.5E-9 to 2500, the rectangle (250,260 to 350,340) gave x 1.565 to 6.055 and y 4.10E-5 to 0.956 (a linear map in log10 of the end values, as the pixel positions predict), the axes stayed logarithmic, and Scale Out restored 0.5 to 20 and 2.5E-9 to 2500.
- **Undo:** the Edit menu reads **"Undo Zoom In/Out"** (Ctrl+Z) after a Scale In, and also after a Scale Out. Ctrl+Z after three Scale In steps undid them one at a time (4.826 to 5.911, then 3.910 to 6.870, then 3.402 to 7.437).
- **Ctrl+drag with Scale In creates a new graph window named "Enlarged"** (the "zoom graph" of the tool tip; the source graph's scale did not change). Its content, link to the source and layout were not studied: **unmeasured**.
- **Double-click "to restore"** (tool tip) did not restore anything in two runs (a double-click on the plot right after choosing the tool left the zoomed range as it was, and with the tool unselected it opened Plot Details > Layer Properties, as a double-click on a layer does). What the double-click needs (the axis? a longer gap between the clicks?) is **unmeasured**; Scale Out is the measured way back.
- Not measured: Esc during the drag, a drag starting on an axis or outside the layer area, Scale In on a graph with an axis break, 3D or polar axes, the status-bar text during the drag, the cursor shape of Scale Out (the cursor over the page was a custom cursor for both), whether a LabTalk or dialog change of the scale clears the Scale Out stack, whether the stack is saved with the project.

### 23.4 What Sheaf takes from this (T69, built 2026-11-15)

- Zoom All: a command on the active graph view only (no reminder, no keys, no mode). It sets the T65 view transform so that the bounding box of the page and every object drawn outside it fits in the window, page at the top-left, with a small margin; with all objects inside the page it shows the page with the same margin. The toolbar displays Sheaf's actual view factor rounded to a whole percent. Origin's percentage for Zoom All is unmeasured. Whole Page restores the fitted page.
- Scale In: a one-shot tool on the Tools bar that maps the dragged rectangle (screen to page to axis units, linear or log10 like the axis) to the active layer's x and y ends, linked layers following in the linked axis only; one undo step named "Zoom In/Out"; the tool ends after the drag; a click or a tiny drag does nothing. Scale Out: a one-shot tool whose click or drag pops the last Scale In of the active layer from a per-window stack (nothing when empty). Ctrl+drag (the "Enlarged" graph) and the double-click restore stay unbuilt.
- Built in `crates/app/src/graph_scale.rs` (pure `rect_to_ends`, `zoom_all`; the stack steps check that the axes still hold the ends the step wrote, so a step undone with Ctrl+Z is dropped, not replayed). Choices that were not measured: a tiny drag (under 4 px on a side) ends Scale In without a change, as a click does in Origin (measured, 23.3); Scale Out ends after one click even with an empty stack; layers with axis breaks, polar and ternary layers are refused (a status hint); Ctrl+drag with Scale In does nothing but a hint; Zoom All centres the page (T65's layout; Origin puts it at the top-left, a known departure) with a 10 px margin and the factor never above 1, and what lies outside the page is not drawn (the raster is the page), so only the fitting is built, not the view of the outside objects.

## 24. User templates: Save Template As and Plot > User Templates (T56)

Sheaf's own file kind and rules; Origin's `.otp`/`.otpu` templates are never read or written, and nothing
here is measured in Origin (to measure on Windows: what an Origin template keeps, how extra or fewer
datasets are styled, the default folder and name).

- **Saving** (File > Save Template As... on a graph; on a workbook it saves a workbook template,
  `workbook-model.md` 13o (T160); greyed without a template folder): a name prompt
  filled with the graph's short name; OK saves the graph's look as `<name>.sheaftemplate` in the folder
  `templates` of the settings folder (`sheaf_io::user_templates::default_dir`; `--templates-dir DIR` and
  `--no-templates` at start). A template of the same name (letter case ignored) is replaced, and the prompt
  says so before OK. Names are cleaned for the file system (`clean_name`: control characters dropped, white
  space collapsed, `/ \ : * ? " < > |` to `_`, device names such as `CON` get `_`, at most 60 characters).
- **What is kept** (`UserTemplate::from_graph`): the whole graph without data: page, layers and their
  placement and links, axes (fixed ranges stay fixed, automatic ones rescale to the new data), each plot's
  style in order, legend and text objects (with `%(...)` substitutions, re-evaluated on the new data), drawn
  objects and straight lines. Each plot becomes a numbered slot; columns, row ranges, data-label and
  tick-label columns are dropped. Refused: treemaps, sunbursts, parallel plots, floating bars and fills
  between two curves, whose look is made of their data. The file is a `.sheaf` archive with the marker
  `"template": "graph"` and no workbook.
- **Plotting** (Plot > User Templates > a saved name, on a workbook with Y columns selected; `sheaf plot -t
  NAME`): column `i` takes slot `i mod n`; columns beyond the slots repeat the styles and take colours from
  the layer's palette in plot order, and the layer's group increments apply; with fewer columns the extra
  slots are left out, and a linked layer left without plots (the right Y of a double-Y template given one
  column) is removed. The new graph's long name is that of the first column's workbook. One undo step.
- **Managing** (Plot > User Templates > Manage...): the saved names with Rename (refused onto another
  template's name) and Delete, and the folder shown. The list is read from the folder at start and after
  every change, so templates survive a restart and two running copies agree.
- **Command line**: `sheaf plot -t NAME` looks a name that is no built-in template up among the user
  templates in `--templates-dir`, else `$SHEAF_TEMPLATES_DIR`, else the settings folder; an unknown name
  lists the built-in templates and points to Save Template As. A user template plots one sheet's Y columns
  (not with `--panels`, `--y2`, `--group` or several input files). MCP's `plot` never reads user templates
  (the folder is outside its allowed roots).
- **File > New > From Template** (T103, measured 2026-11-16 in Origin 2016, `re-workspace/scratch/t103`):
  Origin's item is a submenu listing templates by file name (workbook templates such as `Assays.ogw`), a
  separator and More... (a file picker). Sheaf lists the saved user templates, then (after a separator) the
  workbook templates of T160, then More... (a file picker for `.sheaftemplate` and `.sheafbooktemplate`
  files starting in the templates folder); a graph template makes a new graph in the template's look
  without data (no plots; axes, text and page as saved), one undo step ("New graph from template"); a
  workbook template makes an empty workbook (`workbook-model.md` 13o).
- **Plot > System Templates...** (T104, measured 2026-11-16 in Origin 2016, `re-workspace/scratch/t104`):
  Origin's "System Templates" window has a Category tree under "Graph Template (*.otp)" (Line, Symbol, Line &
  Symbol, Columns & Bars, Multi-Curve, 3D XYY, Statistics, Area, Contour, Specialized, Stock, 3D Surface, 3D
  Symbols & Bars & Vector, Image), a Preview Window (picking Line drew its sample graph), Description,
  Location (the `.otp` file) and Date Modified, and Plot Setup..., Plot (greyed until a template is picked)
  and Cancel. Sheaf's window lists its 62 built-in templates in those categories (only the ones it has;
  Sheaf files templates Origin keeps elsewhere by its own choice, `dialogs/system_templates.rs`), previews a
  picked template with sample data, and Plot plots the selected columns with it (the Plot menu's own action,
  one undo step). Not done: Plot Setup..., user templates in the tree (Plot > User Templates has them).
- Tests: `crates/core/src/user_template/tests.rs`, `crates/io/src/user_templates/tests.rs`,
  `crates/plot/src/user_template_tests.rs`, `crates/app/src/app/ui_tests/user_templates.rs`,
  `crates/cli/tests/user_templates.rs`.

## 25. File > New > Graph and the Standard toolbar's New Graph (T77)

Origin's documentation (not measured, Origin 2016 is on Windows only) says both open an empty graph
built from the default graph template, named Graph1, Graph2, ..., with no shortcut. [W]
(https://www.originlab.com/doc/origin-help/standard-toolbar, https://www.originlab.com/doc/origin-help/create-childwindow)
Sheaf: `Project::add_graph(Template::default())` (the Line template, which carries the owner defaults), no plots;
axes with nothing to plot run 0 to 1. The graph is one undo step ("New graph") and is shown in front.
UNMEASURED Sheaf choices: the axis ranges and the look of an empty layer.

## 26. Copy Format and Paste Format (T72, measured 2026-11-15/16, Origin 2016)

Measured in OriginPro 2016 on our own synthetic data (X = 1 to 10, Y = 2X+1 and 20-X, Line + Symbol graphs). One styled source graph ("Src": axis colours, tick-label sizes and colours, layer background yellow, x scale 2 to 8, layer width 40 % and top 5 %, page width 5200, a text label of size 40 in red, plot colours, symbol size and shape) and untouched targets of the same data; clicks, right-clicks and menu choices sent by `tools/origin-ref/origin-dialog.ps1 -NoDialog` (`@CLICK`, `@RCLICK`, `@MOVE`, `@KEY`, `@SHOT`), the layer, axis, page and text-object properties read back with `?layer.x.color`, `?layer.x.label.pt`, `?layer.color`, `?layer.x.from`, `?layer.width`, `?txName.fsize` and so on (raw outputs and pictures are local: `re-workspace/scratch/t72/`). Anything not listed is **unmeasured**.

### 26.1 Method notes (for repeating this)

- Origin's collapsible menus hide items and re-order the short form by use. Unticking Tools > Options > Miscellaneous > Enable Collapsible Menus and pressing OK did not stick (the box read ticked again in the next instance), so each run used the keyboard: Up then Enter on an open menu expands it (the last item is the chevron). A submenu opens on a click on its parent item (a plain mouse move does not open it); an item inside it reacts to a click only after the pointer was moved onto it first.
- **The format clipboard survives a restart of Origin** (a copy made in an earlier instance was pasted in a later one), so a run that pastes without copying uses a stale clipboard. Fonts once pasted the Colors of the run before for this reason.
- The Apply Formats window appears about 4 s after the menu click; a screenshot at 1.5 s showed nothing and looked as if the command had done nothing.
- `get <dataset> -c/-z/-k`, `range ... .symbol.size` and `page.color` did not read plot or page formats back in this build (zero or False); plot formats were judged from pictures only.

### 26.2 Where the commands are

- **Edit menu** (a graph window): Undo, Copy, Copy Page, **Copy Format** ▸, **Paste Format...**, Find in Project... in the short form; the full form adds Cut, Paste, Paste Link, Paste Link Transpose, Paste Special, Clear, Insert New Object, Links, Button Edit Mode, Update Client. Copy Format ▸, full: **All** | **All Style Formats**, **Colors**, **Fonts**, **Background**, **Symbol, Line and Fill** | **Dimensions**, **Scales** | **Text**. With an axis selected, Edit > Copy Format > All copied an *axis* format (Paste Format... then showed the axis check boxes, 26.5).
- **Graph page context menu** (right-click on the page outside the layer), full form: Paste, Paste Link, Paste Link Transpose, Add Text..., Add/Modify Layer Title, Insert Images From Files..., Insert OLE Object ▸, Layer Management..., Add Axis Scrollbar ▸, New Layer(Axes) ▸, Extract to Graphs..., Copy Sparkline, Fit Page To Layers..., Fit Page To Printer, Show Active Layer Only, Set Active Layer By Layer Icon Only, Print View, Rotate Page, Add Shortcut to Favorites, Go to Original Folder, Copy Page, Export Graphs..., Send Graphs to PowerPoint..., View Full Screen, Print..., Mask ▸, **Copy Format** ▸, **Paste Format**, Save Format as Theme..., Paste Operation. Its Copy Format ▸: **Page Background**, **Legend Translation Mode**, **Dimensions**, **Scales** | **Colors**, **Fonts**, **Background**, **All Style Formats** | **All** (no Symbol, Line and Fill and no Text here; the order in the short form follows use).
- The item after Copy Format is **Paste Format** (no dots, acts at once) when the clipboard holds a graph format, and **Paste Format To** ▸ when it holds an axis format; its first level on the page menu is Bottom Axes ▸, All X Axes ▸, All Axes ▸, All ▸ (second level not opened). In the Edit menu it is always **Paste Format...** with dots (26.5).
- **Layer area context menu** (right-click inside the layer frame, no object under the pointer) has Copy Format ▸, Paste Format and Save Format as Theme... as well, among Plot Setup... items; its Copy Format list was not opened.
- **Axis context menu** (click the axis line first, then right-click it): Hide Axis, Hide Grids (ticked when grids are hidden), Rescale To Show All (Ctrl+R), Axis Zoom In, Axis Zoom Out, Center | **Copy Format** ▸, **Paste Format To** ▸ (only with an axis format on the clipboard), Save Format as Theme... | Scale..., Tick Labels..., Properties.... Copy Format ▸: **Styles**, **Scales**, **Dimensions** | **All**. Paste Format To ▸: **Tick Direction**, **Ticks**, **Line**, **Tick Labels**, **Title Styles**, **Colors**, **Thickness**, **Styles** | **Scales** | **Title Dimensions**, **Title Text** | **All** (the same twelve names as the Apply Formats check boxes).
- **Legend context menu** (select the legend, right-click its frame edge; a right-click on an entry gives only Hide Data Plot and Hide Others): Plot Details..., Legend ▸, Add Color Scale, Add Bubble Scale, Cut, Copy, Delete, Bring to the front, Push to the back, Bring to front of data, Push to back of data, Programming Control..., Properties... | **Copy Format** ▸, Save Format as Theme.... Copy Format ▸: **Styles**, **Dimensions** | **All**. No Paste Format appeared with an empty or axis clipboard; what a legend format does when pasted is unmeasured.
- **Save Format as Theme...** (B11, not this slice) lists "Formats to Save": All, Colormap, All Styles, Fonts, Colors, Symbol, Line and Fill, Dimension, Scale, Background, Text: the category names behind the Copy Format choices.

### 26.3 Graph: what each Copy Format choice copies (page menu or Edit menu to another graph's Paste Format)

Pasted into an untouched graph of the same data and read back (changed properties only):

- **All**: axis colours (x 2, y 3), tick-label sizes (30, 26) and colour (4), axis thickness, layer background colour, **x scale ends (2 to 8)**, **layer width, left and top** (40, 17.9, 5.0), **page width** (5200), the text object's size (40) and colour (2) **and its text** (the target's own label read "SourceText" afterwards, the object kept its name), plot line and symbol colours, symbol sizes, line style (dashed) and the legend. One step, no window.
- **All Style Formats**: colours, fonts and thickness (axis colours, tick-label sizes and colour, y thickness, layer background, text size and colour). **Not** the scale ends, the layer and page size or the text content.
- **Colors**: axis line colours, tick-label colour, layer background colour, text colour, plot line and symbol colours; not sizes, fonts, scales or text content.
- **Fonts**: tick-label sizes (x 30, y 26) and the text object's size (40); no colours.
- **Dimensions**: layer width, left and top and the page width; the layer height was not tested (it equalled the source's).
- **Scales**: the x scale ends (2 to 8); the y ends were the same in both graphs, so whether y follows is unmeasured.
- **Background**: the layer background colour (7), and, unexplained, both axis thicknesses read 4 and the text colour read 2 afterwards (repeated in a second run); what Background includes beyond the fill is unmeasured.
- Not measured: Symbol, Line and Fill and Text (Edit menu only), Page Background and Legend Translation Mode (no readable change; the page colour cannot be set by LabTalk here), Copy Format with objects selected on the page (a text label, a drawn shape).
- **Undo text: "Undo Apply Theme"** (Edit menu, Ctrl+Z) after a graph paste.

### 26.4 Axis

- With the Src X axis selected, Copy Format > All and, in the target with its X axis selected, Paste Format To > All: **only that axis changed**: line colour (2), tick-label size (30) and colour (4) and the scale (2 to 8). The Y axis (colour, label size, thickness, ends), the layer width and background and the text size stayed. The picture shows the axis, its ticks and numbers recoloured. Undo text again "Undo Apply Theme".
- Styles, Scales and Dimensions of the axis menu were not pasted one by one: unmeasured (the twelve Paste Format To names are the Apply Formats check boxes).

### 26.5 Edit > Paste Format... (the Apply Formats window)

- Opens a modeless window titled **Apply Formats**, slowly (about 4 s). **Apply From**: Clipboard (default) or Theme File (a list and a "..." button), and a Description line. With an **axis** format on the clipboard: **Formats to Apply**: All (ticked, the rest greyed while All is ticked), Line, Colors, Scales, Tick Direction, Tick Labels, Thickness, Title Dimensions, Ticks, Title Styles, Styles, Title Text; **Apply Scope** ("Selection" with an axis selected, "Active Graph" when the page is active), and then **Apply To** ("Bottom Axes"). Buttons: a pencil tool button with a chevron (its menu was not opened), Save, Save As..., **Undo Apply**, **Apply**, **Close**.
- With a **graph** format on the clipboard (seen once, the grid still empty then): a Property / Value grid, List View and Collections check boxes, **Apply Selected Properties Only**, **Apply Scope** "Active Graph", Undo Apply, Apply, Close. The grid contents, the scope list and what Apply does there were not measured.

### 26.6 Mismatches (Copy Format All of a two-plot, one-layer graph)

- No message appeared. A target with **one plot** took the first plot's colours and symbol size (its symbol shape and line colour did not visibly follow: reason unmeasured); a target with **three plots** got the two source styles on plots 1 and 2 and a third, blue dashed, large-triangle style on plot 3 (not derived from anything we set); a target with **two layers**: the active layer read back with the source's values (the other layer was not read).
- Graph clipboard onto an axis, and legend clipboards: not measured. An axis clipboard with the page active offers Paste Format To (Bottom Axes first); nothing was pasted that way.

### 26.7 What Sheaf takes from this (T72)

- The format clipboard has a type: Page, Axis or Legend; the paste command offered follows the type (Paste Format for a page format, Paste Format To for an axis format; Edit > Paste Format... opens Apply Formats).
- Build the measured lists as they are (26.2) and the property groups of 26.3 and 26.4; undo text "Apply Theme". Choices that were not measured (Symbol, Line and Fill, Text, Page Background, Legend Translation Mode, legend paste, mismatched plots and layers) are Sheaf's own: plot formats go to plots by index with surplus plots left alone; the target layer is the active one; say so in the item's status.

### 26.8 As built (T72 step 2)

- Core `crates/core/src/format_clip.rs`: a copy holds the formatted values of every layer (or of one axis) and the groups the choice copies; a paste applies the groups it asks for that the copy holds. Groups: Dimensions (layer left, top, width; page width, not heights), Scales (scale type, limits, steps, reverse, first tick, minor ticks, rescale margin, both axes), Colors, Fonts (family, size, bold, italic; colours stay), Background (layer colour, gradient, border), Thickness (axis line and tick width), Symbol/Line/Fill (the plots' whole line and symbol style), Text (the words of the text labels). All = all eight; All Style Formats = Colors, Fonts, Background, Thickness, Symbol/Line/Fill (no Dimensions, Scales, Text); Colors also takes the layer background colour and the plots' line and symbol colours; Fonts takes tick-label, title, legend and text fonts.
- Sheaf's own choices where Origin was not measured: layers and plots are paired by position, surplus target layers and plots are left as they are, surplus source ones are dropped; text labels are paired by position and are never created; a paste that changes nothing makes no undo step; the y axis follows Scales; the fill under a curve is not part of Symbol/Line/Fill. The clipboard stays internal; the private profile snapshot now survives a restart (26.10). Another running instance does not automatically refresh its in-memory copy.
- Enabled (measured behaviour): Edit > Copy Format ▸ All, All Style Formats, Colors, Fonts, Background, Dimensions, Scales; Edit > Paste Format... (the Apply Formats window: Apply From Clipboard, one check box per group the clipboard holds or Scales and Styles for an axis format, Apply To for an axis, Undo Apply, Apply, Close; the grid Origin shows for a graph format is unmeasured, so this window is Sheaf's reading of it); page menu Copy Format (same items) and Paste Format; axis menu Copy Format ▸ All and Paste Format To ▸ All. One undo step "Apply Theme" each.
- At the end of this slice the remaining entries included Symbol/Line/Fill and Text (Edit), Page Background and Legend Translation Mode (page), axis Dimensions and other Paste Format To choices, the page menu's Paste Format To submenus, and legend Dimensions and All. The axis Scales and Styles follow-ups are in 26.9 and 26.11. Legend Styles, Dimensions and All follow in 26.12 and 26.16.

- Added after review: Edit > Copy Format with an axis selected copies an axis format (All, All Style Formats and Scales map to the axis parts; Colors, Fonts, Background and Dimensions copy nothing, since an axis format for them is unmeasured); the context menus and Paste Format To follow the clipboard kind. Colors and axis styles also carry the "follows plot colour" flags (Sheaf's reading; unmeasured in Origin). Dimensions pastes a layer frame only between layers of the same kind (root to root, linked to linked) because a linked frame is relative to its parent; mixed pairs are left alone (unmeasured in Origin).

### 26.9 Axis scale-only context entries (T72 follow-up)

The axis Copy Format and Paste Format To submenus now enable Scales through the
existing AxisPart::Scales capture/apply path. Origin2016 menu presence and the
All operation are measured in 26.2/26.4. The
[official selective-copy guide](https://docs.originlab.com/origin-help/copypaste-format/)
describes choosing object properties; it does not establish the exact Scales
per-choice whitelist. The following property mapping is existing Sheaf policy,
not a new Native measurement. Live COM is unavailable; no retry or repair is made.

Copy holds scale values only. Paste applies scale type, min/max, automatic-range
policy, rescale margin, major step, minor-tick count, reversal and first tick to
the selected axis only, and only if an axis clipboard contains scales. It leaves
fonts, colours, thickness, title/text, data, other axes/layers and page geometry
unchanged. A scale-only clipboard pasted with All still cannot supply styles;
Scales pasted from All ignores its styles. Existing typed-clipboard and no-op
guards retain one Apply Theme Undo step per effective paste and none for repeats
or invalid targets. No core, persistent schema or clipboard lifecycle changes.

Axis Styles later gains its context entries in26.11. Other still-grey choices
remain deferred, including Dimensions, other property groups, legend and page crossing. Existing All and
Apply Formats behavior remains. UI regressions check root/inset X/Y targets,
partial capture/paste, full-graph preservation, Undo/Redo, repeated and stale
requests; Native per-choice parity remains unmeasured.

### 26.10 Format clipboard across restart (T72)

The accepted Origin2016 observation in 26.1 establishes that a copied format can
be pasted after restarting the reference application. Sheaf now retains that
behavior through a private profile snapshot. The file name, JSON encoding,
resource bounds, error handling and instance policy below are Sheaf choices,
not measurements of Origin storage. No system clipboard format is used.

Normal startup loads format-clipboard.json beside the existing profile settings.
Startup.format_clipboard_path can instead select an isolated path; None keeps
memory only and reads/writes nothing. Disabling recovery does not disable this
separate clipboard. Only successful Copy Format requests save a version-1 JSON
envelope containing the existing Core FormatClip. Graph groups and axis
styles/scales flags survive exactly; unavailable groups cannot be pasted merely
because a stored axis contains other captured fields. Stale/invalid copy requests
leave the held snapshot alone. Copy still changes no document revision/history.

The reader accepts at most 16 MiB. Missing/unreadable/corrupt/unknown-version or
oversized records start empty and are not rewritten by loading. A new valid copy
can replace a damaged record. Writes create a unique same-directory temporary
file, synchronize it and rename it over the destination, with cleanup limited to
the owned temporary file on failure. Oversized snapshots or I/O failures retain
the current in-memory copy and report once until a save succeeds; the last
successfully saved snapshot remains available at the next startup.

There is no live file watcher: each app loads once, and the last successful writer
controls the next startup. Paste, Undo/Redo, closing a project and Apply Formats
do not save or clear the clipboard. Existing typed paste, scope, no-op and history
rules remain; no Core/project/IO schema, menu, action or dialog behavior changes.
Isolated tests exercise graph/partial-axis restarts after explicitly clearing
memory, corrupt/unknown/oversized files, concurrent atomic replacement, failed
writes with memory fallback and document/storage independence.

### 26.11 Axis Styles context entries (T72 selective-format audit, 2026-11-17)

The axis Copy Format > Styles and Paste Format To > Styles entries now call the
existing AxisPart::Styles capture/apply path, already used by the Apply Formats
window. Origin2016 menu names and All effects are measured in26.2/26.4; the
[official selective-copy guide](https://docs.originlab.com/origin-help/copypaste-format/)
documents object-specific groups but does not specify this exact Styles whitelist.
The field group below remains existing Sheaf policy, not new Native2016 parity.
No Core, clipboard lifecycle/profile, schema or Apply Formats-body change is made.

Copy stores only the styles-available flag. Paste can apply axis colour and the
axis/title follows-plot-colour flags; tick colour; axis/tick thickness; tick/title
fonts including their colours; major/minor tick directions and lengths. It does
not apply scale type, limits, range policy, steps/minor-tick count, reversal/first
tick, raw title text, grids/display/labels, data, other axes/layers or geometry.
Styles copied then pasted with All cannot supply scales. Styles pasted from All
ignores its scales; a scales-only clipboard supplies no styles. Existing typed
scope, stale-target and effective no-op checks preserve one Apply Theme Undo per
real paste and no history for repeats or unavailable groups.

The audit leaves Dimensions and legend formats deferred. The axis has no typed
Dimensions group, so the graph Dimensions group is not substituted. The current
[legend reference](https://docs.originlab.com/origin-help/pd-dialog-legendtitles-tab/)
includes translation mode/custom-format settings in legend Copy Format > All;
existing graph font groups alone do not establish that legend contract. The
selective-copy guide also distinguishes raw inline escape formatting in text
objects from copied object properties. No graph/axis/legend crossing, raw-text
rewriting or invented property whitelist is introduced.

### 26.12 Legend Styles context entries (T72)

Origin 2016 shows Styles under the legend's Copy Format menu (26.2). Origin's documentation
describes applying customizations between legends through their context menus and places
translation mode and custom format strings under Copy Format > All.
[Origin legend guide](https://docs.originlab.com/origin-help/legend-colorscale/),
[Legends/Titles reference](https://docs.originlab.com/origin-help/pd-dialog-legendtitles-tab/)

Sheaf's legend Copy Format > Styles captures the complete legend font, frame switch, and
background color. Open the target legend's Paste Format To > Styles to apply those fields.
The field mapping is Sheaf policy. Origin 2016's exact per-field Styles behavior remains
unmeasured. Copy is read-only and uses the private clipboard snapshot. A changed paste
is one Apply Theme step. Repeated pastes of the same style add no history.

### 26.13 Axis Paste Format To groups (T171, 2026-11-19, not measured)

The axis's Paste Format To ▸ names twelve groups (26.2); Styles, Scales and All were built (26.8, 26.9,
26.11). The other groups now paste too, each a part of what Styles pastes, with Sheaf's field choices
(Origin's per-group fields were not measured): **Tick Direction** the major and minor tick directions;
**Ticks** the directions, lengths, tick width and tick colour; **Line** the axis colour (and following the
plot) and line thickness; **Tick Labels** shown or hidden, the tick-label font, angle, number format, prefix
and suffix, minor labels and the end label; **Title Styles** the title font and following the plot's
colour; **Colors** every colour (line, ticks, tick labels, title) and the follow-the-plot switches;
**Thickness** the line and tick widths. They need a clipboard that holds Styles (Copy Format > Styles or
All); a Scales copy pastes nothing for them. **Title Text** pastes the title's text and whether it is shown,
only from a Copy Format > All clipboard. No group touches the scale, the other axis or other layers; each
paste is one undo step ("Apply Theme", as the other pastes). **Title Dimensions** stays grey: Sheaf's axis title has no
position or size of its own to paste.

### 26.14 Page Paste Format To with an axis format (T174, 2026-11-19, not measured)

With an axis format copied, the page's right-click menu shows Paste Format To ▸ Bottom Axes ▸, All X Axes
▸, All Axes ▸, All ▸ (26.2; Origin's second level was not opened, and OriginLab's Copying and Pasting
Formats page does not name these targets). Sheaf's meanings: **Bottom Axes** the X axis of every layer that
is drawn at the bottom (not moved to the top, not hidden); **All X Axes** every layer's X axis; **All Axes**
every layer's X and Y axes. Each opens the axis groups of 26.13 (Tick Direction ... Styles, Scales, Title
Text, All; Title Dimensions grey), pasted onto each of those axes as on one axis; the whole paste is one
undo step ("Apply Theme"). The first-level **All** stays grey: Sheaf cannot tell it apart from All Axes.

### 26.15 Copy Format: Symbol, Line and Fill, Text and Page Background (T176, 2026-11-19, not measured)

The three were not measured (26.3). Sheaf's choices: the Edit menu's **Symbol/Line/Fill** copies the
plots' lines and symbols (the Symbol, Line and Fill group that All already holds) and **Text** the words of
the text labels (the Text group of All), each alone; the page menu's **Page Background** copies the page
colour, which Paste Format (page menu; it now pastes every group the clipboard holds) and Paste Format...
(Apply Formats, listed as "Page Background") give the target page. All and All Style Formats keep Origin's measured groups and do not carry the page
colour. Each paste is one undo step ("Apply Theme"). **Legend Translation Mode** stays grey (unknown).

### 26.16 Legend Copy Format Dimensions and All (T179, 2026-11-19)

Origin 2016's legend Copy Format lists Styles, Dimensions | All (26.2); OriginLab's legend guide puts the
translation mode and custom format strings under All. Sheaf's choices: **Dimensions** copies the legend's
position (automatic corner or manual place) and its number of columns; **All** copies the styles (26.12),
the dimensions and the legend text (Sheaf's custom text, `None` for the automatic one). The legend's
Paste Format To then lists Styles, Dimensions | All: Styles and Dimensions are grey when the copy does not
hold them (a Dimensions copy holds no styles), and All pastes whatever the copy holds. Each paste is one
undo step ("Apply Theme"); hidden legends are left alone. Clipboards saved before this read as Styles copies.

**Origin 2016 format API (2026-11-21).** On the synthetic legend, Origin C
`GetFormat(FPB_DIMENSION, ...)` returned the position, coordinate units, attachment
and background margins. Applying the tree with `ApplyFormat` changed Left and Top
to the source values. The target kept its text and 9-point font. Applying the
`FPB_ALL` tree changed position, text colour, font size (9 to 19 points) and
the literal legend text. The Dimensions tree did not expose a column count.
Margins and attachment had equal values in this fixture. These results are for
the format API. The context-menu field mapping, column layout and transfer of
the layer's legend translation mode still need live verification.
[Origin C GetFormat](https://docs.originlab.com/originc/ref/originobject-getformat/) ·
[Origin C ApplyFormat](https://docs.originlab.com/originc/ref/originobject-applyformat/)

## 27. Colour scales and bubble scales: Graph > New Color Scale, legend Add Color Scale / Add Bubble Scale (T80)

Not measured: Origin 2016 is on Windows only and this slice was built on macOS from Origin's help, so everything
below that is not a quoted fact is an unmeasured Sheaf choice. The one measured fact is in 14.16 (the bubble size
legend: four circles at the smallest Z and three equal steps up to the largest, the step truncated to two decimals,
labelled 1.00, 3.66, 6.32, 8.98 for Z 1..9, titled with the Z column).

Origin's documentation [W] (https://www.originlab.com/doc/origin-help/colorscale,
https://www.originlab.com/doc/origin-help/bubblescale):
- A colour scale is an object of the layer reached by Insert > Color Scale, its toolbar button and the legend's Add
  Color Scale. It can follow a plot's colour map (Fill > Map) or stand alone, and has its own Colour Scale Control
  dialog (Fill, Levels, Labels, Layout, Title, Line and Ticks). A Bubble + Color Map plot gets a bubble scale by
  default but no colour scale: the user adds it.
- A bubble scale is a label-type object, a legend of sample bubbles with numeric labels, for any symbol plot that
  uses size indexing; Layout is Linear (a row) or Nested; Levels by increment, count or dataset.
- When the items are enabled, where the objects land by default and what they take as defaults are undocumented.

Sheaf (this slice):
- Model: `Layer::scales: Vec<LayerScale>` (`sheaf_core::layer_scale`; serde default, skipped while empty,
  `FORMAT_VERSION` unchanged). `LayerScale { kind: Color | Bubble, title: Option<String> }`. A scale describes the
  first visible plot of its layer that has `color_from` (colour) or `size_from` (bubble); there is no Map setting.
  A scale whose layer has no such plot draws nothing and takes no room.
- Where: right of the frame, side by side in the order they were added, each in a slot of the width the hexbin
  colour scale takes (`hexbin::scale_reserve`; the bubble scale 6 pt less than that plus its widest circle), the
  room added to the layer's label edges (`layer_scale::reserve`). Not drawn on polar and ternary layers.
- Colour scale: the bar of `hexbin::draw_color_bar` (64 strips of `plots::map_color`, title in the Y title font,
  nice ticks from `sunburst::scale_ticks` over `plots::z_range`, the smallest to the largest finite Z of the
  plot's rows). Title: the Z column's name.
- Bubble scale: four circles of the plot's symbol style, sized as the symbols are (`BUBBLE_MIN_PT` to
  `BUBBLE_MAX_PT`, linear in Z) at Z = lo + k * step, smallest at the top, labels with two decimals in the Y tick font,
  titled with the Z column; one circle when all values are equal. The step follows the measured 14.16 rule: `(hi -
  lo) / 3` truncated to two decimals (Z 1..9 gives 2.66 and labels 1.00, 3.66, 6.32, 8.98), and the exact step when
  that truncation gives zero. The truncation rule is inferred from that one measurement, not confirmed on other ranges.
- Items: Graph > New Color Scale (active layer) is enabled when the active layer has a colour mapped plot, else
  grey with a reason; the legend context menu's Add Color Scale / Add Bubble Scale are enabled when the layer has a
  colour (size) mapped plot. One undo step each ("Add color scale", "Add bubble scale"). Graph > New Bubble Scale
  (listed in ui-command-map) is not in Sheaf's menu.
- Selecting: a click picks a scale (`ObjectKind::Scale(i)`, rank of the legend), Delete or its context menu removes
  it ("Delete scale"), a double click opens Plot Details. Selection JSON: `{"kind": "scale", "index": n}`.
- Files: `.sheaf` keeps scales; `.opj` writer leaves them out and the save warns (the companion keeps them).
- Not done: moving by dragging, a properties dialog (orientation, levels, labels, background, nested layout), a
  Map link to a chosen plot, scales for other plot kinds (hexbin, sunburst, waterfall, parallel keep drawing their
  own), a CLI flag, the real default colour map.

## 28. Batch plotting: Window > Duplicate (Batch Plotting) (T99, measured 2026-11-16, Origin 2016)

With a graph active, Origin 2016's Window menu has Duplicate (Batch Plotting) > **Duplicate with New
Sheets/Books...** and **Duplicate with New Columns...** (screenshots `re-workspace/scratch/t99`, local
only; Book1 with two sheets of A (X), B, C, and Graph1 a line plot of Sheet1's B):

- *With New Sheets/Books...* opens **Select Worksheet**: a list of the other sheets (`[Book1]Sheet2`),
  Match Plot Columns by (Column Index), Graph Short Name and Graph Long Name (`<auto>`), OK (greyed until a
  sheet is picked) and Cancel. Picking Sheet2 made **Graph2**, long name **"Graph1 - Copy"**, plotting
  Sheet2's B (`Book1_B@2`): the columns at the same positions of the chosen sheet.
- *With New Columns...* opens **Select Column - [Book1]Sheet1**: the sheet's other Y columns (C), Use
  Associated X (ticked), the two name fields, OK and Cancel. Picking C made a copy plotting C against A,
  the axis title and legend following.
- After making the copy Origin asked "A significant portion of the new data lie outside the current axes
  limits. Would you like to rescale the axes to show all the data?" (Yes by default); the axes were
  rescaled (Y 100 to 425 for Sheet2, 4 to 9 for C).

Sheaf (`sheaf_core::batch_plot`, window `dialogs/batch_plot.rs`): the same two windows with tick boxes
(several sheets or columns at once, one copy each), Match Plot Columns by Column Index, Short Name or Long
Name, Use Associated X; each copy is `<long name or name> - Copy` and its axes rescale without asking
(Origin's default answer). One undo step for all the copies. The graph page's right-click menu has the same
submenu with Duplicate (a plain copy, as Window > Duplicate) first. Not done: the Graph Short/Long Name
fields (Sheaf names the copies itself), books as a whole.

## 29. Graph themes: Save Format as Theme and Theme Organizer (T102, measured 2026-11-16, Origin 2016)

Measured with screenshots (`re-workspace/scratch/t102`, local only):

- **Theme Organizer** (Tools, F7): "A system theme can be applied automatically when making new graphs";
  Current system theme (Dark Colors & Light Grids); tabs Graph, Worksheet, Dialog, Function; a list with
  Name, Size, Path (System or User) and Date (Origin's system themes listed first, e.g. B&W Publication);
  "Right-click to set system theme or to choose other operations"; Apply Theme to (Current Graph), Apply
  Now, Undo Apply; Exclude increment lists (ticked); Close.
- **Save Format as Theme...** (an axis's right-click menu, after Copy Format): Name of the new theme
  (**Theme1**, a list of the existing names), Description, Current System Theme (read-only), Set as System
  Theme, Formats to Save (All ticked and the rest greyed: Colormap, All Styles, Fonts, Colors, Symbol Line
  and Fill, Dimension, Scale, Background, Text), OK, Cancel.

Sheaf (`sheaf_io::graph_themes`, windows `dialogs/graph_themes.rs`): Save Format as Theme... on an axis
copies the axis's format as Copy Format > All does (section 26) and saves it as `<name>.json` in
`graph-themes` of the settings folder (`--graph-themes-dir DIR` at start), with the description and the time;
the name starts as the first free ThemeN and a theme of the same name is replaced. Theme Organizer lists the
saved themes (Name, Size, Path User, Date); Apply Now gives the current graph's every axis the theme (one
undo step, "Apply theme"); Delete removes a theme. Sheaf ships no system themes (Origin's are its own) and
has no system theme, Worksheet/Dialog/Function tabs, Undo Apply button (Edit > Undo does it) or Formats to
Save choice yet.

**Legend themes** (T167, 2026-11-19, not measured): the legend's right-click Save Format as Theme... (last
item of the menu) saves the legend's Styles as Copy Format > Styles copies them (font, frame, background;
section 26.12) through the same window and file; it needs a shown legend. Apply Now of such a theme gives
every shown legend of the current graph those styles (one undo step, "Apply theme"); hidden legends stay
as they are. Origin's legend theme options (Formats to Save) were not measured.

## 30. Vector plots: Plot > Specialized > Vector XYAM and Vector XYXY (T107, measured 2026-11-16, Origin 2016)

Measured with screenshots and LabTalk (`re-workspace/scratch/t107`, local only):

- **Vector XYAM** with A(X), B(Y), C, D selected plots B at its X with C as the angle and D as the
  magnitude (the plot is named `D(M), C(A), A(X), B(Y)`). Plot Details > Vector: Color Black, Width 0.5,
  Arrowheads Length 45, Angle 30, Closed; Magnitude Multiplier (0.16 for magnitudes 50 to 100, 9.6 for 0.5
  to 2), Magnitude in real world space off; Position Tail; Angle Units System, Convention Polar, CCW. The
  "System" unit is radians: angles 0, 45 and 90 drew arrows at 0, 58.3 and 116.6 degrees (45 and 90
  radians). The axes span the start points only (X 0.75 to 4.25 for X 1 to 4).
- **Vector XYXY** needs the columns designated X, Y, X, Y (otherwise "To make this plot, you need to select
  columns designated as 'XYXY'"); it draws an arrow from (X1, Y1) to (X2, Y2), and the axes take in both
  ends (X 0.8 to 2.7 and Y 0.75 to 3.25 for ends up to 2.5 and 3). Plot Details > Vector: Color Black,
  Width 0.5, Arrowheads Length 65, Angle 30, Closed; End Point X End, Y End.
- The archived `xyam.png` and `xyxy.png` captures both show a horizontal, right-pointing
  arrow before the series text in the legend.

Sheaf (`sheaf_core::vector`, `sheaf-plot` `vector_plot.rs`): plot kinds `vector_xyam` and `vector_xyxy`
with the extra columns and the arrow settings in `PlotDecor::vector` (saved in `.sheaf`). XYAM takes the
selection's Y columns three at a time (Y, angle, magnitude), radians by default, length magnitude x
multiplier points from the start; the multiplier is 16 points over the largest magnitude (Origin's 0.16 for
100; Origin's rule for other data is not known: 9.6 for magnitudes up to 2 gives 19.2 points). XYXY takes the
Y columns two at a time with the X column of each and refuses a selection without a second X column. Both
draw a black 0.5 pt shaft and a closed 30-degree head, 4.5 pt (XYAM) or 6.5 pt (XYXY) long (Origin's 45 and
65 read as tenths of a point, matching the size on screen). `sheaf plot -t vector-xyam` / `vector-xyxy`. The
`.opj` writer saves them as scatter plots of their start points and says so.

Plot Details > Vector edits the shaft Color and Width and the arrowhead Length, Angle and
Closed/Open style for both plot kinds. XYAM also has Head, Midpoint and Tail positions,
the magnitude multiplier, and explicit Radians and Degrees options. The saved X,Y point
anchors the selected position in page space. XYXY keeps both data endpoints. Older
`.sheaf` projects load with Tail and Closed. Changes use the live preview, Apply, Cancel
and Undo behavior of Plot Details.

The Origin 2016 captures `pd1-c.png` and `pd-xyxy-c.png` show the available controls and
initial settings. The [Origin Vector tab documentation](https://docs.originlab.com/origin-help/pd-dialog-vector-tab/)
describes the positions, angle units and open head. Sheaf shows head dimensions in points
and renders the open head as two unfilled arms meeting the shaft at its tip. The changed
positions and open-head geometry have no separate native measurement. Origin's exact initial
multiplier rule remains open.

The archived Vector tab shows **Angle** and **Magnitude** in Vector Data for XYAM and
**X End** and **Y End** in End Point for XYXY, with column names such as `Col(C)`. Each
selected column supplies values from the corresponding rows. [Origin Vector tab](https://docs.originlab.com/origin-help/pd-dialog-vector-tab/),
[Origin dataset controls](https://docs.originlab.com/origin-help/plotattribute-datasetcontrol/)

Sheaf offers columns with numeric storage from the plotted Y column's worksheet. Existing
external bindings display their qualified names, and unavailable bindings display Missing
column until a replacement is chosen. A selection changes the corresponding saved vector
column ID and uses the standard Plot Details live preview, Apply, Cancel and Undo. The
selection scope and handling of unavailable bindings are Sheaf policy. Constant values for
XYAM and reassigning to a different worksheet remain open.

The archived XYAM Vector tab shows Polar, Meteorological and Custom conventions. Sheaf's
Polar preset measures counterclockwise from +X. Meteorological measures clockwise from +Y.
The preset offsets and directions are read-only. Custom lets the user set the zero direction
in degrees counterclockwise from +X and choose CW or CCW for the signed data angles, which
use the Angle Units setting. Older projects keep Polar. Origin's documentation gives the
preset values and editable Custom fields. The custom angle calculation follows Sheaf's
reading of those fields. [Origin Vector tab](https://docs.originlab.com/origin-help/pd-dialog-vector-tab/)

The archived XYAM Vector tab shows **Magnitude in real world space**, off by default.
With the switch on, Sheaf computes both endpoints using the scaled magnitude and angle
in X and Y data units. Tail, Midpoint or Head still identifies the data anchor. Changing
the axis scales changes the arrow's page length while its data endpoints stay fixed.
Sheaf's auto ranges include both endpoints of visible rows, subject to each axis scale.
The switch is saved and defaults off in older projects. The exact Origin 2016 auto range
behavior remains unmeasured. Per-axis magnitude options and the reference ruler remain
open. [Origin Vector tab](https://docs.originlab.com/origin-help/pd-dialog-vector-tab/)

Standard legend samples use the same shaft stroke and head style as the plotted
arrows. The sample points right and occupies the regular legend sample width. Sheaf limits
the head to half that width and one font height so long or wide heads stay within the row.
The shared scene carries the sample to the exports.

## 31. Stock plots: Plot > Stock > High-Low-Close, Japanese Candlestick, OHLC Bar Chart (T108, measured 2026-11-16, Origin 2016)

Probed with X = 1..5 (and 1..8) and Open, High, Low, Close columns.

- **High-Low-Close** (Plot > Stock, Plot > Specialized and the 2D Graphs toolbar) takes X and exactly three Y
  columns, read as High, Low, Close. Any other selection is refused: "To make this plot, you need to select
  columns designated as 'XYYY' with exact number of columns". Each row draws a vertical line from high to
  low and a tick to the right at the close. Y title "B - C - D", X title "A".
- **Japanese Candlestick** and **OHLC Bar Chart** take X and exactly four Y columns: Open, High, Low, Close.
  - A candle is a body from open to close, about 0.4 of the X spacing wide, with wicks from the body to the
    high and the low.
  - An OHLC bar is a vertical high-low line with a tick to the left at the open and one to the right at the
    close, each about 0.3 of the X spacing long.
- Colours (Plot Details' "Advancing Price" and "Declining Price" tabs): a day whose close is below its open
  is drawn entirely red (the candle's body filled red, its wicks and the OHLC bar red too). Every other day
  is black, the candle's body hollow (Fill None). Lines are 0.5 pt. One five-day probe left a falling OHLC
  day black; the eight-day probe coloured every falling day red, which Sheaf follows.
- Axes: X ran exactly from the first to the last X (1 to 5, 1 to 8), so the end candles are cut in half;
  Y took in every high and low (8.5 to 15.5 for prices 9 to 15). Origin's X axis was a date axis showing
  "--" for plain numbers; Sheaf keeps a numeric X axis.
- **OHLC-Volume** (T109) takes X and exactly five Y columns, read as Open, High, Low, Close, Volume;
  otherwise "To make this plot, you need to select columns designated as 'XYYYYY' with exact number of
  columns". It makes a two-layer graph:
  - Layer 1 (price) sits at left 8.66 %, top 4.69 %, width 88.23 %, height 65.92 % of the page. It shows
    candlesticks as above but with bodies half the X spacing wide, and its X axis has tick labels on top
    and ticks only at the bottom (showAxes 3).
  - Layer 2 (volume) is linked to layer 1 at left 0, top 100 %, width 100 %, height 34.98 % in units of
    layer 1. Its X axis is shared and drawn at the bottom; its Y axis ran 0 to 325 (step 50, labels "50.0").
    The volume columns are grey (128, 128, 128) with a black border and take half the X spacing.
  - Neither layer has a legend.
- **Stock > Line** (T109) is a line plot of every selected Y column in one layer. Its colours come from the
  theme list (black, red, blue, magenta, green) and it has a legend; Y title "B", X title "A", Y axis
  0 to 325 for volumes up to 300.
- Sheaf: plot kinds `high_low_close`, `candlestick`, `ohlc_bar` with the price columns in
  `PlotDecor::stock` (`sheaf_core::stock::StockSpec`: open, high, low, close, the falling colour, the
  width fraction); templates `high-low-close`, `candlestick`, `ohlc-bar`, `ohlc-volume` (the volume layer's 50 % gap is set on
  the plot, not in the template) and `stock-line`. Sheaf's price layer leaves the usual margin at the X
  ends where Origin ran exactly from the first to the last X. The .opj writer saves the first
  price column as a scatter plot and warns.

## 32. Marginal plots: Plot > Statistics > Marginal Histograms, Marginal Box Charts (T110, measured 2026-11-16, Origin 2016)

Probed with twelve (X, Y) points selected as A(X), B(Y).

- Both make a three-layer graph:
  - Layer 1 is the scatter of B against A, at left 15 %, top 25.01 %, width 60 %, height 60 % of the page.
  - Layer 2 is linked to layer 1 at left 0, top -33.01 %, width 100 %, height 33.01 % (in units of layer
    1), so it sits directly above. It shares the X scale and holds the X values' histogram or box.
  - Layer 3 is linked at left 100 %, top 0, width 33.01 %, height 100 %, directly right. It shares the Y
    scale and holds the Y values' histogram (bars lying on their side) or a standing box.
  - No legends; the margins show no tick labels or titles.
- **Marginal Histograms:**
  - Black scatter dots. Light grey (192, 192, 192) bars with black edges.
  - The bins were 2 wide from 0 for values from 1.2 to 8.8.
  - The count axes start at 0: 0 to 7 for a largest count of 6, 0 to 4.5 for 4.
  - Each margin draws only the base line under its bars, which closes the scatter's frame. The scatter
    itself has only its bottom and left axes.
- **Marginal Box Charts:**
  - Blue scatter dots. Red boxes, each about 0.4 of the box layer's 0.5 to 1.5 position span wide.
  - The top box lies along X; the right one stands along Y.
  - Each box has a median line and the mean as a small hollow square. The whiskers reached the minimum and
    maximum, marked with crosses, for data without outliers.
  - The scatter's frame is closed; the box layers draw no axes. Origin's scatter Y axis ran -2.5 to 12.5.
- Sheaf: plot kinds `marginal_histogram` and `marginal_box`, with the margin in `PlotDecor::marginal`
  (`top`, `right`); templates `marginal-histograms` and `marginal-box-charts`. Plot > Statistics takes one Y
  column with its X column.
  - Sheaf's choices: bins come from the histogram's automatic binning. Box whiskers follow Sheaf's box chart
    rule (1.5 IQR, outliers drawn as small crosses). The scatter dots are 4 pt circles, and the scatter's
    axes keep Sheaf's usual end margins.
  - The .opj writer saves the margins as scatter plots of their values, with a warning.

## 33. QC (X bar R) chart: Plot > Statistics > QC (X bar R) Chart (T111, measured 2026-11-16, Origin 2016)

Probed with 20 values in column B, selected alone.

- Origin first asks "Please specify subgroup size for Book1_B" (window "X bar R chart", default 3).
- It writes a new workbook `QC1`:
  - columns Mean(Y), R(Y) and Sigma(Y), one row per subgroup;
  - a "Make QC Chart" panel (Worksheet, Group Size, Num Sigma 3.0, Column 1, Column 2, Graph Window).
- It then plots a graph `XbarR1`.
- Subgroups are taken in row order, and a short last subgroup is kept: 20 values in groups of 3 gave 7
  subgroups, the last of 2. Sigma is the subgroup's sample standard deviation.
- Statistics (graph table "X bar R chart"):
  - X Bar is the mean of the means; R Bar is the mean of the ranges.
  - Sigma = R Bar / (d2 sqrt(n)), with the exact d2 (1.692569 for n = 3; the rounded 1.693 does not match).
  - UCL and LCL = X Bar +- 3 Sigma.
  - Range UCL = D4 R Bar with the table D4 (2.575 for n = 3); Range LCL = D3 R Bar (0 for n < 7).
  - Probe: X Bar 10.13095, R Bar 0.6714, Sigma 0.2290, UCL 10.81804, LCL 9.443862, Range UCL 1.728929,
    Range LCL 0.
- Layers:
  - Layer 1 (range chart) sits at left 13.99 %, top 47.95 %, width 65.01 %, height 37 % of the page.
  - Layer 2 (mean chart) is linked at left 0, top -107 %, width 100 %, height 100 % (in units of layer 1).
  - X runs 0 to subgroups + 1 with step 1, shared.
  - The mean chart's Y axis runs from LCL - 0.3 (UCL - LCL) to UCL + 0.3 (UCL - LCL); the range chart's
    from 0 to 1.5 Range UCL. Tick labels have two decimals.
  - Titles: "Subgroup Mean (XBAR)", "Range" and "Subgroup Number"; the mean chart has no X tick labels.
    No legends.
- Drawing:
  - The means are small black squares, each with a line to the centre line.
  - The ranges are red, black-edged bars from the centre line, 0.8 of the spacing wide.
  - Each chart has a blue centre line and red limit lines, labelled right of the frame in their colours:
    "UCL = 10.82", "CL = 10.13", "LCL = 9.444", "UCL = 1.729", "CL = 0.6714", "LCL = 0" (four
    significant digits).
  - A hatched band fills the gap between the layers. The statistics table stands right of the page.
- Sheaf (`sheaf_calc::qc`, `Project::plot_qc_xbar_r`, plot kinds `qc_means` / `qc_ranges` with
  `PlotDecor::qc_center`, template `qc-xbar-r`):
  - Same workbook, statistics, layers, ranges, lines and labels. The lines are the layers' straight lines;
    the labels are free text labels.
  - Subgroup sizes are 2 to 10, at 3 sigma.
  - Left out: the hatched band, the statistics table, the "Make QC Chart" panel, and a Num Sigma setting.
  - `plot_columns` with the template (the CLI) uses subgroups of 3.

## 34. Zoom graph: Plot > Specialized > Zoom (T112, measured 2026-11-16, Origin 2016)

Probed with X = 1..50 and Y = 10 sin(X/5), B selected.

- Two layers:
  - **Layer 1** (the whole curve) sits at left 16 %, top 8 %, width 75 %, height 30 % of the page. It is a
    closed box with grey (128, 128, 128) major grid lines, and has no axis titles or legend. Its axes ran
    -10 to 60 and -15.4 to 15.4.
  - **Layer 2** (the zoom) is linked at left -5.979 %, top 127.1 %, width 106 %, height 152 % (in units of
    layer 1), so it sits below it, a little wider. It has a cyan (0, 255, 255) background and bottom and
    left axes only.
- Layer 1 holds a cyan rectangle with a blue edge: the region. Layer 2's X and Y ranges are exactly the
  region's (-5.7125 to 7.704 and -2.759 to 6.716). Dragging the rectangle in Origin moves the zoom.
- The region starts centred on the first data point. Its size is 0.19167 of layer 1's width and 0.30764
  of its height.
- Sheaf (`Project::plot_zoom`, template `zoom`):
  - **Zoom field:** `Layer::zoom` names the rectangle among the root layer's shapes, and the layout gives the
    zoom layer that rectangle's extent in the root's data. Moving or resizing the rectangle like any other
    shape moves the zoom.
  - **Placing the region:** `sheaf_plot::place_zoom_regions` centres the region on the first point on the
    resolved axes. The app and the CLI call it after plotting.
  - **Differences from Origin:** shapes are drawn over the data, so the region hides the curve under it
    (Origin draws the curve on top). The axes keep Sheaf's automatic ranges.

## 35. Scatter matrix: Plot > Statistics > Scatter Matrix (T114, 2026-11-16; measured by T313, 2026-11-21)

Built first from OriginLab's pages for `plot_matrix`. T313 measured Origin 2016's own (its window and
graphs of 2, 3 and 4 variables; probes in `re-workspace/scratch/t313`).

- **Origin's window (measured):** Input, Grouping Range; Matrix Display (Square, Upper Triangular, Lower
  Triangular); Show in Diagonal Cells (`<None>`, Box Chart, Histogram); Variables in Diagonal Cells (ticked);
  Show Tick and Label (None, All by default, Alternate, Bottom&Left, Bottom&Right, Top&Left, Top&Right); Gap (in %
  of Page Dimension), 0; Confidence Ellipse and its level 95; Linear Fit; Pearson's r; Adj. R-Square; Exclude
  Missing Value Listwise; Graph Templates; Output Results. The graph is named ScatterMatrix1.
- **Origin's graph (measured; Sheaf does the same for a square matrix):**
  - The usual landscape page (6432 x 4923). The grid starts 8 % from the left and 5 % from the top, and its cells
    are 87 / n % wide and tall (29 % for three variables, 21.75 % for four, 43.5 % for two) with no gap.
  - Show Tick and Label All: every outer side of the grid has tick labels, the top row's at the top and the right
    column's at the right, except in the diagonal cells. The bottom row and the left column carry the variables'
    names as axis titles, the diagonal cells too. The names are also written in the middle of the diagonal cells.
  - Symbols: blue squares of size 9.
  - Tick step: a quarter of the axis range rounded to two significant digits, the ticks at its multiples (range
    5.25 gives 1.3 with ticks 2.6, 3.9, 5.2, 6.5; 21 gives 5.3; 1.61 gives 0.4).
  - Axis limits follow a rule not found (data 2.3..6.2 gives 1.5..6.75, 12..30 gives 10.5..31.5, 0.4..1.6 gives
    0.23..1.84, 1..10 gives 0..12.25, 0..100 gives -19.5..117, -3..5 gives -4.65..6.2, 100..109 gives 98..110.25,
    0.001..0.010 gives 0..0.01225). Sheaf keeps its automatic range (below).

- Input:
  - Origin takes two or more selected columns, every one of them a variable whatever its designation.
  - Sheaf takes 2 to 8 numeric columns (8 x 8 = 64 layers, the most a grid has), in selection order. With no
    selection it takes the sheet's numeric Y columns, whatever their units.
  - A selection that includes a text column is refused.
- Layout: one graph, one layer per cell of an n x n grid, the cells row by row from the top left.
  - The cell in row r, column c plots column c (X) against column r (Y).
  - So a grid column shares its X variable and a grid row its Y variable. Cells (r, c) and (c, r) are the same
    pair either way round.
- Options (Origin's names, its defaults first):
  - **Matrix Display:** Square (every cell), Upper Triangular (the diagonal and the cells right of it), or
    Lower Triangular (the diagonal and the cells left of it). Cells left out get no layer.
  - **Show in Diagonal Cells:** None, or Histogram (the variable's histogram with automatic bins, counts from
    0).
  - **Variables in Diagonal Cells:** on. The column's long name (else its short name) is written in its
    diagonal cell, in the middle, or near the top above a histogram.
  - **Gap:** 0 to 10 % of the page between neighbouring cells, 0 by default.
  - Origin's Mixed display, box chart and distribution curves on the diagonal, Show Tick and Label modes,
    confidence ellipse, linear fit with Pearson's r / adjusted R² / equation labels, significance marks,
    grouping and listwise missing values are not built. The dialog shows the ellipse, the fit and the
    listwise switch disabled.
- Sheaf's own choices (unmeasured):
  - **Triangular matrices:** the margins above; an upper matrix mirrors them (5 % left, 8 % right, 8 % top, 5 %
    bottom). Only the outer cells carry tick labels: the bottom row (X) and the left column (Y) of a lower matrix,
    the top row and the right column of an upper one, with both axes of every cell moved to those sides. No axis
    titles. A diagonal histogram never labels its counts.
  - **Axes and marks:** no legends. Each cell is a closed box with inward ticks and no minor ticks.
  - **Sizes:** tick labels (18 pt) and tick lengths shrink by 2/n from three variables on, labels to 9 pt at
    least and ticks to half length. Symbols stay at size 9.
  - **Ranges:** every cell gets its X variable's limits and step on X, and its Y variable's on Y, so a grid
    column or row lines up even with missing values. A diagonal histogram's counts are the exception: they
    start at 0 on an automatic scale.
    - The limits are the automatic range of the variable's finite values (T20's rule, the limits a scatter
      of it gets alone). A constant variable gets one unit either side.
    - The step is Origin's: a quarter of the range to two significant digits.
  - **Diagonal names:** each is written at 1.25 times the tick-label size. A long name is made smaller so that
    it takes at most 80 % of the cell's width, assuming an average glyph of 0.55 em. It is centred across the
    cell, at mid-height, or 10 % from the top above a histogram, where a tall bar can run under it.
  - **Recalculation:** the graph is not recalculated from the dialog's settings, as Origin's lock icon does.
    The limits are fixed when the graph is made and stay as they are when the data changes.
- Sheaf:
  - `Project::plot_scatter_matrix(columns, &ScatterMatrixOptions)` in `core/scatter_matrix.rs`, with the
    template `scatter-matrix`. Plot > Statistics > Scatter Matrix opens the options window. OK makes the graph
    in one undo step.
  - CLI: `sheaf plot -t scatter-matrix [--matrix-display square|upper|lower] [--matrix-diagonal
    none|histogram] [--matrix-gap PCT] [--matrix-no-names]`.
  - MCP: the `plot` tool's `matrix_*` options.
  - The graph is ordinary scatter, histogram and empty layers with text labels, so `.sheaf` and `.opj` save it
    with no new fields.

## 36. Wind rose: Plot > Specialized > Wind Rose - Binned Data, Wind Rose - Raw Data (T113, 2026-11-16; Raw Data measured by T324, 2026-11-21)

**Origin 2016, Wind Rose - Raw Data (`plot_windrose`, measured through its menu on 20 rows, speeds also x3 and x0.1;
probes in `re-workspace/scratch/t324`):**

- Options and defaults: From Minimum 0, To Maximum 337.5, Number of Direction Sectors 16 (4, 8 or 16), Direction
  Labels N-NNE-NE... (16), speeds automatic (Step by Increment or Intervals), Orientation Direction (Blowing
  From), Quantity to Compute **Count** (or Percent Frequency), Subtotal Count off, Output Worksheet new.
- It writes a sheet WindRose1 into the book: Bin Centers of the direction column (0, 22.5, ..., 337.5), one Count
  column per speed band (its comment the band, "0 - 2"), and Calm.
- Sectors are centred on 0, 22.5, ...; a direction on a boundary goes to the next sector (11.25 counts under
  22.5).
- Speed bands: the step is the 1-2-5 value nearest a fifth of the top speed (8.5 gives 2, 25.5 gives 5, 0.85
  gives 0.2). The bands run [k s, (k + 1) s) up to the first multiple above the top speed, then an open band
  ">= that" (8.5: 0 - 2, ..., 8 - 10, >= 10; 25.5: 0 - 5, ..., 25 - 30, >= 30).
- The wedges are counts, piled from the slowest band out. Band colours black, yellow, red, blue, green, cyan,
  with black edges. The legend lists the fastest band first. The circle is labelled N, NNE, ... clockwise
  from north. The radius axis runs 0 to 4.5 for a pile of 4, drawn left of the circle. The page also has
  "Calms: 0" and the direction column's name at the bottom left.

**Sheaf** (Raw Data copies the measured bands, counts and colours):

- **Layer:** a wind rose is a polar layer on the polar page (graphing 14.18). North (0 degrees) points up,
  the angles run clockwise in degrees, and the radius is the frequency.
- **Plots:** one plot per speed band, so the legend lists the bands with filled swatches. In each direction
  sector a band is a wedge, piled on the bands plotted before it in the same layer.
- **Colours:** bands are black, yellow, red, blue, green and cyan (Origin's), with black 0.5 pt edges.
- **Binned Data** (measured by T327: Origin 2016 opens no window and plots the selection at once; eight
  directions 45 degrees apart with three frequency columns gave wedges 22.5 degrees wide, piled in column order,
  coloured black, yellow, red, the radius axis 0 to 6.5 for a pile of 6, compass labels, the legend last column
  first):
  - The X column holds the sector directions in degrees. A text X column, such as N, NNE, ..., spreads
    its rows evenly round the circle.
  - Each selected Y column is one band's frequencies, piled in selection order.
  - Wedges are 22.5 degrees wide as Origin's (360 / rows when there are more than 16 rows, Sheaf's choice).
- **Raw Data:** the X column holds directions in degrees; the one selected Y column holds the speeds. 16
  sectors and Origin's speed bands (above, `windrose::raw_bands`); a wedge is the count of the rows in its
  sector and band.
- **Sheaf internals:** plot kind `wind_rose` with `PlotDecor::wind_rose` (sectors, the band's speed range);
  templates `wind-rose-binned` and `wind-rose-raw`. The .opj writer saves the columns as scatter plots,
  with a warning.
- **Legend:** the fastest band (the last plot) first, as Origin's: the legend text is `\l(n) %(n)` down to
  `\l(1) %(1)`.
- **Left:** Origin's WindRose1 sheet, the Calm count and note, compass labels on the circle, and the options
  window (sectors, Percent Frequency, Blowing To).

## 37. Smith chart: Plot > Specialized > Smith Chart (T115, 2026-11-16, **not measured**)

Origin 2016 lists the item under Plot > Specialized (seen in the menu on 2026-11-16). The probes could not
run because Origin's COM class was unregistered (see 36), so this is Sheaf's version, built from the
standard Smith chart and Origin's documented behaviour. Measure it when automation works again.

- **Layer:** a Smith chart is a square layer on the polar page (graphing 14.18) with `Coords::Smith`. It
  holds ordinary line + symbol plots.
- **Data mapping:** each plot's X column is the resistance and its Y column the reactance of an impedance,
  divided by the layer's characteristic impedance `z0` (1 by default: the data are already normalised).
  The point drawn is the reflection coefficient gamma = (z - 1) / (z + 1):
  - a matched load (1 + 0j) is the centre;
  - a short circuit is the left end and an open circuit the right end;
  - pure +j is the top.
- **Axes:** they run from -1 to 1 and are not drawn.
- **Grid:** light grey (204, 204, 204) 0.5 pt circles of constant resistance and arcs of constant
  reactance (+-) at 0.2, 0.5, 1, 2 and 5, and the real axis. The unit circle is a black 1 pt edge.
- **Templates:** `smith-chart`.
- **To measure:** Origin's grid values and labels, its Z0 setting, its page, its plot type, and whether
  it takes impedance or reflection data.

## 38. 2D kernel density: Plot > Statistics > 2D Kernel Density... (T116, 2026-11-16; measured by T289, 2026-11-21)

Origin 2016's item runs `plot_kde2`. T289 measured it on 20 points (scratch `re-workspace/scratch/t289`; the
matrices are in `crates/calc/tests/fixtures/kde2d_origin.rs`).

- **Input:** one Y column with its X column.
- **Origin's dialog:** Method "Bivariate Kernel Density Estimator" (default) or "Rule of Thumb"; plot type
  Contour (default) or Image; a number of lowest-density points to display, 0 by default, with an All tick.
- **Origin's output:** a 32 x 32 density matrix. Each axis runs from min - range / 4 to max + range / 4, both
  ends included. The density is the Gaussian product kernel `1 / (2 pi n wx wy) * sum exp(-((x - xi) / wx)^2 / 2
  - ((y - yi) / wy)^2 / 2)`: fitted to Origin's matrix it agrees to 5e-8 relative.
- **Bandwidths (measured):**
  - Rule of Thumb: `sd * n^(-1/6) / 2` per axis, with the sample SD (n - 1). Exact: 0.412003 and 0.386052
    on the test data. Sheaf matches Origin's matrix to 1e-15 of its peak.
  - Bivariate: data-driven, 0.732066 and 0.662596 on the test data. The rule is not documented. Sheaf uses
    the published diffusion estimator of Botev, Grotowski and Kroese (2010), with the data scaled over the
    grid's extent and 256 cosine terms per axis. It gives 0.7493 and 0.6653 (+2.4 % and +0.4 %), and the
    matrix is within 1.3 % of the peak everywhere. Where the estimator finds no root, Rule of Thumb is used.
- **Sheaf's density:** `sheaf_calc::kde::density_2d_with` (`Kde2dMethod::Bivariate` by default, or
  `RuleOfThumb`) on Origin's grid.
- **Drawing (Sheaf, not yet matched):** Sheaf draws the grid as cells in its shared colour map, from 0 to
  the highest density, which is closest to Origin's Image type. Origin's default is a contour plot. Sheaf
  draws all the points on top as 3 pt black dots (a second, scatter plot), where Origin's default shows 0
  points. The axes take in the grid. No legend. The plot has no method choice yet.
- **Sheaf internals:** plot kind `kde2d`, template `kde-2d`.
- **Not measured:** Origin's contour levels and colours, its colour scale, and its picture (the dialog could
  not be confirmed from the screenshot tool).

## 39. Piper diagram: Plot > Specialized > Piper (T117, 2026-11-16, **not measured**)

Origin 2016 lists the item under Plot > Specialized (seen in the menu on 2026-11-16). The probes could not
run because Origin's COM class was unregistered (see 36), so this is Sheaf's version of the standard Piper
(trilinear) diagram. Measure Origin's input rules and look when automation works again.

- **Input:** six Y columns, in order Ca, Mg, Na+K, Cl, SO4, HCO3+CO3, in any unit (meq/L or percent).
  Each row's cation triple and anion triple are turned into percentages of their sums. Rows with an empty
  or non-positive triple are left out.
- **Geometry:** in one layer with equal X and Y units (`Layer::fit_xy_ratio(1)`), with sides of length 1:
  - **Cation triangle:** Ca at (0, 0), Na+K at (1, 0), Mg at the top (0.5, 0.866).
  - **Anion triangle:** 0.2 to its right, with HCO3+CO3 at the bottom left, Cl at the bottom right and SO4
    at the top.
  - **Diamond:** above them. Each sample sits where the line from its cation point, parallel to the cation
    triangle's right side, meets the line from its anion point, parallel to the anion triangle's left
    side. Pure Ca + HCO3 water is the diamond's left corner, Na + Cl its right corner, Na + HCO3 its
    bottom.
- **Drawing:**
  - Black 1 pt outlines, light grey grid lines every 20 %.
  - Each sample as a red dot with a black edge in all three figures.
  - Corner labels as free labels; hidden axes; no legend.
- **Sheaf internals:** plot kind `piper` with `PlotDecor::piper` (the five columns after Ca), template
  `piper`.
- **To measure:** Origin's column rules (it may take 8 columns with K, CO3 apart), its labels, ticks, colours
  and symbols.

## 40. Data markers and display range: graph Data menu (T118, 2026-11-16, **not measured**)

Origin 2016's graph Data menu has Set Display Range, Reset to Full Range, Mark Data Range (Ctrl+Alt+M) and
Clear Data Markers (Ctrl+Alt+N). They could not be probed (Origin's COM class unregistered, see 36), so
this follows Origin's documented behaviour.

- **Mark Data Range** puts two data markers on the active layer's first plot, at its first and last plotted
  rows (inside its display range when it has one).
  - The markers are drawn as 24 pt black I-beams at the points.
  - A Data tool then runs: a click on a point moves the nearer marker of that point's plot to its row.
    Each move is one undo step. Esc or a right-click ends the tool; the markers stay.
- **Set Display Range** gives every plot with markers the rows between them, both included
  (`Plot::rows`), and removes the markers.
- **Reset to Full Range** plots every row of every plot again.
- **Clear Data Markers** removes every marker of the graph.
- **Sheaf internals:** core `data_markers` (`Project::mark_data_range`, `Graph::set_data_markers`,
  `clear_data_markers`, `set_display_range`, `reset_full_range`); markers in `PlotDecor::markers`.
- **To measure:** Origin's marker look, whether markers are per plot or per layer, and how markers are
  moved (drag or click).

## 41. Add Function Graph: Graph > Add Function Graph... (T119, 2026-11-16; partly measured by T322, 2026-11-21)

**Origin 2016 (measured through its menu, T322):** Graph > Add Function Graph... (in the expanded Graph menu, after
Add Error Bars) adds a function plot F1 to the active layer and opens Plot Details on its Function tab:
Points 100; From and To greyed, with **Auto** ticked, so the curve follows the X axis range; Y(x) = (empty); a
Before Formula Script box; Plot Type Line (or Scatter, Line + Symbol, Column / Bar); a Workbook button that turns
the function into worksheet data; tabs Line, Function, Drop Lines and Label. Origin keeps the curve as a function
dataset. What its OK draws was not read back (the collapsible menu moved the item between runs).

**Sheaf:**

- **The window "Add Function Graph":**
  - y(x) = an expression in Sheaf's expression language (`docs/spec/expression-language.md`), with the
    variable `x` (default `sin(x)`).
  - From and To: the X range; from Graph > Add Function Graph the range the layer's X axis shows (as Origin's Auto,
    but fixed once added), from File > New > Function Plot 0 to 10.
  - Points: the number of evenly spaced points (default 100 as Origin's, at least 2).
  - Plot Type: Line (default), Scatter, Line + Symbol or Column / Bar, as Origin's.
- **OK:** samples the function and adds its curve as a line plot to the active layer, one undo step.
  - An expression error is reported, and the window stays open.
  - A point where the function has no value is missing in the curve.
- **Where the points go:** Origin keeps such a curve as a function dataset. Sheaf writes the points to a
  new workbook, `FuncN` (long name "y = <expression>"), with an X column `x` and a Y column whose long name
  is the expression. The curve is therefore an ordinary plot that saves, exports and edits like any other.
- **Sheaf internals:** `sheaf_core::function_graph::sample`, `Project::add_function_graph`.
- **Left:** a curve that follows the X axis as it changes (Origin's Auto), Before Formula Script, the function as
  a dataset of its own.


- **File > New > Function Plot (T213, not measured):** opens the same window for a graph that does
  not exist yet. OK makes a new graph from the default template with the function's curve in it, as
  one undo step ("New function plot"), and shows it. Cancel makes nothing.
## Analysis markers: graph Data > Analysis Markers (T135, 2026-11-18, **not measured**)

Origin 2016's graph Data menu has Analysis Markers with Hide, Show Partial Ranges, Show All, then Tiny,
Small, Medium and Large. They could not be probed (Origin's COM class was unregistered), so this follows
Origin's documented behaviour. The markers they act on are the data markers of Data > Mark Data Range
(section 40).

- **Scope:** each item applies to every layer of the graph, as one undo step ("Analysis markers"). The
  setting is kept per layer (`Layer::markers`, saved with the project).
- **Which markers are drawn:**
  - Show All (the default): every plot's markers.
  - Show Partial Ranges: only markers that leave out some of the rows the plot draws.
  - Hide: none.
- **Size:** the height of the markers' I-beams: Tiny 8 pt, Small 16 pt, Medium 24 pt (the default, as in
  section 40), Large 36 pt.
- **Analysis locks and Data > Lock Position (T186, not measured).** Origin marks an analysis that
  follows a graph's data with a lock icon on the graph; its hotkey reference lists Data > Lock Position >
  Layer Icon, First Point and Last Point for where it sits. Sheaf draws a lock, on screen only (not in
  exports or prints), for each shown layer whose first visible plot with such an analysis has its Y
  column read by an Auto or Manual analysis link (Recalculate Auto or Manual in the analysis window):
  - a 10 x 8 px padlock, green while the results are current and yellow while a result waits for
    Recalculate; its hover text names the plot, the mode and the state;
  - Layer Icon (the default) puts it just below the layer's icon (T195: clear of it, so the icon keeps
    its clicks); First Point and Last Point
    beside the plot's first or last drawn point (8 px left of it and 12 px above).
  - Lock Position is a page setting (`PageStyle::lock_position`, ticked in the menu), one undo step
    ("Lock position"); the `.opj` written here does not hold it (its save warns that page settings
    were not written).
  - **Click menu (T195):** a click on the lock opens Recalculate, which runs the analysis again on its
    result columns as the column menu's Recalculate does (one undo step, "Recalculate"; the lock turns
    green), and Go to Results, which brings the workbook of its first result column forward on that
    sheet.
  - Not done: Origin's Change Parameters and View > Show > Lock Icons. [Origin hotkeys](https://docs.originlab.com/origin-help/hotkey-in-origin/)

## Tables: Graph > New Table (T136, 2026-11-18, **not measured**)

Origin 2016's Graph menu has New Table, which puts a table object on the graph. It could not be probed
(Origin's COM class was unregistered), so this follows Origin's documented behaviour.

- **Making a table:** Graph > New Table opens the "New Table" window with a grid of 3 x 3 empty cells. Add
  Row, Remove Row, Add Column and Remove Column change the size. OK puts the table into the active layer,
  its top-left corner at 10 % of the frame from the left and top, in the layer's new-label font. It is one
  undo step.
- **Editing:** a double-click on a table opens the same grid, titled "Table". OK changes the cells in one
  undo step.
- **Drawing:**
  - Each column is as wide as its widest cell (at least 2.5 font sizes) plus 0.4 font size on each side.
  - Each row is 1.6 font sizes high.
  - Grid lines are black 0.5 pt; cell text is left aligned and centred in the row.
- **The object:** the whole grid is the object. It selects, moves and deletes like a free text label.
- **Model:** a table is a free text label with `table: true`. Its lines are the rows and tabs part the
  cells, so it saves with the project. The .opj writer writes it as a text label.

## Text label order: Bring to the front / Push to the back (T137, 2026-11-18, **not measured**)

A graph text label's right-click menu (Origin 2016) has Bring to the front and Push to the back. The
labels of a layer are drawn in list order, so the last one is drawn on top.

- **Bring to the front** moves the label to the end of its layer's labels; **Push to the back** moves it to
  the start. Each is one undo step ("Bring text to front", "Push text to back").
- **Selection:** the label stays selected at its new place, and another selected label of the layer keeps
  pointing at the same label.
- **Tables:** tables (T136) are labels and order the same way.
- **Scope:** the order is among the layer's text labels only. Drawn shapes have their own order (T24), and
  across kinds the drawing order stays legend, labels, then shapes. Origin's order across object kinds is
  not measured.

## Rise Time gadget: Gadgets > Rise Time... (T138, 2026-11-18, **not measured**)

Origin 2016's Gadgets menu has Rise Time.... It could not be probed (Origin's COM class was unregistered),
so this follows Origin's documented behaviour. It reuses the Integrate gadget's range.

- **Opening:** the gadget opens on the selected plot of the active layer, or its first visible plot of
  numbers, over the middle half of the X axis. Origin first shows a settings window; Sheaf has none.
- **Moving the range:** its edges and body drag as the Integrate gadget's do.
- **What it measures** (`sheaf_calc::risetime::rise_time`), inside the range (the first unbroken run of
  rows whose X lies in it):
  - The amplitude runs from the smallest to the largest Y.
  - The levels are 10 % and 90 % of the amplitude above the smallest Y.
  - A range whose last value is not below its first is a rise; otherwise it is a fall.
  - The time is the X distance from the first crossing of the first level (10 % for a rise, 90 % for a
    fall) to the next crossing of the other. Crossings are interpolated between neighbouring points.
- **What it draws:** blue lines at the two levels across the range, and red circles at the two crossings.
  The label at the top reads "Rise Time = t" or "Fall Time = t" (six significant digits), or
  "Rise Time: no step in the range".
- **Right-click menu:** Delete (Esc) closes the gadget.

## Intersect gadget: Gadgets > Intersect... (T139, 2026-11-18, **not measured**)

Origin 2016's Gadgets menu has Intersect.... It could not be probed (Origin's COM class was unregistered),
so this follows Origin's documented behaviour. It reuses the Integrate gadget's range.

- **Opening:** the gadget opens on the selected plot of the active layer, or its first visible plot of
  numbers, over the middle half of the X axis. The range drags as the Integrate gadget's does.
- **What it finds:** where the gadget's plot crosses each other visible plot of numbers of the same layer,
  with X inside the range. Crossings are computed by `sheaf_calc::curves::intersections`, with both curves
  taken as polylines in row order; points where the curves only touch are left out.
- **What it draws:** a red circle at each crossing with "(x, y)" beside it (four significant digits). The
  label at the top reads "N intersections", "1 intersection" or "No intersection in the range".
- **Right-click menu:** Delete (Esc) closes the gadget.
- **Limits:** the points are found in the plots' own values. On stacked (offset) plots they are drawn with
  the gadget plot's offset.

## Quick Peaks gadget: Gadgets > Quick Peaks... (T140, 2026-11-18, **not measured**)

Origin 2016's Gadgets menu has Quick Peaks.... It could not be probed (Origin's COM class was
unregistered), so this follows Origin's documented behaviour. It reuses the Integrate gadget's range.

- **Opening:** the gadget opens on the selected plot of the active layer, or its first visible plot of
  numbers, over the middle half of the X axis. The range drags as the Integrate gadget's does.
- **What it finds:** the positive peaks among the rows inside the range (the first unbroken run of rows
  whose X lies in it). It uses `sheaf_calc::peaks::find_peaks` with Peak Analyzer's defaults: a peak is
  higher than its 2 neighbours on each side, and it keeps peaks of at least 20 % of the largest magnitude.
- **What it draws:** a red triangle above each peak with its X value (four significant digits). The label
  at the top reads "N peaks", "1 peak" or "No peak in the range".
- **Right-click menu:** Delete (Esc) closes the gadget.

## Quick Sigmoidal Fit gadget: Gadgets > Quick Sigmoidal Fit... (T141, 2026-11-18, **not measured**)

Origin 2016's Gadgets menu has Quick Sigmoidal Fit.... It could not be probed (Origin's COM class was
unregistered), so this follows Origin's documented behaviour. It reuses the Integrate gadget's range.

- **Opening:** the gadget opens on the selected plot of the active layer, or its first visible plot of
  numbers, over the middle half of the X axis. The range drags as the Integrate gadget's does.
- **What it fits:** the built-in Boltzmann function `A2 + (A1 - A2) / (1 + exp((x - x0) / dx))`, fitted to
  the rows inside the range (the first unbroken run of rows whose X lies in it). It uses Nonlinear Curve
  Fit with its default options and guessed starting values. At least five points are needed.
- **What it draws:** the fitted curve across the range in red, 2 pt. The label at the top reads
  "x0 = …, dx = …", or "Sigmoidal fit failed in the range".
- **Right-click menu:** Delete (Esc) closes the gadget.

## FFT gadget: Gadgets > FFT... (T142, 2026-11-18, **not measured**)

Origin 2016's Gadgets menu has FFT.... It could not be probed (Origin's COM class was unregistered), so
this follows Origin's documented behaviour. It reuses the Integrate gadget's range.

- **Opening:** the gadget opens on the selected plot of the active layer, or its first visible plot of
  numbers, over the middle half of the X axis. The range drags as the Integrate gadget's does.
- **What it computes:** the one-sided amplitude spectrum of the rows inside the range (at least four, all
  with numbers).
  - It uses `sheaf_calc::fft::fft_spectrum` with Analysis > FFT's defaults (rectangle window, amplitude
    correction).
  - The sampling interval is the mean X step of those rows.
- **What it draws:**
  - A small chart in the top third of the range: frequency across, amplitude up, both scaled to their
    largest values.
  - The label at the top reads "Strongest frequency = f" (the largest amplitude above zero frequency), or
    "FFT: too few points in the range".
- **Right-click menu:** Delete (Esc) closes the gadget.
- **Not done:** Origin shows its spectrum in a preview graph window and can output it to a sheet; Sheaf has
  neither yet.

## Statistics and Quick Fit gadgets: Gadgets > Statistics..., Quick Fit... (T143, 2026-11-18, **not measured**)

Origin 2016's Gadgets menu starts with Statistics... and Quick Fit.... They could not be probed (Origin's
COM class was unregistered), so this follows Origin's documented behaviour. Both reuse the Integrate
gadget's range, which opens on the active plot over the middle half of the X axis and drags as before.
They work on the points with numbers among the rows inside the range.

- **Statistics:** the label reads "N = n, Mean = m, SD = s" and, on a second line, "Min = a, Max = b" (five
  significant digits; SD is the sample standard deviation), or "No points in the range".
- **Quick Fit:** a straight line fitted by least squares, drawn across the range in red, 2 pt. The label
  reads "y = a + b x" and "R² = r", or "Quick Fit: too few points in the range" (fewer than two points or no
  spread in X).
- **Right-click menu:** Delete (Esc) closes either gadget.
- **Not done:** Origin's Quick Fit offers other functions and both gadgets can write their results to a
  sheet; Sheaf's do not yet.

## Vertical Cursor gadget: Gadgets > Vertical Cursor... (T144, 2026-11-18, **not measured**)

Origin 2016's Gadgets menu has Vertical Cursor.... It could not be probed (Origin's COM class was
unregistered), so this follows Origin's documented behaviour. It reuses the Integrate gadget's range.

- **Opening:** a thin band (2 % of the X axis) in the middle of the active layer. Its centre is the cursor;
  dragging the band moves the cursor.
- **What it shows:**
  - A red vertical line at the cursor; the label at the top reads "x = …".
  - For every visible plot of the layer whose curve spans the cursor, a red circle at the curve's Y there
    and the value beside it (five significant digits). The Y comes from straight interpolation on the
    first segment that spans the cursor.
- **Right-click menu:** Delete (Esc) closes the gadget.

## Differentiate and Interpolate gadgets: Gadgets > Differentiate..., Interpolate... (T145, 2026-11-18, **not measured**)

Origin 2016's Gadgets menu has Differentiate... and Interpolate.... They could not be probed (Origin's COM
class was unregistered), so this follows Origin's documented behaviour. Both reuse the Integrate gadget's
range: they open on the active plot over the middle half of the X axis, and the range drags as before.

- **Differentiate:**
  - Computes the first derivative at the rows inside the range, with Analysis > Mathematics >
    Differentiate's defaults (`sheaf_calc::differentiate`).
  - Draws it as a small chart in the top third of the range, scaled to its own extent.
  - The label reads "dy/dx = d at x = x", taken at the row nearest the middle of the range, or
    "Differentiate: too few points in the range".
- **Interpolate:**
  - Draws the not-a-knot cubic spline through the points with numbers inside the range, at 200 even
    steps, in red, 2 pt.
  - At least three points are needed. The label reads "Cubic spline through the range", or
    "Interpolate: too few points in the range".
- **Right-click menu:** Delete (Esc) closes either gadget.

## Cluster gadget: Gadgets > Cluster... (T146, 2026-11-19, **not measured**)

Origin 2016's Gadgets menu has Cluster.... It could not be probed (Origin's COM class was unregistered),
so this follows Origin's documented behaviour.

- **Region:** a rectangle on the active plot. It opens over the middle half of both axes. The left and
  right edges and the top and bottom edges each drag one side, and dragging inside moves the rectangle,
  all kept inside the layer frame.
- **Result:** every point with numbers inside the rectangle (edges included) gets a red ring. The label
  reads "N = n, Mean X = x, Mean Y = y", or "No points in the region".
- **Right-click menu:** Delete (Esc) closes the gadget.

## 2D Integrate gadget: Gadgets > 2D Integrate... (T189, 2026-11-20, **not measured**)

Origin's help ("2D Integration Gadget", OriginPro) describes it for a contour or image plot of a matrix
or of XYZ data: the volume from a base to the surface inside a region, with N, minimum, maximum, mean,
standard deviation, sum of Z and FWHM, updated as the region moves. Sheaf's first version:

- **Where:** the active layer when it has a heat map or contour plot (`Layer::field`), else the
  graph's first such layer; on other graphs it says that it needs one.
- **Region:** a rectangle, opened over the middle half of both axes and dragged like the Cluster
  gadget's.
- **Result:** of the grid cells whose centres are inside the rectangle (edges included, missing Z
  left out): "Volume = v", then N, Mean, SD (n - 1), Min, Max and Sum of Z. The volume is the sum of Z
  times the area of one cell (|dX dY| of the grid), from a zero base. XYZ columns are read on the grid
  the plot uses. "No cells in the region" when none is inside.
- **Right-click menu:** New Output (T198) adds a row to the "2D Integrate Results" workbook, made
  the first time with the columns X From, X To, Y From, Y To (the rectangle, smallest first), Volume,
  N, Mean, SD, Min, Max, Sum, FWHM X and FWHM Y; each output is one undo step ("2D Integrate").
  Delete (Esc) closes the gadget.
- **FWHM (T203):** the label's last line, FWHM X and FWHM Y: along the row (and the column) of the
  highest cell in the region, the width of the run around it that stays at or above half the maximum
  (zero base), its ends found linearly between neighbouring cells. Shown as NaN when the values do not
  fall below half on both sides inside the region.
- **Region (T201):** the right-click menu's Region > Rectangle (the default) or Circle, ticked. The
  circle is the ellipse that fills the rectangle (a circle when the rectangle is square on screen,
  drawn in red over it); it takes the cells whose centres are inside it, edge included, and its
  outputs keep the rectangle's From and To. The edges still drag the rectangle.
- **Base (T206):** the right-click menu's Base: Plane Through Boundary (and back with Base: Zero)
  fits the least-squares plane through the region's boundary cells (in each row its first and last
  cell, in each column its lowest and highest), as Origin's help says the boundary is used to fit the
  base plane. Volume and FWHM then use the height above that plane; N, Mean, SD, Min, Max and Sum stay
  statistics of Z. Too few boundary cells for a plane (or all on one line) leave the zero base.
- **Not done:** free-hand regions. [Origin 2D Integration Gadget](https://www.originlab.com/doc/Origin-Help/surface-integ-gadget)

## Step and spline line plots: Plot > Line > Horizontal Step, Vertical Step, Spline Connected (T63 groups A and B, 2026-11-19, **not measured**)

Origin 2016's Plot > Line submenu has these three items under Line. Origin's COM class was
unregistered, so Origin's own templates for them were not checked.

- **Templates:** each is the Line template with another connection between the points: Step,
  horizontal first (`step-horizontal`), Step, vertical first (`step-vertical`) and Spline
  (`spline-connected`). These are the same connections as Plot Details > Line > Connect. The names
  also work with `sheaf plot --template`.
- **2D Graphs toolbar:** Bar, Stacked Area, Box Chart, Histogram and Bubble are live. Each runs the
  Plot menu item of the same name on the selected columns, as Line, Scatter and Column already did.

## Axis Tick Labels menu: Format > Axis Tick Labels (T63 group I, 2026-11-19, **not measured**)

- Format > Axis Tick Labels > X Axis Tick Labels... and Y Axis Tick Labels... open the axis window on
  that axis's Tick Labels tab. Origin's item names say what they open; the window was not compared.
- An axis's right-click Tick Labels... opens the same tab; Scale... and Properties... open the Scale tab.
- The worksheet column header's Frequency Count... (T63 group K) runs the same Frequency Counts
  window as Statistics > Descriptive Statistics > Frequency Counts..., since Origin uses `freqcounts`
  for both.

## Format > Object Properties... on a graph (T150, 2026-11-19, **not measured**)

- With an object of the graph selected, Format > Object Properties... opens what a double-click on
  that object opens:
  - the axis window for an axis or its title (on that axis), Plot Details for a plot or a scale,
    and the legend settings for the legend;
  - the text window for a text label, and the drawn object, slope or straight line window for those.
- With nothing selected it is greyed. On a workbook it stays greyed (Sheaf has no worksheet objects
  yet).

## Histogram + Probabilities: Plot > Statistics (T85, 2026-11-19; Origin 2016 measured in the T85 brief)

- **Graph:** `Template::HistogramProbabilities` (`histogram-probabilities`) needs one column of numbers
  (`Project::plot_histogram_probabilities`). It makes two layers:
  - Layer 1 is the histogram of the column, in the Histogram template's look, with the Y axis titled
    "Counts" as Origin.
  - Layer 2 is a right-Y layer linked to layer 1 (X follows it, as Double-Y), fixed to 0 - 100. It has
    the cumulative percent of the same bins as a blue step line (`StatPlot::HistogramCumulative`, a
    Line plot with the horizontal-first step connection). The line is flat across each bin at the
    percent reached at its right edge and rises at the bin edges, from the first bin edge to the last.
- **Bins:** the curve takes the bins of its own plot's histogram settings. These are the same automatic
  bins as the histogram's until one of the two is changed (Sheaf does not tie them together).
- **Not done:** the Results Log line "Histogram and Probabilities for ..." with Mean, SD, Maximum,
  Minimum and Size. The right axis title ("Cumulative %" in Sheaf) and the exact start of Origin's step
  line were not measured.

## Legend order: right-click Bring to the front, Push to the back, front / back of data (T152, 2026-11-19, **not measured**)

- **Over or under the labels:** a legend is drawn under its layer's free text labels and drawn objects,
  as before. The legend's right-click Bring to the front draws it over them (`Legend::on_top`), and Push
  to the back puts it under them again. One undo step each ("Bring legend to front", "Push legend to
  back").
- **Behind the data:** Push to back of data (`Legend::behind_data`) draws the legend before the plots
  of the layer its items are in, in the overlay part drawn before the data (`LayerScene::underlay_len`).
  Bring to front of data undoes it.
  - A legend that lists several layers is drawn with the last layer, so it goes under that layer's
    plots only.
  - A click where the legend lies under the data picks the plots and axes first.
  - With Data on Top of Axes, the legend goes under the data together with the axes.
  - Hit-testing now reads `LayerScene::data_on_top` for that mode, not "the underlay is not empty".
- Both flags are saved only when set, so older files load unchanged.
- **Cut and Copy (T178, not measured):** the legend's right-click Cut and Copy, and the graph Edit
  menu's Cut, Copy and Ctrl+X/C with the legend selected, copy it as a free text label. The label has
  the legend's text as drawn, in Origin's notation (its custom text, or the automatic
  `\l(1) %(1)` lines), and the legend's font. The page's Paste puts it at the clicked point, as for
  other labels (17.8). Cut also hides the legend, as one undo step ("Delete legend").
- **Plot references in free text labels (T178):** Origin's text objects may hold `\l(n)` and `%(n)`,
  and a legend is such a text object. A free text label that refers to a plot this way is drawn with
  the legend's line layout and the plots of its own layer: a line and symbol sample per `\l(n)`, the
  plot's name per `%(n)`, one row per line. It has no frame and no fill, whatever the legend has. The
  top of its first row sits half a font size above the anchor, so a label keeps about its place when
  `\l(n)` is typed into it. Labels without such references, labels turned away from 0 degrees and
  labels in a layer without plots keep the plain text path, where the escapes stay literal text.
  This rendering is a Sheaf choice; Origin's exact spacing for text objects is not measured.
- **Axis titles (T153):** an axis title's right-click Bring to the front (`Axis::title_on_top`) draws
  it last in its layer, over the labels, drawn objects and legend. Push to the back returns it to its
  place among the axes (the default). One undo step each ("Bring axis title to front", "Push axis
  title to back"). Picking still tries labels before titles.

## Aligning drawn objects and text: Object Edit toolbar Align, Make Same Width/Height (T159, 2026-11-19, from Origin's documentation, **not measured**)

- **Origin (Object Edit Toolbar help page):** each button acts on the selected objects and takes the
  **last-selected** object as the reference. Left, Right, Top and Bottom align the objects with that
  edge of the reference. "Vertical" aligns them on the reference's vertical axis and "Horizontal" on
  its horizontal axis. "Uniform Width" and "Uniform Height" give them the reference's width or height.
- **Several objects selected:** Shift+click on a drawn object or a free text label adds it to the
  selection, and it becomes the reference. Shift+click on one that is already selected takes it out.
  A plain click, a drag or a click on anything else selects one object again. Every selected object
  gets handles. A right click on one of them keeps the selection. Objects in different layers can be
  selected together.
- **Buttons:** Align Left, Align Right, Align Top, Align Bottom, Align Horizontal Center (same centre Y:
  Origin's "Horizontal"), Align Vertical Center (same centre X: Origin's "Vertical"), Make Same Width
  and Make Same Height. They have no menu entries and are state-disabled until two or more drawn
  objects or text labels are selected. Each press is one undo step ("Align objects", "Make same width",
  "Make same height"). A press that changes nothing adds no step. The selection stays, so the next
  button acts on the same objects.
- **Sheaf choices, not measured:**
  - Alignment uses each object's drawn bounds (what the handles surround).
  - A resized drawn object keeps its left (or top) edge. Its new size is the reference's own extent,
    from its two points without line width or arrow heads, or a text label's drawn size.
  - A text label keeps its size, because its font sets it. A line with no extent in that direction is
    left as it is.
- **Index safety:** the selection names drawn objects by their place in the layer's list. Any other
  change to that list (delete, reorder, undo, redo) takes them out of the selection. Deleting a layer
  ends the selection.
- **Group and Ungroup (T162, Origin: "Group the selected objects" / "Ungroup the selected objects",
  not measured):**
  - Group gives the selected drawn objects and text labels a new group number of their layer
    (`Shape::group`, `TextLabel::group`). It needs two or more objects in one layer that are not
    already exactly one group. A member of another group leaves it, and the rest of that group stays
    together.
  - Ungroup dissolves every group a selected object is in. Both buttons are state-disabled when they
    have nothing to do. One undo step each ("Group objects", "Ungroup objects").
  - A click on a grouped object selects the whole group, with the clicked object as the reference.
    Shift+click adds or removes the whole group.
  - A pasted copy of a member is not in the group. Delete removes only the reference object.
  - The group number is saved in the `.sheaf` file only when set, so older files load unchanged.
    `.opj` files do not carry it.
- **Dragging a selection (T162):** a drag that starts on one of several selected objects, or on a
  grouped object, moves all of them by the same page distance. Their handles follow the pointer, and
  the move is made when the button is released, as one undo step ("Move objects"). The selection
  stays.

## 3D Scatter and 3D Trajectory: Plot > 3D Symbol/Bar/Vector (T155, 2026-11-19, **not measured**)

Origin draws its 3D graphs with OpenGL, and they were not probed (its COM class was unregistered).
Sheaf draws a fixed view of the XYZ box in the ordinary 2D renderer, so it exports to every format.

- **Data:** like a ternary plot, each plot of a Y column takes its Z column: the Z-designated column
  after it, else the next numeric column. Z is stored in `Plot::size_from` and sizes no symbol. X is
  the plot's X column, or the row numbers.
- **Layer:** `Coords::Space3d` with an azimuth (turn about Z, 40 degrees) and an elevation (tilt, 25
  degrees), Sheaf's choice.
  - The layer frame is square. Its own X and Y axes are hidden and run from -1.8 to 1.8
    (`SPACE3D_EXTENT`).
  - Each variable's automatic range (with about five ticks) maps to the cube -1..1. A point is drawn
    at `u = x cos a - y sin a`, `w = z cos e + (x sin a + y cos a) sin e`.
- **Box:** the three far planes (the two back walls and the floor) have LT Gray grid lines at the
  major ticks and black outlines.
  - Tick labels and the column names as titles sit on the near edges: X and Y along the floor, Z up the
    near-left edge.
  - Where two labelled edges meet, X leaves out its end label and Z its bottom one.
- **Templates:** `3d-scatter` (symbols) and `3d-trajectory` (line and symbols).
- **3D Scatter + Error Bar (T156, `3d-scatter-error`):** the numeric column after the Z column holds
  the Z errors (`Plot::y_error`). A change of Z moves a point straight up the page, so the ordinary
  error bars draw them, scaled by `2 cos e / (Z range)`.
- **3D Bars (T156, `3d-bars`):** each bar stands from the floor of the box under its point up to the
  point. It is drawn as a flat bar facing the page, red with a black edge, not as a prism.
  - The bars are drawn from the back to the front, so nearer bars cover farther ones. The rows are
    reordered for that, so masks and row filters do not apply to 3D bars.
- **3D Vector XYZ XYZ and XYZ dXdYdZ (T157, `3d-vector-xyz-xyz`, `3d-vector-xyz-dxdydz`):**
  `Project::plot_vectors_3d` takes the Y column's X, then the next four numeric columns after it:
  Z, then the end X, Y and Z (or the changes of X, Y and Z).
  - The arrow runs from each row's point to its end, as the 2D Vector XYXY arrow draws it. The head
    is 6.5 pt at 30 degrees, black, 0.5 pt.
  - The box takes the ends into its ranges. The end columns are in `VectorSpec::a`, `b` and `c`, and
    `delta` marks dXdYdZ.
- **3D XYY (T158):** XYY 3D Bars (`xyy-3d-bars`), 3D Walls (`3d-walls`) and 3D Waterfall
  (`3d-waterfall`, also under Plot > Y-offset/Waterfall) set `Space3dCoords::xyy`.
  - Each selected Y column is one plot. Its values are Z, and it stands at its own depth along Y:
    the first column farthest back, so later columns are drawn over it. No Z column is read.
  - The depth axis has one tick per plot, labelled with the plot's name, and no title. The Z title
    is the first plot's column.
  - XYY 3D Bars are 3D Bars per column. Bars of different columns are drawn column by column, so a
    bar can be misordered against another column's bar.
  - 3D Walls fill each curve down to the floor of the box (Area plots with the floor as their base).
  - 3D Waterfall draws the curves as lines.
- **3D Ribbons (T161, `3d-ribbons`):** a 3D Walls layer with `Space3dCoords::ribbons` set. Each Area
  plot is a ribbon instead of a wall: the curve at its depth widened to half a depth step, from a
  quarter step in front of it to a quarter step behind (Sheaf's width, not measured).
  - Each pair of consecutive rows is one quadrilateral, filled with the plot's fill colour and edged
    with its line. A missing value breaks the ribbon.
  - The quadrilaterals of a ribbon are drawn from the farthest to the nearest; the ribbons in plot
    order, the first farthest back.
- **3D Axis window (T164, Sheaf's window; Origin's 3D axis dialog was not probed):** Format > Axes >
  Z Axis... opens it on Z, and X Axis... or Y Axis... (also Axes...) open it on X or Y when the active
  layer is 3D, in place of the 2D axis dialog (the layer's own axes are the hidden frame of the view).
  Z Axis... is greyed on other layers.
  - X Axis, Y Axis and Z Axis buttons choose the axis; From, To and Increment are each Auto until
    set (`Space3dCoords::scales`). OK and Apply set all three axes as one undo step ("3D Axis").
  - From must be less than To and the Increment more than 0, or the window says so and changes
    nothing. Auto shows what the box shows now.
  - A set From or To fixes that end of the box; with Auto Increment the box keeps its about five
    steps. Points outside the box (and ribbon rows) are not drawn.
  - The Y axis of a 3D XYY layer keeps one tick per plot and is not set here.
  - **View (T172):** Turn (the azimuth, -180 to 180 degrees, default 40) and Tilt (the elevation, 0
    seen from the side to 90 seen from above, default 25) turn the box about Z and tilt it towards
    the viewer, in the same undo step as the scales. The labelled edges and the back planes follow
    the turn. Tools > 3D OpenGL Settings... stays grey: Sheaf draws 3D without OpenGL.
- **3D Surface from XYZ columns (T166, not measured):** Color Map Surface (`3d-colormap-surface`),
  Color Fill Surface (`3d-color-fill-surface`), Wire Frame (`3d-wire-frame`) and Wire Surface
  (`3d-wire-surface`) set `Space3dCoords::surface`. They read X, Y and Z columns as 3D Scatter does
  (Origin also draws them from matrices, which Sheaf does not have yet).
  - Each plot's points inside the box are joined by a Delaunay triangulation over X and Y (in the
    box's units, so both axes weigh alike). Of points with the same X and Y the first is kept; fewer
    than three points, or points on one line, draw nothing.
  - The triangles are drawn from the farthest to the nearest (by their corners' mean depth), each
    edged with the plot's line: black, 0.5 pt. The mesh keeps its line colour when plots are
    recoloured, as a column's outline does.
  - Color Map fills each triangle by its mean Z over the plot's Z range with the colour-mapped
    symbols' map (blue, cyan, green, yellow, red); Color Fill with the plot colour (red first);
    Wire Surface with white, which hides the mesh behind; Wire Frame does not fill, so every line shows.
  - **Colormap Surface with Projection (T168, `3d-colormap-surface-projection`):** the Color Map
    surface, and under it the same triangles flat on the floor of the box in the same colours,
    without mesh lines (each edged in its own colour, 0.25 pt, so that no seams show). The
    projection is drawn before the surface.
  - **X Constant with Base and Y Constant with Base (T170, `3d-x-constant-base`,
    `3d-y-constant-base`):** the points of each X value (or Y value) are joined in order of Y (or X)
    into a curve of Z and filled down to the floor of the box: one wall per value, in the plot
    colour (red first) with the black line as edge. Values with one point draw nothing. The walls
    are drawn from the farthest to the nearest by their mean depth.
  - **3D Ternary Colormap Surface (T173, `3d-ternary-colormap-surface`):** `Space3dCoords::ternary`.
    The plot's X, Y and Z columns are the three parts of a composition (normalised as in a ternary
    layer: rows with a missing or negative part, or a zero sum, are left out); the numeric column
    after Z is the height (`Plot::color_from`).
    - The floor is an equilateral triangle as the 2D ternary layer has it: Z = 1 at the front
      left, X = 1 at the front right, Y = 1 at the back (at Turn 40). The height runs up the box.
    - The box is a prism: grid lines at every 0.2 of each part on the floor, the floor and top
      triangles and the three upright edges in black. The fractions 0.2 to 0.8 label the sides
      (X along Z-X, Y along X-Y, Z along Y-Z); a side behind the middle of the floor has its labels
      under the data. The parts' column names are at their corners; the height's ticks and column
      name are up the leftmost upright edge.
    - The Color Map surface (T166) is drawn over the triangle.
- **Not done:**
  - rotating the view with the mouse or a 3D Rotation toolbar; axis titles, tick label and scale
    type settings of the 3D axes;
  - hiding points behind each other (points are drawn in row order);
  - surfaces from matrices, a colour scale for colour map surfaces, and 3D OpenGL settings.
  - The Data Reader shows the view's coordinates, not X, Y and Z.

