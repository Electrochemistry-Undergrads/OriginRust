# How Origin 2016 looks and behaves: the interface, described for Sheaf

Status: written from pictures of the installed Origin 2016 (OriginPro 2016 64-bit, version 9.3)
taken on 2026-11-11 and 2026-11-12 (first study) and on 2026-11-12 (second study, which closed
the gaps of the first), plus the existing specs. The pictures show OriginLab's interface, so they
live only in the git-ignored folders `re-workspace/scratch/gui-study/` (first study) and
`re-workspace/scratch/gui-study2/` (second study); file names are quoted below so a later reader
can find them. This text is our own description; no OriginLab help text, icons or pictures are
copied. Menu and dialog names are quoted because they are names, not help text.

Tags: **[seen]** means seen directly in one of those pictures (first study) and **[seen 2]**
means seen in a picture of the second study (look in `gui-study2/`). **[spec]** means taken from
an existing spec (mostly `ui-command-map.md`, `feature-inventory.md`, `graphing.md`). **[not
seen]** marks something the pictures did not show; it is listed so nobody mistakes it for
verified. Sizes are in screen pixels of the main window on a Windows 11 screen at 100 % scaling
(the window placed at the top-left corner, 1600 x 1000, on a 1920 x 1080 screen); they are good
to a few pixels.

How the pictures were made: `tools/origin-ref/origin-screenshot.ps1` started a temporary visible
Origin, ran our own LabTalk setup, sent keys or mouse clicks and saved the screen. The first
study used `gui-setup2.labtalk.txt` (one workbook with four number columns and a text column, a
line graph and a column graph made from it). The second study used `gui-setup4.labtalk.txt`
(a workbook "Book1" with four number columns A to D plus a column named "Label", a line graph
"Graph1" with two plots and a column graph "Graph2" with two plots). The second study drove
Origin with a step file (`-StepsFile`) that holds one step per line; mouse clicks, key presses
and small LabTalk lines (to pick a window) can be mixed in a step. The first study's step list
was cut after its first step when passed with commas through `powershell -File`; that is fixed
in the script by `-StepsFile`. Nothing was saved in Origin and the instance was closed
afterwards. A check run showed that the Options dialog still opens on its Numeric Format tab
after an earlier run had paged to another tab, so no Origin setting of the owner was changed
by looking at dialogs [seen 2, `opts-check.png`].

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
    "Comments", "F(x)=") [seen]. Origin 2016 also has **Sparklines**, **Parameters** and user
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
  Add Color Scale (grey), Add Bubble Scale (grey), | Cut (Ctrl+X), Copy (Ctrl+C), Delete
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

Pictures of Sheaf: `re-workspace/scratch/gui-study/sheaf/sheaf-none.png` and the other
`sheaf-*.png` files (made with `cargo run -p sheaf-app -- d.csv --show ITEM --screenshot FILE`).

| Origin 2016 | Sheaf now |
|---|---|
| Child windows floating in a grey workspace, cascade, own title bars with minimise/maximise/close | Tabs ("Book1 - d", "Graph1 - d" with an x) in `egui_dock` filling the whole centre |
| Project Explorer is an auto-hide panel at the left edge (folder tree over window list with Name, Long Name, Short Name, Type) | Always-open left panel "Project" with a flat list of windows |
| Menu bar changes per window type with about 12 menus | One bar: File, Edit, Worksheet, Plot, Analysis, Graph, Window, Help |
| Two rows of toolbars, a Tools toolbar at the left, 2D Graphs toolbar at the bottom, Object Edit at the right | A small button row only inside the graph (Axes..., Plots..., Legend..., Page..., Export...) |
| Column label rows Long Name, Units, Comments, F(x)= (and Sparklines) over the data; label "A(X1)" | Seen in `sheaf-book.png`: label block Long Name / Units / Comments with no F(x)= or Sparklines row, label text above the data instead of in a titled grey row header, column names "A(X)" without the group number, sheet tab shown as a blue chip with the book name, values shown with 2 decimals |
| Status bar with data statistics, AU, angle unit, active object | One line message and "unsaved changes" |
| Plot Details dialog with layer/plot tree on the left, tabs; axis dialog with Horizontal/Vertical list | `egui::Window` dialogs with different layout (`sheaf-graph-settings_y.png`) |
| Light, small, flat Windows controls (9 pt Segoe UI), grey workspace | egui default dark-grey-on-white theme at a larger size |


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
6. Dark mode: Origin 2016 has none; Sheaf can offer it as an extra.
7. What decides which tab the Options dialog opens on (it showed Numeric Format every time).

What the second study settled: the column header, cell, sheet tab, legend and layer icon menus;
a maximised and a minimised child window; the File, Edit, View, Window and Help menus of the
workbook and the File and Window menus of the graph; NLFit, Import Wizard, Peak Analyzer first
page; Plot Details of a line and of a column plot; every tab of the axis dialog; Tools > Options
with Numeric Format and Text Fonts; Set Column Values.
