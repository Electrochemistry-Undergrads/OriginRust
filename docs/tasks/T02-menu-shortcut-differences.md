# T02: Menu labels, shortcuts and context menus that differ from Origin

**Status:** done, merged (`bbdb60a`) (items 1-7, 9, 10 done; 8 kept; see Result); Go To and Find/Replace follow-ups measured and updated (2026-11-14/15), gui-look section 5. Wider search scopes and the project-wide Column Browser remain. **Size:** M. **Branch:** `menu-diffs`. **Model:** Sonnet.
**Detail:** audit section A, bullet "Different" (`docs/audit/origin-differences-2026-11-12.md`).
**Spec:** `docs/spec/gui-look.md` sections 1 and 3, `docs/spec/ui-command-map.md`.

## Goal

Make existing menus, shortcuts and right-click menus match Origin where they only differ in
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

## Result (branch `menu-diffs`)

1. Done: `Action::ImportPick`; Single ASCII (Ctrl+K) picks one file, Multiple ASCII several, Comma
   Delimited several with the comma preset (`ImportDialog::preset_comma`). Toolbar Multiple ASCII
   button now sends the Multiple command.
2. Done: Edit > Go to... (Ctrl+G) opens `dialogs/go_to.rs` (row, column number or name); in a graph
   Ctrl+G exports (graph File menu has "Export Graphs..." with the key in place of the Export submenu,
   as Origin's). The shortcut loop skips a shared key whose command cannot run in that window.
3. Done: Recent Graphs only in the graph File menu, Re-Import items only outside graphs, "Open Sample
   Projects".
4. Done: Ctrl+Shift+C = Copy including label rows.
5. Done: shortcut text added; Find in Project (F3) remains greyed, Find in Sheets (Ctrl+F), Replace
   (Ctrl+H) and Go to (Ctrl+G) are live.
6. Done: F2 / Delete / Ctrl+A in the Explorer while it has the keyboard (last click inside it); Ctrl+A
   highlights every window of the folder, Delete deletes the highlighted ones (one undo step each).
   Folder menu Rename and Delete Folder live (not for the root); Delete Folder removes its windows as
   one undo step.
7. Done: legend menu has Bring to front of data / Push to back of data (greyed, no stacking model);
   page menu shows Ctrl+Shift+J; layer-icon plots read `g1 [Book]Sheet! "X"(X), "Y"(Y) [1*:N*]` and the
   hint line "Double click to open Plot Setup" was added. Move to Left/Right Y Axis kept.
   Unverified against Origin: the "g1" group number (always 1 here) and whether the column names in
   the plot line are long names or short names (Sheaf uses the long name when set).
8. Kept: the header menu reads "Statistics on Columns..." as the Statistics menu does; the spec's
   "Statistics on Column" comes from a decoded resource string, not a screenshot. Recalculate and Add New
   Columns stay (Origin's Column menu has them). Not measured in Origin.
9. Done: title `Sheaf - <path or UNTITLED> *`, plus ` - [Window *]` while a child is maximised.
10. Done: status reference `1: [Book]Sheet!A:B[r1:r2]` (graph `1: [Graph]1!`). The meaning of the leading
    "1:" is assumed.

Go To follow-up (2026-11-14): Origin 2016's `dlgRowColGoto -d;` was captured with synthetic
data, including its controls and a clear screenshot. Column comes before Row, both start blank,
Column receives focus, and Size/Current and zero/end hints precede the inputs. Extend Selection
starts unchecked. The revised dialog uses that order, focus, hints and the existing worksheet view
capacity. A missing coordinate preserves that selection axis; both blank make no change. Extend
Selection uses the measured directional edge rule, including shrinking an existing region. Zero
selects the last column or the destination/first selected column's last data row; a row past capacity clamps to capacity, while a column
past the last is refused. Selection changes create no document edit or undo entry; Cancel/Escape
preserve the selection. The viewport clears stale reveal geometry before scrolling and reveals
explicit far-column jumps in one step; OK/Cancel sit at the right edge.

Source probes and exceptions are recorded in gui-look.md's Go To subsection. The 2026-11-15 UI
follow-up measures the flyout, Reset and typed names: short names and long names ignore ASCII
case, and duplicate long names choose the first column. The picker displays short names,
designations and long names, allowing a later duplicate to be chosen; navigation still waits for
OK. Reset clears only the column, and Escape closes the flyout before the dialog. A parser fixture
and four UI regressions cover name lookup, duplicate picking, Reset, Escape and unchanged history.
Origin's separate project-wide Column Browser was captured but remains unimplemented; Select
Columns stays disabled.
Sheet row capacity is still Sheaf's data length plus 30 spare rows, rather than a persisted Origin
allocation. Six new UI regressions, a new unequal-column parser fixture and the expanded existing
parser regression cover this follow-up.

Find/Replace follow-up (2026-11-15): direct Origin 2016 captures show one tabbed window, Numeric
initial mode, selected display text in Find what, active-sheet scope and expanded Options. The
dialog now implements Numeric conditions, relative tolerance, missing values, absolute-value and
sign-preserving replacement, clearing nonmatches, backward search and String wildcards. The
measured strict `!=` rule is distinct from tolerant equality. Empty Numeric replacement clears
the cell. String mode keeps the existing case/whole-cell and substring rules. Four new UI tests,
four matching fixtures and the updated existing substring regressions cover the change.
All probes and the remaining wider-scope, labels, cell-link, history and query-menu
limitations are recorded in gui-look section 5. `--show find` / `--show replace` open native previews.

Selected-range follow-up (2026-11-15): eleven successful Origin GUI cases establish the checked
range option for blocks, whole columns and rows, its retention across restricted hits and its
removal after an unchecked search collapses the range. Sheaf snapshots the opening scope for
both data modes, forward/backward/wildcard searches and both replacement paths, including
clearing numeric nonmatches. Four UI regressions and one matching fixture cover this addition.
The visual difference (Sheaf selects each hit while retaining the saved scope; Origin retains
the rectangle) and the remaining gaps are recorded in gui-look section 5.
