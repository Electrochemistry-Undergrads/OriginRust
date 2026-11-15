//! The desktop application.
//!
//! ```text
//! sheaf-app                      start with an empty project
//! sheaf-app lab.sheaf            open a project (Save writes .opj by default)
//! sheaf-app old.opj              import the workbooks of an Origin project
//! sheaf-app run1.txt run2.txt    import data files with default settings and plot them
//! sheaf-app results.xlsx         import every sheet of an Excel workbook (also .xls, .xlsm)
//! ```
//!
//! The project is autosaved to a recovery folder of Sheaf's own while it has unsaved changes, and the
//! next start offers it back after a crash (`docs/spec/autosave.md`). `--recovery-dir DIR` uses another
//! folder and `--no-recovery` turns recovery off for the run (no folder is read or written).
//!
//! `--mcp` turns on the AI link at start, as Tools > AI Assistants (MCP) does: an assistant started
//! with `sheaf-mcp --attach` can then work on the open project (`docs/spec/mcp.md` section 11). The
//! switch is never saved, so a run without the flag starts with the link off.
//!
//! Two more options exist so the interface can be checked without a person
//! at the screen: `--screenshot FILE.png` saves a picture of the window after
//! a moment and quits, and `--show ITEM` opens a dialog first (`graph-settings:x`,
//! `graph-settings:plots`, `panel:explorer` (also `-docked`, `-hidden`; `objects`, `messages`,
//! `results`), `hide-plot:2`, `text:1` (the Text window of the first label), `shape:1` (the window of the first drawn object), `column`, `export`, `import:FILE`, `import-excel:FILE` (with `import-excel-page:mode|header|columns|partial|misc`), `export-excel` (the first workbook), `import-wizard:FILE` (with `import-wizard-page:header|names|file|data|save`), `analysis:smooth`, `analysis-preview:smooth`, `focus:Book1`, `menu:Plot` to
//! hold a top-level menu open (`menu:File>Recent Projects` also opens a submenu), `select-column:B` and `column-menu:B` for the first workbook, `autosave` for Tools > Autosave).

// In release builds on Windows, do not open a console window next to the app.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod actions;
mod app;
mod autosave;
mod dialogs;
mod gadget;
mod graph_view;
mod grid;
mod live;
#[cfg(target_os = "macos")]
mod macprint;
mod menus;
mod panels;
mod reader;
mod recent;
mod switcher;
mod theme;
mod toolbar;
mod winclip;
mod winprint;
mod wks_clipboard;
mod workspace;

use std::path::PathBuf;

/// Sheaf's icon (the owner's S logo in paper white on a rust tile, made by `tools/make-icon.py`). eframe shows it on the
/// window, in the macOS Dock and in the Windows taskbar, also when the app is started with `cargo run`.
const ICON_PNG: &[u8] = include_bytes!("../assets/icon/sheaf-256.png");

fn app_icon() -> eframe::egui::IconData {
    eframe::icon_data::from_png_bytes(ICON_PNG).unwrap_or_default()
}

fn main() -> eframe::Result {
    let mut startup = app::Startup {
        recovery_dir: sheaf_io::recovery::default_dir(),
        settings_path: sheaf_io::recovery::default_settings_path(),
        recent_path: sheaf_io::recent::default_path(),
        themes_dir: sheaf_io::themes::default_dir(),
        mcp_dirs: Some(sheaf_io::recovery::run_dir_candidates()),
        ..Default::default()
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--recovery-dir" => startup.recovery_dir = args.next().map(PathBuf::from),
            "--no-recovery" => startup.recovery_dir = None,
            "--screenshot" => startup.screenshot = args.next().map(PathBuf::from),
            "--show" => startup.show.extend(args.next()),
            "--themes-dir" => startup.themes_dir = args.next().map(PathBuf::from),
            "--no-themes" => startup.themes_dir = None,
            "--mcp" => startup.mcp = true,
            _ => startup.paths.push(PathBuf::from(arg)),
        }
    }
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title(app::APP_NAME)
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([640.0, 400.0])
            .with_drag_and_drop(true)
            .with_app_id("sheaf")
            .with_icon(app_icon()),
        ..Default::default()
    };
    eframe::run_native(app::APP_NAME, options, Box::new(|cc| Ok(Box::new(app::SheafApp::new(cc, startup)))))
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_embedded_icon_is_a_256_pixel_square_with_see_through_corners() {
        let icon = super::app_icon();
        assert_eq!((icon.width, icon.height), (256, 256));
        assert_eq!(icon.rgba.len(), 256 * 256 * 4);
        // The corner lies outside the rounded tile, the centre is on it.
        assert_eq!(icon.rgba[3], 0, "top-left corner is transparent");
        let centre = (128 * 256 + 128) * 4;
        assert_eq!(icon.rgba[centre + 3], 255, "the middle of the icon is opaque");
    }
}
