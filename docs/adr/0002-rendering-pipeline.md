# ADR 0002: One scene, SVG as the common output, raster for the screen

Status: accepted (2026-11-11)

## Decision

`sheaf-plot` lays a graph out into a scene (lines, polygons, positioned text, in points) and writes
that scene as SVG. Every other output is derived from the SVG:

- PNG: rasterise the SVG with the `resvg` crate at the requested DPI.
- PDF: convert the SVG with the `svg2pdf` crate (vector output, text stays text).
- Screen: the application rasterises the same SVG at the window's pixel density and shows the
  image. Mouse interaction uses the axis transforms that the layout step reports.

Text is positioned by our own layout (each run is its own SVG text element at a computed position)
so that no renderer has to make text layout decisions.

## Reasons

- The owner's requirement is that a figure looks the same on screen and in exports. With a single
  scene and a single rasteriser this holds by construction; there is no second drawing code path
  to keep in sync.
- Rendering can be tested without a window: the CLI renders to files and tests compare SVG text.
- `resvg`, `usvg` and `svg2pdf` are maintained, pure Rust, and permissively licensed.

## Costs and how they are handled

- Re-rasterising on every change is slower than drawing with the GUI toolkit directly. The layout
  step drops points that are closer than 0.05 pt to their neighbour, which keeps even
  million-point curves to a few tens of thousands of segments. The app re-renders only when the
  graph, its data or the view size changes, and caches the texture.
- Layout needs text widths, so the renderer loads system fonts. Tests use a fixed-metrics measurer
  so that expected SVG does not depend on installed fonts.

## Alternatives considered

- Drawing with egui's painter on screen and a separate exporter: two code paths that drift apart.
- The `plotters` crate: fixed chart styles, weak control over publication details such as tick
  direction, closed boxes and text markup.
