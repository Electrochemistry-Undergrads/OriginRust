//! The desktop application.
//!
//! ```text
//! sheaf-app                      start with an empty project
//! sheaf-app lab.sheaf            open a project (Save writes .opj by default)
//! sheaf-app old.opj              import the workbooks of an Origin project
//! sheaf-app run1.txt run2.txt    import data files with default settings and plot them
//! ```
//!
//! Two more options exist so the interface can be checked without a person
//! at the screen: `--screenshot FILE.png` saves a picture of the window after
//! a moment and quits, and `--show ITEM` opens a dialog first (`graph-settings:x`,
//! `graph-settings:plots`, `column`, `export`, `import:FILE`, `focus:Book1`, `menu:Plot` to
//! hold a top-level menu open).

// In release builds on Windows, do not open a console window next to the app.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod actions;
mod app;
mod dialogs;
mod graph_view;
mod grid;
mod menus;
mod theme;
mod workspace;

use std::path::PathBuf;

fn main() -> eframe::Result {
    let mut startup = app::Startup::default();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--screenshot" => startup.screenshot = args.next().map(PathBuf::from),
            "--show" => startup.show.extend(args.next()),
            _ => startup.paths.push(PathBuf::from(arg)),
        }
    }
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title(app::APP_NAME)
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([640.0, 400.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(app::APP_NAME, options, Box::new(|cc| Ok(Box::new(app::SheafApp::new(cc, startup)))))
}
