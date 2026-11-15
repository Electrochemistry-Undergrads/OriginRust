# T46: Saved import settings (Dialog Theme: last used and named themes)

**Status:** claimed 2026-11-15 (branch `import-themes`, Claude macOS "mac-themes"). Brief written from
parity row "Import filters and saved settings" (P1, ✗) and the Dialog Theme row measured in Origin's
impASC window (import-ascii.md, "The impASC window, measured in Origin").
**Size:** M. **Branch:** `import-themes`. **Model:** Sonnet. No Origin needed for the mechanism; the
theme list's entries and defaults are measured once on Windows.
**Spec:** new section in `docs/spec/import-ascii.md` ("Dialog themes") plus a short note in
`import-excel.md` and the CHI import spec.

## Goal

The owner imports the same instrument files again and again. Each import window (ASCII, CH Instruments,
Excel) gets Origin's **Dialog Theme** row at the top: `<default>` (the built-in defaults), `<last used>`
(the settings of the last OK in that window, kept across restarts) and named themes the owner saves
("CHI CV 3 header lines"). Choosing a theme fills the window; Save / Save As / Delete manage the list.
`sheaf import --theme NAME` (and `--last-used`) applies the same stored settings on the command line.

## Origin facts

- Measured (import-ascii.md): impASC has a Dialog Theme row at the top, above the description.
- Not measured: the row's list (`<default>`, `<last used>`, saved names?), which theme a fresh window
  opens with, whether the theme stores the file name and output, where Origin keeps themes (per user?),
  and how the Import Wizard's Import Filter list relates to themes. Measure on Windows with
  `tools/origin-ref` (screenshots of the theme drop-down and its menu into `re-workspace/scratch/t46/`).
  Until then: Sheaf's choices, labelled: a fresh window opens with `<last used>` when it exists, else
  `<default>`; themes store every option except the file list and the target window; one store per
  importer kind.
- Origin's `.oif` filter files are Origin's own format: out of scope (clean-room; no reading or writing).

## Where

- `crates/io`: `Serialize`/`Deserialize` for `AsciiOptions`, `ChiOptions`, `ExcelOptions` and the batch
  options the import windows hold (with `#[serde(default)]` so older theme files keep loading after new
  options appear); a `themes` module: `ThemeStore` per importer kind (`ascii`, `chi`, `excel`) in the
  settings folder (`sheaf_io::recovery::default_settings_path`'s folder, `import-themes/<kind>.json`),
  `last_used`, named themes, load tolerant of missing/corrupt files, atomic locked re-read-and-merge
  writes like `sheaf_io::recent::update` (T38), names sanitised and unique (case-insensitive).
- `crates/app/src/dialogs/import.rs`, `import_excel.rs`, the CHI import window (find by name), and the
  Import Wizard if it shares the option struct: the Dialog Theme row (combo + a small menu: Save,
  Save As..., Delete); OK records `<last used>`; a `Startup.themes_dir: Option<PathBuf>` like T38's
  `recent_path` (tests pass `None` or a temp folder, never the real one).
- `crates/ops` / CLI: `--theme NAME`, `--last-used` on `sheaf import` (and the commands that import data
  files), applied before the explicit flags so flags still win; an unknown name lists the saved themes.

## Steps

1. Spec section (Sheaf's choices labelled; Origin measurements to make listed).
2. Serde derives with defaults; `sheaf_io::themes` and tests (round trip of every option, missing fields
   from an older file, corrupt file, two writers, name rules).
3. Theme row in the three import windows; `<last used>` recorded on OK only (Cancel records nothing);
   UI tests: a saved theme refills the window after restart; `<last used>` restores the last OK;
   Delete removes it; the real settings folder is never touched.
4. CLI flags with tests; parity row to ◐ (Origin `.oif` filters and the Import Filters Manager left).

## Done when

A theme saved in an import window survives a restart and refills that window, `<last used>` brings back
the last import's settings, `sheaf import --theme` uses the same store, fmt, clippy and tests pass.

## Don't

- Don't read or write Origin's `.oif` files.
- Don't store the file list or the target book in a theme.
- Don't let a theme file from a newer Sheaf with unknown fields fail to load (ignore unknown fields).
