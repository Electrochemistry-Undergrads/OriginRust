# T02: Menu labels, shortcuts and context menus that differ from Origin

**Status:** open. **Size:** M. **Branch:** `menu-diffs`. **Model:** Sonnet.
**Detail:** audit section A, bullet "Different" (`docs/audit/origin-differences-2026-11-12.md`).
**Spec:** `docs/spec/gui-look.md` sections 1 and 3, `docs/spec/ui-command-map.md`.

## Goal

Make existing menus, shortcuts and right-click menus match Origin 2016 where they only differ in
label, place or key. No new features.

## Items (from the audit; verify each in code first)

1. **Import items:** File > Import > Single ASCII, Multiple ASCII and Comma Delimited all open the
   same multi-file picker (`menus.rs`, search "Single ASCII"). Origin: Single ASCII picks one file;
   CSV presets the comma delimiter.
2. **Ctrl+G** is unbound (no `Key::G` anywhere). Origin: Edit > Go to (Ctrl+G) in a workbook;
   File > Export Graphs... (Ctrl+G) in a graph. Bind Go to (a small row/column dialog) and, in a graph,
   point Ctrl+G at the existing export dialog (T05 rebuilds that dialog later).
3. **File menus:** Recent Graphs and Re-Import appear in every File menu; Origin shows Recent Graphs
   only in the graph File menu and no Re-Import there. "Open Sample OPJ" → "Open Sample Projects".
4. **Ctrl+Shift+C** (copy with label rows) missing; the code for label-row copy exists in
   `crates/core/src/clipboard.rs` (Copy with label rows). Bind it.
5. **Edit > Find/Replace/Go to** drawn without shortcut text (F3/Ctrl+F, Ctrl+H, Ctrl+G); add the
   key text even while greyed (use `grey_keyed`).
6. **Project Explorer keys** F2 (rename), Del (delete), Ctrl+A (select all): no key handling in
   `crates/app/src/panels/explorer.rs`. Folder menu Rename and Delete Folder are greyed (only New
   Folder is live).
7. **Context menus:** legend menu lacks Bring to front / Push to back of data; page menu shows
   Full Screen without its Ctrl+Shift+J text; layer-icon menu lists plots as `1 name` where Origin
   shows `1 g1 [Book1]Sheet1! "Conc"(X), "Signal"(Y) [1*:10*]`, and lacks the hint line "Double click
   to open Plot Setup". Keep Sheaf's extra Move to Left/Right Y Axis (needed for double-Y).
8. **Column header menu** reads "Statistics on Columns..." and adds Recalculate / Add New Columns;
   check against ui-command-map and fix the label.
9. **Main title:** `name* - Sheaf`. Origin: `<App> - <path> *`, and `- [Window *]` when a child is
   maximised. Keep "Sheaf" as the app name (clean room), copy the pattern.
10. **Status bar reference** reads `[Book]Sheet!`; Origin `1: [Book]Sheet!col[r1:r2]`.

## Keep (deliberate, do not change)

Redo (Ctrl+Shift+Z) exists although Origin has none (ADR 0007). Save Project As stays Ctrl+Shift+S
unless gui-look says otherwise. Delete in the Explorer asks no confirmation because it is undoable.

## Done when

Each item fixed or listed as "kept, because ..." here; a UI test per new key binding; checks pass.
