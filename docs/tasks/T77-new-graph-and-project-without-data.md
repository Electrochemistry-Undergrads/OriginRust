# T77: New Graph, Save Project without Data, Duplicate Project without Data (B12, slice 1)

**Status:** done 2026-11-16 (branch `new-graph-nodata`; graphing.md 25, project-file-format.md "Projects without data").
**Size:** M. **Inventory:** File > New > Graph, Standard > New Graph, File > Save Project without Data...,
File > Duplicate Project without Data (4).

## Goal

Four greyed items work. New Graph (menu and button) adds an empty graph from the default template.
The two project items make a copy of the project with every cell value taken out.

## Origin facts (from Origin's documentation, NOT measured; Origin 2016 is only on Windows)

- New Graph, menu and Standard button, opens an empty graph window built from the default graph
  template (`ORIGIN.OTPU`), named Graph1, Graph2, ... (docs.originlab.com/origin-help/standard-toolbar,
  .../create-childwindow). No shortcut.
- Origin 2016 does not document the two "without Data" items. The nearest documented feature is
  File > Clone Project of the X-Function `clrAllData` (Origin 2020), whose mode 0 clears all data and
  keeps windows and operations; it asks to save a changed project first and opens the clone as
  UNTITLED. The "Exclude imported data when saving project" workbook option is a different feature.
- To measure on Windows: what an empty New Graph looks like (axis ranges), whether Duplicate shows a
  dialog, what it clears (masks, notes, formulas, matrices) and whether the copy becomes the open project.

## Built

- core `without_data.rs`: `Project::without_data()` (values, masks, notes, cell and conditional formats,
  filters, analysis marks and links, comparison sources removed; windows, names, labels, units,
  designations, formulas and all graphs kept; the original is not touched).
- app `app/no_data.rs`: `new_graph` (one undo step "New graph", focused), `save_project_without_data[_to]`
  (chooser, writes a copy; the open project, its file name and saved state stay), and
  `duplicate_project_without_data` (asks about unsaved changes like New Project; the copy replaces the
  project as an unsaved UNTITLED project).
- Menu items, toolbar button, `Cmd`/`Action` entries; tests in core, io (`.sheaf` and `.opj` round trip),
  plot (empty graph renders) and app (`ui_tests/new_graph.rs`).

## Don't

B6's sheet-tab Duplicate Without Data, Re-Import, Worksheet as Image, View Windows, folders, sample projects.
