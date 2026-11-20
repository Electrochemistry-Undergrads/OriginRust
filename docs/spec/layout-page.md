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

## Not yet

- Moving and resizing items, Add Worksheet..., text and drawn objects on the page, picture holders,
  Layout Page properties (page size), print and export of the page, and `.opj`.
