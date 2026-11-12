# How Origin 2016 looks and behaves: the interface, described for Sheaf

Status: written from pictures of the installed Origin 2016 (OriginPro 2016 64-bit, version 9.3)
taken on 2026-11-11 and 2026-11-12, plus the existing specs. The pictures show OriginLab's
interface, so they live only in the git-ignored folder
`re-workspace/scratch/gui-study/` (file names are quoted below so a later reader can find them).
This text is our own description; no OriginLab help text, icons or pictures are copied.

Tags: **[seen]** means seen directly in one of those pictures. **[spec]** means taken from an
existing spec (mostly `ui-command-map.md`, `feature-inventory.md`, `graphing.md`). **[not seen]**
marks something the pictures did not show; it is listed so nobody mistakes it for verified.
Sizes are in screen pixels of a 1600 x 1000 capture of the main window on a Windows 11 screen at
100 % scaling; they are good to a few pixels.

How the pictures were made: `tools/origin-ref/origin-screenshot.ps1` started a temporary visible
Origin, ran our own LabTalk setup (`gui-setup2.labtalk.txt`: one workbook with four number
columns and a text column, a line graph and a column graph made from it), sent keys or mouse
clicks and saved the window. Nothing was saved in Origin and the instance was closed afterwards.
Known weakness: the Windows desktop behind and beside the window (other programs) appears in
some pictures because the capture copies screen pixels; ignore those parts. Origin's own File
menu picture `wb-file.png` was spoiled by another program's menu opening on top of it, so File
menu contents come from the spec only.

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
   end with a small "double chevron" row that expands the rest [seen in Graph, Tools, Help and
   right-click menus]; there is also a "Most Recently Used Items" entry at the end of Analysis,
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
- Graph default text is Arial; axis titles 18 pt in a default graph (the picture is small, so
  that value is [spec graphing.md], not measured here).

## 2. Child windows

Pictures: `gr-plain.png`, `grc-page.png`, `rc-titlebar.png`.

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
  Book1 at about (85,130) and a size of about 395 x 320 px, and Graph1 at (110,157) sized 505 x 345 [seen].
- Minimise shrinks the window to a small title-bar icon at the bottom-left of the workspace [spec F,
  [not seen]]. Maximise fills the workspace and moves the title buttons to the right end of the menu
  bar row [not seen; standard Windows MDI behaviour, [spec] Window menu].
- **Window menu** (same for all window types) [seen as a top-level item; contents spec A1]:
  Cascade, Tile Horizontally, Tile Vertically, Arrange Icons, Duplicate, Refresh, then a numbered
  list of the open windows with a tick on the active one [spec A1].
- Dragging the title bar moves the window; dragging the edge or the bottom-right grip resizes it;
  windows can be dragged partly outside the workspace [standard MDI, not individually checked].
- Window types in the Project Explorer list: Workbook (grid icon), Graph, Matrix, Layout, Notes,
  Function, Excel [spec F].
- Closing a child window does not close the project; closing a workbook that graphs depend on
  asks first, and "Hide" is offered instead of delete [spec F, not seen].

## 3. Workbook window

Pictures: `gr-plain.png` area, `wb-file.png`, `rc-*.png`, `wb-m04-column.png`.

- The window top is a grid with a **column header band** and the **column label rows** [seen]:
  - First row (grey, centred): the column label "A(X1)", "B(Y1)", "C(Y1)", "D(Y1)". The letter is
    the column name, the type in brackets is X, Y (also Z, yEr, L for label, M for disregard)
    followed by the group number of the data set (X1, Y1) [seen]. The text column
    shows as "E(L)" [spec; its column was scrolled off in the picture].
  - Then four label rows in the order **Long Name, Units, Comments, F(x)=**, in light yellow cells,
    left of which a grey cell shows the row title right-aligned ("Long Name", "Units",
    "Comments", "F(x)=") [seen]. Origin 2016 also has **Sparklines**, **Parameters** and user
    defined label rows, switched on in View > Show / the right-click menu "Row Labels for
    Column Label Rows" [spec; the menu `colhdr.png` shows exactly this list of options, see
    below]. Sparklines were not turned on in the picture [not seen].
  - Then the data rows with a grey **row header** (numbers 1, 2, 3, right-aligned, about 85 px wide
    together with the label titles). Numbers right-aligned, text left-aligned [seen]. Empty
    cells below the last value stay blank with the row header numbers continuing (11 is shown
    for the first empty row) [seen].
- Column width about 67 px (default, 8 characters), row height 16 px, header rows 16 px [seen].
  Cells have thin grey gridlines in both directions (View options "Column Grid", "Row Grid" tick
  boxes in the menu) [seen].
- Number formats: up to about 5 significant decimal digits are shown by default (0.84147, 0.9093,
  -0.7568) [seen]; the separators and digits come from Tools > Options > Numeric Format, which
  has "Digits: Decimal Digits, Number of Digits 5", "Convert to Scientific Notation" thresholds
  (upper 6, lower -3), Angular Unit Radian [seen, `dlg-options.png`].
- **Sheet tabs** at the bottom-left, inside the window: scroll arrows, then tab "Sheet1" with a
  slanted right edge, then a horizontal scroll bar next to it [seen]. A "+" to add sheets sits
  after the last tab [not seen at this size; spec says New Sheet button]. Vertical scroll bar at the
  right edge; bottom-right a small resize grip [seen].
- The selected column is shown with a **black selection** over the column's cells and the
  header turned dark (`dlg-nlfit.png`: B(Y1) selected, cells black with white text) [seen].
- **Right-click, column header cell** [seen, partial]: in my run a right-click at B(Y1) opened a
  small menu with tick items "Row Labels for Column Label Rows" (sub-list), "Column Header",
  "Row Header", "Column Grid", "Row Grid", "Page Break Preview Lines" (`fresh/colhdr.png`). That is the
  menu of the corner/label area, so the real column-header menu (Cut, Copy, Paste, Clear, Set as
  X/Y, Set Column Values, Properties, Plot ...) was **not captured** [spec D5, F]. A repeat with a
  click inside the label letter row is needed.
- **Right-click, cell and row number and sheet tab** pictures exist (`rc-cell.png`,
  `rc-rowheader.png`, `rc-sheettab.png`) but I did not review them in detail; the item lists come
  from [spec F].
- **Mouse**: click on a column letter selects the column; Shift or Ctrl click extends; click on a
  row number selects the row; click on the corner selects all; double-click the column header
  opens Column Properties ("Properties", "Plot Setup" and so on) [spec D5]; double-click a cell edits it,
  Enter commits; double-click a label cell (Long Name, Units, Comments) edits that label in place [spec];
  dragging the header border resizes the column [spec]. Drag and drop of columns onto a graph
  adds them as a plot (the "Set Column Values" helper note shows "drag to the layer" text) [spec; hint
  yellow note seen in `m4.png`].
- **Set Column Values dialog** [seen, `m4.png`]: title "Set Values - [Book1]Sheet1!Col("Signal")",
  menu line "Formula, wcol(1), Col(A), Function, Variables, Options", a row "Row (i): From <auto>
  To <auto>", a label "Col(B)=" with a multi-line formula box (grey hint text), buttons
  Recalculate drop-down ("Manual"), a small icon button, OK, Cancel, Apply, a collapse arrow, and a
  second box "Before Formula Scripts" [seen]. Left of it a yellow hint balloon appears first time [seen].
  Dialog is a normal small modeless-looking window with a close X and a disabled maximise [seen].

## 4. Graph window

Pictures: `gr-plain.png`, `gr-graph.png`, `grc-*.png`, `gdlg-*.png`.

- Inside the child window the **page** is white with a thin border; where the page does not fill
  the window the rest is mid grey; the page is shown at its true aspect ratio and scaled to the
  window ("fit page to window"). Left of the page there is a column of small boxes
  (ruler-like gutter) [seen].
- **Layer icons**: small square number buttons "1" and "2" at the top-left of the page, one per
  layer; the active layer is drawn with a dark/black number on a darker button, the others with
  grey numbers [seen]. Click an icon to make that layer active; double-click opens Plot Details
  for it; right-click gives the layer menu [spec]. In the picture of the column graph the second
  layer holds the columns.
- Default look of a graph made by `plotxy`: black axes with ticks pointing out, left Y and bottom
  X only, axis titles from long name and units (left "Signal (mA)", bottom "Conc (mM)"), a legend
  box at the top right inside the layer with the plot names, and column plots with the first
  plot black, the second red [seen]. Our figure rendering spec already covers the details [spec].
- Toolbars: the graph window enables the **Graph** toolbar (zoom, rescale, exchange axes, layers),
  the **2D Graphs** toolbar (bottom), the **Object Edit** toolbar (right), and the **Tools**
  toolbar (left) [seen, spec C].
- **Graph menu** [seen, `gr-graph.png`]: Add Plot to Layer (submenu), Layer Management..., Add
  Straight Line..., New Layer(Axes) (submenu), Merge Graph Windows..., Legend (submenu), Fit Page to
  Layers..., Exchange X-Y Axes, then a chevron to show more [seen]. (Fuller list: spec A3.)
- **Other top-level graph menus** were photographed (`gr-file.png`, `gr-edit.png`, `gr-view.png`,
  `gr-data.png`, `gr-analysis.png`, `gr-gadgets.png`, `gr-tools.png`, `gr-format.png`, `gr-window.png`,
  `gr-help.png`); I confirmed that they open and that the first-level items match [spec A1, A3];
  I did not transcribe each from the pictures.
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
- **Right-click, legend, layer icon, title** pictures exist (`grc-legend.png`, `grc-layer.png`,
  `grc-title.png`) but were not reviewed in detail; menu lists come from [spec F].
- **Double-click** [seen as dialogs]:
  - on an axis: the dialog "Y Axis - Layer 1" (`gdlg-axis.png`): a left list "Horizontal /
    Vertical" with icons (current one selected in blue), tabs **Scale, Tick Labels, Title, Grids,
    Line and Ticks, Special Ticks, Breaks**; Scale tab fields From, To, Type (Linear...), Rescale
    (Normal), Rescale Margin(%), Reverse tick box, a "Major Ticks" group (Type "By Increment",
    Value, First Tick) and a "Minor Ticks" group (Type "By Counts", Count); bottom "Select multiple
    axes to customize together." and buttons Apply To..., OK, Cancel, Apply [seen]. The
    dialog opened with the "From" box focused and its text selected.
  - on a plot: "Plot Details - Plot Properties" (`gdlg-plot.png`): left tree Graph1 > Layer1 /
    Layer2 > each plot (with tick boxes), right side tabs (for a column plot: **Group, Pattern,
    Spacing, Label**), bottom "Plot Type" drop-down ("Column / Bar"), buttons ">>", Workbook, OK,
    Cancel, Apply [seen]. The Group tab shows "Edit Mode: Independent / Dependent", an increment
    table (Border Color, Fill Color, Border Type, Fill Pattern each with "By One" and a list of
    colours), "Group Members" with colour samples and a Subgrouping box [seen].
  - on the legend, text, layer icon, graph page: opens the matching Plot Details page / text
    editor [spec D1, D3].
- Colour list of the default palette ("By One"): black, red, green, blue... [spec graphing.md].

## 5. Dialogs, as seen

All dialogs are ordinary Windows dialogs with a title bar (text left, "?" help button and
close X on the right for the larger ones), tabs or a left list, and buttons in the bottom right,
in the order **OK, Cancel, Apply** (Plot Details has "Workbook" and ">>" in front) [seen].

- **Tools > Options** (`dlg-options.png`): 12 tabs in two rows: Miscellaneous, Excel, Open/Close,
  System Path, Numeric Format, File Locations, Axis, Graph, Text Fonts, Page [seen]. Buttons: Restore,
  Reset, OK, Cancel. The open tab is Numeric Format with the groups described in section 3.
- **Import**: File > Import > Single ASCII opens a standard Windows file chooser titled "ASCII",
  starting in the Origin Samples folder, with file type "*.dat", a tick box "Show Options Dialog:" and
  "Open as read-only" [seen, `dlg-ascii.png`]. The options dialog is only shown when that box is
  ticked or when the file is imported from the Import Wizard [spec B2]. The Import Wizard itself was
  [not seen].
- **Export Graphs** (`dlg-export.png`): title "Export Graphs: expGraph", top "Dialog Theme" box and
  a "Description" line, a form with Image Type (a drop-down, default shown "Encapsulated Postscript
  (*.eps)"), Select Graph(s) ("Active Page"), File Name(s) ("<long name>") with an "Enable
  Substitution" tick, a line "File will be saved as Graph1", Path, Overwrite Existing ("Ask"),
  Graph Theme ("<Original>"), then collapsible groups "Export Settings", "Image Size", "Image
  Settings"; a preview pane at the right with tabs "Image" and "Graph" and the text "No Preview";
  bottom row: "Auto Preview" tick, Preview, OK, Cancel, Apply, and a "<<" to hide the preview [seen].
- **Fitting Function Organizer** (opened by Tools > Fitting Function Builder, F8, `dlg-nlfit.png`,
  not the NLFit dialog itself): a category tree (Origin Basic Functions, Implicit, Convolution,
  Exponential, Growth/Sigmoidal, Hyperbola, Logarithm, Peak Functions, Piecewise, Polynomial,
  Power, Rational, Waveform, Surface Fitting, PFW, Baseline, Chromatography, Electrophysiology,
  Enzyme Kinetics, Pharmacology, Rheology, Spectroscopy, Statistics), buttons Save, New Category,
  New Function, Builder..., Duplicate, Add, Remove, Cancel, OK, Simulate, Reset, Search, and tabs
  Equation, Sample Curve, Hints [seen]. The real **NLFit dialog** (Analysis > Fitting > Nonlinear
  Curve Fit) was [not seen]; its structure is in [spec D7, fitting.md].
- **Analysis menu** [seen, `wb-analysis.png`]: Mathematics, Data Manipulation, Fitting, Signal
  Processing, Peaks and Baseline (each a submenu), then "Most Recently Used Items". **Statistics**:
  Descriptive Statistics, Hypothesis Testing, ANOVA, Nonparametric Tests, Survival Analysis,
  Multivariate Analysis, Power and Sample Size, ROC Curve... **Image**: Adjustments, Arithmetic
  Transform, Conversion, Geometric Transform, Spatial Filters. **Tools**: Options... (Ctrl+U),
  Apps on File Exchange, Fitting Function Builder... (F8), Theme Organizer... (F7), Virtual Matrix
  Manager..., Transfer User Files..., Digitizer..., Video Builder..., R Console..., Rserve Console...,
  Python Console... **Format**: Menu, Worksheet... (F4), Column..., Cell..., Snap to Grid, Merge cells
  (Ctrl+R), Programming Control..., Object Properties... **Help**: Use Online Help (tick), Origin,
  Programming, Open Sample Projects, Online Resources (submenus), Check for Updates..., Deactivate
  License..., About Origin... [seen].
- **Plot menu of a workbook** [seen, `wb-m04-column.png` shows the Plot menu]: Line, Symbol, Line +
  Symbol, Column/Bar/Pie, Multi-Y, Y-offset/Waterfall, Multi-Panel, 3D XYY, 3D Surface, 3D
  Symbol/Bar/Vector, Statistics, Area, Contour/Heat Map, Profile, Specialized, Stock, System
  Templates..., User Templates, Most Recently Used (all with arrows except the templates dialog)
  [seen]. Those under "Line" and so on open galleries [spec].

## 6. Mouse and keyboard summary

- Pointer tool is the default; every other tool (zoom, data reader, text...) stays active until
  the pointer is chosen or Esc is pressed [spec C, not tested].
- Double-click: axis -> axis dialog; plot -> Plot Details; legend/text -> edit; layer icon ->
  layer properties; column header -> column properties; cell -> edit; sheet tab -> rename; window
  title -> maximise [spec / seen where noted above].
- Right-click gives context menus per object (sections 3 and 4). Hovering the status bar shows hints.
- Drag a column from a workbook onto a graph adds it as a plot; drag column headers inside a
  workbook to reorder; drag the sheet tab to reorder sheets; drag window title to move; drag a
  file from Explorer into the workspace to import it [spec, not individually tested].
- Keyboard shortcuts: see `ui-command-map.md` section E. Seen in menus: Ctrl+R (rescale to show
  all in graph / merge cells in workbook Format menu), Ctrl+J (copy page), Ctrl+Shift+J (full
  screen), Ctrl+U (Options), F4, F7, F8 [seen]; Ctrl+N shown by *Claude's own* File menu in a
  spoiled picture is not an Origin item.

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


## 8. Unknowns left from this study

1. Right-click menu of a worksheet column header, of a cell, of a layer icon, legend, title:
   pictures exist (`rc-*.png`, `grc-*.png`) but the column header one did not open the intended menu.
2. Maximised and minimised child window look; Window menu contents as seen; File menu as seen.
3. Real NLFit dialog, Import Wizard, Peak Analyzer wizard.
4. Exact RGB values (measured by eye from compressed pictures, not sampled).
5. How windows behave when the main window is resized, and the exact docking behaviour of the
   Object Manager and of the Results Log.
6. Dark mode: Origin 2016 has none; Sheaf can offer it as an extra.
