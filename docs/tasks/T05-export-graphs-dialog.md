# T05: Export Graphs dialog, raster formats, units, Ctrl+G, Copy Page

**Status:** done, merged (`c32e585`). Items 1 and 2 are in: the Export Graphs window (Select Graph(s), File Name(s) with `<long name>`, `<short name>`, `<index>` substitution, Path, Overwrite Existing, Export Settings, Image Size, Image Settings, preview with Auto Preview, OK, Cancel, Apply); JPEG, BMP and GIF; 256 colours / 24-bit / 8-bit grey; TIFF compression none, PackBits, LZW; JPEG quality. Items 3 and 4 were done earlier (`export-margin`). TGA, PCX and PSD added later: 24-bit, grey or 256 colours. PSD is a flattened image with resolution metadata, no editable layers; its documented 30000-pixel side limit is checked before raster allocation. TIFF also supports an indexed 256-color palette with all three compression choices. Saved export Dialog Themes/defaults were added later (9.8). Left: Graph Theme (greyed), the Graph tab of the preview, more export choices (Select Graph(s) only has Active Page and All Graphs in Project), Monochrome, 16 Colors and 16-bit Color (BMP, TGA) added 2026-11-14 with Origin's per-format lists and its ordered dither (graphing 9.4). Default type is PNG at 24-bit (Origin: EPS, 256 colours) because 256 colours spoils colour figures. **Size:** M.
**Branch:** `export-dialog`. **Model:** Sonnet.
**Detail:** audit B "Export Graphs", C "Templates, themes, export".
**Spec:** `docs/spec/graphing.md` section 9 (9.1 formats, 9.4 colour depth/compression, 9.6 Copy
Page), `docs/spec/gui-look.md` (export dialog).

## Export path

`crates/app/src/dialogs/export.rs` offers PNG, TIFF, JPEG, BMP, GIF, TGA, PCX, PSD, SVG, PDF,
EPS and EMF in Export Graphs. Raster exports share SVG rasterisation (`crates/plot/src/raster.rs`);
`psd.rs` writes flattened PSD files from the published Adobe specification. The CLI selects the
same exporter by output extension. Image-size units, margins, resolution and compatible color-depth
choices use the existing export controls. Copy Page is separate; see graphing.md 9.6.

## Goal (Origin)

1. Dialog title "Export Graphs" with groups: Select Graph(s), File Name(s) with substitution
   (`<long name>` style), Path, Overwrite, Graph Theme; Export Settings / Image Size / Image Settings
   groups; preview pane with Auto Preview; buttons OK, Cancel, Apply. Open from File > Export Graphs...
   (Ctrl+G in a graph; T02 binds the key).
2. Formats: add JPEG, BMP, GIF (the `image` crate already used by raster.rs likely covers them;
   check Cargo.toml). Colour depth (256-colour / 24-bit) and TIFF compression choices (spec 9.4).
3. Units: inch, cm, pixel, page ratio %.
4. Copy Page: size default from spec 9.6 (likely 40 % of page size) and also put EMF on the clipboard
   (EMF writer exists: `crates/plot/src/emf.rs`).

## Done when

Dialog matches the spec layout (screenshot via `--show export --screenshot`), the new formats write
files that open (check headers in a test), Copy Page puts both bitmap and EMF; checks pass.

## Don't

Don't change the scene/EPS/EMF renderers beyond what the formats need. EPS was never checked with
ghostscript (not installed); don't claim it was.


## Dialog Theme follow-up (2026-11-15)

The Dialog Theme row now saves/reloads named export settings, saves a custom default,
restores/reset defaults, confirms deletion and remembers successful exports as `<Last used>`.
Save As/Cancel/Escape and Delete prompts preserve the graph and undo history; settings IO
failures do not turn an already written export into a failure. The store is separate from
importers and uses their existing atomic, locked persistence API. Graphing.md 9.8 records the
five native sessions, the PNG -> TIFF -> PNG control checks, cleanup and explicit limits.

Four storage/export regressions and three UI workflows cover reopening, exact supported
settings, default precedence/reset, another instance's named themes, malformed/older settings,
Cancel/Escape, deletion, real Apply output, failed/pending exports and IO warnings.
Graph Theme, Compare/Generate Script, graph-specific settings and `.ois`
interchange remain open; PNG/24-bit remains Sheaf's deliberate system default.

Final validation with comparison plots, worksheet formatting, T48 measurements and browser Exclude
integrated: formatting, strict workspace all-target Clippy and 3,443 default-parallel workspace
tests pass (eight existing ignored). All three focused UI workflows pass, including normalized-name
collisions, modified-theme marking and Save. The real desktop screenshot was inspected. The greyed
inventory check passes at 459; Graph Theme remains disabled. The published upstream Windows path
assertion repair `81765e8` is retained, with no additional MCP changes. Earlier baseline hello timing
failure evidence remains in peer notes; the final full run passed unchanged.
