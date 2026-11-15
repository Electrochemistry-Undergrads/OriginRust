//! Export Dialog Themes (graphing.md 9.8). Reuses the locked, atomic theme store,
//! with files separate from each importer's themes. No project/window ids are saved.

use super::*;
use serde_json::{Value, json};
use sheaf_io::themes::{ThemeStore, clean_name, update};

const DEFAULT: &str = "<default>";
const LAST: &str = "<Last used>";
const SYSTEM: &str = "System Default";
const FILE: &str = "graph-export.json";
const DEFAULT_FILE: &str = "graph-export-default.json";

#[derive(Default)]
pub(super) struct Themes {
    dir: Option<PathBuf>,
    system: Value,
    names: Vec<String>,
    selected: String,
    loaded: Option<Settings>,
    has_default: bool,
    has_last: bool,
    prompt: Option<String>,
    focus_prompt: bool,
    delete: Option<String>,
    message: Option<String>,
}

pub(super) enum Command {
    Choose(String),
    SaveDefault,
    ResetDefault,
    Save,
    SaveAs(String),
    Delete(String),
}

impl Themes {
    pub(super) fn modal_open(&self) -> bool {
        self.prompt.is_some() || self.delete.is_some()
    }

    fn refresh(&mut self) {
        if let Some(dir) = &self.dir {
            let store = ThemeStore::load(&dir.join(FILE));
            self.names = store.names();
            self.has_last = store.last_used.is_some();
            self.has_default = ThemeStore::load(&dir.join(DEFAULT_FILE)).last_used.is_some();
        }
    }

    pub(super) fn ui(&mut self, ui: &mut egui::Ui, modified: bool) -> Option<Command> {
        self.dir.as_ref()?;
        let mut command = None;
        ui.horizontal(|ui| {
            ui.label("Dialog Theme");
            let mut shown = if self.selected == SYSTEM {
                String::new()
            } else {
                format!("{}{}", self.selected, if modified { " *" } else { "" })
            };
            ui.add(egui::TextEdit::singleline(&mut shown).interactive(false).desired_width(320.0));
            ui.menu_button("▶", |ui| {
                let named = self.names.contains(&self.selected);
                if ui.button("Save as <default>").clicked() {
                    command = Some(Command::SaveDefault);
                    ui.close();
                }
                if ui.add_enabled(named, egui::Button::new("Save")).clicked() {
                    command = Some(Command::Save);
                    ui.close();
                }
                if ui.button("Save As...").clicked() {
                    self.prompt = Some(if named { self.selected.clone() } else { String::new() });
                    self.focus_prompt = true;
                    ui.close();
                }
                ui.separator();
                if ui.button(SYSTEM).clicked() {
                    command = Some(Command::Choose(SYSTEM.into()));
                    ui.close();
                }
                if self.has_default && ui.button("Reset").clicked() {
                    command = Some(Command::ResetDefault);
                    ui.close();
                }
                if ui.add_enabled(named, egui::Button::new("Delete...")).clicked() {
                    self.delete = Some(self.selected.clone());
                    ui.close();
                }
                ui.separator();
                for name in self
                    .has_default
                    .then_some(DEFAULT.to_string())
                    .into_iter()
                    .chain(self.has_last.then_some(LAST.to_string()))
                    .chain(self.names.clone())
                {
                    if ui.selectable_label(self.selected == name, &name).clicked() {
                        command = Some(Command::Choose(name));
                        ui.close();
                    }
                }
            })
            .response
            .on_hover_text("Dialog Theme options");
        });
        if let Some(message) = &self.message {
            ui.colored_label(egui::Color32::from_rgb(190, 40, 40), message);
        }
        command
    }

    pub(super) fn prompt_ui(&mut self, ctx: &egui::Context) -> Option<Command> {
        let mut command = None;
        if let Some(mut name) = self.prompt.clone() {
            let mut close = false;
            egui::Window::new("Theme Save as...")
                .id(egui::Id::new("export_theme_save"))
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Theme Name");
                        let edit = ui.add(egui::TextEdit::singleline(&mut name).desired_width(260.0));
                        if std::mem::take(&mut self.focus_prompt) {
                            edit.request_focus();
                        }
                    });
                    let checked = clean_name(&name).and_then(|name| {
                        if name.eq_ignore_ascii_case(SYSTEM) {
                            Err(sheaf_io::themes::NameError::Reserved)
                        } else {
                            Ok(name)
                        }
                    });
                    if let Err(error) = &checked {
                        if !name.trim().is_empty() {
                            ui.label(error.to_string());
                        }
                    } else if checked
                        .as_ref()
                        .is_ok_and(|name| self.names.iter().any(|n| n.to_lowercase() == name.to_lowercase()))
                    {
                        ui.label("A theme of this name will be replaced.");
                    }
                    ui.horizontal(|ui| {
                        if ui.add_enabled(checked.is_ok(), egui::Button::new("OK")).clicked()
                            || (checked.is_ok() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                        {
                            command = checked.ok().map(Command::SaveAs);
                        }
                        if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                            close = true;
                        }
                    });
                });
            self.prompt = if close || command.is_some() { None } else { Some(name) };
        }
        if let Some(name) = self.delete.clone() {
            let mut close = false;
            egui::Window::new("Delete Dialog Theme")
                .id(egui::Id::new("export_theme_delete"))
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(format!("Delete '{name}'?"));
                    ui.horizontal(|ui| {
                        if ui.button("Delete").clicked() {
                            command = Some(Command::Delete(name.clone()));
                        }
                        if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                            close = true;
                        }
                    });
                });
            if close || command.is_some() {
                self.delete = None;
            }
        }
        command
    }
}

/// The controls a theme restores. Runtime state, preview textures and the active graph stay local.
#[derive(Clone, Debug, PartialEq)]
struct Settings {
    format: ExportFormat,
    which: Which,
    file_name: String,
    substitution: bool,
    path: String,
    overwrite: Overwrite,
    multipage: bool,
    dpi: f64,
    depth: ColorDepth,
    tiff_compression: TiffCompression,
    jpeg_quality: u8,
    width: Option<f64>,
    width_unit: SizeUnit,
    margin: Margin,
    border_px: Option<f64>,
}

macro_rules! fields {
    ($source:expr) => {
        Settings {
            format: $source.format,
            which: $source.which,
            file_name: $source.file_name.clone(),
            substitution: $source.substitution,
            path: $source.path.clone(),
            overwrite: $source.overwrite,
            multipage: $source.multipage,
            dpi: $source.dpi,
            depth: $source.depth,
            tiff_compression: $source.tiff_compression,
            jpeg_quality: $source.jpeg_quality,
            width: $source.width,
            width_unit: $source.width_unit,
            margin: $source.margin,
            border_px: $source.border_px,
        }
    };
}

fn choice<T: Copy + std::fmt::Debug>(v: &Value, choices: &[T]) -> Option<T> {
    let name = v.as_str()?;
    choices.iter().copied().find(|c| format!("{c:?}") == name)
}

fn number(v: &Value, min: f64, max: f64) -> Option<f64> {
    v.as_f64().filter(|v| v.is_finite() && (min..=max).contains(v))
}

fn optional_number(v: &Value, min: f64, max: f64) -> Option<Option<f64>> {
    if v.is_null() { Some(None) } else { number(v, min, max).map(Some) }
}

impl Settings {
    fn value(&self) -> Value {
        json!({
            "format": format!("{:?}", self.format), "which": format!("{:?}", self.which),
            "file_name": self.file_name, "substitution": self.substitution, "path": self.path,
            "overwrite": format!("{:?}", self.overwrite), "multipage": self.multipage,
            "dpi": self.dpi, "depth": format!("{:?}", self.depth),
            "tiff_compression": format!("{:?}", self.tiff_compression), "jpeg_quality": self.jpeg_quality,
            "width": self.width, "width_unit": format!("{:?}", self.width_unit),
            "margin": format!("{:?}", self.margin), "border_px": self.border_px,
        })
    }

    fn read(value: &Value, defaults: &Self) -> Result<Self, String> {
        let object = value.as_object().ok_or("The export theme is not a settings object.")?;
        let mut out = defaults.clone();
        macro_rules! read {
            ($field:ident, $parse:expr) => {
                if let Some(value) = object.get(stringify!($field)) {
                    out.$field = ($parse)(value)
                        .ok_or_else(|| format!("Invalid export theme setting: {}", stringify!($field)))?;
                }
            };
        }
        read!(format, |v| choice(v, &ExportFormat::ALL));
        read!(which, |v| choice(v, &[Which::ActivePage, Which::AllGraphs]));
        read!(file_name, |v: &Value| v.as_str().map(str::to_string));
        read!(substitution, Value::as_bool);
        read!(path, |v: &Value| v.as_str().map(str::to_string));
        read!(overwrite, |v| choice(v, &[Overwrite::Ask, Overwrite::Rename, Overwrite::Replace, Overwrite::Skip]));
        read!(multipage, Value::as_bool);
        read!(dpi, |v| number(v, 36.0, 2400.0));
        read!(depth, |v| choice(
            v,
            &[
                ColorDepth::Mono,
                ColorDepth::Colors16,
                ColorDepth::Colors256,
                ColorDepth::Bit16,
                ColorDepth::Bit24,
                ColorDepth::Grey8
            ]
        ));
        read!(tiff_compression, |v| choice(
            v,
            &[TiffCompression::None, TiffCompression::PackBits, TiffCompression::Lzw]
        ));
        read!(jpeg_quality, |v: &Value| v.as_u64().filter(|v| (1..=100).contains(v)).map(|v| v as u8));
        read!(width, |v| optional_number(v, 0.1, 20000.0));
        read!(width_unit, |v| choice(v, &[SizeUnit::Inch, SizeUnit::Cm, SizeUnit::Pixel, SizeUnit::PageRatio]));
        read!(margin, |v| choice(v, &[Margin::Auto, Margin::Border, Margin::Tight, Margin::Page, Margin::TightInPage]));
        read!(border_px, |v| optional_number(v, 0.0, 500.0));
        Ok(out)
    }

    fn apply(self, dialog: &mut ExportDialog) {
        macro_rules! set {
            ($($field:ident),*) => { $(dialog.$field = self.$field;)* };
        }
        set!(
            format,
            which,
            file_name,
            substitution,
            path,
            overwrite,
            multipage,
            dpi,
            depth,
            tiff_compression,
            jpeg_quality,
            width,
            width_unit,
            margin,
            border_px
        );
        dialog.pending = None;
        dialog.preview = None;
    }
}

impl ExportDialog {
    /// Uses the injected theme folder; a missing folder keeps the dialog free of settings IO.
    pub fn with_themes(mut self, dir: Option<PathBuf>) -> Self {
        self.themes.system = fields!(self).value();
        self.themes.dir = dir;
        self.themes.refresh();
        let initial = if self.themes.has_default {
            DEFAULT
        } else if self.themes.has_last {
            LAST
        } else {
            SYSTEM
        };
        self.themes.selected = initial.into();
        if self.themes.dir.is_some() {
            self.theme_command(Command::Choose(initial.into()));
        }
        self
    }

    fn load_theme(&mut self, entry: &str) -> Result<(), String> {
        let dir = self.themes.dir.as_ref().ok_or("No export theme folder.")?;
        let value = match entry {
            SYSTEM => self.themes.system.clone(),
            DEFAULT => ThemeStore::load(&dir.join(DEFAULT_FILE)).last_used.ok_or("No default export settings.")?,
            LAST => ThemeStore::load(&dir.join(FILE)).last_used.ok_or("No last-used export settings.")?,
            name => ThemeStore::load(&dir.join(FILE))
                .find(name)
                .map(|t| t.settings.clone())
                .ok_or_else(|| format!("No export theme named '{name}'."))?,
        };
        let defaults = Settings::read(&self.themes.system, &fields!(self))?;
        Settings::read(&value, &defaults)?.apply(self);
        self.themes.selected = entry.into();
        self.themes.loaded = Some(fields!(self));
        Ok(())
    }

    pub(super) fn theme_modified(&self) -> bool {
        self.themes.loaded.as_ref().is_some_and(|settings| *settings != fields!(self))
    }

    pub(super) fn theme_command(&mut self, command: Command) {
        let Some(dir) = self.themes.dir.clone() else { return };
        let result: Result<(), String> = (|| {
            match command {
                Command::Choose(entry) => self.load_theme(&entry)?,
                Command::SaveDefault => {
                    let value = fields!(self).value();
                    update(&dir.join(DEFAULT_FILE), |s| s.last_used = Some(value)).map_err(|e| e.to_string())?;
                    self.themes.selected = DEFAULT.into();
                    self.themes.loaded = Some(fields!(self));
                }
                Command::ResetDefault => {
                    update(&dir.join(DEFAULT_FILE), |s| s.last_used = None).map_err(|e| e.to_string())?;
                    self.load_theme(SYSTEM)?;
                }
                Command::Save | Command::SaveAs(_) => {
                    let name = match command {
                        Command::SaveAs(name) => name,
                        _ => self.themes.selected.clone(),
                    };
                    let name = clean_name(&name).map_err(|e| e.to_string())?;
                    if name.eq_ignore_ascii_case(SYSTEM) {
                        return Err("This name is reserved for the system settings.".into());
                    }
                    let value = fields!(self).value();
                    let (_, saved) = update(&dir.join(FILE), |s| s.put(&name, value)).map_err(|e| e.to_string())?;
                    self.themes.selected = saved.map_err(|e| e.to_string())?;
                    self.themes.loaded = Some(fields!(self));
                }
                Command::Delete(name) => {
                    update(&dir.join(FILE), |s| s.remove(&name)).map_err(|e| e.to_string())?;
                    self.load_theme(SYSTEM)?;
                }
            }
            Ok(())
        })();
        self.themes.message = result.err();
        self.themes.refresh();
    }

    pub(super) fn remember_export(&mut self, outcome: ExportOutcome) -> ExportOutcome {
        let Some(dir) = &self.themes.dir else { return outcome };
        let value = fields!(self).value();
        if let Err(error) = update(&dir.join(FILE), |s| s.last_used = Some(value)) {
            let warning = format!("The export settings could not be saved as {LAST}: {error}");
            self.themes.message = Some(warning.clone());
            return match outcome {
                ExportOutcome::Saved(path) => ExportOutcome::SavedWithNotes(path, vec![warning]),
                ExportOutcome::Applied(text) => ExportOutcome::Applied(format!("{text}; {warning}")),
                other => other,
            };
        }
        self.themes.message = None;
        if self.themes.selected == LAST {
            self.themes.loaded = Some(fields!(self));
        }
        self.themes.refresh();
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sheaf-export-themes-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn dialog(dir: &Path, graph: u64) -> ExportDialog {
        ExportDialog::new(WindowId(graph), Some(dir.join("output"))).with_themes(Some(dir.join("themes")))
    }

    #[test]
    fn named_export_themes_preserve_every_setting_and_other_sessions_themes() {
        let dir = folder("round-trip");
        let mut d = dialog(&dir, 7);
        d.format = ExportFormat::Tiff;
        d.which = Which::AllGraphs;
        d.file_name = "figure_<index>".into();
        d.substitution = false;
        d.path = dir.join("journal").display().to_string();
        d.overwrite = Overwrite::Rename;
        d.multipage = true;
        d.dpi = 600.0;
        d.depth = ColorDepth::Colors256;
        d.tiff_compression = TiffCompression::PackBits;
        d.jpeg_quality = 73;
        d.width = Some(6.35);
        d.width_unit = SizeUnit::Cm;
        d.margin = Margin::Border;
        d.border_px = Some(2.5);
        let expected = fields!(d);
        d.theme_command(Command::SaveAs("  Journal   figures  ".into()));
        assert!(d.themes.message.is_none());
        assert!(!dir.join("journal").exists(), "saving settings exported a graph");
        let mut other = dialog(&dir, 88);
        other.theme_command(Command::SaveAs("Other instance".into()));
        d.dpi = 900.0;
        assert!(d.theme_modified());
        d.theme_command(Command::Save);
        assert!(!d.theme_modified());
        assert_eq!(ThemeStore::load(&dir.join("themes").join(FILE)).names(), ["Journal figures", "Other instance"]);
        let mut reopened = dialog(&dir, 99);
        reopened.pending = Some(true);
        reopened.theme_command(Command::Choose("Journal figures".into()));
        let mut expected = expected;
        expected.dpi = 900.0;
        assert_eq!(fields!(reopened), expected);
        assert_eq!(reopened.graph, WindowId(99));
        assert!(reopened.pending.is_none());
        assert!(reopened.preview.is_none());
        assert!(reopened.themes.message.is_none());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn export_theme_load_is_atomic_and_missing_fields_use_system_settings() {
        let dir = folder("damaged");
        let mut d = dialog(&dir, 1);
        let path = dir.join("themes").join(FILE);
        update(&path, |s| {
            s.put("Bad", json!({"file_name":"changed", "dpi":0})).unwrap();
            s.put("Older", json!({"format":"Svg", "future_setting":true})).unwrap();
        })
        .unwrap();
        d.dpi = 600.0;
        d.width = Some(8.0);
        let before = fields!(d);
        d.theme_command(Command::Choose("Bad".into()));
        assert_eq!(fields!(d), before);
        assert!(d.themes.message.as_ref().unwrap().contains("dpi"));
        d.theme_command(Command::Choose("Older".into()));
        assert_eq!(d.format, ExportFormat::Svg);
        assert_eq!(d.dpi, 300.0);
        assert_eq!(d.width, None);
        assert_eq!(d.file_name, "<long name>");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn export_defaults_reset_independently_of_named_and_last_used_settings() {
        let dir = folder("defaults");
        let mut d = dialog(&dir, 1);
        d.format = ExportFormat::Svg;
        d.theme_command(Command::SaveAs("Vector".into()));
        d.theme_command(Command::SaveDefault);
        assert_eq!(dialog(&dir, 2).format, ExportFormat::Svg);
        d.format = ExportFormat::Pdf;
        let _ = d.remember_export(ExportOutcome::Applied("written".into()));
        assert_eq!(dialog(&dir, 3).format, ExportFormat::Svg, "saved default takes precedence");
        let bytes = std::fs::read(dir.join("themes").join(DEFAULT_FILE)).unwrap();
        d.theme_command(Command::Choose(SYSTEM.into()));
        assert_eq!(d.format, ExportFormat::Png);
        assert_eq!(std::fs::read(dir.join("themes").join(DEFAULT_FILE)).unwrap(), bytes);
        d.theme_command(Command::ResetDefault);
        let reopened = dialog(&dir, 4);
        assert_eq!(reopened.format, ExportFormat::Pdf);
        assert_eq!(reopened.themes.selected, LAST);
        assert_eq!(reopened.themes.names, ["Vector"]);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn only_successful_exports_record_last_used_and_storage_errors_keep_the_export() {
        let dir = folder("completed");
        let mut project = sheaf_core::Project::new();
        let graph = project.add_graph(sheaf_core::template::Template::Line);
        let doc = Document::new(project);
        let mut d = ExportDialog::new(graph, Some(dir.join("output"))).with_themes(Some(dir.join("themes")));
        d.format = ExportFormat::Svg;
        d.file_name = "paper".into();
        assert!(matches!(d.run(&doc, false, false), ExportOutcome::Applied(_)));
        let store = dir.join("themes").join(FILE);
        let saved = std::fs::read(&store).unwrap();
        d.dpi = 600.0;
        assert!(matches!(d.run(&doc, false, false), ExportOutcome::Open), "replacement must wait");
        assert_eq!(std::fs::read(&store).unwrap(), saved);
        d.pending = None;
        let blocked = dir.join("not-a-folder");
        std::fs::write(&blocked, b"block").unwrap();
        d.path = blocked.display().to_string();
        assert!(matches!(d.run(&doc, false, false), ExportOutcome::Failed(_)));
        assert_eq!(std::fs::read(&store).unwrap(), saved);
        d.path = dir.join("output").display().to_string();
        d.file_name = "still-written".into();
        d.themes.dir = Some(blocked);
        assert!(matches!(d.run(&doc, true, false), ExportOutcome::SavedWithNotes(_, notes) if notes.len() == 1));
        assert!(dir.join("output/still-written.svg").is_file());
        let _ = std::fs::remove_dir_all(dir);
    }
}
