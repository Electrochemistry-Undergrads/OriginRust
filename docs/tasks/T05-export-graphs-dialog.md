# T05: Export Graphs dialog, raster formats, units, Ctrl+G, Copy Page

**Status:** done, merged (`c32e585`). Items 1 and 2 are in: the Export Graphs window (Select Graph(s), File Name(s) with `<long name>`, `<short name>`, `<index>` substitution, Path, Overwrite Existing, Export Settings, Image Size, Image Settings, preview with Auto Preview, OK, Cancel, Apply); JPEG, BMP and GIF; 256 colours / 24-bit / 8-bit grey; TIFF compression none, PackBits, LZW; JPEG quality. Items 3 and 4 were done earlier (`export-margin`). Left: Graph Theme (greyed), Dialog Theme row, the Graph tab of the preview, more export choices (Select Graph(s) only has Active Page and All Graphs in Project), TIFF 256-colour palette, mono and 16-colour modes, PSD (TGA and PCX added later: 24-bit, grey or 256 colours). Default type is PNG at 24-bit (Origin: EPS, 256 colours) because 256 colours spoils colour figures. **Size:** M.
**Branch:** `export-dialog`. **Model:** Sonnet.
**Detail:** audit B "Export Graphs", C "Templates, themes, export".
**Spec:** `docs/spec/graphing.md` section 9 (9.1 formats, 9.4 colour depth/compression, 9.6 Copy
Page), `docs/spec/gui-look.md` (export dialog).

## Today

`crates/app/src/dialogs/export.rs`: PNG, TIFF, SVG, PDF, EPS, EMF; units inch and cm; only "Save as"
and "Close"; title `Export {name}`. PNG and TIFF are rasterised from the SVG
(`crates/plot/src/raster.rs`). CLI already takes in, cm, mm, pt, px (`crates/cli/src/args.rs`).
Copy Page (Ctrl+J) is a fixed 300 dpi bitmap (`app.rs`, search "Copy Page").

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
