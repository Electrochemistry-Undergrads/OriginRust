# T27: Print, Print Preview, Page Setup

**Status:** Windows graph printing merged (2026-11-14): Page Setup model and window, Print, Print Preview, headless print check, graphing 19. Left: workbook printing, Origin's Print/Print Preview windows (unmeasured). (Brief written 2026-11-14 from T18's "Print" row and parity row "Print, Print Preview,
Page Setup", P1).
CLI graph printing added on macOS without Origin (`cli-print`, 2026-11-14):
`sheaf print PROJECT [--graph NAME] --output FILE.pdf` writes a vector PDF on the saved paper size,
with the graph scaled and centred by `PageSetup::place`; without `--output`, macOS/Linux submit
that PDF through `lp` to the default or `--printer NAME` destination. Windows supports PDF output
only. The native macOS graph print panel is added in `mac-print-panel` (2026-11-15), using
PDFKit and the same print PDF. The Standard toolbar Print button now shares the menu command
and availability rules; workbook printing remains separate. No physical printer was used
in the automated checks. Formatting, strict all-target Clippy and 2,119 workspace tests passed (one ignored).

**Size:** M. **Branch:** `print`. **Model:** Sonnet. Needs a Windows session for the Windows path and a
macOS session for the macOS path; each half can merge on its own.
**Detail:** audit A (File menu: Print, Print Preview, Page Setup greyed; Standard bar Print; View >
Print View).
**Spec:** `docs/spec/graphing.md` (the page tabs Print/Dimensions of section 18 and export section 9
are the page-size facts to reuse); new section "Printing" in the same file.

## Goal

File > Print... (Ctrl+P) prints the active graph page at its page size, through the system print dialog;
Print Preview shows the page as it will print; Page Setup chooses paper, orientation and margins. A
workbook prints its active sheet as a table. Both work without any file export step for the user.

## Origin facts

- Measured: the graph page size and the page tab's Print/Dimensions settings (graphing.md 18, page
  tabs). Not measured: Origin's Print dialog (printer, copies, "Fit to page"/scale, page range for
  workbooks, "Print to file"), Page Setup (paper, orientation, margins) and Print Preview. Open each with
  `tools/origin-ref/origin-dialog.ps1 -MenuClicks` (as done for the Peak Analyzer, T04) and screenshot
  into `re-workspace/scratch/t27/`; copy the layout, not the text.
- Origin prints a graph as vector output scaled to the paper, page-centred, by default.

## Where

- Rendering exists: `sheaf_plot::render_emf` and `render_pdf` (`crates/plot/src/lib.rs`), and the
  clipboard's EMF path `crates/app/src/winclip.rs` (`put_bitmap_and_emf`), which already talks to Win32.
- Windows: a new `crates/app/src/winprint.rs` (cfg windows): `PrintDlgW`/`PrintDlgExW` for the printer
  and copies, `StartDocW`/`StartPage`, `PlayEnhMetaFile` of the page's EMF scaled into the printable
  area (`GetDeviceCaps` PHYSICALOFFSETX/Y, HORZRES/VERTRES), `EndPage`/`EndDoc`. `windows-sys` is in
  the lock file already (via eframe); add it as a direct target dependency with the needed features
  (`Win32_Graphics_Gdi`, `Win32_Storage_Xps`, `Win32_UI_Controls_Dialogs`).
- macOS: hand the page's PDF to the system print panel (an `NSPrintOperation` of a PDF view through
  `objc2`, or `lpr` with `-o` options as a first step); Linux: `lp` with the PDF.
- Preview: the existing page renderer in the graph window, with the paper outline, margins and the
  scaled page drawn on it (an egui window; no new renderer).
- Page Setup: settings stored on the project (paper, orientation, margins), saved in `.sheaf` and the
  `.opj.sheaf` companion; look for where the page tab settings live in `crates/core/src/graph.rs`.
- Menus: `crates/app/src/menus.rs` `file_menu` (`grey_keyed("&Print...", ctrl(Key::P))`,
  `grey("Print Pre&view")`, `grey("Page Setup...")`) and the View menu's `Print View`.
- Workbooks: render the visible sheet as a table into the same EMF/PDF path (later step; may split into
  its own brief if large).
- CLI: `sheaf print PROJECT [--graph NAME] [--printer NAME | --output FILE.pdf]` uses saved Page Setup
  (graphing.md 19.6). `sheaf plot ... -o x.pdf` exports at graph-page size instead.

## Steps

1. Measure and screenshot Origin's three dialogs; spec section.
2. Page Setup model and dialog, saved with the project; test the round trip.
3. Windows printing of a graph; a test that the scaling rectangle is right for a few paper/page sizes
   (pure function, no printer). Manual check: print to "Microsoft Print to PDF" and compare with the
   PDF export (save the comparison in `re-workspace/scratch/t27/`).
4. Print Preview window.
5. macOS/Linux path (separate session or branch if this one is on Windows).
6. Workbook printing.

## Done when

Ctrl+P on a graph opens the system dialog and prints the page at the right size on the chosen paper
(checked with Microsoft Print to PDF), Preview shows the same placement, Page Setup persists across
save and reopen; fmt, clippy and tests pass; parity row updated.

## Don't

- Don't rasterise the page for printing when the EMF/PDF vector path works.
- Don't print from tests; keep the printing calls behind a function that tests can skip.
