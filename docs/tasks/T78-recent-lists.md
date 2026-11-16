# T78: Most Recently Used lists and Recent Exports (B10)

**Status:** done 2026-11-16 (branch `recent-lists`, Claude macOS "mac-format-bar"; spec `docs/spec/recent-lists.md`).
**Size:** S. **Branch:** `recent-lists`. **Spec:** `docs/spec/recent-lists.md`.
**Inventory:** Plot > Most Recently Used; Worksheet, Analysis and Statistics > Most Recently Used Items; File > Export > Recent Exports (workbook and no window); Recent Exports in the graph File menu (6, all `B10`).

## Goal

The six greyed items work. Reuse the T38 recent file (`recent.json`, locked merge) and the T05 export dialog.

## Origin facts

Only documentation and screenshots: up to 10 recently used commands at the bottom of the Plot, Worksheet, Analysis
and Statistics menus (FAQ-873, user guide); `1 Nonlinear Curve Fit: <default>...` seen in the Analysis menu. No
page describes Recent Exports; Origin 2016's `MRFiles.ini` has no export list. Everything else is an unmeasured
Sheaf choice, labelled so in the spec.

## Steps (done)

1. `sheaf_io::recent`: `exports` and `commands` in `RecentList`, `push_export`, `push_command`, `clear_exports`, the
   locked `record_*` helpers; damaged and older files load.
2. `crates/app/src/mru.rs`: menu keys (path of labels), lookup in the bar.
3. `menus.rs`: `Entry::Mru`, `Entry::Exports`, the six items, `Cmd::ClearRecentExports`; one recording point in
   `show_entries` (a clicked, enabled item of Plot, Worksheet, Analysis or Statistics pushes `Action::UsedCommand`).
4. `app.rs`: `UsedCommand`, `OpenRecentExport`, `ClearRecentExports`; the export dialogs report written files.
5. Tests and docs.

## Done when

The six items are gone from `greyed-items.md`; a command run from one of the four menus is listed and chosen again;
a successful export is listed and opens its dialog again; empty, stale and missing-window cases behave as the spec.

## Don't

Record toolbar or shortcut runs, store settings with MRU entries, re-run an export without its dialog, or touch the
user's real settings folder in tests.
