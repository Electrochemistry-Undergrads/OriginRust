# T250: Export Graphs from the active folder

**Status:** done 2026-11-20 (Claude Windows "claude:3"), the folder order not measured for expGraph.
Export Graphs' Select Graph(s) gets All in Active Folder and All in Active Folder (Recursive): the
graphs of the Project Explorer folder open when the window opened, in Merge Graph Windows' measured
order (the scope code is shared, `dialogs/merge_graphs/folders.rs`). The parity row's "name templates"
gap was stale (File Name(s) substitution exists) and is corrected.
**Size:** S. **Source:** feature-parity "Batch export and naming" (P1).
**Spec:** `docs/spec/graphing.md` 9.7b.

## Tests

- `crates/app/src/dialogs/export.rs`: `the_folder_choices_export_the_graphs_of_the_explorer_folder`.

## Left

- Open (visible only), embedded graphs and Specified; measure expGraph's folder order.
