# ADR 0003: Desktop GUI with eframe/egui and egui_dock

Status: accepted (2026-11-11)

## Decision

The desktop application uses `eframe`/`egui` with `egui_dock` for dockable window tabs (workbooks
and graphs side by side, a project explorer panel on the left) and `rfd` for native file dialogs.
The worksheet grid is drawn with a virtualised table: only the rows visible in the viewport are
laid out, so scrolling cost does not depend on the number of rows.

## Reasons

- Pure Rust, one static executable, starts fast, works on Windows 11 without extra runtimes.
- Immediate-mode drawing makes a virtualised million-row grid simple: per frame the grid formats
  only about 50 visible rows.
- `egui_dock` gives tabbed and split windows, which covers the "several workbooks and graphs open
  at once" workflow of the original program.

## Consequences

- Dialogs (import options, plot details, axis settings) are egui windows, not native dialogs.
- The look is our own. No OriginLab icons or bitmaps are used.
- The app crate is excluded from the Linux CI job's test step if it needs system libraries that
  the runner lacks; all logic that needs tests lives in the other crates.
