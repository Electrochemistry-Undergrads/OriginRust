# Matrices, heat maps and contour plots

**Summary.** A matrix window holds one or more matrix sheets; a matrix sheet is a grid of numbers
(`rows` x `cols`) whose first and last column are mapped onto an X range and whose first and last row
are mapped onto a Y range. This document says how Sheaf models matrices, which operations it offers and
what Origin 2016 does for each of them (measured with `tools/origin-ref/matrix-gen-*.py`, results in
`crates/core/tests/data/origin_matrix.json`), and how heat maps and contour plots are built and drawn
(measured on pictures Origin 2016 exported of the same matrices). Everything here is our own wording of
black-box observations; no Origin code, template or help text was used. Stage 1 covers the model, the
operations, the plots and the command line; the desktop window, menus and dialogs (stage 2) and the `.opj`
format (stage 3) are not done.

Origin names, for reference: windows are "matrix books" (`MBook1`) of "matrix sheets" (`MSheet1`); the
X-Functions used are `mdim`, `msetvalue`, `mtranspose`, `mflip`, `mrotate90`, `mexpand`, `mshrink`, `mstats`,
`w2m`, `m2w` and `xyz_regular`; the plots are made with `worksheet -p 226 <template>` (templates `contour`,
`contline`, `HeatMap`).

## 1. Model (`crates/core/src/matrix.rs`)

- `Window::Matrix(MatrixBook)`: `id`, `name` (short name, `MBook<n>`, unique among all windows), `long_name`,
  `sheets`, `active_sheet`.
- `MatrixSheet`: `name`, `data_id`, `rows`, `cols`, `x_from`, `x_to`, `y_from`, `y_to`, and three labels
  `x`, `y`, `z` with `long_name`, `units` and `comments` each (the Z label describes the cell values), and the
  numbers row by row (`NaN` is a missing value).
- Coordinates: column `j` (0-based) has X = `x_from + j * (x_to - x_from) / (cols - 1)`, row `i` has
  Y = `y_from + i * (y_to - y_from) / (rows - 1)`; with a single column or row the coordinate is the first
  value. Row 0 is the row of `y_from`, whichever of `y_from` and `y_to` is larger. Ranges may be reversed.
  (Measured: a 4 x 3 matrix with X 10 to 40 and Y 1 to 3 has column X values 10, 20, 30, 40 and row Y values
  1, 2, 3; a reversed range 5 to -5 gives 5, 0, -5.)
- A new matrix sheet is 32 x 32, all missing, with X and Y both 1 to 10 whatever the number of columns and rows
  (measured: the 32 X values run 1, 1.29, ... 10). Changing the size (`mdim`) keeps the ranges, so the spacing
  changes; cells that exist before and after keep their values, new ones are missing (not measured; Origin's
  `mdim` on data was not run). The ranges can be set with the size.
- `data_id` is an identifier shared with window and column ids, never reused; it names the sheet's member in
  the project file.
- Operations are `MatrixOp` values applied with `Project::apply_matrix_op`, so a `Document::edit` makes each one
  one labelled undo step ("Set Matrix Values", "Transpose Matrix", ...) exactly as for workbooks. A failing
  operation changes nothing.

## 2. Project file

See `project-file-format.md`: matrix windows appear in `project.json` as `{"type": "matrix", ...}` and the
numbers of each sheet are the member `data/m<data_id>.f64` (rows * cols little-endian doubles, row by row).
Files without matrices are unchanged and read as before. A heat map or contour plot is a `field` object of a
graph layer (section 6) and refers to its matrix by window id and sheet number, or to three column ids.

## 3. Set Matrix Values

`MatrixSheet::set_values(formula)` evaluates one expression (the language of `expression-language.md`) for
every cell with the variables `i` (row, from 1), `j` (column, from 1), `x` and `y` (the coordinates of the
cell). A result that is not a finite number is stored as missing. Checked against Origin for `i*100+j`,
`x*1000+y`, `sin(x/10)*cos(y)+ln(i+j)`, `i^2 - j^0.5 + mod(i*j,3)`, a conditional with `&&` and `?:`, and a
formula that gives `0/0` (a missing value, as in Origin). Differences to Origin's formula language, which is
LabTalk: `if(c,a,b)` and `iff` do not exist in Origin's matrix formulas (the conditional is `c ? a : b`; Sheaf
also has `ifelse` and `if`); Origin's `mat(1)[i,j]`, `Cell(i,j)`, `wks.ncols` and the like are not available (a
formula cannot read other cells in stage 1); the variable `z` is not the cell's old value in Origin either (it
was measured as 0).

## 4. Structure operations (checked cell by cell and coordinate by coordinate against Origin)

For a matrix with `rows` x `cols`:

| Operation | Result |
|---|---|
| Transpose | `cols` rows x `rows` columns, `new[c][r] = old[r][c]`; the new X range is the old Y range and the new Y range is the old X range (labels swap too) |
| Flip horizontal / vertical | the data are mirrored; the coordinates stay, so every value moves to another coordinate. With "flip coordinates" the X (horizontal) or Y (vertical) range is reversed too, so every value keeps its coordinate |
| Rotate 90 clockwise | `new[r][c] = old[rows-1-c][r]`, size `cols` x `rows`; new X range = old Y range reversed (`y_to` to `y_from`), new Y range = old X range |
| Rotate 90 counter-clockwise | `new[r][c] = old[c][cols-1-r]`; new X range = old Y range, new Y range = old X range reversed |
| Rotate 180 | both axes mirrored; both ranges reversed (the coordinates travel with the values) |
| Expand by `fc` x `fr` (Origin's default 2 x 2) | `cols * fc` columns and `rows * fr` rows; the first and last column and row stay where they are (the ranges do not change); new values by bilinear interpolation at fractional position `k * (n - 1) / (N - 1)` |
| Shrink by `fc` x `fr` (default 2 x 2) | `ceil(cols / fc)` columns and `ceil(rows / fr)` rows; every block of `fc` x `fr` cells becomes its mean, blocks cut short at the edge average the cells they have; ranges unchanged. Missing cells are left out of the mean: not measured in Origin |
| Statistics (`mstats`) | `n` (cells holding a number), `missing`, `mean`, `sd` (sample, n - 1), `min`, `max`, `sum`; measured with two missing cells |

## 5. Worksheet and matrix conversions (`matrix_convert.rs`)

### Worksheet to matrix (Origin's `w2m`, Convert to Matrix > Direct)

Origin's defaults: no coordinates (`xy` = none), trim missing rows and columns (`trim` = 1), tolerance 5 for
even spacing (`tol`), no linear fit (`lr` = 0). Measured results:

- No coordinates: sheet column `c` becomes matrix column `c`, sheet row `r` becomes matrix row `r`; X is
  1..cols and Y is 1..rows. A short column is padded with missing cells. With `trim`, rows and columns that
  hold no number at all are left out; the range still runs from the coordinate of the first to the coordinate of
  the last row (column) that is kept (a matrix of 4 rows whose third row is empty has 3 rows and Y values 1,
  2.5, 4).
- X across columns (`xy` = xcol): each data column has an X value, read from the first data row (`xlabel` =
  row1; that row is then not data) or from a label row of the column (`param`: long name, units, comments or a
  user parameter). With "first column holds Y" (`ycol`) the first column holds the Y values (its first cell, in
  the X row, is ignored); without it Y is 1..rows. The matrix X range runs from the first to the last X value
  (reversed ranges are allowed: descending X values give a reversed range).
- Y across columns (`xy` = ycol): the same with the roles swapped; sheet column `k` becomes matrix row `k`, and
  the first column (`xcol`) holds the X values.
- Even spacing: the coordinates of a matrix are always evenly spaced, so uneven coordinates are refused when they
  deviate by more than the tolerance. Measured: X values 10, 20, 50 are refused with the default 5 and accepted
  with 50 (range 10 to 50); 10, 20, 30.5 are accepted with 5 (range 10 to 30.5); the linear-fit option does not
  turn uneven values into acceptable ones. Our measure of the deviation is the largest difference between a step
  and the mean step, divided by the mean step, in percent; it agrees with all four observations, but Origin's
  exact definition was not found. With the linear-fit option the first and last coordinate are placed on the
  least-squares line through all coordinates (Origin's meaning of `lr` was not checked on accepted data).
- The Z label of the matrix is taken from the first data column (long name, units).

### XYZ columns on a regular grid (Origin's `xyz_regular`)

The distinct X values and the distinct Y values are sorted; the grid step is the smallest gap between
neighbouring values, so a grid with gaps keeps them (measured: X 0, 1, 3 gives columns at 0, 1, 2, 3, with the
column at 2 missing). The result is independent of the order of the points (x-major, y-major, shuffled and
descending were measured to give the same matrix); rows run from the smallest Y up and columns from the smallest
X. A point missing from the grid leaves a missing cell; two points in one cell are averaged (measured: 110 and -5
give 52.5). Sheaf refuses points that do not fall on the grid with an error that says the points are scattered
(Origin, given scattered points, makes a grid of its own with a guessed size that has no meaning; this was
measured for one set and is not copied).

Other gridding methods of Origin (random: Renka-Cline, Shepard, thin plate spline; sparse) are not implemented.
For scattered points Sheaf has its own method, `scatter_to_matrix`: Delaunay triangulation (Bowyer-Watson, up to
20 000 points) read off a regular grid by linear interpolation, `4 * sqrt(n)` nodes on each side (at least 50, at
most 250) over the bounding box; nodes outside the convex hull are missing. It is used by contour plots of XYZ
columns and is not meant to reproduce any Origin method.

### Matrix to worksheet (Origin's `m2w`)

- Direct, no coordinates (default): sheet column `c` is matrix column `c`, same row order.
- Direct with X across columns: the first data row holds the X values and the first column holds the Y values
  (the corner cell is missing); the data follow. With Y across columns the sheet is transposed: the first row
  holds the Y values, the first column the X values (corner missing), and each following row is one X position,
  so sheet column `k` is matrix row `k` (measured).
- XYZ columns: three columns X, Y, Z, one row per cell. Default order "X const first" (X stays while Y runs
  through the rows of that column), or "Y const first". Cells without a number are written as missing rows
  unless "trim" is set (measured).
- Origin can also store the coordinates in the system or user-defined parameter rows of the columns; Sheaf can
  write them into the long name, units, comments or a parameter row, which was not checked in Origin.

## 6. Graphs: heat maps and contour plots

A layer may hold a `field` (`FieldPlot` in `core/graph.rs`); such a layer draws the field instead of curves. The
field has a source (`Matrix {window, sheet}` or `Xyz {x, y, z}`), a kind (contour or cells), flags for fill, lines
and labels, a palette, the levels and a colour scale. The templates (`core/template.rs`, `FieldTemplate`) make
the graph of each kind and set Origin's defaults; the numbers below were measured on pictures of Origin 2016
(page 800 x 600 pixels = 6.4 x 4.8 inches at 125 dpi; the page and frame fractions below are fractions of the
page).

### Templates

| Template | Kind | Fill | Lines | Labels | Palette | Major steps x bands | Colour scale |
|---|---|---|---|---|---|---|---|
| `contour` (Origin: Contour - Color Fill) | contour | yes | at the 7 inner major levels, 0.5 pt black, no labels | no | Rainbow | 8 x 5 = 48 bands | yes |
| `contour-lines` (Contour - Lines) | contour | no | at the 9 inner levels (10 steps), with labels | yes | - | 10 x 1 | no |
| `heatmap` (Heat Map) | cells | yes | no | no | Blue-white-red | 5 x 6 = 30 bands | yes |

Frame of the first two with a colour scale: left 0.14, top 0.113, width 0.5725, height 0.733 of the page; without
a scale (contour lines): left 0.179, width 0.6835. Axes end exactly at the first and last column and row
(contour), or half a cell beyond them (heat map: each cell is a block centred on its coordinates). Tick labels
of a heat map's X axis are written rotated by 90 degrees. Ticks: the step nearest to a seventh of the range, in
1, 2, 5 times a power of ten (Origin: -3..3 steps 1; 0..4 and 0..3 steps 0.5; Sheaf's usual rule would give 2 for
-3..3). Tick and title fonts are 10 and 12 pt, smaller than the line graph template.

### Levels

- Range of the scale: let `s` be the span of the data and `q` the largest of 1, 2, 5 times a power of ten that is
  at most `s / 200`. The lowest level is the data minimum rounded down to a multiple of `q`; the highest level is
  the data maximum rounded up to a multiple of `q`, or the maximum itself when it already is one (within 1e-9).
  Measured on more than twenty data ranges (for example -4.9615..9.9410 gives -5..9.95, 0.1235..0.8765 gives 0.122..0.878,
  13.37..98.76 gives 13.2..98.8, -73.1..12.9 gives -73.2..13, -99123..200456 gives -100000..201000, 0..7.3 stays,
  0.1235..43.12 gives 0..43.2). The user can set both ends instead.
- The range is divided into `major * minor` equal bands; the value of band edge `k` is
  `lo + k * (hi - lo) / n`, and of major level `j` it is `lo + j * (hi - lo) / major`. Float noise is part of
  what Origin shows (a scale from -3.3 to 3.3 has the middle label `-4.441E-16`); the same formulas reproduce it
  (tested).
- A value belongs to band `floor((v - lo) / (hi - lo) * n)`, the highest value to the last band. Values outside
  the scale (only possible with levels set by hand) are painted white above and black below for the rainbow
  palette (white both ways for blue-white-red).
- Labels (colour scale): four significant digits, trailing zeros kept; scientific `d.dddE+nn` when the exponent is
  below -3 or at least 4 (`8.750E-04`, `2.010E+05`, `0.001220`, `9880`, `12.15`, `-4.441E-16`); an exact zero is
  `0.000`. Contour line labels: two significant digits by the same rule (`-3.5`, `4.0`, `-0.51`, `0.98`).

### Colours

The colour of band `k` of `n` is the palette read at `k / (n - 1)`: Rainbow runs linearly through blue (0,0,255),
cyan, green, yellow and red (255,0,0) (measured 48 bands; Origin's channel values are quantised and differ from
the linear ones by at most 6 of 255); blue-white-red runs from (0,0,255) through white (255,255,255) to
(255,0,0) (measured 30 bands, linear to within 2). Gray runs black to white (ours).

### Contour fill and lines

For every cell of the matrix and every band its polygon is cut along the level lines: the corners that lie in the
band plus the points where each of the four edges reaches the lower or the upper level of the band (linear along
edges), taken in order around the cell. Cells with a missing corner are left out. Each band is one path, painted
from the lowest band up, and each band reaches 5 % of a band into the next one so that no hairline of the
background shows between them. Level lines are traced with marching squares (a saddle is decided by the mean of
the four corners) and joined into polylines; all lines of the plot are one path. A line label sits on the longest
line of its level, at a place that differs from level to level (a fraction `0.2 + 0.6 * frac(j * 0.618)` of the
way along the line), on a white box; labels are left out where the line is shorter than 1.5 times the label.
Origin places its labels by a rule we did not find (they stand at various places on the loops); the exact
places differ.

### Heat map

One block per cell, centred on its coordinates, coloured by its band; blocks of equal colour in a row are merged
and every block extends 0.8 pt to the right and down under its neighbours (they are painted from the top left) so
that no hairline shows. Cells without a value are not drawn.

### Colour scale

A bar to the right of the frame, as tall as the frame: it starts `0.0875 * page width` right of the frame and
is `0.04625 * page width` wide. The top and bottom 2 % of its height are the cells for values above and below
the scale; between them the `n` bands. A tick (3 pt) and a label (8 pt) stand at each major level, the first at
the bottom edge of the bands and the last at the top; the whole bar has a 0.75 pt black outline. The title
(12 pt) is centred over the bar, 0.45 of its size above the frame's top; the default is the Z label of the
matrix. (Origin writes the placeholder "Color Scale Title" when there is no label; Sheaf writes nothing.) Axis
titles default to the X and Y labels of the matrix (long name and units in parentheses).

### From XYZ columns

XYZ columns on a regular grid are first made into a matrix (section 5) and drawn as above, as a contour plot or a
heat map. Scattered points are triangulated and read off a grid (section 5) and drawn as a contour plot only; a
heat map of scattered points is refused. Origin draws such data with its own "triangular contour" template
(`TRICONTOUR`), which paints the triangles themselves; Sheaf's grid gives jagged edges at the border of the hull
where the grid cells with a missing corner are left out. Not compared with an Origin picture (the Origin run
that was meant to make one did not finish).

## 7. Command line (`sheaf matrix`)

| Command | What it does |
|---|---|
| `matrix new -o P --cols N --rows N --x-range=A:B --y-range=A:B --formula F` | adds a matrix to project P (made if needed) |
| `matrix convert INPUT -o P [--xyz] [--columns A,B,C] [--layout none|x-across|y-across] [--labels none|row1|param:L] [--coords-in-first-column] [--tolerance 5] [--linear-fit] [--no-trim]` | worksheet or XYZ columns of a data file or project to a matrix |
| `matrix set P [--formula F] [--cols N --rows N --x-range --y-range]` | Set Values and Set Dimensions |
| `matrix transform P transpose|flip-horizontal|flip-vertical|rotate-cw|rotate-ccw|rotate-180|expand|shrink [--cols f --rows f] [--coordinates]` | structure operations |
| `matrix to-sheet P [--method direct|xyz] [--layout ...] [--y-const-first] [--skip-missing] [--export FILE]` | matrix to a new worksheet |
| `matrix stats P` | n, missing, mean, sd, min, max, sum |
| `matrix plot INPUT -o fig.png [-t contour|contour-lines|heatmap] [--xyz X,Y,Z] [--major N --minor N --z-min --z-max] [--lines|--no-lines] [--labels] [--no-scale] [--palette ...] [--save P]` | heat map or contour plot of a matrix or of XYZ columns |

`--matrix` and `--sheet` choose a matrix window and sheet. Results are saved back to the project (or to `-o`);
saving as `.opj` writes a companion `.opj.sheaf` because the Origin writer does not store matrices yet (it names
each matrix as not written).

## 7a. The matrix window in the app (T192 slice 2, 2026-11-19, not measured)

- File > New > Matrix... and the Standard toolbar's New Matrix add a matrix book with one sheet of
  Origin's default size (32 x 32 missing values), named MBook1, MBook2, ..., as one undo step
  ("New matrix"). Sheaf asks for no size first; Set Dimensions is the later Matrix menu.
- The window shows the active sheet's cells in a grid, with the column numbers across the top and the
  row numbers down the left. Missing values show as `--`, and numbers use the General format. A book
  with more than one sheet shows its sheet tabs below the grid. A tab click changes the active sheet as
  view state, with no undo step.
- A double click on a cell edits its number. Enter or a click elsewhere writes it as one undo step
  ("Edit matrix cell"). Esc cancels. Empty text or `--` clears the cell. Text that is not a number
  changes nothing and says so on the status bar.
- Not yet in the app: the Matrix menu (Set Values, Set Dimensions, Transpose, ...), X/Y coordinate
  headers, selection and copy, Plot > Contour/Heat Map from a matrix window, and Convert to Matrix in
  the Worksheet menu. The command line already has all of these (section 7).

## 7b. The Matrix menu and Plot > Contour/Heat Map in the app (T192 slice 3, 2026-11-19, not measured)

- A matrix window has its own menu bar: File, View, Plot, Matrix, Tools, Window, Help. Each item
  works on the active sheet and is one undo step named after the operation (section 4).
  - **Set Matrix Values...:** a formula in i, j, x and y.
  - **Set Dimensions/Labels...:** columns, rows and the X and Y ranges, filled from the sheet.
  - **Transpose.**
  - **Rotate90:** 90 CCW, 90 CW, 180.
  - **Flip:** Horizontal, Vertical. The coordinates are not mirrored.
  - **Expand...** and **Shrink...:** column and row factors, 2 by default.
  - **Convert to Worksheet:** Direct, with no coordinates, into a new workbook.

  The labels follow Origin's documented Matrix menu but are not measured. A window shows the refusal
  of its operation (for example a bad formula) and stays open.
- Plot > Contour/Heat Map > Color Fill, B/W Lines + Labels and Heat Map make a graph from the
  sheet's templates (section 6). In a matrix window they plot the active sheet. In a workbook they plot
  three selected columns, in selection order, as X, Y and Z: a contour from scattered or gridded
  points, and a heat map only from points on a regular grid. Gray Scale Map, the polar and ternary
  contours and the profiles stay grey.

## 7c. Worksheet > Convert to Matrix in the app (T192 slice 4, 2026-11-19, not measured)

- **Direct** converts the selected columns, or every column of the active sheet when none is
  selected, to a new matrix book. It uses `w2m` with Origin's defaults (section 5): no coordinates,
  5 % tolerance, empty rows and columns trimmed. Sheaf opens no options window; the command line
  has the options (`sheaf matrix convert`).
- **XYZ Gridding** takes exactly three selected columns as X, Y and Z on a regular grid
  (`xyz_regular`, section 5). Scattered points are refused with the core's message. Origin's
  random/sparse gridding methods are not built.
- Each conversion is one undo step ("Convert to Matrix"). Extract stays grey.

## 7d. Analysis > Mathematics > Inverse (T193, 2026-11-20, from Origin's `minverse` documentation, not measured)

- The matrix window's Analysis > Mathematics > Inverse..., and the same item in the workbook and
  graph Analysis menus, invert the active square matrix sheet. The item is disabled unless a matrix
  window is active. The inverse goes to a new matrix book named "Inverse of MBookN", which keeps the
  sheet's ranges and labels. It is one undo step ("Matrix Inverse").
- The method is Gauss-Jordan elimination with partial pivoting. It refuses a matrix that is not
  square, that has a missing value, or that is singular (a pivot below 1e-12 of the largest cell),
  and says why on the status bar. Sheaf opens no options window: `minverse` has only its input and
  output.

## 7e. Analysis > Signal Processing > FFT > 2D FFT and 2D IFFT (T196, 2026-11-20, not measured)

- **Where:** the matrix window's Analysis > Signal Processing > FFT and the workbook and graph
  Analysis menus. The items work on the active matrix window and are disabled otherwise. Each run
  makes a new matrix book ("2D FFT of MBookN" / "2D IFFT of MBookN") as one undo step.
- **2D FFT:** transforms the active sheet's values into four sheets: Real, Imaginary, Amplitude
  (|F|) and Phase (atan2, in radians).
- **2D IFFT:** uses the book's sheets named Real and Imaginary when it has both, of one size, so a
  2D FFT book transforms back. Otherwise it uses the active sheet as real values. It writes Real and
  Imaginary.
- **Conventions:** numpy's `fft2`/`ifft2`. The forward transform is not normalized, and the inverse
  divides by rows x cols, so IFFT(FFT(a)) = a. The output is not shifted: the zero frequency is the
  first cell. Missing values are refused.
- Origin's own `fft2` options (normalization, shift, amplitude units, windowing) are not measured.

## 7f. File > Import > Binary 2D Array (T199, 2026-11-20, from Origin's `impBin2D` documentation, not measured)

- File > Import > Binary 2D Array... asks for a file, then opens the options window:
  - **Data Type:** 8/16/32-bit signed or unsigned integers, or 32/64-bit floats.
  - **Columns** and **Rows.**
  - **Header Bytes** to skip.
  - **Byte Order:** little or big endian.
- The numbers are read row by row into a new matrix book whose long name is the file's name. The
  import is one undo step ("Import Binary 2D Array"). Bytes after the array are ignored, and NaN or
  infinite floats become missing values.
- Sheaf's starting guess: 32-bit floats, square when their count is a square number, else one row.
  Changing the type guesses the size again. A file too short for the options is refused, and the
  window says why and stays open.
- Not built: Origin's import filters and templates for binary files, and several frames from one file.

## 7g. Plot > Contour/Heat Map > Gray Scale Map (T201, 2026-11-20, not measured)

- A heat map of the matrix (or of XYZ columns on a regular grid) whose cells go from black at the
  lowest band to white at the highest. It has the heat map's levels (5 major steps of 6 bands), frame
  and colour scale (section 6) and no level lines. Template `grayscale` (`FieldTemplate::GrayScale`),
  also `sheaf matrix plot --template grayscale`.
- This is Sheaf's reading of Origin's Gray Scale Map; Origin's own levels and layout are not measured.

## 7h. File > Import > Image (T204, 2026-11-20, not measured)

- File > Import > Image... reads a PNG, JPEG, GIF or BMP picture into a new matrix book named after
  the file, as one undo step ("Import Image"). It works with no window open or with a workbook.
- Each pixel becomes its luminance, 0 (black) to 255 (white), from the `image` crate's Rec. 709
  weights. The first matrix row is the picture's top row. The X and Y ranges are the default 1..cols
  and 1..rows.
- **Colour (T218):** a picture with any coloured pixel gets three sheets, Red, Green and Blue, each
  0 to 255. A gray picture gets one sheet named Gray. Origin keeps colour pictures as one RGB image;
  Sheaf keeps the channels as plain matrices, so every matrix tool works on each of them. TIFF and
  other formats are left.

## 7i. The Image menu on a matrix (T207, 2026-11-20, Sheaf's formulas, not measured)

- **Where:** the matrix window's Image menu and the workbook's Image menu. The items work on the
  active matrix sheet, in place, as one undo step each ("Image Brightness", ...). They are disabled
  unless a matrix window is active. Missing cells stay missing.
- **Adjustments:**
  - Brightness adds an amount (10 by default).
  - Contrast stretches the values about their mean by a factor (1.5 by default).
  - Gamma maps each value's place t between the sheet's lowest and highest value to t^(1/gamma)
    (1.5 by default).
- **Arithmetic Transform > Math:** +, -, * or / by an operand. Division by zero is refused.
- **Conversion > To Binary:** 1 where the value is at least the threshold, else 0. The threshold
  starts at the sheet's mean.
- **Geometric Transform:** Rotate by 90 CCW, 90 CW or 180 (90 swaps the size), and Flip, horizontal
  or vertical.
- **Spatial Filters:** these run with no window, on 3 x 3 neighbourhoods. Edge cells repeat
  outward.
  - Smooth: the mean.
  - Sharpen: 5v minus the four neighbours.
  - Edge Detection: the Sobel gradient magnitude.
- **T210 additions:**
  - **Levels:** input low/high, which start at the sheet's range, to output low/high (0..255). Values
    outside the input range take the nearest end.
  - **Subtract:** the active sheet of another matrix book of the same size, cell by cell.
  - **Crop:** 1-based first and last row and column, with the cells' X and Y coordinates kept.
  - **Resize:** a new size by bilinear resampling, end to end, with the ranges kept. A missing
    neighbour makes a missing cell.
- **Conversion (T218):** these need a book with Red, Green and Blue sheets of one size; any other
  book is refused with a note.
  - **Color to Gray:** a new book "Gray of MBookN" with the luminance 0.2126 R + 0.7152 G + 0.0722 B.
  - **RGB Split:** three new one-channel books, "Red of MBookN" and so on.
  - Each is one undo step.

## 7j. Analysis > Fitting > Nonlinear Surface Fit on a matrix (T221, 2026-11-20, not measured)

- The matrix window's Analysis > Fitting > Nonlinear Surface Fit... first writes the active sheet to
  a new workbook of X, Y and Z columns (`m2w` XYZ, X constant first, missing cells skipped), as one
  undo step ("Matrix to XYZ for surface fit"). It then opens the usual surface-fit window (Find Z,
  the 22 surface functions) on that workbook's Z column.
- **Sheaf choice:** Origin fits a matrix directly; Sheaf goes through XYZ columns, so the fit
  results, residuals and report are those of an ordinary XYZ surface fit.

## 8. What was not verified, and what is left

- Not run in Origin: `mdim` on a matrix that holds data; shrink with missing values; `w2m` options `lr` on accepted
  data and the exact tolerance formula; `m2w` with system or user parameter rows; XYZ gridding of scattered data
  by any of Origin's methods; Origin's triangular contour picture; Origin's minor-tick, line-label and
  colour-scale-title details; contour plots of data with missing values; reversed-axis matrices in plots.
- Stage 2 (app): the matrix window (cells, column and row headers with coordinates, sheet tabs), menus for matrix
  windows (Matrix, Plot > Contour, Image), dialogs (Set Values, Dimensions, Convert, Plot Details levels tab),
  New > Matrix, explorer type "Matrix", copy and paste of cells.
- Stage 3 (`.opj`): reading and writing matrix windows in Origin projects.
