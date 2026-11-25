//! File > Import > Auto Refresh... (T442): how often an imported workbook re-imports its source file, whether only
//! when the file changed, and whether the timer is paused; Stop Refreshing removes it. The timer itself runs in
//! `SheafApp::run_auto_refresh`. Rules in `sheaf_core::auto_refresh`.

use eframe::egui;
use sheaf_core::WindowId;
use sheaf_core::auto_refresh::{AutoRefresh, MAX_SECONDS, MIN_SECONDS};

pub enum RefreshOutcome {
    Open,
    Closed,
    /// Set the refresh (`None` stops it).
    Set {
        book: WindowId,
        refresh: Option<AutoRefresh>,
    },
}

pub struct AutoRefreshDialog {
    pub book: WindowId,
    /// The source file, shown.
    pub path: String,
    pub refresh: AutoRefresh,
    /// The book refreshes already (Stop Refreshing is offered).
    pub active: bool,
}

impl AutoRefreshDialog {
    pub fn new(book: &sheaf_core::Workbook) -> Option<Self> {
        let path = book.source.as_ref()?.path.clone();
        Some(AutoRefreshDialog {
            book: book.id,
            path,
            refresh: book.refresh.unwrap_or_default(),
            active: book.refresh.is_some(),
        })
    }

    pub fn show(&mut self, ctx: &egui::Context) -> RefreshOutcome {
        let mut open = true;
        let mut outcome = RefreshOutcome::Open;
        egui::Window::new("Auto Refresh")
            .id(egui::Id::new("auto_refresh"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(format!("Source: {}", self.path));
                ui.horizontal(|ui| {
                    ui.label("Check every");
                    ui.add(egui::DragValue::new(&mut self.refresh.every_seconds).range(MIN_SECONDS..=MAX_SECONDS));
                    ui.label("seconds");
                });
                ui.checkbox(&mut self.refresh.only_when_changed, "Only when the file has changed");
                ui.checkbox(&mut self.refresh.paused, "Paused");
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        outcome = RefreshOutcome::Set { book: self.book, refresh: Some(self.refresh) };
                    }
                    if ui.add_enabled(self.active, egui::Button::new("Stop Refreshing")).clicked() {
                        outcome = RefreshOutcome::Set { book: self.book, refresh: None };
                    }
                    if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        outcome = RefreshOutcome::Closed;
                    }
                });
            });
        if !open { RefreshOutcome::Closed } else { outcome }
    }
}
