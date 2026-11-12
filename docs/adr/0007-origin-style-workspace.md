# ADR 0007: An Origin-style workspace in egui

Status: accepted (2026-11-12). Replaces the tab-and-dock layout chosen in ADR 0003 for the main
window. Description of the target look: `docs/spec/gui-look.md`.

## Context

The owner wants Sheaf to look and work like Origin 2016 so that their skills carry over. The
study (pictures in the git-ignored scratch folder, text in `gui-look.md`) shows an MDI program: one
main window with menus, toolbars, auto-hide panels and a grey workspace in which workbooks and
graphs are floating child windows. Sheaf today uses `egui_dock` tabs, one menu bar for everything
and a permanent project list (ADR 0003).

Constraints: clean room (no OriginLab icons, bitmaps, help text or templates); egui 0.33; the
document model in `crates/core` and the logic crates must not change for look-and-feel work; the
app must stay testable with `egui_kittest` and `--screenshot`.

## Decision

1. **Child windows.** Each workbook and graph is an `egui::Window` constrained to the central
   area (`constrain_to` the workspace rectangle). The central panel is filled with the Origin
   workspace grey (about RGB 171,171,171). A new type `Workspace` owns, per `WindowId`: position,
   size, state (normal, minimised, maximised) and a z-order list. We draw the title bar ourselves
   (`title_bar(false)` plus a custom row) because egui's window bar has no minimise or maximise
   buttons: icon, name with `*` when changed, and three buttons. Active window: blue frame and red
   close button; inactive: grey. Maximised: the window fills the workspace. Minimised: a small bar at
   the bottom left of the workspace. New windows cascade by about 26 px. Window > Cascade, Tile
   Horizontally, Tile Vertically and the numbered window list act on `Workspace`. `egui_dock` is
   removed from the app crate when this lands. Window geometry is saved in the project file under a
   new optional key so that reopening restores the arrangement (this needs a change to
   `docs/spec/project-file-format.md` and ADR 0004 first; absent geometry means cascade).
2. **Menus follow the active window.** `menu_bar` takes the active window type (none, workbook,
   graph) and builds the matching list from section 1 of the spec (workbook: File, Edit, View,
   Plot, Column, Worksheet, Analysis, Statistics, Image, Tools, Format, Window, Help; graph: File, Edit,
   View, Graph, Data, Analysis, Gadgets, Tools, Format, Window, Help; none: File, View, Tools,
   Window, Help). Items for features Sheaf does not have yet are shown greyed with their Origin
   name, so a user sees where the command lives. A menu-tree table in code (`menus.rs`) is the
   single source and a unit test checks its top-level names against `ui-command-map.md` section
   A. Mnemonic letters (Alt+F and so on) are drawn underlined. We show all items of a long menu
   instead of Origin's "more" chevron (a deliberate difference).
3. **Toolbars.** A `toolbar.rs` module draws docked rows of small flat icon buttons with
   tooltips: Standard, Edit/Format, Graph, 2D Graphs (bottom), Tools (left, vertical), Object Edit
   (right, vertical). Each button is `{icon, tooltip, Action, enabled-for-window-type}`; a click
   sends the same `Action` the menu sends. Icons are our own simple vector shapes drawn with
   `egui::Painter` (a grid for workbook, a polyline for Line, dots for Scatter, bars for Column,
   and so on), 16 px, two colours. View > Toolbars toggles each.
4. **Panels.** The left edge gets an auto-hide strip with vertical tab names (Project Explorer,
   Quick Help, Messages Log, Smart Hint Log); the right edge gets "Object Manager". Project
   Explorer has a folder tree above and a window list (Name, Long Name, Short Name, Type) below,
   the active window in bold; it slides over the workspace on hover and can be pinned. Folders
   map to the folders of `crates/core` if the model has them (to be checked; otherwise one root
   folder is shown). Messages Log shows the status messages the status bar shows now. The Object
   Manager lists layers and plots of the active graph with tick boxes.
5. **Workbook look.** The grid draws the label band like Origin: a column-letter row "A(X1)"
   with the group number, then Long Name, Units, Comments, F(x)= (later Sparklines) rows with a
   light yellow tint and grey titles at the left, a grey row header, thin gridlines, and sheet
   tabs at the bottom with a horizontal scroll bar. Default number display is 5 decimal digits as
   in Tools > Options > Numeric Format.
6. **Graph look.** The graph window shows the page on a grey ground, layer number buttons at the
   top left, and double-click on axis, plot, legend or layer icon opens our existing dialogs
   restyled to Origin's layout (axis dialog: Horizontal/Vertical list and tabs Scale, Tick
   Labels, Title, Grids, Line and Ticks, Special Ticks, Breaks; Plot Details: left tree, right
   tabs). Right-click menus reproduce the lists in the spec.
7. **Theme.** A `theme.rs` sets egui `Visuals` and `Style` once: light only, 12 px UI text, square
   corners, one-pixel borders, flat buttons, small spacing, tick boxes and radio buttons close to
   Windows. The font is egui's built-in one unless Segoe UI is found in the Windows font folder at
   run time (we do not ship it). Dark mode is not offered at first.
8. **Dialogs** use `egui::Window` with the Windows button order OK, Cancel, Apply at the bottom
   right and, for the large ones, a "?" button.
9. **Saving as .opj.** The owner also wants exported projects to be `.opj`. That is not a look
   change and is separate: a writer in `crates/io/src/opj/` built from `docs/spec/opj-format.md`
   and checked by reading the result back with our reader and by opening it in Origin 2016 through
   COM and comparing cells. The owner decided (2026-11-11) that `.opj` is the default format of
   File > Save Project, Save Project As and the command line's `--save`. While the writer cannot
   store everything a project holds (graphs at first), saving also writes a `.sheaf` copy next to
   the `.opj` and says what only the copy holds; the copy is dropped once nothing would be lost.
   `.sheaf` stays available as a lossless format.

## What stays different, and why

- **Icons and artwork** are ours. They will be recognisable by meaning and position, not equal in
  pixels. OriginLab's icons are copyrighted and the ground rules forbid them.
- **Text**: menu and dialog names are used (short names are allowed); help text, sample data and
  templates are not.
- **Features Sheaf lacks** appear as greyed items so the layout matches. "All of Origin's
  features" cannot be reached in this project: Origin has hundreds (3D graphs, Origin C, LabTalk,
  Apps, matrix and image tools, database access). `feature-inventory.md` sets the real scope with
  P0/P1/P2 tags. The honest statement to the owner: their own workflows first, then the rest by
  priority.
- The "Apps" panel, online help and licence items are not reproduced.
- egui is immediate mode: text rendering, hover colours and control metrics will differ from
  native Windows controls by a few pixels.
- Windows keep a minimum size and cannot leave the workspace entirely.

## Risks

1. **egui window behaviour.** `egui::Window` has no minimise or maximise and no z-order API beyond
   layer ids and `move_to_top`; resizing next to scroll areas is touchy. Mitigation: build and
   test `Workspace` first with a dummy window type and keep window state in our own struct.
2. **Large worksheets in windows.** The virtualised grid must keep its speed in a resizable
   window; test with 100 000 rows.
3. **Test churn.** Existing `ui_tests.rs` click tests use tab labels such as "Graph1 - d". Titles
   and menus change, so tests are updated in the same commit as each UI step.
4. **Scope creep.** Origin's toolbars have hundreds of buttons. Only buttons for existing
   features are enabled; no feature is added just to fill a button without a spec.
5. **Clean room.** Pictures are used for layout notes only; no pixel is traced. Icons are drawn
   from scratch. Measured colours are approximate.
6. **.opj writing** is the largest risk: the format is only partly specified, Origin may reject
   deviating files, and a bad file can lose data. The first writer writes only what was verified by
   reading it back in Origin, warns about anything dropped and never overwrites an existing file
   without asking.
7. **Measurements** came from compressed pictures at one DPI. Check at 125 % and 150 % scaling.
