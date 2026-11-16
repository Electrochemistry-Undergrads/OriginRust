# Most Recently Used lists and Recent Exports (T78)

Tags: [W] Origin's public documentation, [seen] a screenshot of Origin 2016 in `gui-look.md`, **UNMEASURED** a
Sheaf choice that Origin 2016 has not been measured on.

## 1. Origin's behaviour

- The user guide says the most recently used commands appear at the bottom of the Worksheet, Analysis,
  Statistics and Image menus, for repeating an operation quickly [W]
  (https://docs.originlab.com/user-guide/origin-interface/).
- FAQ-873 says the recently used items sit at the bottom of a menu's list, up to 10 items, and are in the Plot and
  Analysis menus (the `@MRU` system variable, Origin 2017 SR1 and later) [W]
  (https://docs.originlab.com/quick-help/remove-mru/). Sheaf keeps 10 per menu.
- Analysis > "Most Recently Used Items" shows numbered entries such as `1 Nonlinear Curve Fit: <default>...`
  [seen, `gui-look.md` 728 and 775]: dialog name, a colon, the theme, an ellipsis. The Plot menu's item has an
  arrow, so it is a submenu [seen].
- The graph File menu has Recent Exports (a submenu) before Recent Books, Recent Graphs and Recent Projects [seen].
  No public page describes what it lists. Origin 2016's `MRFiles.ini` has no export section (`recent-files.md` 2).

## 2. Most Recently Used commands (Sheaf)

Plot > Most Recently Used, Worksheet > Most Recently Used Items, Analysis > Most Recently Used Items and
Statistics > Most Recently Used Items are submenus. All choices below are **UNMEASURED** unless said.

| Point | Sheaf |
|---|---|
| What is recorded | a command that ran from a click on an item of the menu in the menu bar and was enabled; not the toolbar buttons, shortcuts or context menus; not Plot > User Templates > Manage and the user template entries |
| Where | one list per menu, `commands` in `recent.json` (`plot`, `worksheet`, `analysis`, `statistics`); the Analysis list is shared by the workbook and graph bars |
| Order, length | newest first, a command used again moves to the top, at most 10 (FAQ-873 [W]) |
| Entry text | numbered `&1` to `&9` like the recent files; the item's own label; the full key (menu and submenu path) as the tooltip. No theme is shown: Sheaf's dialogs have no themes |
| Choosing | the same as clicking the item in its menu: the command is looked up again in the bar and resolved against the current selection. It re-opens the dialog the item opens (the Origin ellipsis suggests the same) and does not re-run with old settings |
| Cannot run now | the entry is greyed with the reason the item itself gives (for example no column selected) |
| Gone from the menus | a stored key the table no longer has is not shown |
| Empty | one greyed "(none)" |
| Storage | text keys, the menu name and the labels joined with ` > `; `Cmd` cannot be saved. The file is read again when one of these menus opens (as File does) and written through `sheaf_io::recent::update` (locked merge). Without a recent file (tests, no settings folder) nothing is kept |
| Clear | none (Origin turns the lists off with `@MRU`, which Sheaf has not) |

## 3. Recent Exports (Sheaf)

File > Export > Recent Exports (workbook, no-window bar) and File > Recent Exports (graph bar) are submenus with
Clear Recent Exports below a separator. All **UNMEASURED**.

| Point | Sheaf |
|---|---|
| What is listed | the files written by a successful export: Export Graphs (every file of an Apply or OK), Worksheet as ASCII and Worksheet as Excel. A failed or cancelled export lists nothing |
| Stored | `exports` in `recent.json`: the canonical path, the kind (`graph`, `sheet`, `excel`) and the title of the exported window. At most 15, newest first, the same file listed once |
| Entry text | numbered like the recent files, the path shortened to 80 characters with the extension kept (the type matters here), the full path and window as the tooltip |
| Choosing | opens the same export dialog again on the window the file was made from, in the file's folder (T05's last used settings apply). It does not write anything by itself |
| Window gone | a message "The window ... is not in this project" and nothing opens; the entry stays (it may belong to another project) |
| Empty | greyed "(none)" and a greyed Clear |
| T05 | the dialog reports the files it wrote (`take_written`); the settings themselves stay in the theme store |

## 4. Tests

`crates/io/src/recent/tests.rs` (order, repeats, length, round trip, damaged and older files),
`crates/app/src/mru.rs` (keys), `crates/app/src/app/ui_tests/recent_lists.rs` (a plot is listed and chosen again,
persistence over a restart, the empty and stale states, every menu records, exports listed, opened again, missing
window, Clear, workbook and graph bars).
