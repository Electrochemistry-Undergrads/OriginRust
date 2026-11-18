# How Origin looks and behaves: the interface, described for Sheaf

Interface compatibility reference for workbook and graph windows, menus, dialogs and controls.
Reference images are local development artifacts; filenames below identify the relevant views.

Tags: **[seen]** means seen directly in one of those pictures (first study) and **[seen 2]**
means seen in a picture of the second study (look in `gui-study2/`). **[spec]** means taken from
an existing spec (mostly `ui-command-map.md`, `feature-inventory.md`, `graphing.md`). **[not
seen]** marks something the pictures did not show; it is listed so nobody mistakes it for
verified. Sizes are in screen pixels of the main window on a Windows 11 screen at 100 % scaling
(the window placed at the top-left corner, 1600 x 1000, on a 1920 x 1080 screen); they are good
to a few pixels.

The reference views cover workbooks with numeric and label columns, line graphs, column
graphs and their settings dialogs. Screenshot and setup utilities are in `tools/origin-ref/`.

Known weaknesses of the pictures: the Windows desktop behind and beside the window (other
programs) appears in some pictures because the capture copies screen pixels; ignore those
parts. The column header, cell, row header, sheet tab, legend and layer icon menus and the File,
Edit, View, Window and Help menus that the first study missed or only saw in their short form
are now complete (section 3, 4 and 5). Colours were judged by eye from the pictures, not sampled.

## 1. Screen zones of the main window (top to bottom, left to right)

Picture: `fresh/empty.png` (no project windows), `gr-plain.png`, `pe-explorer.png`.

1. **Title bar** (about 30 px) [seen]: small program icon, then the text "OriginPro 2016
   64-bit - " followed by the project path (here `...\User Files\UNTITLED`), with a `*` at the end
   when the project has unsaved changes [seen]. Standard Windows minimise, maximise, close buttons at
   the right.
2. **Menu bar** (about 25 px, text row at y 43) [seen]. It changes with the active child window:
   - No child window: File, View, Tools, Window, Help [seen] [spec A0 id 480].
   - Workbook: File, Edit, View, Plot, Column, Worksheet, Analysis, Statistics, Image, Tools,
     Format, Window, Help [seen].
   - Graph: File, Edit, View, Graph, Data, Analysis, Gadgets, Tools, Format, Window, Help [seen].
   - Matrix, Layout, Notes, Excel, Image windows have their own bars [spec A0].
   Every item has an underlined Alt-letter. Long menus show only the commonly used items first and
   end with a small "double chevron" row that expands the rest [seen in Graph, Tools, Help,
   File, Edit, View and right-click menus; sections 3 to 5]; there is also a "Most Recently Used Items" entry at the end of Analysis,
   Statistics, Image and Plot [seen].
3. **Toolbars below the menu bar** (two rows, each about 26 px) [seen]. They are docked, have a
   dotted grip at their left end and a small "more buttons" arrow at the right end.
   - Row 1, "Standard" (large set of about 50 buttons, in groups separated by thin gaps): new
     (project, folder, workbook, Excel, graph, matrix, layout, notes), open, save, recalculate,
     import (wizard, single ASCII, multiple ASCII), print, refresh/duplicate/zoom box ("100 %"
     drop-down), panel toggles (Project Explorer, Object Manager, Results Log...), add columns,
     then the "Worksheet data" group (statistics, sort, set values, filter) and the "Column" group
     (X, Y, Z, error, label, disregard, move) [spec C] [seen as icon groups].
   - Row 2: Edit group (cut, copy, paste), then the Format toolbar: font name box "Default: Arial",
     size box, bold, italic, underline, superscript, subscript, Greek, bigger, smaller, alignment;
     then Style: fill colour, border colour, line colour, line style, line width box, border
     box, pattern, borders; then more buttons for mask and lock [seen].
   Buttons are small (16 px icons, 22 px cells), flat, with a light blue frame on hover [seen].
   Drop-down boxes are greyed when the active window cannot use them (the empty window shows
   them all grey) [seen]. Contents change with the window: a graph window enables the font and
   style boxes and shows the line width box with "0.5" or "1.5" depending on the selected object [seen].
4. **Tools toolbar** (vertical, left edge, x 33 to 55, y 115 to 720) [seen]: pointer (selected by
   default), zoom in, zoom out ("scale"), screen reader, data reader, data selector, draw data,
   region selection, text tool "T", arrow, line, rectangle, hand ("pan"), and further tools
   (rotate, slideshow like items) after a divider; a second small group below it. Only the pointer is
   shown pressed. The names are in the spec (Tools toolbar row of section C).
5. **Auto-hide panel tabs** (very left, x 5 to 25) [seen]: vertical text tabs "Project Explorer",
   "Quick Help", "Messages Log", "Smart Hint Log". Hovering or clicking one slides the panel out
   over the workspace (`pe-explorer.png`: the Project Explorer slid out, 280 px wide, over the
   workspace, with title "Project Explorer", a pin and a close button). The panel can be pinned to
   stay docked. The Object Manager is the same kind of panel on the **right** edge: tab text
   "Object Manager" at x 1575, panel about 90 px wide at x 1478 to 1570, showing an "Apps" panel
   by default (button "Add Apps") [seen]. `dlg-export.png` shows the Object Manager open: a tree
   Graph1 > Layer1 > g1 > plot names, each with a tick box, coloured line samples for the plots [seen].
6. **Workspace** (the dark-grey area, x 63 to 1446, y 108 to 943) [seen]. Colour about
   RGB(171,171,171). It holds the child windows (section 2). When a child is maximised it fills this area.
7. **Right toolbars** (vertical, x 1450 to 1475) [seen]: two columns of small buttons in groups:
   layer / axis tools (top group), page layout tools, then alignment, "make same size", order,
   group buttons (bottom). They are the "Object Edit", "Layer" and "Arrow" style toolbars [spec C].
8. **Bottom toolbar** (y 959, about 26 px high, x 63 to 1020) [seen]: the 2D Graphs
   toolbar. In a workbook and in a graph alike it begins with: line, scatter, line + symbol,
   column, area, high-low-close like stock plot, pie, and more, with drop-down arrows that open a
   gallery of variants (the spec lists them under "2D Graphs"). Next come the 3D / contour group, the mask
   group and the "Layout" group, finishing with a font-size box ("10") [seen].
9. **Status bar** (about 18 px, y 984) [seen]. Left: a hint text for the item under the mouse
   (e.g. "Select and arrange toolbars"). Right, in boxes separated by thin lines: for a
   selected data range "Average=12 Sum=120 Count=10", then "AU : ON" (auto update of
   dependent items), then the active object as a reference such as `1:[Book1]Sheet1!` and
   finally the angle unit "Radian" [seen]. In a graph the box shows the theme
   "Dark Colors & Light Grids", the selected column range `1:[Book1]Sheet1!Col("Signal")[1:10]` and
   the layer reference `1:[Graph1]1!1` [seen].
10. **Docked windows not seen open by default** [spec F]: Project Explorer docked left (Alt+1),
   Results Log (Alt+2), Command Window (Alt+3), Code Builder (Alt+4), Quick Help (Alt+5),
   Messages Log (Alt+6), Smart Hint Log (Alt+7), Object Manager (Alt+8). Where each docks when
   opened is [not seen] except Project Explorer and Object Manager.

**Project Explorer** [seen, `pe-explorer.png`]: split in two. The top part is a folder tree with
the project at the root (here "UNTITLED", a folder icon, highlighted). The bottom part is a list
with columns "Name", "Long Name", "Short Name", "Type" (sorted by Name, an arrow in the header)
listing the windows of the selected folder, each with its window-type icon; the active window is in
bold [seen]. The horizontal divider can be dragged. Context menus [spec F].

**Default fonts and colours of the interface** [seen]:
- Menus, dialogs, panels and toolbars use the Windows system UI font (Segoe UI, 9 pt, so about
  12 px text). Menus of the main window use a flat white popup with a thin grey border, items
  22 px high, a left icon column, shortcuts right-aligned in the same colour, disabled items in grey
  [seen]. Hover colour in menus is light blue.
- Window frame area: white title bar. The menu bar and toolbars sit on a very light grey/white
  background (about RGB(250,250,252)) [seen].
- Worksheet cells use Arial, about 9 pt, black on white; the column label rows use a
  light yellow tint (about RGB(255,255,204)); the row-number and the label-name cells are mid grey with
  a raised look [seen].
- Graph default text is Arial; the axis title dialog and the Text Fonts page of Options both show a
  size of 22 for the default text [seen 2, `ax-title.png`, `opts-numfmt.png`]; the older note of 18
  pt in graphing.md is not what this installation shows. Tick label size was not read.

### Existing command entry points (T63)

Sheaf's Standard Duplicate button uses the active workbook or graph, even when a different
Explorer row is selected, and is state-disabled without an active window. Explorer's window
context Duplicate uses that clicked window, including a hidden one. Both call the existing
Window > Duplicate operation, show and activate the copy, and create one undo step. Four UI
regressions cover target identity, copies of empty/populated books and graphs, hidden windows,
context cancellation and Undo/Redo; the existing Explorer and Window-menu tests also pass.

The Plot > Statistics Probability Plot and Q-Q Plot entries remain unavailable, assigned to
B8/T14 after the T63 C review. The published [plot_prob defaults](https://docs.originlab.com/x-function/ref/plot_prob/)
include Blom scores and a 95% confidence band; Sheaf's existing descriptive-statistics tools
use Filliben positions and lack that band control. Their dispatch cannot establish equivalent
Plot-menu behavior. The existing tools remain documented in statistics section 16; no new
Origin 2016 numeric measurement was made by this entry-point review.


### Format bar: selected graph text (Sheaf, T58 slice 3)

For a selected title, label or legend, the bar shows its installed font family and point size,
pressed Bold/Italic state, size increase/decrease and a Font Color menu with all 24 Classic
colours (eight basic choices plus More Classic colors) and Custom RGB... with validated
0 to 255 fields and a preview (graphing 7.4). Without a text target,
or while a dialog/cell editor owns editing, these controls are disabled; the size reads 0.
Enter commits a size draft; Escape or loss of focus cancels it. Text and worksheet numbers are
unchanged. Graphing 7.4 records the Origin 2016 size/font/colour controls and the unresolved
native whole-object Bold/Italic difference. Other Format controls remain disabled in this slice.

## 2. Child windows

Pictures: `gr-plain.png`, `grc-page.png`, `rc-titlebar.png`; second study `graph2-plain.png`,
`max-book.png`, `max-graph.png`, `max-window-menu.png`, `min-three.png`, `min-menu.png`.

- Every workbook, graph, matrix, layout and notes window is a **child window** that floats inside
  the workspace [seen]. It has its own small title bar (about 22 px) with: window icon (a grid for
  workbooks, a graph icon for graphs), the window name, a `*` after the name when
  the window changed, and three buttons at the right: minimise (a flat bar), maximise/restore
  (a box) and close (an X) [seen]. The title shows the short name by default; long name and short
  name can be switched in the Properties [spec F].
- The **active** window has a blue-ish title bar frame and a red close button; inactive windows
  have grey text, a pale title bar and a grey close button [seen]. The active window is drawn in
  front; clicking another window brings it forward (cascade-like stacking by click order) [seen].
- Default placement is a cascade: each new window starts a little right and below the last, with
  Book1 at about (85,130) and a size of about 395 x 320 px, Graph1 at (110,157) and Graph2 at
  (137,181), both about 505 x 345 [seen, seen 2].
- **Maximised window** [seen 2, `max-book.png`, `max-graph.png`]: the window fills the whole
  workspace and its own title bar disappears. The window icon moves to the far left of the menu
  bar row, and three tiny buttons (minimise, restore, close) appear at the far right of that
  row (at about x 1546 to 1582, y 45). The main title bar gets the window name in square
  brackets after the project path, for example "...\UNTITLED * - [Book1 *]". A maximised
  workbook shows as many rows as fit (41 here, row numbers continuing past the data), keeps the
  column width of 67 px, and has its sheet tab and horizontal scroll bar at the bottom of the
  workspace (y 936). A maximised graph scales the page to the workspace height; the page keeps
  its aspect ratio, and the area on its right stays mid grey; fonts grow with the page (the
  axis numbers and titles look much larger than in the small window). The menu bar belongs to
  the maximised window's type (the graph menus appear when the maximised window is a graph).
  Maximising is a property of the window, not of the workspace: activating another window
  by name shows that one maximised too (the second window I activated was also maximised).
- **Minimised window** [seen 2, `min-three.png`]: a window shrinks to a small bar of about 160 x
  24 px that sits at the bottom-left of the workspace, just above the 2D Graphs toolbar (y 918
  to 942). It shows the window-type icon, the name cut to three letters and a dot ("Gra...",
  "Bo..."), then three buttons: restore (two boxes), maximise (one box) and close (X). Several
  minimised windows line up from left to right in the order they were minimised, each about 160
  px wide with a gap of about 3 px. One of them (the one Origin passes focus to) has the red active
  close button. Because Book1 was minimised last, it stands at the right end.
- **Window menu** [seen 2, `wb-window.png`, `max-window-menu.png`, `min-menu.png`] has the same
  items for all window types except one: Cascade, Tile Horizontally, Tile Vertically, Arrange
  Icons (all four with an underlined letter), separator, Refresh (F5), Duplicate, then **Split**
  when the active window is a workbook but **Duplicate (Batch Plotting)** (a submenu) when it is
  a graph, separator, Properties... (Alt+Enter), separator, Command Window (Alt+3), Script
  Window (Shift+Alt+3), separator, Folders (submenu), separator, then a numbered list of all
  windows ("1 Book1 *", "2 Graph1 *", "3 Graph2 *"), the active one with a tick. The hint line at the
  bottom of the main window says what an item does when the mouse is over it (for Arrange Icons
  it names the minimised windows) [seen 2].
- Dragging the title bar moves the window; dragging the edge or the bottom-right grip resizes it;
  windows can be dragged partly outside the workspace [standard MDI, not individually checked].
- Window types in the Project Explorer list: Workbook (grid icon), Graph, Matrix, Layout, Notes,
  Function, Excel [spec F].
- Closing a child window does not close the project; closing a workbook that graphs depend on
  asks first, and "Hide" is offered instead of delete [spec F, not seen].

## 3. Workbook window

Pictures: `gr-plain.png` area, `wb-*.png`, `rc-*.png`, `wb-m04-column.png`; second study
`colhdr.png`, `colhdr-more.png`, `cell.png`, `cell-more.png`, `rowhdr.png`, `sheettab.png`,
`wb-file-more.png`, `wb-edit-more.png`, `wb-view-more.png`, `wb-help-more.png`, `max-book.png`.

- The window top is a grid with a **column header band** and the **column label rows** [seen]:
  - First row (grey, centred): the column label "A(X1)", "B(Y1)", "C(Y1)", "D(Y1)". The letter is
    the column name, the type in brackets is X, Y (also Z, yEr, L for label, M for disregard)
    followed by the group number of the data set (X1, Y1) [seen]. A column named "Label" that my
    setup made with `wks.col5.type = 4` shows as "Label(X2)" [seen 2, `max-book.png`], so a
    column designated as text label ("E(L)" style) was still [not seen].
  - Then four label rows in the order **Long Name, Units, Comments, F(x)=**, in light yellow cells,
    left of which a grey cell shows the row title right-aligned ("Long Name", "Units",
    "Comments", "F(x)=") [seen]. Origin also has **Sparklines**, **Parameters** and user
    defined label rows, switched on in View > Show / the right-click menu "Row Labels for
    Column Label Rows" [spec; the menu `gui-study/fresh/colhdr.png` of the first study shows exactly this list of
    options, see below]. Sparklines were not turned on in the pictures [not seen]. The column
    header menu has an item "Add or Update Sparklines..." [seen 2].
  - Then the data rows with a grey **row header** (numbers 1, 2, 3, right-aligned, about 85 px wide
    together with the label titles). Numbers right-aligned, text left-aligned (the text cells
    "alpha", "beta", "gamma" start at the left edge of their cell) [seen, seen 2]. Empty
    cells below the last value stay blank with the row header numbers continuing (11 is shown
    for the first empty row) [seen].
- Column width about 67 px (default, 8 characters), row height 16 px, header rows 16 px [seen].
  Cells have thin grey gridlines in both directions (View options "Column Grid", "Row Grid" tick
  boxes in the menu) [seen].
- Number formats: up to about 5 significant decimal digits are shown by default (0.84147, 0.9093,
  -0.7568; whole numbers have no decimals; 0.1 shows as 0.1) [seen]. The separators and digits
  come from Tools > Options > Numeric Format (section 5).
- **Sheet tabs** at the bottom-left, inside the window: scroll arrows, then tab "Sheet1" with a
  slanted right edge, then a horizontal scroll bar next to it [seen]. Vertical scroll bar at the
  right edge; bottom-right a small resize grip [seen]. Whether a "+" button to add sheets exists
  is [not seen].
- Selection looks [seen, seen 2]: a selected column is a **black block** over its cells with
  white text and a dark header cell, a clicked cell has a black frame, a selected row has its
  row-header cell light blue-grey (the cells themselves are not filled) [seen 2, `rowhdr.png`].
- **Status bar while selecting** [seen 2]: for a selected column "Average=12 Sum=120 Count=10", for
  one cell "Average=3 Sum=3 Count=1", for a row "Average=2.76028 Sum=11.04112 Count=5" (the four numbers of that
  row; the text cell in the fifth column is counted too); then "AU : ON" and, as the active-object box, `1: [Book1]Sheet1!2[1:10]`
  for column 2 rows 1 to 10 and `1: [Book1]Sheet1!1[3:3]` for one cell. The hint box on the left
  shows "For Help, press F1" or the text of the item under the mouse.
- **Right-click, column header** [seen 2, `colhdr.png`, `colhdr-more.png`]. The column must be
  selected first (click the letter); right-clicking an unselected header is not what was
  photographed. The menu opens in the short form (Plot, Copy, Copy Columns to..., Delete, Set
  As, Set As Categorical, Set Column Values..., Mask, Sort Worksheet, Hide/Unhide Columns,
  Format Cells..., Properties..., and the chevron). Clicking the chevron gives the long form,
  about 600 px high:
  1. **Plot** (submenu)
  2. Cut (Ctrl+X), **Copy** (submenu), Copy Columns to..., Paste (Ctrl+V)
  3. Insert, Delete, Clear (Delete key), Remove Links
  4. **Set As** (submenu), Set As Categorical
  5. Set Column Values... (Ctrl+Q), Set Multiple Columns Values... (Ctrl+Shift+Q)
  6. **Fill Column with** (submenu: Row Numbers, Uniform Random Numbers, Normal Random Numbers,
     A set of Numbers..., A set of Date/Time values..., Arbitrary set of Text&Numeric
     values...), Set Sampling Interval..., Show X Column...
  7. Filter (submenu), Mask (submenu), Mask Cells by Condition...
  8. Sort Column (submenu), Sort Worksheet (submenu), Sort Columns by Label..., Reverse Order
  9. Normalize..., Frequency Count..., Statistics on Column...
  10. Hide/Unhide Columns (submenu), Move Columns (submenu), Swap Columns..., Column Width...
  11. Slide Show of Dependent Graphs, Add or Update Sparklines...
  12. Format Cells..., Properties...
  Groups are separated by thin lines; items with a small icon in the left column include
  Cut, Copy, Paste, Set Column Values and Statistics on Column.
- **Right-click, cell** [seen 2, `cell.png`, `cell-more.png`]. Short form: Cut (Ctrl+X), Copy
  (Ctrl+C), Paste (Ctrl+V), Insert, Delete, Clear (Delete key), Insert Graph..., Format Cells...
  and the chevron. Long form: Cut, Copy, Copy (full precision) (Ctrl+Alt+C), Paste, Paste
  Transpose, Paste Link (Ctrl+Alt+V, grey), Paste Link Transpose (grey), then Insert, Delete,
  Clear, Remove Links, then Set as Begin, Set as End, then Insert Arrow (submenu), Insert
  Graph..., Insert Images from Files..., Insert Notes, Insert Sparklines..., Insert
  Variables..., then Format Cells..., Mask (submenu), Set Data Style (submenu).
- **Right-click, row number** [seen 2, `rowhdr.png`]: the short form is the same eight items as the
  cell menu (Cut, Copy, Paste, Insert, Delete, Clear, Insert Graph..., Format Cells...) and
  the chevron; the long form of the row menu was [not seen].
- **Right-click, sheet tab** [seen 2, `sheettab.png`]: Insert, Delete, Add, Add Graph as Sheet...,
  Add Notes as Sheet, separator, Duplicate Without Data, Duplicate, separator, Sheet Name to Show
  (submenu), Name and Comments..., separator, Navigate.... No chevron.
- **Right-click, corner/label area** [seen, `fresh/colhdr.png`]: a small menu with tick items "Row Labels
  for Column Label Rows" (sub-list), "Column Header", "Row Header", "Column Grid", "Row Grid",
  "Page Break Preview Lines".
- **Mouse**: click on a column letter selects the column; Shift or Ctrl click extends; click on a
  row number selects the row; click on the corner selects all; double-click the column header
  opens Column Properties ("Properties", "Plot Setup" and so on) [spec D5]; double-click a cell edits it,
  Enter commits; double-click a label cell (Long Name, Units, Comments) edits that label in place [spec];
  dragging the header border resizes the column [spec]. Drag and drop of columns onto a graph
  adds them as a plot: a yellow hint balloon appears when a column is selected and says that the
  mouse pointer changes at the right edge of the selected column and that dragging from there onto a
  graph layer adds the data [seen, `m4.png`, `fit1.png`].
- **Ctrl+U in a workbook**: the Tools menu shows Ctrl+U beside Options [seen 2, `toolsmenu.png`],
  but pressing Ctrl+U with a column selected in a workbook did not open Options; it switched
  on the Underline button of the format toolbar and underlined the cell text [seen 2,
  `opts.png`]. Sheaf should not bind Ctrl+U to Options inside a worksheet.
- **Set Column Values dialog** [seen, `m4.png`; seen 2, `setvals.png`, opened by Ctrl+Q with the
  column selected]: title "Set Values - [Book1]Sheet1!Col("Signal")", a text menu line "Formula,
  wcol(1), Col(A), Function, Variables, Options", a row "Row (i): From <auto> To <auto>", a
  label "Col(B)=" with a few small round buttons (find, first, previous, next, last formula) and
  a multi-line formula box with grey hint text that says to enter a one-line expression or load
  samples from the Formula menu, then "Recalculate" with a drop-down ("Manual"), an icon button,
  OK, Cancel, Apply (grey until something changes), a "collapse" arrow button, and below a
  second box "Before Formula Scripts" with its own grey hint. The dialog is small (about 425 x
  335 px), has minimise, a disabled maximise and a close X (no "?"); the formula box has the
  text cursor on opening [seen 2].
- **Sheaf Set Multiple Columns Values (T53, 2026-11-15).** Column > Set Multiple Columns Values...
  (Ctrl+Shift+Q; the Column menu and the column right-click menu) is enabled when two or more
  columns of the sheet are selected and greyed with the hint "Select two or more columns first"
  otherwise; Set Column Values (Ctrl+Q) stays one-column only. It opens the same Set Values
  window with these differences, all Sheaf choices and not measured in Origin: the title reads
  "Set Multiple Columns Values - [Book1]Sheet1!Col(B:D)" (`B:D` for three or more side-by-side
  columns, otherwise the short names with commas, such as `Col(B, D)`); the `Col(B:D)=` label
  has no first/previous/next/last buttons (the window edits the selection together); the
  Formula menu starts with "Scale by column position" (`col(A)*j`) and "Double the column to the
  left" (`wcol(j-1)*2`); the `wcol(1)` menu starts with `wcol(j-1)` and `wcol(j+1)`; the Variables
  menu lists `j` (the column position in the sheet, counted from 1). The row range and Recalculate
  apply to every column and OK or Apply is one undo step, "Set multiple columns values". The
  fields start from what the columns share (formula, rows, recalculation); columns that differ
  start empty and Manual. An error (an unknown column, a loop) keeps the window open with the
  message and changes nothing. `--show set-values-multiple[:B,C,D]` opens it for a screenshot.
  Windows measurements still to make with `tools/origin-ref`: the window title, whether the
  navigation buttons are hidden or step through the selection, and the label text beside the
  row range (see `docs/spec/expression-language.md` section 7 for the `j` measurements).
- **Sheaf Set Values function search (T09, 2026-11-14).** Find beside the column navigation,
  or Function > Search Functions, opens Search and Insert Functions. Keyword matches cover
  the names, categories, arguments and descriptions in the existing Function-menu catalog;
  multiple words must all match, without case sensitivity. A selected entry shows Sheaf's
  own description and example. Add Function, a double click, or Enter from the search field
  inserts through the same formula-cursor path as the Function menu. Cancel, Escape and the
  search window's close button keep the staged formula and worksheet data. Choosing a
  function stages an edit; only Set Values Apply/OK changes the worksheet, with undo.
  The parent dialog is disabled while search is open. An empty result list disables insertion.
  This follows Origin's published [Search and Insert Functions behavior](https://docs.originlab.com/origin-help/wks-setcolval-menuopt-dialog/).
  The installed-2016 capture attempts (local `scratch/t09-function-search`: Ctrl+Q found no
  dialog, and `csetvalue -d` returned false) did not produce a usable control capture. The
  search-window layout and the multi-keyword matching rule are Sheaf choices; no new installed
  Origin layout/default measurement is claimed. Formula-search UI tests live in
  `dialogs/set_values.rs`, separately from other sessions' app UI regressions.
- **Workbook File menu** [seen 2, `wb-file.png`, `wb-file-more.png`]. Short form: Open... (Ctrl+O),
  Open Sample Projects (submenu), Save Project As..., Save Template As..., Save Workbook As Analysis
  Template..., Add Word Bookmarks to Analysis Template..., Import (submenu), Recent Books,
  Recent Projects, Exit, chevron. Notice that New and Save Project are not in the short form.
  Long form: **New** (submenu), Open... (Ctrl+O), Open Excel... (Ctrl+E), Open Sample Projects, Append...,
  Close, | Save Project (Ctrl+S), Save Project As..., | Save Window As..., Save Template As..., |
  Save Workbook As Analysis Template..., Add Word Bookmarks to Analysis Template..., Batch
  Processing..., | Save Project without Data..., Duplicate Project without Data, | Print...
  (Ctrl+P), Print Preview, Page Setup..., | Import (submenu), Recent Imports (submenu),
  Re-Import Directly (Ctrl+4), Re-Import..., Database Import (submenu), | Export (submenu),
  Recent Exports (submenu), | Recent Books, Recent Projects (both submenus), | Exit.
  The Import submenu in short form: Single ASCII ..., Multiple ASCII..., Excel (XLS, XLSX,
  XLSM)...; long form: Import Wizard... (Ctrl+3), Single ASCII ..., Multiple ASCII..., Comma
  Delimited (CSV)..., Excel (XLS, XLSX, XLSM)..., Thermo (SPC, CGM)..., pCLAMP (ABF, DAT, AB?)...,
  Princeton Instruments (SPE)..., Sound (WAV)..., NetCDF (NC)..., CDF (CDF)..., HDF5 (H5, HE5,
  HDF5)..., NI DIAdem (DAT)..., NI TDM (TDM, TDMS)..., Famos (DAT, RAW)..., ETAS INCA MDF (DAT,
  MDF)..., IgorPro (PXP, IBW)..., SigmaPlot (JNB)..., Prism (PZFX, XML)..., Matlab (Mat)...,
  Origin Project (OPJ, ORG, OGW)..., Add/Remove File Types....
- **Workbook Edit menu** [seen 2, `wb-edit.png`, `wb-edit-more.png`]. Short form: Undo ("Undo Create"
  here, the item names the last action; Ctrl+Z), Copy (submenu), Select..., Delete, Find in
  Project... (F3), Go to... (Ctrl+G), Merge (Embedded) Graphs..., chevron. Long form: Undo, | Cut
  (Ctrl+X, grey without a selection), Copy (submenu), Paste (Ctrl+V), Paste Transpose, Paste
  Link (Ctrl+Alt+V), Paste Link Transpose, Paste Special, | Select..., | Clear (Delete),
  Insert, Delete, Remove Links, Set As Begin, Set As End, Reset to Full Range, | Find in
  Project... (F3), Find in Sheets... (Ctrl+F), Replace... (Ctrl+H), Go to... (Ctrl+G), Merge
  (Embedded) Graphs..., | Button Edit Mode (Ctrl+Alt+B). Items that do not apply are grey.
- **Workbook View menu** [seen 2, `wb-view.png`, `wb-view-more.png`]. Short form: Toolbars... (Ctrl+T),
  Apps (tick, Alt+9), Project Explorer (Alt+1), chevron. Long form: Menu (submenu), |
  Toolbars... (Ctrl+T), Status Bar (tick), | Command Window (Alt+3), Code Builder (Alt+4),
  Quick Help (Alt+5), | Apps (tick, Alt+9), Project Explorer (Alt+1), View Windows
  (submenu), | Results Log (Alt+2), View Mode (submenu), | Messages Log (Alt+6), Smart Hint
  Log (Alt+7), | Actively Update Plots (tick), | Page Break Preview Lines. Object Manager (Alt+8)
  was not in this list; it may sit under View Windows [not seen].
- **Workbook Window menu**: see section 2.
- **Workbook Help menu** [seen 2, `wb-help.png`, `wb-help-more.png`]. Short form: Use Online Help
  (tick), Origin (submenu), Programming (submenu), Open Sample Projects (submenu), Online
  Resources (submenu), Check for Updates..., Deactivate License... (grey), About Origin..., chevron.
  Long form: Use Online Help (tick), | Origin (submenu), X-Functions, LabVIEW VI, Programming
  (submenu), | Open Sample Projects (submenu), | Contact Support, Online Resources (submenu),
  View Origin Info, | Check for Updates..., Register on-line..., Deactivate License... (grey), |
  Change Language... (grey), | Reactivate All Hints, Reactivate Reminder Messages, |
  OriginLab on Facebook..., | About Origin....
- **Collapsible menus**: when a long menu is expanded for the first time a yellow hint note at
  the bottom-left of the screen says that collapsible menus can be switched off in Tools >
  Options on the Miscellaneous tab by clearing the box "Enable Collapsible ..." [seen 2,
  `wb-file-more.png`]. Menus that showed no chevron in these pictures: the Window menu and the
  right-click menu of the sheet tab.

### Frozen panes (T34)

Origin facts (from the help pages, not measured; no Origin was available) [W:origin-help/freeze-rows-columns,
W:labtalk/ref/wks-obj, W:originlab.com/2023]:

- Origin 2023 added View > Freeze/Unfreeze Panes (a toggle, Ctrl+Shift+F, also a Mini Toolbar button
  "Freeze Panes"). It keeps the first n rows and/or columns in view while the rest scrolls.
- Select a row, a column or one cell whose index is between 1 and 10 and choose it. To unfreeze, click
  the upper-left cell and choose Unfreeze Panes, or use the View toggle again.
- Frozen row and column headers are shaded darker.
- LabTalk `wks.frozen$="C 3"` freezes columns A to C and rows 1 to 3, the same as selecting cell D4 and
  clicking Freeze Panes; `wks.frozen$=""` unfreezes. So the frozen part is everything above and left of
  the selected cell; a whole row freezes rows only and a whole column columns only (inferred).

Sheaf's choices, all **unmeasured** (to check on Windows against Origin):

- **Counting.** Rows count in displayed rows (with a filter, the first n rows that pass) and columns in
  visible columns (hidden ones are skipped), both in sheet order. The label rows are not counted: they
  always stay on top, above the frozen rows. Spare entry rows are never frozen. A filter or hide change
  therefore changes which data is frozen.
- **Index.** n = index - 1, as in the LabTalk example (D4 freezes 3 and 3). An index above 10 (n above 9)
  is refused with a status message, as is A1 (nothing above or left) and a selection of a label cell, the
  whole sheet or nothing.
- **Toggle.** The one command freezes from the selection when nothing is frozen and unfreezes whatever is
  frozen otherwise, whatever the selection. It is one undo step, "Freeze panes" or "Unfreeze panes". The
  shortcut does not work while a cell or label is being edited.
- **State.** Per sheet, saved in `.sheaf` (`frozen`: rows, columns; absent when nothing is frozen). The
  `.opj` writer cannot store it: saving to `.opj` warns and the companion keeps it.
- **Clamping.** The setting is never changed by the window size; when drawing, the frozen rows leave at
  least three body rows and the frozen block (row header and columns) takes at most half of the width.
  What does not fit is shown as scrolling cells.
- **Look.** The heads of frozen columns and the row numbers of frozen rows are a darker grey; a 2 px dark
  line separates the frozen block from the scrolling cells (right of the frozen columns, under the frozen
  rows). With frozen columns the row numbers stay with them; with frozen rows only they scroll sideways.
- **Scrolling.** The wheel over the frozen columns scrolls the body; over the head and frozen rows it
  scrolls the body up and down. Dragging a selection out of the body into the frozen block scrolls back
  toward the start; a drag that began in a frozen column (row) never scrolls that axis.
- **Navigation.** Keys, Go To and typing never scroll to a frozen target (it is always in view); a
  target in the scrolling part is revealed below and right of the frozen block.
- **Width.** A frozen column keeps the width it had; dragging the right edge of its head (a 6 px strip)
  changes it, and the width survives unfreezing and freezing again. The drag is clamped so that the
  frozen block never goes over the half-width cap above: the column stops growing there instead of
  dropping out of the block.

### Conditional formatting (T35)

Origin facts (from the help pages and X-Function reference, not measured; no Origin was available)
[W:originlab.com/2025b, W:origin-help/conditional-format, W:x-function/ref/wcolorbycr]:

- Origin 2025b: "conditional formatting to highlight min/max column-wise or row-wise".
- Worksheet > Conditional Formatting (also on the right-click menu of selected cells) holds Highlight
  Min Max, Highlight Top Bottom, Conditional Highlight, Conditional Heatmap, Conditional Pareto,
  Conditional Outliers, Remove Conditional Format and Conditional Format Manager.
- A rule's Name defaults to the first and last cell of its range, `C<c1>R<r1>-C<c2>R<r2>`, R0 meaning
  "to the end" (example `C1R1-C4R0`). Where rules overlap, the upper rule in the Manager wins.
- Highlight Min Max is `wcolorbycr rng name dir max.fill max.font min.fill min.font`; `dir` 0 is
  row-wise (the default), 1 column-wise, each row or column on its own. Defaults (0x01BBGGRR ints):
  max fill 30067455 = RGB(255,202,202), max font 20132812 = RGB(204,51,51), min fill 33085116 =
  RGB(188,214,248), min font 26692113 = RGB(17,74,151).
- Highlight Top Bottom is `wcolortb`: Top N Items, Top N%, Bottom N Items, Bottom N%, on the selected
  columns. Its default N and colours are not documented.

Sheaf's choices, all **unmeasured** (to check on Windows against Origin):

- **Rules.** Per sheet (`cond_formats`, saved in `.sheaf`, absent when empty), a list: name, range
  (columns by id plus first row and an optional last row, none meaning to the end of the data), rule
  and colours. A selection of whole columns, rows or the sheet, or a block of cells, gives the range;
  a block that reaches the last data row follows the data from then on. A label cell or nothing
  selected gives a status message and no window (the menu items are greyed without a selection).
- **Evaluation.** Live: the grid shows the rules on the current values, with no stored colours. Only
  numbers count; text, empty and NaN cells are ignored, and so are masked cells (they take no part in
  the ranking and keep the red mask look). Ties: every cell equal to the maximum (minimum, or to the
  N-th value for Top Bottom) is highlighted. A row or column with one number counts as its maximum.
  Max takes precedence over min where they coincide (all values equal).
- **Min Max.** Window with Name, Row-wise / Column-wise radio buttons and a Fill and a Font colour (each
  switchable off) for the maximum and the minimum, Origin's defaults above, row-wise.
- **Top Bottom.** Window with Name, Mode (Top N Items, Top N%, Bottom N Items, Bottom N%), N (default
  10, so "Top 10 Items") and one Fill/Font pair, defaulting to the Min Max maximum colours for Top and
  the minimum colours for Bottom (Sheaf's own). Each column of the selection is ranked on its own. N %
  is taken of the numbers in the column and rounds up (at least one item).
- **Order.** A new rule goes to the top of the list; the earlier rule wins the whole cell, so a cell
  coloured by a rule is not touched by a later one. Filtered (hidden) rows are ranked like the others.
- **Remove.** Remove Conditional Format removes every rule whose range touches the selection, whole
  rules and not just the selected part; with none it says so in the status bar. Each command is one
  undo step ("Highlight min max", "Highlight top bottom", "Remove conditional format").
- **Copies and edits.** Window > Duplicate, Worksheet > Duplicate sheet and File > Append keep the rules,
  pointing at the copy's own columns. Deleting columns takes them out of every rule and drops a rule
  left with no column (in the same undo step as the delete). Inserting rows at or above a rule's first
  row moves it down, inside it makes it longer; deleting rows moves or shortens it, and a rule whose
  whole row span is deleted is dropped; a rule that runs to the end stays open. Only operations on a
  column of the rule count.
- **Cost.** The styles are computed once per change of the document (not per frame or per cell) and
  read per drawn cell; a 1,000,000-row column takes about 3 ms (Min Max) to 10 ms (Top N %) in a
  release build.
- Renaming and editing rules: the Manager (T122). Conditional Highlight: T123. The `.opj` writer cannot store the rules: saving to `.opj` warns and the
  companion keeps them.

#### Conditional Heatmap (T120)

Origin facts (docs.originlab.com/origin-help/conditional-format, read 2026-11-17, not measured: Origin
2016 has no Conditional Formatting). Level: **Auto Rescale** (checked by default, the range's minimum
and maximum), else **From** and **To**, **Find Min/Max**, scale **Type**, levels **By Increment** or
**By Count**, **First Level** ("larger than or equal to the default value of the first level").
Colors: **Format Type** Fill Background or Color Text, **Contrast Color for Text**, **Color
Generation** (Limited Mixing, 3-Color Limited Mixing with **Middle Position** By Percentile, By
Percent or By Value, Introducing Other Colors in Mixing, Load Palette), **Flip**, **Below** and
**Above** colours for values outside From..To. A new rule goes on top; the upper rule wins.

Sheaf's choices, all **unmeasured**:

- **Scope.** One set of levels over the whole range (all its columns and rows together); the same
  numbers count as for the other rules (text, empty, NaN and masked cells are left out). Each level
  is a palette class of the rule, at most 250 levels.
- **Levels.** In scale units (log10 of the value for Log10, where numbers at or below zero get no
  colour). By Count n cuts First Level..To into n equal levels; By Increment d gives
  ceil((To - First Level) / d) levels (one decade is 1 on Log10). A level holds its lower bound;
  To belongs to the last level. First Level defaults to From and is clamped into From..To; numbers
  between From and First Level take the first level. Auto Rescale with one distinct number gives
  that number the first level; set limits that are equal, reversed or not finite colour nothing.
- **Colours.** Level k of n gets t = k / (n - 1) (0 for one level), mixed in RGB from From to To;
  3-colour mixing goes From to Middle up to the middle's place and Middle to To after it. The
  middle's place along the levels is the percent itself (By Percent), the value's place (By Value),
  or the place of that percentile of the range's numbers (By Percentile, linear between order
  statistics), clamped to 0..1. Flip swaps From and To (Middle keeps its place). Below and Above
  apply only with Auto Rescale off; switched off, numbers outside From..To stay plain.
- **Text.** Fill Background with Contrast Color for Text puts black or white text on each fill,
  whichever has more contrast (BT.601 luma below 128 gives white); Color Text colours the text and
  leaves the fill.
- **Defaults** (Origin's are not documented): Auto Rescale, Linear, By Count 10, Fill Background
  with contrast text, Limited Mixing from white RGB(255,255,255) to RGB(248,105,107); Middle
  RGB(255,235,132) at 50 %. Introducing Other Colors in Mixing and Load Palette are shown disabled.
- **Files.** `.sheaf` and the `.opj.sheaf` companion keep the rule; `.opj` warns as for the other
  rules; XLSX writes the current colours as a snapshot, with the same warning.
- One undo step, "Conditional heatmap"; Remove Conditional Format removes it like the other rules.


#### Conditional Pareto and Conditional Outliers (T121)

Origin facts (docs.originlab.com/origin-help/conditional-format, read 2026-11-17, not measured):
Conditional Pareto "counts all items in the range" and ranks them from most to least frequent, with
Most Frequent N Items, Most Frequent N% (items whose cumulative frequency is <= N %), Least Frequent N
Items and Least Frequent N% (cumulative frequency >= 1 - N %). Conditional Outliers colours outliers
"as in a box chart": **IQR** with **Coef** (default 1.5; fences Q1 - Coef x IQR and Q3 + Coef x IQR,
3 for the outer fences) or **SD** (the box chart's Whisker Range = SD) with Coef.

Sheaf's choices, all **unmeasured**:

- **Pareto items.** Every non-empty, unmasked cell of the whole range is an item: a number (equal
  numbers, 0 and -0 included, are one item) or a text (text cells of Text & Numeric and text
  columns count). Items rank by count, equal counts in order of first appearance (column by column,
  top to bottom). N Items takes floor(N) items from the top (or bottom) and every item with the same
  count as the last one taken; N% follows Origin's wording with the cumulative frequency including
  the item, so Least Frequent N% always takes the least frequent item for N > 0. N of 0 or less takes
  nothing. Default Most Frequent 10 Items; Least modes switch to the Min Max minimum colours.
- **Outliers.** Each column of the range on its own (one box per column). IQR quartiles follow the
  box chart's Origin rule (`quantile_origin`, graphing box chart); SD is the sample standard deviation
  (n - 1) around the mean, and needs two numbers. Values strictly outside the fences are coloured.
  The Coef box offers 1, 1.5, 2 and 3 and takes any value >= 0; the default 1.5 applies to SD too.
  Colours default to the Min Max maximum colours.
- One undo step each ("Conditional pareto", "Conditional outliers"); saving, Remove and XLSX snapshot
  as for the other rules.


#### Conditional Format Manager (T122)

Origin facts (docs.originlab.com/origin-help/conditional-format, read 2026-11-17, not measured): the
Manager lists all conditional formats of the active worksheet; **Name** is renamed by a
double-click, **Range** is reselected or typed, **Edit Rule** reopens the rule's dialog, **Move
Up**/**Move Down** (or dragging a row) reorder and **Remove** deletes; a new format is added at the
top and the upper format wins where ranges overlap.

Sheaf's choices, all **unmeasured**:

- The window lists Name, Range (the columns' short names and rows, for example `B, C; rows 1 to
  end`) and Rule (a few words: `Top 10 Items`, `Highlight Min Max, column-wise`, `Heatmap, 10
  levels`, `Most Frequent 10 Items`, `Outliers, IQR x 1.5`). It reads the sheet again every frame,
  so undo shows at once; it opens without a cell selection.
- Every change applies at once as one undo step ("Rename conditional format", "Move conditional
  format up/down", "Remove conditional format", "Edit conditional format"); there is no OK/Cancel,
  only Close. A rename to an empty or unchanged name, or Escape, changes nothing.
- Edit Rule opens the rule's own window (Highlight Min Max, Top Bottom, Heatmap, Pareto, Outliers)
  filled with its settings and name; OK replaces the rule in its place, keeping its range. OK with
  nothing changed adds no undo step; if the list changed meanwhile (the rule at that place is no
  longer the same), OK does nothing.
- **Left out:** changing a range (reselecting or typing), dragging rows to reorder.


#### Conditional Highlight (T123)

Origin facts (X-Function `wcolor` and "Details of TreeNodes in Wcolor", docs.originlab.com, read
2026-11-17, not measured): conditions `cond1`/`cond2` 0 equal to, 1 not equal to, 2 greater than,
3 greater than or equal to, 4 less than, 5 less than or equal to, 6 between (`val11`, default 0, and
`val12`, default 1), 7 text that contains (`text1`), 8 custom (`custom1`, a condition in `x`, for
example `x == "Male"`); Date/Time columns name 2 to 5 after, on or after, before, on or before.
`join` 0 None (default), 1 And, 2 Or. Background 30067455 = RGB(255,202,202) and Text Color
20132812 = RGB(204,51,51) by default. `extend` colours whole rows; values may come from a fly-out
of statistics functions.

Sheaf's choices, all **unmeasured**:

- The window: Condition (Origin's nine names, a value, two values for Between, a text box for Text
  that contains and Custom), Join (None, And, Or) with a second condition, and the Fill/Font pair
  (Origin's defaults). Defaults: Equal to 0, None.
- Numeric conditions match numbers only: text, empty, NaN and masked cells never match, not even
  "not equal to". Equality is exact. Between includes both values, in either order.
- Text that contains matches the cell's text as the worksheet shows it (numbers too, for example
  `5` in `2.5`), case-sensitive; an empty text matches nothing.
- Custom is an expression of Sheaf's expression language in `x`, the cell's number, true when not
  zero (`x > 2 && mod(x, 2) == 0`); it may use functions and constants but no other names and no
  columns, and OK refuses one that does not parse or uses another name, saying why. Text values in
  custom conditions (`x == "Male"`) are not supported: text cells never match Custom.
- **Left out:** Date/Time names and date entry (dates compare as their stored numbers), statistics
  functions as values, extending the colour to the whole row (`extend`).

### Cell notes (T36)

Origin facts (help pages, not measured; no Origin was available) [W:originlab.com/2022,
W:origin-help/insnote-to-wkscell, W:x-function/ref/insertcmt]:

- Origin 2022: "Add Notes to individual Worksheet Cells", similar to Excel and Google Sheets. Select the
  cell and click the Mini Toolbar Add Note button; the note displays as a popup when hovering on the
  tab in the upper-right corner of the cell; the system variable `@NCP = 0` stops the popups. Notes
  are edited in a Notes window ("Open in Notes Window"); contents are Text or Origin Rich Text. The
  X-Function `insertCmt` adds a note to a selected cell (search snippet, unconfirmed).
- Not documented: how to delete a note, the tab's colour and size, behaviour under sort/insert/delete,
  export.

Sheaf's choices, all **unmeasured** (to check on Windows against Origin):

- **Storage.** Per sheet (`Worksheet.notes`, core `notes.rs`, saved in `.sheaf`, absent when empty): a
  list sorted by column id and data row, so the saved form is deterministic and a lookup is a binary
  search (no per-cell scan). Plain text only; empty text is never stored. A note may sit on an empty
  cell (beyond the data rows) and then follows its row position like any other.
- **Indicator.** A small filled dark-red triangle (7 px) in the upper-right corner of the cell, also in
  frozen (overlay) cells and over conditional-format fills. Origin's colour and size are unmeasured.
- **Popup.** Hovering anywhere on the noted cell (not only the tab) shows the text in a tooltip, wrapped
  at 300 px. There is no `@NCP` switch. A tooltip cannot scroll, so a long note is cut at 20 lines or
  1,500 characters and ends with "…" (the full text stays in the editor).
- **Editing.** The cell right-click menu replaces the greyed "Insert Notes" item: "Add Note..." on a cell
  without a note, "Edit Note..." and "Delete Note" on one with a note, for a single selected cell
  (otherwise a status message). Inside a multi-cell selection the note entries act on the right-clicked
  cell (choosing one reselects that cell; the other cell-menu entries still act on the selection). The
  editor is a small window with a multi-line text box (scrolls above 300 px, so OK/Cancel stay on screen) and OK/Cancel
  (no Notes window, no Mini Toolbar button, no rich text). Each change is one undo step ("Add note",
  "Edit note", "Delete note"; emptying the text of an existing note deletes it). Clear (Delete key)
  clears values and leaves the note.
- **Sort.** Notes move with their row: Sort Worksheet, Sort Range, nested/custom sort (also of selected
  rows) and Filter Rows (in place, dropping the notes of removed rows) permute the notes of the sorted
  columns exactly as the values; Sort Columns (each column on its own) and Reverse Order move only that
  column's notes.
- **Rows and columns.** Insert rows shifts the notes of the columns inserted into at or below the row;
  delete rows drops the notes on the deleted cells and shifts those below. Columns have stable ids, so
  Move Columns and column insert need nothing; deleting a column drops its notes (same undo step).
  Duplicate (window or sheet) and File > Append keep the notes, on the copy's own columns.
- **Not carried.** Operations that build a new sheet (Transpose, Stack/Unstack, Split Columns, Reduce
  Rows, Filter Rows to a new sheet) do not copy notes; neither do the `.opj` writer (save to `.opj`
  warns "N cell note(s) are not written"; the `.opj.sheaf` companion keeps them), CSV and Excel export.
- **Left out.** Notes window, rich text, `@NCP`, Mini Toolbar button, hovering only on the tab, notes
  on labels/headers.

### Cell formatting (T59)

Origin 2016 has Format Cells on the cell, column-header and row-number context menus,
Format > Cell... and a separate Format > Worksheet... (F4). Ctrl+U underlines a selected column;
the cell-format X-Function is `wcellformat`. The dialog/menu measurements below were taken on
Windows, 2026-11-16, using only synthetic worksheet values. Effects of the worksheet-wide
settings, manual versus conditional precedence, sort behaviour and `.opj` storage remain open.

Origin 2016 worksheet-toolbar measurements (Windows, 2026-11-15, synthetic two-column
worksheet): selecting A1 shows Default: Arial and 9 pt. Bold and Italic change only A1; choosing
Times New Roman changes that cell's glyph family. Opening the font-colour palette and choosing
its red swatch in the same UI interaction paints only A1 red. A range containing 20 pt A1 and
9 pt B1 has a blank Size field. The mixed Bold button follows A1, and clicking it clears Bold
for both selected cells. The font-size step buttons remain disabled. Increasing a cell or label
row to 20 pt grows that row; reducing A1 to 9 pt leaves Origin's enlarged row height intact.
Raw captures remain local in `re-workspace/scratch/t59-format-toolbar/`; palette captures that
foregrounded the main window between opening and picking were excluded because that closes the
popup. The toolbar and worksheet area used for measurements were unobscured.

Sheaf's shared **Format toolbar** targets selected cells, whole rows/columns, or the selected
label cell. Font, Size, Bold, Italic, Underline, the basic eight text colours and eight fill colours plus None apply in one "Format cells"
undo step, leaving values and number formats untouched. Mixed family/size fields are blank;
Bold/Italic/Underline use the first displayed selected cell's state (mixed Underline is Sheaf's
existing toolbar policy; its mixed state was not separately measured). Selecting another cell, sheet or
workbook, editing a cell, or changing the document makes an older toolbar request inert.
Whole columns include future rows and keep compact format ranges. Size accepts 4..96 pt (0
restores the default); these bounds remain Sheaf's existing policy, not a measured Origin limit.
Worksheet size-step controls stay disabled as observed; graph controls keep their own behaviour.

Further Origin 2016 measurements (Windows, 2026-11-16, synthetic cells): the U button and
Ctrl+B/I/U toggle Bold/Italic/Underline on the selected cell. Fill Color red changes only A1's
background; None clears it. Ctrl+U on a Long Name label toggles that label separately. Ctrl+U
on a whole column underlines its data cells while its Long Name label stays
unchanged. Sheaf implements these target distinctions through the existing format ranges.
Keyboard formatting belongs to worksheet selections when the grid has the keyboard; text
editors, dialogs, popups and Project Explorer retain their keys. Graph Ctrl+I stays Zoom In.
Each key press is one Undo step; multiple presses in a frame retain their order. Fill palettes
capture their opening selection/revision and cannot follow a changed target. Manual fills keep
the existing conditional-colour precedence (still unmeasured). The palette is the basic eight
colours plus None, with the custom RGB choice described below; native custom/system/recent
system/recent palette storage is not implemented. Explicit reusable Custom wells
use the separate profile policy below.

**Custom worksheet colours (Origin 2016, measured 2026-11-16).** Both Font Color and Fill Color
palettes have custom slots. Double-clicking an empty slot opens Colors on its Custom tab, with
RGB and HSL fields, preview, OK and Cancel. An empty slot initially showed RGB 64/192/192.
Entering RGB 17/34/51 for text and 32/128/224 for fill, then OK, changed only selected A1;
neighbouring cells and values stayed intact. The Custom list dropdown instead offers Clear,
Load and Open Increment Editor (not invoked). Captures/control dumps remain local in
`re-workspace/scratch/t59-custom-colors/`; only the owned unobscured dialogs/results are evidence.

Sheaf adds **Custom...** to the worksheet Font Color and Fill Color menus. Its one-off RGB
chooser validates whole numbers 0..255, previews the opaque colour and applies on OK or Enter;
Cancel, Escape and the close box discard the draft. Starting from the current manual colour
(white when fill is None) is Sheaf policy, differing from the measured empty native slot. A
choice changes only the captured range and property, preserving other styles and data, with
one Undo for an effective change. Selection, project-generation or revision changes invalidate
the request. Text and fill survive native project and `.opj` companion reopen through the
existing fields. Named lists/project palettes, HSL and eyedropper controls remain open; graph Format controls and the
separate T58 text editor retain their own behaviour.

**Reusable worksheet Custom wells (T59, documented contract, 2026-11-17).**
[FAQ130](https://docs.originlab.com/quick-help/define-custom-colors/) includes
Format Cells and documents reusable chooser colours, with Add to Custom for2016+.
No fresh Native measurement is claimed. Worksheet toolbar Custom... dialogs now
offer the graph/editor pool from graphing7.12: Add stores a valid opaque numeric
draft without formatting the sheet, and a saved choice seeds the numeric fields.
The dialog remains open until its existing OK/valid numeric-field Enter or Cancel.
Captured range/revision/generation, no-op and history rules are unchanged.

Format Cells offers the same pool beside each optional text/fill colour. Choosing
a well enables that colour in the dialog draft; only its existing Apply/OK commits
changed properties. Add saves a present opaque draft colour, without changing the
draft or document. None and nonopaque drafts cannot be added. The existing RGBA
control and optional-colour checkbox retain their behavior. Cancel keeps added
preferences and discards unapplied formatting. Context removal edits only the
pool, preserving drafts and applied colours. Compact menu layout, shared profile,
capacity/order, atomic storage and failure behavior are explicit Sheaf policies,
not measured Native2016 layout or file formats. Category defaults/families, model,
grid, print/export and IO engines are unchanged; existing fields preserve applied
colours. Unused wells remain profile-only, with no project palette or new list format.

**Classic worksheet presets (Origin 2016, measured 2026-11-16).** Both worksheet palettes
show the original 24 Classic colours above the additional colour lists. All 24 swatch-centre
RGB triples were measured from each palette and agree with the published [Classic list](https://docs.originlab.com/labtalk/ref/list-of-colors/).
Selecting Dark Yellow text, Dark Cyan fill and Royal text affected only A1; Fill None cleared
that fill while retaining Royal text, with neighbouring cells and source values unchanged.
Captures and RGB measurements remain local in `re-workspace/scratch/t59-classic-colors/`.
Dark Yellow is RGB 128/128/0, Olive is 0/128/0 and Royal is 0/0/160; Green remains 0/255/0.

Sheaf keeps the eight existing named choices and adds **More Classic colors** with the other
16 to worksheet Font Color and Fill Color. The compact text-submenu grouping is a Sheaf UI
choice; Origin uses swatches. These choices use the existing captured range requests, effective
no-op checks and one-step Undo, including independent labels and compact whole columns. Both
menu levels close after a choice; the separate Custom... RGB chooser remains available. Graph
Fill/Line palettes and the T58 text editor keep their own controls; graph Font Color now shares
these 24 presets (graphing 7.4). Other native colour lists and palette
storage remain open.

Cells use installed regular/bold/italic faces with UI glyph fallbacks; missing styles use the
existing synthetic fallback. The default is Arial 9 pt (12 logical pixels at 96 dpi). Explicit
point sizes convert by 4/3 for the grid. Row heights derive from font metrics, with a 16 px
minimum, including label rows, filtered views and frozen panes. Geometry stores formatting
intervals and draws only visible rows. **Difference:** Sheaf recomputes and can shrink heights
when the font shrinks; it has no saved row-height field, so Origin's sticky enlarged heights
are left for later work. Printed sheets and worksheet images use the same resolved styles,
font metrics and enlarged rows; their existing 8 pt default scales explicit sizes by 8/9.
Printing retains its existing omission of label rows with no text, including empty styled labels;
native printed-layout and range rules remain T27 work.

**Origin's dialogs (measured 2026-11-16).** F4 opens Worksheet Properties, with View, Size,
Format, Miscellaneous, Print/Export and Script tabs (the printing/script tabs were not inspected).
The Format tab's Apply To list is Standard, Column Labels, Data, Column Header, Row Header,
Long Name, Units, Comments and F(x)=. Data is initially selected; the dialog displays Data > Standard
as its application priority. Data initially has Rich Text off, Allow Enter/Wrap Text/Float on,
Disable Editing/Show Missing as Blank off, and separate unchecked Font and Color override switches.
These controls address worksheet categories. Further Data-font measurements below establish
family/size and future-column behavior; label-size measurements follow below.

**Data category fonts (Origin 2016, 2026-11-16).** `wproperties execute:=get iotrgui:=tr`
returns the Data font size through `tr.TextControl.FontControl.Size`; changing that child to
18 and executing `set` changes existing cells and a newly added C column. The UI Font group
has family and size controls. Selecting Times New Roman with an actual dropdown/keyboard
commit, then applying 20 pt, gives serif glyphs in existing and newly added columns. A manually
formatted A1 at 12 pt keeps that size after a worksheet Data change to 20; neighboring cells
and new C use 20. All synthetic values (A=1..5, B=2*A, C=101..105) remain unchanged.
Unchecking Font and pressing Apply leaves existing and new-column Data18 intact: this checkbox
controls which settings the dialog applies, not an operation that removes an inherited font.
The parent FontControl is a tree, not a supported numeric property; error-30.3 parent scalar
results are excluded, as are uncommitted family selections and early paint frames. The Face
getter's numeric rendering is not used as a font identifier. The successful actors, screenshots,
control lists and PID/start-time cleanup proofs stay in ignored scratch
`t59-worksheet-default-fonts/`; each owned Origin, helper and guard closed.

Changing this worksheet category does **not** auto-fit Origin's existing fixed-height rows;
large text clips. A later manual cell-font edit can grow its row based on neighboring text.
Sheaf retains its existing derived-height policy: effective Data fonts grow/shrink rows in the
grid, print and image paths. Persistent event-driven/sticky heights remain separate work.

**Label category sizes (Origin 2016, 2026-11-16).** Five isolated synthetic cases use
`wproperties execute:=get iotrgui:=tr`, set `tr.TextControl.ApplyTo$` to Long Name, Units,
Comments or Column Labels, change `tr.TextControl.FontControl.Size`, then execute `set`.
Long Name18 changes only Alpha/Beta; Units18 changes only V/mA; Comments18 changes only
note A/B. New C inherits the corresponding Gamma/nA/note C size. Other label categories and
data retain 9. Column Labels18 followed by Long Name24 gives specific24 over the other labels18;
reversing those operations retains the specific24, including new C. Category changes keep
fixed 16 px label heights and clip larger text. No family or manual-label precedence was established:
`wcellfmt` get failed in 2016, one toolbar attempt had no glyph effect and another found no
visible combo. Those attempts and an identity-snapshot race are excluded. Five accepted cases,
screenshots, scripts and PID/start-time cleanup hashes remain in ignored
`t59-label-default-fonts/native-proof.json`. Each new private COM instance and guard closed;
preexisting process identities were preserved. No global input was used for these label probes.

Sheaf enables Format > Worksheet... (F4) with a separate Worksheet Properties window, a Format
section and Data, Long Name, Units and Comments Apply To categories. Data family/size reach
all current and future data cells;
manual per-range properties take precedence. Drafts and Cancel/Escape are inert, each Apply/OK
is one Undo step, and stale document/sheet requests close. Unchanged OK creates no history.
The three supported label categories expose point size only and reach existing/future columns
independently. Switching categories retains drafts; Apply/OK writes all changed drafts together.
The compact layout, draft policy and manual-label precedence are Sheaf choices; manual-label
priority remains unverified against Native. Existing derived label heights remain in use.
The native Font apply-mask checkbox is omitted. Label family, Standard, headers, bulk Column
Labels and F(x)= controls remain disabled; further mutation/precedence rules need measurement.
Other worksheet tabs and color/
text-control defaults remain unavailable. The [official Format tab reference](https://docs.originlab.com/origin-help/wksproperties-dialog-format-tab/)
and [wproperties reference](https://docs.originlab.com/x-function/ref/wproperties/) describe the
current interface; the 2016 observations above come from the local synthetic probes.

Origin's **cell** dialog is separate: its Cell Range identifies the selected A1 in the probe.
The visible Text tab has Rich Text, Allow Enter, Wrap Text, Float, Disable Editing, Ellipsis,
Dynamic Merge, per-cell numeric display/precision selectors and a Font selector. In this new book,
Rich Text/Float/Disable Editing are off, Allow Enter/Wrap Text are on, Ellipsis is ### and Dynamic
Merge is None. Other controls and mutation/precedence rules were not measured.

The stock **Set Data Style** submenu contains Rich Text, Wrap Text, Float and More...; there is
no Default entry. More... opens that same selected-range Format Cells dialog. Sheaf enables More
through its existing Format Cells command. Rich Text/Wrap Text/Float remain disabled, assigned to
B5's worksheet text controls; they are not a Clear Formats operation. The earlier Default entry
was a placeholder, not an Origin measurement. The [Origin 2016 training manual](https://blog.originlab.com/ftp/dist/Origin_2016_Intermediate_Training_Manual.pdf)
also uses Set Data Style > Rich Text for report cells. Raw captures and control lists stay local in
`re-workspace/scratch/t59-worksheet-dialog/`; delayed initial dialog captures and the misleadingly
named default-action follow-up were excluded from behavioural claims.

Sheaf's **Format Cells** window (cell/column/row menus, Format > Cell..., Set Data Style > More...)
retains its existing manual-style subset: Font, Size (0 = Default), Bold/Italic/Underline,
text/fill colour, horizontal alignment, Clear Formats, OK, Cancel and Apply. It starts from the
first selected cell and applies only changed properties. The layout and choices of that subset
remain Sheaf's policy; Origin's text-control and per-cell numeric-display features are not supplied
by this window. Vertical alignment, named/project palettes and remaining dialog/precedence
measurements remain open.
**Vertical alignment probe (Origin 2016, 2026-11-16).** A numeric data cell A1 was
selected before Format > Cell; the dialog reported `[Book1]Sheet1!A[1:1]`. Its Text
controls include font/size and text/fill colour, with no top/middle/bottom text-alignment
control found in the inspected list. Worksheet Properties > Format with Data selected
and its Size tab likewise supplied no vertical text-alignment choice in these probes.
The Size tab controls worksheet dimensions; Dynamic Merge's Vertical option describes
merging. This is bounded negative evidence for these routes, not a claim about every
cell type, category, version or possible command.

The [official Format-tab reference](https://docs.originlab.com/origin-help/wksproperties-dialog-format-tab/)
lists font family/size and colour controls; that current reference is separate from the
2016 measurements. A supported vertical-alignment route and its mutation rules still
need to be established before adding a per-range model field or a live control.
Sheaf retains its current placement. The successful selected-A1 and settled Format/Size
captures, control lists, hashes and cleanup proof are local in
`re-workspace/scratch/t59-cell-vertical-align/native-proof.json`. The unsuccessful script
command, unselected-cell dialog, incorrect tab clicks and early paint snapshots are
excluded. Every temporary Origin instance, helper and identity guard closed.

**Documentation scope audit (2026-11-16).** The current
[worksheet Format-tab](https://docs.originlab.com/origin-help/wksproperties-dialog-format-tab/)
and [wproperties tree](https://docs.originlab.com/x-function/ref/wproperties/)
references establish no per-range vertical-alignment route. This adds no new
negative Native measurement. The
[2023b ORG-26070 release note](https://www.originlab.com/ReleaseNotes/detail.aspx?id=2023b3ORG-26070)
concerns Data Info and point-tooltip layouts. Their
[current column settings](https://docs.originlab.com/origin-help/data-info-tool/#columns-settings)
apply vertical alignment to the whole report table. Those controls are a different
target from worksheet cells and cannot establish worksheet-range mutation rules.
Per-range vertical alignment remains deferred until a supported worksheet route
and its behavior are established; no CellFormat field or live control is added.

Model and persistence rules: workbook-model.md 13b.

## 4. Graph window

Pictures: `gr-plain.png`, `gr-graph.png`, `grc-*.png`, `gdlg-*.png`; second study
`graph1.png`, `graph2-plain.png`, `legend.png`, `layer.png`, `gr-file.png`, `gr-file-more.png`,
`pd-*.png`, `ax-*.png`, `max-graph.png`.

- Inside the child window the **page** is white with a thin border; where the page does not fill
  the window the rest is mid grey; the page is shown at its true aspect ratio and scaled to the
  window ("fit page to window"). Left of the page there is a column of small boxes
  (ruler-like gutter) [seen].
- **Layer icons**: small square number buttons at the top-left of the page, one per layer (in
  `graph2-plain.png` a single "1"); the active layer is drawn with a dark/black number on a darker
  button, the others with grey numbers [seen]. Click an icon to make that layer active.
  **Right-click on the layer icon** [seen 2, `layer.png`] gives: Hide Layer, Hide Other Layers
  (grey with one layer), Show Active Layer Only, Set Active Layer By Layer Icon Only, Delete
  Layer, | Layer Contents... (F12), Plot Setup..., a text line "Double click to open Plot Setup"
  (a reminder, so a double-click on the layer icon opens Plot Setup, not Plot Details), | Axis...,
  Layer Properties..., Layer Management..., | a list of the plots in the layer, one line each,
  numbered, written like `1 g1 [Book1]Sheet1! "Conc"(X), "Signal"(Y) [1*:10*]`, with a tick
  at the active one.
- Default look of a graph made by `plotxy`: black axes with ticks pointing out, left Y and bottom
  X only, axis titles from long name and units (left "Signal (mA)", bottom "Conc (mM)"), a legend
  box at the top right inside the layer with the plot names (the long names, or the comment
  when there is one: "Scan 1" for a column with the comment "Scan 1", "Wave" for the one without),
  line plots in black and red (line width 0.5 pt), column plots with the first plot black, the
  second red and the bars of a group side by side [seen, seen 2]. Our figure rendering spec already
  covers the details [spec].
- Toolbars: the graph window enables the **Graph** toolbar (zoom, rescale, exchange axes, layers),
  the **2D Graphs** toolbar (bottom), the **Object Edit** toolbar (right), and the **Tools**
  toolbar (left) [seen, spec C]. The format toolbar shows the font size 22 while the legend is
  selected, and "0" when nothing is selected [seen 2, `legend.png`, `layer.png`].
- **Graph menu** [seen, `gr-graph.png`]: Add Plot to Layer (submenu), Layer Management..., Add
  Straight Line..., New Layer(Axes) (submenu), Merge Graph Windows..., Legend (submenu), Fit Page to
  Layers..., Exchange X-Y Axes, then a chevron to show more [seen]. (Fuller list: spec A3.)
- **Graph File menu** [seen 2, `gr-file.png`, `gr-file-more.png`]. Short form: Open... (Ctrl+O), Open
  Sample Projects (submenu), Save Project As..., Save Template As..., Import (submenu), Export
  Graphs... (Ctrl+G), Recent Books, Recent Projects, Exit, chevron. Long form: New (submenu),
  Open..., Open Excel... (Ctrl+E), Open Sample Projects, Append..., Close, | Save Project
  (Ctrl+S), Save Project As..., | Save Window As..., Save Template As..., | Batch Processing...,
  | Save Project without Data..., Duplicate Project without Data, | Print... (Ctrl+P), Print
  Preview, Page Setup..., | Import (submenu), Recent Imports (submenu), | Export Graphs...
  (Ctrl+G), Recent Exports (submenu), | Recent Books, Recent Graphs, Recent Projects (all
  submenus), | Exit. Compared with the workbook File menu: no Save Workbook As Analysis Template, no
  Word bookmarks, no Re-Import items, no Database Import, an "Export Graphs..." item in place of
  the Export submenu, and an extra "Recent Graphs". Its Import submenu short form has Single ASCII, Multiple
  ASCII and Excel like the workbook's [not opened in the picture; assumed from the same
  structure, [not seen]].
- **Other top-level graph menus** were photographed in the first study (`gr-edit.png`,
  `gr-view.png`, `gr-data.png`, `gr-analysis.png`, `gr-gadgets.png`, `gr-tools.png`,
  `gr-format.png`, `gr-window.png`, `gr-help.png`); the first-level items match [spec A1, A3];
  they were not transcribed item by item. The Window menu of a graph is in section 2.
- **Right-click, plot (a column)** [seen, `grc-plot.png`]: Change Plot to (submenu), Change X
  (grey), Change Y (grey), separator, Hide Data Plot, Go to Book1, separator, Ungroup, separator,
  Plot Details..., chevron [seen]. The picked plot shows small square handles at its data points
  [seen].
- **Right-click, axis** [seen, `grc-axis.png`]: Hide Axis, Hide Grids (ticked in my graph),
  separator, Rescale To Show All (Ctrl+R), Axis Zoom In, Axis Zoom Out, Center, separator, Copy
  Format (submenu), Save Format as Theme..., separator, Scale..., Tick Labels..., Properties... [seen].
- **Right-click, page (empty gutter near the page)** [seen, `grc-page.png`]: Add Text..., Layer
  Management..., New Layer(Axes) (submenu), Add Shortcut to Favorites, Copy Page (Ctrl+J), Send
  Graphs to PowerPoint..., View Full Screen (Ctrl+Shift+J), Copy Format (submenu), Duplicate
  (Batch Plotting) (submenu), chevron [seen].
- **Right-click, graph window title bar** [seen 2, `gr-titlebar-more.png`]: Show Script Panel,
  Save As..., | Hide, | Copy Page (Ctrl+J), Export Graphs... (Ctrl+G), Send Graphs to
  PowerPoint..., View Full Screen (Ctrl+Shift+J), Print... (Ctrl+P), | Add Shortcut to
  Favorites, Go to Original Folder (grey), | Duplicate, Duplicate (Batch Plotting) (submenu), |
  Refresh (F5), | Save Template, Save Template As..., | Properties... (Alt+Enter). The short form
  (before the chevron is clicked) has Hide, Copy Page, Send Graphs to PowerPoint..., View Full
  Screen, Add Shortcut to Favorites, Duplicate (Batch Plotting), Save Template As..., Properties...
  [seen 2, `gr-titlebar.png`].
- **Right-click, legend** [seen 2, `legend.png`]: the legend is selected first (eight green
  diamond handles on its corners and edges), then the menu: Plot Details..., Legend (submenu),
  Add Color Scale, Add Bubble Scale (enabled with a colour or size mapped plot, T80), | Cut (Ctrl+X), Copy (Ctrl+C), Delete
  (Delete), | Bring to the front, Push to the back, Bring to front of data (grey), Push to
  back of data, | Programming Control..., Properties..., | Copy Format (submenu), Save Format
  as Theme.... No chevron.
- **Right-click, title** (`grc-title.png`): not reviewed, [spec F].
- **Double-click** [seen as dialogs]:
  - on an axis: the dialog "Y Axis - Layer 1" (`gdlg-axis.png`, `ax-*.png`), described in
    section 5.
  - on a plot: "Plot Details - Plot Properties" (`gdlg-plot.png`, `pd-*.png`), described in
    section 5.
  - on the legend, text, graph page: opens the matching Plot Details page / text
    editor [spec D1, D3]. On the layer icon: Plot Setup (see its menu above) [seen 2].
- Colour list of the default palette ("By One"): black, red, green, blue... [spec graphing.md]; in
  the Group tab of Plot Details the strip behind "By One" shows a long list of colours.

### 4.x Zoom and pan (T65, measured 2026-11-15)

Origin 2016 (scripted clicks and pictures, scratch `re-workspace/scratch/t60/zoom`, `zoom2`):
- View > Zoom In (Ctrl+I) is a mode: nothing changes until a click on the page, which doubles the
  zoom (100, 200, 400, 800, 1600 % seen) and **re-centres the view on the clicked point**; the mode
  ends after the click. Scroll bars appear above 100 %.
- View > Zoom Out (Ctrl+M) acts at once and halves: 800, 400, 200, 100, 50, 25 %, then nothing.
- View > Whole Page (Ctrl+W) returns to 100 %: the page fitted to the window, no scroll bars.
- The Standard toolbar's zoom combo lists 25, 50, 75, 100 and 200 % and takes a typed value
  (300 gave 300 %); it shows the zoom rounded to whole percent.
- The Tools toolbar's magnifiers are **Scale In** and **Scale Out** (axis magnification: "double-click
  to restore, ctrl-drag to create zoom graph"), not the page zoom. The hand is the **Zoom-Panning
  Tool**: drag the page (or arrow keys) to pan, mouse wheel (or + and -) to zoom; "+" zooms by 1.25
  and "-" by 0.85 about the middle of the view, an arrow key moves about 2 points; a click with it
  changes nothing.
- View > Zoom All: corrected in graphing 23.1 (a zoom of the active graph so that all objects, also those outside the page, fit; the "Hit ESC" reminder belongs to Full Screen).

Sheaf (`crates/app/src/graph_zoom.rs`, `graph_view.rs`): the same Zoom In mode (one click, x2,
re-centred, up to 3200 %), Zoom Out by halves to 25 %, Whole Page, the Zoom-Panning tool (drag, +/-
x1.25/x0.85 about the middle, arrows 2 pt, wheel x1.25/x0.85 a notch about the pointer: the wheel's
factor and anchor are Sheaf's choice) on the Tools bar, manual scroll bars, and, without the tool,
the wheel scrolling a zoomed page (Shift+wheel sideways) and Ctrl+wheel zooming about the pointer
(Sheaf's choices: Origin's wheel without the tool was not measured). Zoom is per window, screen-only
and not saved; every click, drag and hit-test goes through the zoomed transform. The page raster is
capped at 4096 px a side (the texture is stretched beyond that). Not built: the zoom combo and the margin Origin lets the view scroll past the page. Zoom All and the Tools-bar
Scale In/Scale Out are T69 (graphing 23, built).

## 5. Dialogs, as seen

All dialogs are ordinary Windows dialogs with a title bar (text left, "?" help button and
close X on the right for the larger ones), tabs or a left list, and buttons in the bottom right,
in the order **OK, Cancel, Apply** (Plot Details has "Workbook" and ">>" in front; Options has
Restore and Reset in front and no Apply) [seen, seen 2]. The font in all dialogs is the system
UI font at about 9 pt; group boxes are drawn as a thin line with the title at the left;
disabled controls are grey; blue bold text marks a hint or a state message.

- **Tools menu** [seen 2, `toolsmenu.png`]. Short form: Options... (Ctrl+U), Apps on File Exchange
  (submenu), | Fitting Function Builder... (F8), Theme Organizer... (F7), Virtual Matrix
  Manager..., | Transfer User Files..., | Digitizer..., Video Builder..., | R Console..., Rserve
  Console..., Python Console..., chevron.
- **Tools > Options** [seen 2, `opts-first.png`, `opts-numfmt.png`; first study `dlg-options.png`].
  The dialog is about 525 x 465 px and has **10 tabs in two rows**: Miscellaneous, Excel,
  Open/Close, System Path, Numeric Format, File Locations, Axis, Graph, Text Fonts, Page (the
  first study counted 12 by mistake). Like the classic Windows tab control, the row that holds the
  selected tab sits at the bottom: Miscellaneous, Excel, Open/Close, System Path on the upper
  row and Numeric Format, File Locations, Axis, Graph, Text Fonts, Page below (the same with
  Numeric Format or Text Fonts selected). The buttons are Restore,
  Reset, OK, Cancel. It opened on **Numeric Format** every time, also in a run after another tab had been
  paged to in an earlier run, so it is not simply the tab used last [seen 2, `opts-check.png`].
  Numeric Format [seen 2]: group "Convert to Scientific Notation" with "Upper Threshold Power" (6)
  and "Lower Threshold Power" (-3), both editable drop-downs; group "Angular Unit" with the
  radio buttons Radian (selected), Degree, Gradian; group "Digits" with radio buttons "Decimal
  Digits" (selected) and "Significant Digits" and "Number of Digits" (5); group "Digits in Report"
  with the same two radio buttons and "Number of Digits" (5); then drop-downs "Separators"
  ("Window Settings"), "Separator for ASCII Import" ("Window Settings") and "Date Format for
  Database Import" (empty). Text Fonts [seen 2, `opts-numfmt.png`]: group "Text Tool" with Font
  ("Default: Arial"), Greek ("Symbol"), Color (Black), Size (22); group "In Percent of Current
  Font" with Line Spacing (10), Subscript & Superscript Size (60), Subscript Offset (50),
  Superscript Offset (30); a drop-down "Default" (Arial); a tick "Align Fonts with Baseline"
  (grey but ticked); ticks "Make text horizontal for in-place editing" (on), "Simple
  selection mode", "Disable in-place editing". So the default text size is 22 pt in the
  current default graph theme, not 18 as an older spec note said.
- **Import**: File > Import > Single ASCII opens a standard Windows file chooser titled "ASCII",
  starting in the Origin Samples folder, with file type "*.dat", a tick box "Show Options Dialog:" and
  "Open as read-only" [seen, `dlg-ascii.png`]. The options dialog is only shown when that box is
  ticked or when the file is imported from the Import Wizard [spec B2].
- **Import Wizard** (File > Import > Import Wizard..., Ctrl+3) [seen 2, `importwiz.png`]: the first
  page is titled "Import Wizard - Source" (a "?" and a close X, about 620 x 595 px, centred on
  the main window). Groups from the top: "Data Type" (radio buttons ASCII (selected),
  Binary, User Defined); "Data Source" ("File" radio button with a drop-down path box and a
  "..." button, and a "Clipboard" radio button); "Import Filter" (a tick "List filters applicable
  to both Data Type and file name" (on), a drop-down "Import Filters for current Data Type"
  (<None>), a read-only "Description"); "Target Window" (radio buttons Worksheet, Matrix, None
  (User Defined filter needs to create window), all grey until a filter is chosen; a drop-down
  "Template" (<default>) with a note that the template is only used when starting new books or
  sheets; a drop-down "Import Mode" (Replace Existing Data)). Buttons at the bottom: Cancel at
  the left, then "<< Back", "Next >>", "Finish" at the right; Back, Next and Finish are grey in the
  picture, where no file was chosen yet. Later pages [not seen].
- **Export Graphs** (`dlg-export.png`): title "Export Graphs: expGraph", top "Dialog Theme" box and
  a "Description" line, a form with Image Type (a drop-down, default shown "Encapsulated Postscript
  (*.eps)"), Select Graph(s) ("Active Page"), File Name(s) ("<long name>") with an "Enable
  Substitution" tick, a line "File will be saved as Graph1", Path, Overwrite Existing ("Ask"),
  Graph Theme ("<Original>"), then collapsible groups "Export Settings", "Image Size", "Image
  Settings"; a preview pane at the right with tabs "Image" and "Graph" and the text "No Preview";
  bottom row: "Auto Preview" tick, Preview, OK, Cancel, Apply, and a "<<" to hide the preview [seen].
- **Dialog Theme row**: the X-Function style dialogs (NLFit, Peak Analyzer, Export Graphs) all
  start with a "Dialog Theme" label, a wide box for a saved theme name and a small black
  right-pointing arrow button that opens a menu of theme actions [seen 2, `nlfit.png`,
  `peak-an.png`]. The menu itself was [not seen].
- **Fitting Function Organizer** (opened by Tools > Fitting Function Builder, F8, `dlg-nlfit.png`,
  not the NLFit dialog itself): a category tree (Origin Basic Functions, Implicit, Convolution,
  Exponential, Growth/Sigmoidal, Hyperbola, Logarithm, Peak Functions, Piecewise, Polynomial,
  Power, Rational, Waveform, Surface Fitting, PFW, Baseline, Chromatography, Electrophysiology,
  Enzyme Kinetics, Pharmacology, Rheology, Spectroscopy, Statistics), buttons Save, New Category,
  New Function, Builder..., Duplicate, Add, Remove, Cancel, OK, Simulate, Reset, Search, and tabs
  Equation, Sample Curve, Hints [seen].
- **Analysis > Fitting submenu** [seen 2, `fit1.png`]: Linear Fit..., Fit Linear with X Error...,
  Polynomial Fit..., Multiple Linear Regression..., | Nonlinear Curve Fit... (Ctrl+Y), Nonlinear
  Implicit Curve Fit..., Nonlinear Surface Fit..., | Simulate Curve..., Simulate Surface..., |
  Exponential Fit..., Single Peak Fit..., Sigmoidal Fit..., | Compare Datasets..., Compare
  Models..., Rank Models.... In this version "Nonlinear Curve Fit..." opens the dialog directly (it
  is not a submenu with an "Open Dialog" item). The Analysis menu ends with "Most Recently Used
  Items", which after one use shows "1 Nonlinear Curve Fit: <default>..." [seen 2, `pk2.png`].
- **NLFit dialog** (Analysis > Fitting > Nonlinear Curve Fit..., Ctrl+Y) [seen 2, `nlfit.png`,
  `nl-data.png`, `nl-curves.png`, `nl-findxy.png`, `nl-adv.png`, `nl-out.png`, `nl-bounds.png`].
  About 780 x 590 px, centred on the main window, title "NLFit ()" (the function name goes
  in the brackets), close X, no "?", disabled maximise. From the top:
  1. "Dialog Theme" row (see above).
  2. Four tabs **Settings, Code, Parameters, Bounds**. Under Settings a narrow list on the left
     (about 220 px) with the pages **Function Selection, Data Selection, Fitted Curves, Find
     X/Y, Advanced, Output** (selected page in blue); the page to the right of it (about 520 x
     180 px).
     - Function Selection: "Category" drop-down (Origin Basic Functions), "Function" drop-down
       (empty), "Iteration Algorithm" (Levenberg Marquardt, grey until a function is chosen),
       "Description", "File Name(.FDF)".
     - Data Selection: "Multi-Data Fit Mode" ("Independent Fit - Consolidated Report", grey
       with one data set), "Weights" ("Use Each Range's Setting", grey), "Input Data" as a
       tree node with a "+" and a read-only box that holds the range as text
       (`[Book1]Sheet1!(A"Conc",B"Signal")` here: the X column and the selected Y column) with a
       "..." button and a small arrow; below it a blue message "Function is empty. Please select a
       fitting function first.".
     - Fitted Curves: a tick "Show Preview on Source Graph" (grey), a group "Fitted Curves Plot"
       with ticks "Plot in Report Table", "Stack with Residual vs. Independents Plot", "Update
       Legend on Source Graph" and drop-downs "Plot Type" (Raw Data), "Plot on Source Graph"
       (Fitted Curve); the page scrolls (a scroll bar at its right).
     - Find X/Y: two collapsible groups "Find X from Y" and "Find Y from X", each with a tick box.
     - Advanced: a drop-down "Recalculate" (Manual), then collapsible groups Replica (grey), Fit
       Control, Quantities, Residual Analysis.
     - Output: a collapsible group "Output" with sub-groups Graph, Dataset Identifier, Report
       Tables, Fitted Curves, Fit Residuals, Find Specific X/Y Tables (grey), and a drop-down
       "Fitted Results Sheet Arrangement" (Combined); scrolls.
     - Bounds tab: a tick "Auto Parameter Initialization" (on), a hint line about double-click
       and right-click on cells, a "Hide..." button at the right and an empty table with the
       columns NO., Meaning, Value, Lower Bounds, "< or <=", Param, "< or <=", Upper Bounds
       (filled once a function is chosen) [seen 2, `nl-bounds.png`]. The Code and Parameters
       tabs were not opened [not seen].
  3. A red message line "Function is empty. Please specify it to continue." (the state of the
     fit; red until the fit can run).
  4. A button row: nine small icon buttons at the left (about 24 x 24 px each, in groups) and the
     text buttons **Fit**, **Done**, **Cancel** at the right (Fit and Done grey until valid); at the
     far right two small blue arrows (up and down) that hide or show the lower half.
  5. The lower half has tabs **Fit Curve, Residual, Formula, Sample Curve, Messages, Function
     File, Hints** and a preview area; with invalid input it shows "No Preview" and "Invalid input
     found! Please correct it to continue fitting." in large type.
  A yellow hint note about dragging columns onto a graph sits behind the dialog; it belongs to the
  workbook, not the dialog.
- **Analysis > Peaks and Baseline** [seen 2, `peak1.png`, `pk2.png`]: Multiple Peak Fit..., |
  Peak Analyzer (submenu: "1 QuickPeaks (System)", | "Open Dialog..."), Batch Peak Analysis Using
  Theme.... The "Most Recently Used Items" part now lists "1 Nonlinear Curve Fit: <default>...".
- **Peak Analyzer, first page** [seen 2, `peak-an.png`]: a dialog about 385 x 590 px, title "Peak
  Analyzer", with the "Dialog Theme" row. Then a **black panel** (about 360 x 195 px) that draws
  the steps as a vertical flow chart: a green square at the first step "Goal - Integrate Peaks"
  (bold, yellow text) and yellow squares for "Baseline Mode", "Baseline Treatment", "Find Peaks",
  "Integrate Peaks", then a red square at "Finish", joined by thin vertical lines; the panel
  has its own scroll bar. Under it the buttons **Prev** (grey), **Next**, **Finish**, **Cancel**
  and two small blue arrows. A line with the page id ("pa_goal") and a one-line hint ("Select
  spectrum data and Goal"). The page itself: "Recalculate" drop-down (Manual), a blue sentence
  describing the goal, **Goal** with five radio buttons (Integrate Peaks (selected), Create
  Baseline, Subtract Baseline, Find Peaks, Fit Peaks (Pro)), and an "Input" tree node with a
  read-only range box (`[Book1]Sheet1!(A"Conc",B"Signal")`). So Sheaf's peak analysis could
  be a wizard of pages with Goal, Baseline Mode, Baseline Treatment, Find Peaks, Integrate Peaks
  and Finish. Pages after the first [not seen].
- **Analysis, Statistics, Image, Format, Help menus** [seen, `wb-analysis.png`]: Analysis:
  Mathematics, Data Manipulation, Fitting, Signal Processing, Peaks and Baseline (each a submenu),
  then "Most Recently Used Items". **Statistics**: Descriptive Statistics, Hypothesis Testing,
  ANOVA, Nonparametric Tests, Survival Analysis, Multivariate Analysis, Power and Sample Size,
  ROC Curve... **Image**: Adjustments, Arithmetic Transform, Conversion, Geometric Transform,
  Spatial Filters. **Format**: Menu, Worksheet... (F4), Column..., Cell..., Snap to Grid, Merge
  cells (Ctrl+R), Programming Control..., Object Properties... For Help see section 3.
- **Plot menu of a workbook** [seen, `wb-m04-column.png` shows the Plot menu]: Line, Symbol, Line +
  Symbol, Column/Bar/Pie, Multi-Y, Y-offset/Waterfall, Multi-Panel, 3D XYY, 3D Surface, 3D
  Symbol/Bar/Vector, Statistics, Area, Contour/Heat Map, Profile, Specialized, Stock, System
  Templates..., User Templates, Most Recently Used (all with arrows except the templates dialog)
  [seen]. Those under "Line" and so on open galleries [spec].
- **Plot Details, line plot** [seen 2, `pd-line.png`, `pd-line-line.png`, `pd-line-drop.png`,
  `pd-line-label.png`, `pd-line-offset.png`, `pd-line-layer.png`]. A wide dialog, about 1005 x
  650 px, title "Plot Details - Plot Properties" (it changes to "Plot Details - Layer
  Properties" when the layer node is selected), "?" and close X. Left (about 315 px): a tree
  "Graph1" > "Layer1" > one line per plot, written like `[Book1]Sheet1! "Conc"(X),
  "Signal"(Y) [1*:10*]`, each with a tick box; the selected plot has a blue frame. Under the
  tree: "Plot Type" with a drop-down ("Line"), then buttons ">>" (expands a side panel), "Workbook",
  OK, Cancel, Apply (grey). Right: tabs **Group, Line, Drop Lines, Label, Offset** (for a line
  plot there is no Symbol tab).
  - Group: a framed pair of radio buttons "Edit Mode: Independent / Dependent" (Dependent is
    chosen), a grid with the columns (blank), "Increment", "Details" and the rows "Line Color"
    (Increment "By One", a strip of colour samples, a "..." button) and "Line Style" (Increment
    "None", a strip of dash samples), a "Group Members" list with a line sample and the
    column name ("Book1_B", "Book1_C"), and a group "Subgrouping" with radio buttons None / By
    Size / By Column Label and a greyed "Subgroup on Increment" row.
  - Line: "Connect" (Straight, a drop-down with a picture), "Style" (Solid), "Width" (0.5),
    "Color" (Black, a drop-down with a pen icon and a small strip of the colour list),
    "Transparency" (a slider with a 0 % spin box), a tick "Fill Area Under Curve" and a grey
    drop-down (Normal).
  - Drop Lines: ticks "Horizontal" and "Vertical", each with Style / Width / Color boxes, and a
    group "Data Points Display Control" with a tick "Skip Points" (value 2) and "No Line Break for
    Skipped Point(s)".
  - Label: tick "Enable", "Show at Specified Indices Only", an indices box, Font ("Default:
    Arial"), Size (22), Color, "White Out", bold / italic / underline buttons, Rotate, Frame,
    Margin, Horizontal and Vertical Offset, Position (Right), Label Form (Y), Numeric Display
    Format, Attach to, X/Y Value, "Auto Reposition to Avoid Overlapping", Reposition Direction
    and a group "Leader Line"; everything is grey until "Enable" is ticked.
  - Offset: Multiplier and Offset for X and Y (grey) with a note that offsets are switched on in
    the Stack tab of the layer.
  - Layer node: tabs **Background, Size/Speed, Display, Stack**; Background has "Color" (None),
    "Border Style" (None) and a "Gradient Fill" group (Mode, Color, Direction).
- **Plot Details, column plot** [seen 2, `pd-col.png`, `pd-col-pattern.png`, `pd-col-spacing.png`,
  `pd-col-label.png`, `pd-col-layer.png`]: same frame; "Plot Type" shows "Column / Bar";
  tabs **Group, Pattern, Spacing, Label**. Group has the rows "Border Color", "Fill Color"
  (both "By One" with colour strips), "Border Type" and "Fill Pattern" (both "None") and a
  Group Members list with filled colour boxes (black, red). Hovering a plot node in the tree
  shows a small coloured tooltip with the book, sheet and column names [seen 2].
  - Pattern: group "Border" (Color, Style Solid, Width 0.5), group "Fill" (Color, Pattern None,
    Pattern Color, Width), "Transparency" slider, a "Preview" box showing the bar, and a
    "Gradient Fill" group (Mode, Color, Direction).
  - Spacing: "Gap Between Bars (in %)" (20), "Overlap (in %)" (0), "Width (in %)" (None) and
    "Scaling Factor" (grey).
  - Label: like the line plot's, but Position is Center, Margin 10 and an extra tick "Hide Label
    if Column/Bar Height Less than (%)" (3).
- **Axis dialog**, double-click on the left axis [seen, `gdlg-axis.png`; seen 2, `ax-*.png`]. Title
  "Y Axis - Layer 1", "?" and close X, about 605 x 610 px, centred on the main window. A narrow
  list at the left (about 75 px) holds one big icon button per target, selected one in blue, and
  the list changes with the tab. At the right the tab row **Scale, Tick Labels, Title, Grids, Line
  and Ticks, Special Ticks, Breaks**. Below the tabs: the text "Select multiple axes to
  customize together." and the buttons "Apply To...", OK, Cancel, Apply (grey).
  - Scale (list: Horizontal, Vertical): From, To, Type (Linear, with a picture), Rescale (Normal),
    Rescale Margin(%) (8), Reverse tick box, group "Major Ticks" (Type "By Increment", Value,
    First Tick) and group "Minor Ticks" (Type "By Counts", Count). The "From" box is focused
    when the dialog opens.
  - Tick Labels (list: Bottom, Top, Left, Right): a tick "Show" and a tick "Use Same Options for
    Left and Right", then four sub-tabs **Display, Format, Table, Minor Tick Labels**. Display:
    Type (Numeric), Display (Decimal:1000), tick "Set Decimal Places", "Divide by Factor",
    "Formula" with an example in blue, Prefix, Suffix, ticks "Plus Sign" and "Minus Sign" (on).
  - Title (list: Bottom, Top, Left, Right): ticks "Show", "Use Same Options for Left and Right",
    "Text" (holds the placeholder `%(?Y)`, which stands for long name plus units), "Color"
    (Black), "Rotate (deg.)" (90), group "Font" with Font (Auto) and Size (22).
  - Grids (list: Vertical, Horizontal): group "Major Grid Lines" (tick Show, Color Gray, Style
    Solid, Thickness 0.5), group "Minor Grid Lines" (Show, Color LT Gray, Style Dash, Thickness
    0.3), group "Additional Lines" (tick "Opposite", tick "X =" with a value). Boxes are grey
    until "Show" is ticked.
  - Line and Ticks (list: Bottom, Top, Left, Right): ticks "Show Line and Ticks", "Use Same
    Options for Left and Right"; group "Line" (Show, Color Black, Thickness 1.5, Axis Position
    "Left"); group "Major Ticks" (Style "Out" with a picture, Length 8, Color Auto, Thickness
    <auto>); group "Minor Ticks" (Style Out, Length <auto>, Color Auto, Thickness <auto>).
  - Special Ticks (list: Bottom, Top, Left, Right): a tick "Use Same Options for Left and Right"; a
    table with columns "At Axis Value", "Label", "Show" and the fixed rows "Axis Begin", "Axis
    End" (both Show "Auto") and an entry row ("<value as entered>", tick); buttons Delete and
    Details...; a short note about adding a tick in the last row.
  - Breaks (list: Horizontal, Vertical): tick "Enable", "Break Half Length" (<auto>), "Number of
    Breaks" (None), a table with the columns Break From, Break To, Auto Position, Position;
    buttons Delete and Details....
  The tab and the list are independent: choosing a different tab changes which targets the list
  offers (two for Scale and Breaks, four for the others).

### Find and Replace (2026-11-15)

Origin 2016's public `findreplace option:=0;` and `option:=1;` commands open the Find and Replace
tabs of one window. The [command reference](https://docs.originlab.com/x-function/ref/findreplace/)
documents this entry point. Clear pictures, control dumps and post-operation values are in
`re-workspace/scratch/t02-find-replace/`. The initial Ctrl+F/Ctrl+H attempts found no dialog and
are excluded as layout evidence; the direct-command captures succeeded.

The initial captures show Numeric selected, condition `=`, Active Worksheet, and expanded Options.
Find what is seeded from the selected cell's display: B2 containing 5 gives `5`, and a stored
100.000001 displayed as 100 gives `100`. Selecting text Alpha seeds `Alpha` but leaves Numeric
selected, with Find Next disabled until the query is valid or the user chooses String.
Skip linked cells is checked. Absolute value, Search up, Keep sign and Set as missing are unchecked;
tolerance is `1e-8`. Numeric conditions are `=`, `<`, `<=`, `>`, `>=`, `!=`.
The Replace tab adds its replacement field and two numeric replacement flags. Its bottom buttons
are Find Next, Replace, Replace All, Close; Find has Find Next and Close at the right.

String mode hides the numeric condition/tolerance and shows Match case, Match whole cell contents,
Use wildcards and Include label rows, initially unchecked, alongside Skip linked cells and Search
up. Look in also offers workbook, folder, recursive folder, open books in folder and project scopes.
Options can persist in Origin across temporary instances: the last probe explicitly
restored the original numeric/string flags, condition, scope and tolerance; the restored image and
action log are retained in `final/restore/` and `conditions/restore/`.

The synthetic GUI probes establish these rules:

| Probe | Observed result |
|---|---|
| Numeric 100, tolerance 1e-8, data 100.000001 and 100 | both found/replaced: relative tolerance, not an absolute 1e-8 difference |
| Numeric 2, data 1.99999999, 2, 2.00000001, missing | `=`, `<=`, `>=` match the three finite cells; `<` matches the first; `>` the third; `!=` matches first, third and missing |
| Absolute 2, replace 7, Keep sign, data -2, 2, -3, 0 | -7, 7, -3, 0 |
| Greater than 2, replace 7, Set as missing, data -2, 2, 3, missing | missing, missing, 7, missing |
| Find numeric `--` with one explicit missing value at A4 | A4; unused allocation beyond the data did not become the result |
| Find 2 upward from B2, B = 2, 5, 2, 8 | B1 rather than B3 |
| String `a*c`, wildcard on, replace Z; abXXc, xabc, abc | Z, xabc, Z: the pattern must match the entire cell |
| Numeric 2, empty replacement | the matching cell becomes missing |

Equality uses the relative difference `abs(a-b)/(abs(a)+abs(b))` with exact equality handling zero.
The inclusive comparisons include that equality test; strict comparisons and `!=` do not use its
tolerance. Thus `!=` is deliberately not the complement of tolerant `=`. Numeric searches use stored
numbers, whereas String searches use displayed text and preserve ordinary substring replacement
when wildcards are off. Wildcard replacement substitutes the entire matching cell with literal text.

Sheaf follows these measured controls and rules in the existing dialog, with a shared Find/Replace
window, preserved inputs across its tabs, selected-cell seeding and one undo step per replacement
action. Formula columns retain their existing replacement protection. Search, invalid input, Close
and Escape create no document edit. Case and whole-cell String behavior remains available; old
substring UI regressions explicitly select String now that Numeric is the initial mode.
Four new UI regressions and four parser/matching fixtures cover these behaviors. `--show find` and
`--show replace` support native screenshots.

The selected-range follow-up uses `re-workspace/scratch/t02-find-selection/verified/` and
`bounds/`: all eleven corrected GUI cases have successful setup/command results. The initial
six cases had no workbook and are excluded. A B2:B3 selection shows **Find/Replace in selected
range**, initially checked, immediately below Skip linked cells. Finding 6 and then 5 stays
inside B2:B3; replacing 5 with 9 changes B2 and leaves the equal cells in C alone. Unchecking
the option lets Find Next reach C1. After that unrestricted hit collapses the range, Origin
hides the option. Unchecking and rechecking before searching retains the original range.
Unchecked Replace All covers the sheet without collapsing the selection, and the option stays
visible. Whole-column and row-header selections offer it, including a column with one populated
value and a single whole row; a single selected data cell does not. The final fresh-range probe
explicitly restores the checked setting; its control dump and action log confirm it.

Sheaf snapshots the opening columns and rows, offering that checked scope for blocks, columns,
rows and Select All. Numeric and String searches (including backward search and wildcards),
single replacement and Replace All respect it. Clearing numeric nonmatches also stays inside
the range. Finding a cell moves Sheaf's worksheet selection to that hit; the saved search range
survives until a successful unchecked search, which hides the option. Origin instead retains
the visible selected rectangle while showing a restricted hit. Four additional UI regressions
and one matching fixture cover range retention, option visibility, both replacement paths,
unchanged search history, undo, noncontiguous columns and conditional missing values.

Remaining: searches still cover active-sheet data only. The wider Look in scopes, label rows,
individual cell links, saved dialog history/settings and the query's
triangle menu are not implemented. Look in and Include label rows are disabled; Skip linked cells
is fixed checked because individual links are not modeled. The worksheet's normal formula-column
protection remains in force independently. These gaps keep the parity row partial.

### Go To (2026-11-14)

Origin 2016's `dlgRowColGoto -d;` opens **Go To: dlgRowColGoto**. The clear screenshot and control
dump in `re-workspace/scratch/t02-go-to/explicit-dialog/` show size/current-selection information,
a zero/end hint, **Go to Column**, **Go to Row**, **Extend Selection** (unchecked), then OK/Cancel.
Both input fields start blank, with the caret in Column. The worksheet chooser is hidden. The
column input has a picker button. Direct `dlgRowColGoto;` does not
open a window, and the separate shortcut-capture attempt found no dialog; neither is UI evidence.
The [published shortcut reference](https://docs.originlab.com/origin-help/hotkey-in-origin/)
identifies this function as the workbook Ctrl+G command.

Headless synthetic probes (`selection*.lines.txt` / `.out.txt` in the same scratch directory)
read `wks.c1/c2/r1/r2/sel` after calling the function:

| Initial selection | Input (1-based) | Result |
|---|---|---|
| C7 | row 10, column 2 | B10 |
| C7 | row 10 only | C10 |
| C7 | column 2 only | B7 |
| B2:C4 | row 10 only | B10:C10 |
| B2:C4 | column 4 only | D2:D4 |
| B2:C4 | row 10, column 4, extend | B2:D10 |
| B2:C4 | row 1, column 1, extend | A1:C4 |
| B2:D10 | row 4, column 3, extend | B2:C4 |
| B2:C4 | row 10 only, extend | B2:C10 |
| B2:C4 | column 4 only, extend | B2:D4 |

An omitted coordinate keeps that whole axis of the selection. For an entered coordinate with
Extend enabled, the fixed edge is the previous far edge when the target is before the region;
otherwise it is the first edge. Thus moving inside the region can shrink it. Row-header selections
become cell ranges over all columns; column-only movement of whole columns retains whole-column
selection. On a sheet allocated 100 rows with four populated rows, row 1000 clamps to 100 without
growing the allocation. Row 0 and column 0 select the last **data** row and last column (D4 here),
not row 100. An unequal-length control (B has two values, C six, D three) shows row 0 goes to B2
when column 2 is entered or the current column is B, and to D3 when column 0 is entered. Omitting
the column for a B:C cell range or whole-column selection also uses B's data end, keeping both
columns selected. Thus the explicit destination column, or the first selected column, determines
the end; the worksheet-wide maximum does not. Column 999 is refused, leaving C7 untouched. The headless column-name string goes
through integer conversion, so no GUI name-resolution rule is inferred from that diagnostic.

The follow-up UI captures in `re-workspace/scratch/t02-go-to-picker/` set the actual Column and
Row edit controls, then click OK (each command returned true). On a four-column sheet with long
names Time, Shared current, Potential, Shared current, column `b` and row 10 select B10;
`Potential` and `potential` both select C10; `Shared current` selects the first duplicate, B10.
The flyout capture lists A(X): Time, B(Y): Shared current, C(Y): Potential and D(Y): Shared current,
followed by Reset and Select Columns. These captures, rather than numeric X-Function argument
conversion, establish the tested short-name, ASCII case and duplicate-long-name behavior.
The verified menu batch chooses D and row 0, reaching D3 despite B having the same long name;
the native picker fills the Column field with `D"Shared current"`. Reset clears an entered B
without clearing row 10, so OK reaches C10 from C7. Escape closes the flyout first and leaves
the Go To dialog open. The earlier all-black choose/reset images are excluded as visual evidence;
the clear `verified-menu/` captures and post-OK coordinates supersede them.
Select Columns opens a separate Column Browser, defaulting to Current Sheet and offering project,
folder, recursive folder and book scopes, a column table and Add/Remove controls. That general
browser is now available from Sheaf's Go To flyout (follow-up below).

The Column Browser follow-up (`re-workspace/scratch/t02-column-browser/verified/` and
`followup/`) uses eight isolated Origin 2016 instances, each with explicit input-field resets.
The initial exploratory batch targeted the hint fields rather than Column/Row; its destination
coordinates are excluded. The browser's upper table lists sheet, book long/short names, index,
column short/long names, comments, format, size and first value. Add puts a qualified column
reference into the lower Range/Rows table, initially `[1:end]`; Remove deletes the selected
lower range. OK transfers the reference to the parent field without navigating. The parent OK
performs the navigation. Browser Cancel preserves the parent's typed column and row.

| Browser probe, original columns A/B/C/D ending at rows 4/2/6/3 | Observed Go To result |
|---|---|
| Add D with duplicate long name, parent Row 0 | qualified D reference; D3 on parent OK |
| Add B, parent Row 0 | B2 |
| Add A then C, parent Row 10 | parenthesized qualified list; D10, the original sheet's last column |
| Add A then C, remove A, parent Row 0 | qualified C reference; C6 |
| Add another book's B (five populated rows), parent Row 0 | original book's B2, not the other book's row 5 |
| Type B/10, add D, cancel browser, then parent OK | B10 |

Thus Go To uses the chosen column's position in its original worksheet; choosing a column from
another book does not activate that book. Several added columns behave as the last column in
this Origin version. Sheaf copies both observed rules, including the qualified display string.
Editing the column field, picking a normal flyout item or Reset discards the browser choice;
Reset still preserves Row. The five scope labels are Current Project, Current Folder, Current
Folder (recursive), Current Book and Current Sheet. Scope can persist in Origin across temporary
instances: the final restore explicitly selects Current Sheet, with its control dump retained.
Cancelled Go To commands return false even though the browser was successfully captured.

Sheaf lists the current project's column metadata and filters these scopes without changing
workbook data, folder contents or activation. Folder scopes use the original workbook's folder;
unlisted windows belong to root. Added ranges survive scope changes. The browser starts at
Current Sheet each time; preferences are not saved. The parent remains open on browser Cancel
or Escape. Five UI regressions cover return/apply, cross-book position mapping, Cancel/Escape,
multiple ranges/Remove and edited/reset references. A fixture covers sheet/book/folder/recursive
scopes, including unlisted root books. Exclude is covered by the later follow-up below.

The top-table selection follow-up (`re-workspace/scratch/t02-browser-selection/origin/` and
`verified/`) has nine valid isolated Origin cases, including a final Current Sheet restore.
Shift+Down extends B through D; ordinary Add appends B, C and D as three lower rows.
Add as One Block instead appends one `[Book1]Sheet1!B"Shared current":D"Shared current"`
range. Ctrl-selected adjacent B/C can form a block; noncontiguous A/C and a range crossing
workbooks disable block add. Ctrl+A selects all four columns and can add one A:D block.
Remove deletes the entire selected lower block; an empty list disables OK. A B:C block plus
a separately added D remains two lower rows and transfers a parenthesized `(B:C,D)` reference;
parent OK with Row 0 still selects original D3, including for B:C alone. Quoted long names
remain on each endpoint. The initial noncontiguous case's programmatic click bypassed the
disabled block button, so its activation/result are excluded; later probes check native
enablement and skip disabled buttons. One composite capture failed to open the browser and
is excluded; its fresh-instance retry supplies the valid block-plus-singleton observation.

Sheaf now supports plain, Ctrl/Command and Shift row clicks, Shift+Up/Down and select-all
while the top table has focus. Add preserves selected columns in table order; block add
requires consecutive columns in one worksheet. Blocks remain single lower rows for Remove
and reference transfer. Scope changes clear the top selection and preserve added ranges.
Five additional UI regressions cover contiguous and noncontiguous selection, adjacent Ctrl
selection, disabled block controls, select-all/group removal, mixed-workbook ranges, scope
changes and block/list transfer. Navigation still waits for the parent, preserving data/history.

The lower row-range follow-up (`re-workspace/scratch/t02-browser-rows/`) opens the ellipsis
in a lower Rows cell. Origin 2016 shows a separate Range dialog, with Entire Column(s)
checked and From/To disabled. In the synthetic sheet whose A/B/C/D columns end at 4/2/6/3,
the defaults are 1 and 6, and the displayed bound is 1 through 6, including when B is selected.
The upper bound follows the sheet's data end, rather than the selected column's end or its
100 allocated view rows. The published [input-data guide](https://docs.originlab.com/origin-help/analydialog-specifyinput/)
also describes row subsets; the installed-version captures establish the details here.

| Native row-editor case | Observed result |
|---|---|
| C From 2, To 4 | lower rows `[2:4]`, qualified C reference with `[2:4]` suffix; parent Row 0 still C6 |
| B From 3, To 6, although B has two values | accepted `[3:6]`; parent Row 0 still B2 |
| C From 4, To 2 | accepted literal `[4:2]`, without sorting or an ordering error |
| Enter 2/4, then Cancel | original entire-column range remains |
| Save 2/4, reopen, select Entire Column(s), reopen | suffix removed, `[1:end]` restored, disabled From/To reset to 1/6 |
| From or To 0, To 7, or nonnumeric `abc` | refused; Range remains open with the existing lower range unchanged |
| Numeric edits `1.5`, `2.9`, `6.9`, `2+1`, `2abc`, `2e1` | leading integers 1, 2, 6, 2, 2, 2; bounds checked after conversion, no arithmetic/exponent evaluation |
| B:D block, From 2, To 4 | endpoint reference `B"Shared current"[2]:D"Shared current"[4]`; parent Row 0 still D3 |
| C From 2, To 2 | explicit `[2:2]` interval retained |
| Empty four-column sheet | disabled From 1/To 0; Enter accepts Entire Column(s), with no row suffix |

Row bounds belong to each added range. The child editor leaves the browser and Go To open,
and accepting it changes only the browser's range; navigation still waits for the parent OK.
Cancel and Escape discard the child's pending bounds, preserving the saved range. Selecting
Entire Column(s) removes the suffix rather than storing an explicit full-length subset.
The measurements comprise five isolated instances and two sweeps with a fresh synthetic project
per case: 18 valid cases. The first empty-sheet attempt used two columns, which shortened the
parent flyout and caused the fixed menu click to miss the browser; that case is excluded and
replaced by the four-column capture. Scope is explicitly restored to Current Sheet.

Sheaf uses the measured sheet bound and leading-integer conversion, retains reversed and
single-row intervals, and refuses invalid bounds without closing the child. A block's row subset
stays attached to its added range when another lower row is removed. Enter accepts the default
without toggling Entire Column(s), and respects a focused Cancel button. Ten additional UI
regressions cover row transfer/end mapping, sheet bounds, reversed and numeric inputs, refusal,
Cancel/Escape/reopening, whole-column reset, empty columns, keyboard focus and block association.
Saved preferences remain open; these row choices stay local
to the open dialog and create no document edit or undo entry.

The Exclude follow-up (`re-workspace/scratch/t02-browser-exclude/`, `verified/`) records ten
valid isolated Origin 2016 cases on the same four-column fixture. Exclude starts disabled with
no upper selection. It removes only selected entries from the upper list: excluding B retains
the distinct D with the same long name and selects C, whose displayed index remains 3. Excluding
the final D also selects C. For Ctrl-selected A and C, both click orders remove A/C and select B.
Selection therefore follows the first removed list position, clamped to the final remaining row.
Excluding every column leaves the table empty and Add/Exclude disabled. Changing scope restores
all columns and clears the selection; cancelling and reopening the browser also restores them.

An already-added B remains in the lower table after excluding upper B; browser OK transfers B,
and parent Row 0 still reaches original B2. With B excluded, selecting visible A/C/D keeps Add as
One Block disabled because the original worksheet columns are not consecutive. Ordinary Add
still adds those three columns individually. All valid cases retain four worksheet columns and
the original B/C values; Cancel preserves C7, while choosing the nearest C with parent Row 0
reaches C6. The failed first follow-up batch left the browser open after an Escape sequence and
ended at its own instance watchdog; it is excluded. The seven repeated cases use fresh instances
and explicit Cancel buttons, and the final capture restores Current Sheet.

Sheaf now implements these temporary exclusions, nearest-row selection, original column ordinals,
scope/reopen reset and independent lower ranges. Seven UI regressions cover both single-row
positions, duplicate names, both multi-selection orders, all-column removal, scope/reopen reset,
already-added ranges and gapped block refusal, with unchanged project data and history.

Sheaf's Go To now follows the measured input order/focus, right-aligned OK/Cancel and optional-coordinate, zero/end,
extension and clamping behavior. Its capacity remains data length plus 30 spare view rows;
there is no new stored worksheet row count. Short names and the first matching long name are
accepted, ignoring ASCII case; numeric input keeps its index meaning. Rows use nonnegative 32-bit
integer input; the exact native upper bound was not stress-tested. The size/current hint reports
Sheaf's actual view and first selected coordinate. No project data or undo state is
changed by navigation, empty OK, Cancel or Escape. A fresh reveal target clears stale cell
geometry so the selected blank row can actually scroll into view. Explicit far-column jumps use
the full horizontal distance rather than stopping after four viewport-sized steps; the column
picker lists column names, designations and long names. Six navigation UI tests cover the
measured axis/range cases, forward/backward/inside extension, capacity/end navigation, visibility,
keyboard continuation, far-column jumps in both directions, invalid entries and cancel/history; parser fixtures include the 100-row,
four-populated-row distinction and a new unequal-column end fixture. `--show go-to` supports native screenshots.

### Window Switcher (T42)

Origin facts (documentation, not measured here): 2025b adds Ctrl+Tab (forward) and Ctrl+Shift+Tab
(backward) "to toggle child windows with preview" [W:originlab.com/2025b]. Pressing Ctrl and then
Tab shows the Window Switcher; each Tab moves on until Ctrl is released; it shows thumbnails of
all windows of the same folder, including floating and hidden windows; the selected window has a
blue frame; arrow keys and a mouse click also select; clicking Ctrl alone closes it. LabTalk:
`doc -nav` then Tab [W:origin-help/operate-childwindow-basic].

Sheaf's choices (unmeasured, the documentation gives no order, Esc or sizes):

- **Order:** most recently activated first (`Workspace::recent_order`; activation by click, menu,
  Project Explorer, Go To, opening). So the first Ctrl+Tab selects the second card, the previous
  window. A window that is closed or minimised drops to the end of the list (or behind the new
  active window), so it is not "previous". Windows never activated follow in creation order.
- **Keys:** Ctrl+Tab opens with the first card that is not the active window selected (the
  second card when the active window is listed first, the first card when the Project Explorer's
  folder does not hold the active window), Ctrl+Shift+Tab with the last. While
  open: Tab / Shift+Tab and Left / Right move and wrap, Up / Down move by a row, Enter confirms,
  Esc cancels, releasing Ctrl activates the selection (showing it if hidden), a click on a card
  activates it, a click outside cancels. Ctrl+Tab is read before the grid, so it works while a
  worksheet has the keyboard; plain Tab still moves the cell. Not opened while a dialog is open.
  While it is open (also when opened from the menu) it is modal: every other key and text event
  (Delete, letters, shortcuts, paste) is swallowed and never reaches the grid, the graph or the
  shortcut table; only the Ctrl state is read to detect the release.
- **macOS:** Ctrl is the physical Control key (Cmd+Tab belongs to the system); the Window menu
  shows the shortcut as the Control symbol plus Tab. On Windows and Linux it is Ctrl+Tab.
- **Windows listed:** all windows of the Project Explorer's selected folder (not subfolders),
  hidden ones marked "(hidden)" and drawn paler. Sheaf has no floating windows. With 0 or 1
  window Ctrl+Tab does nothing.
- **Menu:** Window > Window Switcher opens it held open (no Ctrl to release): Tab / arrows move,
  Enter or a click chooses, Esc closes.
- **Look:** a dimmed backdrop and a centred panel of 168 x 134 pt cards (up to six per row),
  each with a thumbnail, the window name and "Graph" or "Workbook"; the selected card has a 3 pt
  blue frame. Graphs are rendered when the switcher opens at 148 x 98 pt (their own
  `render_raster`, not the window cache, which is sized to the window and absent for closed
  graphs) and the textures are dropped on close; at most 24 graphs per opening, the rest show the
  type icon. Workbooks show a drawn miniature of the first four columns and five rows.
  Nothing is redrawn per frame. `--show switcher` opens it for screenshots.

## 6. Mouse and keyboard summary

- Pointer tool is the default; every other tool (zoom, data reader, text...) stays active until
  the pointer is chosen or Esc is pressed [spec C, not tested].
- Double-click: axis -> axis dialog; plot -> Plot Details; legend/text -> edit; layer icon ->
  Plot Setup [seen 2]; column header -> column properties; cell -> edit; sheet tab -> rename; window
  title -> maximise [spec / seen where noted above].
- Right-click gives context menus per object (sections 3 and 4). Hovering the status bar shows hints.
- Drag a column from a workbook onto a graph adds it as a plot; drag column headers inside a
  workbook to reorder; drag the sheet tab to reorder sheets; drag window title to move; drag a
  file from Explorer into the workspace to import it [spec, not individually tested].
- Keyboard shortcuts: see `ui-command-map.md` section E. Seen in menus: Ctrl+R (rescale to show
  all in graph / merge cells in workbook Format menu), Ctrl+J (copy page), Ctrl+Shift+J (full
  screen), Ctrl+U (shown for Options, but in a worksheet it toggles underline, see section 3), Ctrl+O,
  Ctrl+S, Ctrl+P, Ctrl+E, Ctrl+3 (Import Wizard), Ctrl+4 (Re-Import Directly), Ctrl+Y (Nonlinear
  Curve Fit), Ctrl+Q (Set Column Values), Ctrl+G (Go to in a workbook, Export Graphs in a
  graph), Ctrl+T (Toolbars), Alt+1 to Alt+9 for panels, F3, F4, F5 (Refresh), F7, F8, F12
  (Layer Contents) [seen, seen 2].

## 7. Differences between Origin's interface and Sheaf's present app

Updated 2026-11-12 after the ADR 0007 work (the first version of this table, from the study, described
the old tabbed app and is in git history). Element by element gaps are listed in
`docs/audit/origin-differences-2026-11-12.md` sections A and B; this table keeps only the big picture.

| Origin | Sheaf now |
|---|---|
| Child windows floating in a grey workspace, cascade, own title bars with minimise/maximise/close | Same (Window > Cascade, Tile). A maximised child still draws its own title bar; Origin moves its buttons into the menu-bar row |
| Project Explorer at the left edge (folder tree over window list with Name, Long Name, Short Name, Type) | Same columns and folder tree (`panels/explorer.rs`); not auto-hide; folder Rename/Delete and keys F2, Del, Ctrl+A missing |
| Menu bar changes per window type with about 12 menus | Same names and order per window type; items without a feature are drawn greyed |
| Two rows of toolbars, a Tools toolbar at the left, 2D Graphs toolbar at the bottom, Object Edit at the right | Standard, Edit, Format (all greyed), Graph, 2D Graphs and Tools bars with fewer buttons; no separate Column, Worksheet Data, Style and other bars |
| Column label rows Long Name, Units, Comments, F(x)= (and Sparklines) over the data; label "A(X1)" | Long Name, Units, Comments and F(x)= rows; "A(X1)" with group numbers; values with 5 decimals (General). Sparklines and Parameters rows missing; label rows cannot be hidden |
| Status bar with data statistics, AU, angle unit, active object | Average/Sum/Count, AU : ON, reference and Radian boxes; reference lacks Origin's `1: ` prefix and `col[r1:r2]` range |
| Plot Details dialog with layer/plot tree on the left, tabs; axis dialog with Horizontal/Vertical list | Same layout with OK/Cancel/Apply; several tabs and controls missing (audit B) |
| Light, small, flat Windows controls (9 pt Segoe UI), grey workspace | Light theme, Segoe UI from the Windows font folder when present, grey workspace |


## 8. Unknowns left after the second study

1. Long form of the row-number right-click menu, the right-click menu of the title text object
   of a graph, and the Import submenu of the graph File menu (assumed equal to the workbook's, not opened).
2. The pages after the first one of the Import Wizard and of the Peak Analyzer; the Code and
   Parameters tabs of NLFit and what the Function drop-down offers after a Category is chosen
   (the drop-down did not open from my click); the "Dialog Theme" arrow menu.
3. Whether a "+" button for new sheets exists, the Column, Worksheet, Plot, Statistics and Format
   menus item by item (first-level names are known from the first study and the spec), the
   Object Manager's place in the View menu, and the Miscellaneous tab of Options with the
   "Enable Collapsible" box (only quoted from the hint note).
4. Exact RGB values (measured by eye from pictures, not sampled).
5. How windows behave when the main window is resized, and the exact docking behaviour of the
   Object Manager and of the Results Log.
6. Dark mode: Origin has none; Sheaf can offer it as an extra.
7. What decides which tab the Options dialog opens on (it showed Numeric Format every time).

What the second study settled: the column header, cell, sheet tab, legend and layer icon menus;
a maximised and a minimised child window; the File, Edit, View, Window and Help menus of the
workbook and the File and Window menus of the graph; NLFit, Import Wizard, Peak Analyzer first
page; Plot Details of a line and of a column plot; every tab of the axis dialog; Tools > Options
with Numeric Format and Text Fonts; Set Column Values.

### Slide Show of Dependent Graphs (T129, 2026-11-18, not measured)

The worksheet column header's right-click menu has Slide Show of Dependent Graphs (Origin 2016). It could not
be probed (Origin's COM class was unregistered), so this follows Origin's documented behaviour.

- **Which graphs:** the graphs that plot any of the selected columns (as X, Y, error or any other
  referenced column), in window order. With none, the status bar warns and nothing happens.
- **The show:** each graph's page over the whole screen, as View > Full Screen draws it (T66), starting at
  the first, without Full Screen's reminder message.
- **Keys:**
  - Space, Page Down, Right or a click: the next graph.
  - Page Up or Left: the previous one.
  - Home and End: the ends. The last graph stays on a further step.
  - Esc ends the show.
- **Sheaf internals:** `view_modes::dependent_graphs`, `FullScreen::slide_show`, `next_slide`.

### Favorites (T130, 2026-11-18, not measured)

Origin 2016's graph page right-click menu has Add Shortcut to Favorites, and its Project Explorer has a
Favorites folder. Neither could be probed (Origin's COM class was unregistered), so this follows Origin's
documented behaviour.

- **Adding a shortcut:** Add Shortcut to Favorites (graph page menu, and the Project Explorer's window
  menu) adds a shortcut to the window. Each window has at most one. It is one undo step, saved with the
  project (`Project::favorites`).
- **The Favorites folder:** it sits under the project's folders in the explorer tree. Selecting it lists
  the shortcut windows with the usual columns and window menu. That menu says Remove from Favorites
  instead of Add Shortcut to Favorites.
- **The windows stay put:** a window stays in its own folder. Deleting it removes its shortcut.

