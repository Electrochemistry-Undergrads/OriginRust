# Layout pages (T237, 2026-11-20, not measured)

A layout page is a window that arranges live pictures of the project's graphs on one page for
print, as Origin's File > New > Layout does. Origin's layout page was not measured (its COM class was
unregistered on the reference machine). Everything below is Sheaf's choice unless a source is named.

## Model (`crates/core/src/layout_page.rs`)

- `Window::Layout(LayoutPage)`: an id, a short name (`Layout1`, `Layout2`, ...), a long name, the
  page size in points (US Letter landscape, 792 x 612, by default), and items.
- An item (`LayoutItem`) is a rectangle in fractions of the page (0,0 at the top left) with its
  content. Slice 1 has one kind, `LayoutContent::Graph { graph }`: a live picture of a graph
  window that follows the graph's changes.
- Renaming, duplicating and appending projects work as for other windows. Appending renumbers the
  graphs an item names. The page is saved in `.sheaf` files (inside `project.json`); `.opj` leaves it
  out with a warning.

## App

- File > New > Layout and the Standard toolbar's New Layout make an empty page as one undo step
  ("New layout"). The new page becomes the active window.
- A layout window has its own menu bar: File, View, Layout, Tools, Window, Help.
- Layout > Add Graph... lists the project's graphs. OK puts the chosen graph's picture in the next
  cell of a two-column grid. The picture keeps the graph page's shape within the cell and is centred
  there. This is one undo step ("Add graph to layout"); rows past the second wrap onto the same two
  rows.
- The window draws the page on a grey ground, scaled to fit, with each graph rendered by
  `sheaf-plot` at the size it takes on screen. A deleted graph leaves an outlined empty box.

## Editing items (T237 slice 2)

- A click on an item selects it: a blue frame and a square handle at the bottom right corner. A
  click on empty page clears the selection; the topmost item under the pointer wins.
- Dragging an item moves it. Dragging the corner handle of the selected item resizes it, keeping
  the top left corner, to at least 2 % of the page each way. The picture is redrawn at the new size,
  so a resized graph picture can change its shape. The change is made when the button is released,
  as one undo step ("Move layout item", "Resize layout item").
- Delete removes the selected item ("Delete layout item") while no text box has the keyboard.

## Export (T237 slice 3)

- Layout > Export Page as Image... asks for a file and writes the whole page at 300 dpi as PNG,
  JPEG, TIFF or BMP, by the file's extension (`.png` when there is none). The page is white; each
  graph is drawn at the pixel size it takes on the page and laid at its place, so a stretched item
  is drawn stretched (nearest pixel). An item whose graph was deleted stays blank. Vector formats
  (SVG, EMF, PDF) are refused with a message. Code: `sheaf_plot::render_layout_raster` /
  `render_layout_image` (`crates/plot/src/layout_render.rs`), `layout_view::write_layout_image`.

## Page size (T237 slice 4)

- Layout > Page Setup... sets the page's width and height, typed in inches, centimetres or points
  (half an inch to 200 inches a side), with Letter, Legal, A4 and A3 presets (either way round) and
  Portrait/Landscape, which swaps the sides. OK makes it one undo step ("Layout page size"). Items
  keep their places as fractions of the page, so they stretch with it; the export follows the new
  size. Code: `dialogs/layout_page_setup.rs`. Origin takes the size from the printer page; Sheaf
  keeps it per page (not measured).

## Not yet

- Add Worksheet..., text and drawn objects on the page, picture holders,
  the rest of the Layout Page properties (background, printer page), print, vector export of the page, and `.opj`.
