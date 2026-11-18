# T106: Import Filters Manager

**Status:** done 2026-11-16 (Claude Windows "t30-origin", import-ascii.md "Import Filters Manager"): Origin's Filter Manager measured (a table of import file types); Sheaf's lists its importers' file types and the saved import settings with Delete. Left: Supported/Drag&Drop switches, Customize Menu, Edit.
**Size:** S. **Source:** greyed inventory package B13 (Tools > Import Filters Manager...).
**Spec:** `docs/spec/import-ascii.md`, "Dialog themes".

## Goal

Tools > Import Filters Manager... lists the saved import settings, as Origin 2016's manager lists its
import filters; in Sheaf these are the import Dialog Themes of each importer (ASCII, CHI, Excel), which the
manager shows by kind and lets the user rename and delete.

## Origin facts

Measure in Origin 2016: the manager window (columns of the filter list, grouping, buttons).

## Done when

The item is enabled and lists, renames and deletes the saved import themes, fmt/clippy/tests pass,
inventory and spec updated.
