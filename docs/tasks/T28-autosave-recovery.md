# T28: Autosave and recovery

**Status:** done, merged 2026-11-14 night (macOS session "mac-autosave"; spec `docs/spec/autosave.md`).
`sheaf_io::recovery` (session lock files instead of process ids, orphan listing, discard, 30-day prune,
`AutosaveSettings`), the app's timer on a worker thread with an injected clock, the start-up Recover /
Discard / Later dialog, Tools > Autosave..., `--recovery-dir`/`--no-recovery`; 31 io and 26 UI tests;
reviewed by three agents with each finding checked, five fixed. Left: measure Origin's defaults and
start-up prompt on Windows (`tools/origin-ref`, screenshot to `re-workspace/scratch/t28/`); Origin's
"backup project before saving" switch; move the setting into Tools > Options when T18's Options exists;
undo back to the saved state does not clear the recovery (`Document` keeps no saved revision).
**Size:** M. **Branch:** `autosave`. **Model:** Sonnet. No Origin needed for the mechanism; the defaults
are measured once on Windows.
**Detail:** audit E (autosave and recovery missing).
**Spec:** `docs/adr/0004-project-file-and-undo.md` and `0008-opj-default-with-companion.md` for the
file formats; new `docs/spec/autosave.md`.

## Goal

An unsaved or modified project is written every N minutes to a recovery file outside the user's own
file, and after a crash the next start offers to open the recovered project. Normal saves never touch
or depend on the recovery file, and a clean exit removes it.

## Origin facts

- Not measured. From Origin's public documentation (paraphrased): Tools > Options > Open/Close has an
  autosave switch with an interval in minutes and a "backup project before saving" switch; after a crash
  Origin offers the autosaved project on the next start. Measure the defaults (on/off, interval, where
  the file goes, its name, what the start-up prompt offers) with `tools/origin-ref` and a screenshot of
  the Options tab into `re-workspace/scratch/t28/`.
- Sheaf already writes atomically (`write_atomically` in `crates/io/src/save.rs`) and has
  `backup_original`; reuse both.

## Where

- `crates/io/src/save.rs`: a `save_recovery(project, dir) -> path` writing the native `.sheaf` format
  (never `.opj`: it must store everything, including what the `.opj` writer cannot).
- Recovery folder: the platform data directory (`%LOCALAPPDATA%\Sheaf\Recovery`, macOS
  `~/Library/Application Support/Sheaf/Recovery`, Linux `$XDG_DATA_HOME/sheaf/recovery`); one file per
  running instance named by process id and start time, with a small JSON sidecar (original path, time,
  Sheaf version).
- `crates/app/src/app.rs`: a timer in the update loop that saves when the project is dirty and the
  interval passed. Serialising must not freeze the UI: clone or snapshot the project, write on a worker
  thread. Look for the dirty flag the title bar's `*` uses.
- Start-up: list recovery files whose process is not running; a dialog "Recover / Discard / Later".
- Settings: interval and on/off stored with the app settings (find where recent files or window
  layout are kept), shown later in Tools > Options (T18) and for now in a small dialog under Tools.

## Steps

1. Spec section with the measured Origin defaults (or Sheaf's choice, labelled as such: 10 minutes, on).
2. `save_recovery`/`list_recoveries`/`discard_recovery` in sheaf-io with tests in a temp directory
   (stale process ids, a corrupt file, two instances).
3. App timer on a worker thread; UI test that a dirty project produces a recovery file after the
   interval (inject the clock) and a clean exit removes it.
4. Start-up dialog; UI test with a planted recovery file.

## Done when

Killing the app after an edit and starting it again offers the edit back; normal save and exit leave no
recovery files; fmt, clippy and tests pass; parity row updated.

## Don't

- Don't write next to the user's project file or overwrite it.
- Don't block the UI thread on large projects.
- Don't keep recovery files forever: drop those older than 30 days at start-up.
