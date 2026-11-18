//! Tools > Import Filters Manager... (T106): Origin 2016's "Filter Manager" lists the import filters
//! (Filter, Folder, Supported, File Extensions, Drag&Drop Exclusion; Customize Menu..., Edit, Apply,
//! Cancel; measured 2026-11-16). Sheaf's lists the file types its importers read and, below, the saved
//! settings of each import window (Dialog Themes, `sheaf_io::themes`) with Delete. Rules in
//! `docs/spec/import-ascii.md`, "Dialog themes".

use eframe::egui::{self, RichText};
use sheaf_io::themes::{AsciiSettings, ChiSettings, ExcelSettings, Kind};
use std::path::{Path, PathBuf};

/// The import filters Sheaf has: (filter, importer, file extensions).
pub const FILTERS: [(&str, &str, &str); 5] = [
    ("ASCII", "Text import", "*.txt;*.dat;*.asc;*.tsv;*.prn"),
    ("CSV", "Text import", "*.csv"),
    ("CH Instruments", "CHI import (text and binary)", "*.txt;*.bin"),
    ("Excel", "Excel import", "*.xls;*.xlsx;*.xlsm"),
    ("Origin project", "File > Open", "*.opj;*.opju"),
];

fn kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::Ascii => "Text import",
        Kind::Chi => "CHI import",
        Kind::Excel => "Excel import",
    }
}

/// The saved theme names of one import window.
pub fn theme_names(dir: &Path, kind: Kind) -> Vec<String> {
    match kind {
        Kind::Ascii => sheaf_io::themes::names::<AsciiSettings>(dir),
        Kind::Chi => sheaf_io::themes::names::<ChiSettings>(dir),
        Kind::Excel => sheaf_io::themes::names::<ExcelSettings>(dir),
    }
}

fn delete(dir: &Path, kind: Kind, name: &str) -> bool {
    let r = match kind {
        Kind::Ascii => sheaf_io::themes::delete_theme::<AsciiSettings>(dir, name),
        Kind::Chi => sheaf_io::themes::delete_theme::<ChiSettings>(dir, name),
        Kind::Excel => sheaf_io::themes::delete_theme::<ExcelSettings>(dir, name),
    };
    r.unwrap_or(false)
}

pub struct FilterManager {
    pub dir: Option<PathBuf>,
    pub themes: Vec<(Kind, Vec<String>)>,
    pub selected: Option<(Kind, String)>,
}

impl FilterManager {
    pub fn new(dir: Option<PathBuf>) -> Self {
        let mut m = FilterManager { dir, themes: Vec::new(), selected: None };
        m.reload();
        m
    }

    fn reload(&mut self) {
        self.themes = match &self.dir {
            Some(d) => Kind::ALL.iter().map(|k| (*k, theme_names(d, *k))).collect(),
            None => Vec::new(),
        };
    }

    /// Deletes the selected saved setting (the Delete button).
    pub fn delete_selected(&mut self) -> bool {
        let (Some(dir), Some((kind, name))) = (self.dir.clone(), self.selected.take()) else { return false };
        let done = delete(&dir, kind, &name);
        self.reload();
        done
    }

    /// Shows the window; false when it was closed.
    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        let mut open = true;
        let mut close = false;
        egui::Window::new("Filter Manager")
            .id(egui::Id::new("filter_manager"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                egui::Grid::new("filter_manager_grid").striped(true).num_columns(4).show(ui, |ui| {
                    for head in ["Filter", "Folder", "Read by", "File Extensions"] {
                        ui.label(RichText::new(head).strong());
                    }
                    ui.end_row();
                    for (filter, by, ext) in FILTERS {
                        ui.label(filter);
                        ui.label("System");
                        ui.label(by);
                        ui.label(ext);
                        ui.end_row();
                    }
                });
                ui.separator();
                ui.label(RichText::new("Saved import settings").strong());
                if self.dir.is_none() {
                    ui.weak("Sheaf has no settings folder for saved import settings.");
                }
                let mut pick = None;
                for (kind, names) in &self.themes {
                    ui.label(kind_name(*kind));
                    if names.is_empty() {
                        ui.weak("   (none)");
                    }
                    for n in names {
                        let on = self.selected.as_ref().is_some_and(|(k, s)| k == kind && s == n);
                        if ui.selectable_label(on, format!("   {n}")).clicked() {
                            pick = Some((*kind, n.clone()));
                        }
                    }
                }
                if pick.is_some() {
                    self.selected = pick;
                }
                ui.horizontal(|ui| {
                    if ui.add_enabled(self.selected.is_some(), egui::Button::new("Delete")).clicked() {
                        self.delete_selected();
                    }
                    if ui.button("Close").clicked() {
                        close = true;
                    }
                });
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    close = true;
                }
            });
        open && !close
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_settings_are_listed_by_window_and_deleted() {
        let dir = std::env::temp_dir().join(format!("sheaf-filter-manager-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        sheaf_io::themes::save_theme(&dir, "Tabbed runs", &AsciiSettings::default()).unwrap();
        let mut m = FilterManager::new(Some(dir.clone()));
        let ascii = m.themes.iter().find(|(k, _)| *k == Kind::Ascii).unwrap();
        assert_eq!(ascii.1, ["Tabbed runs"]);
        m.selected = Some((Kind::Ascii, "Tabbed runs".into()));
        assert!(m.delete_selected());
        assert!(theme_names(&dir, Kind::Ascii).is_empty());
        assert!(FILTERS.iter().any(|(f, _, e)| *f == "Excel" && e.contains("xlsx")));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
