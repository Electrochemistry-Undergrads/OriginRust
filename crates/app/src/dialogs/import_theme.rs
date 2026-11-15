//! The Dialog Theme row at the top of the import windows (T46): a list of `<default>`,
//! `<last used>` and the saved themes, and a small menu with Save, Save As... and Delete. The
//! windows keep a [`ThemeRow`] and implement [`Themed`]; the stores are read when a window opens
//! and after a change, never per frame. Rules: `docs/spec/import-ascii.md`, "Dialog themes".

use eframe::egui::{self, RichText};
use sheaf_io::IoError;
use sheaf_io::themes::{self, DEFAULT_LABEL, LAST_USED_LABEL, ThemeError};
use std::path::{Path, PathBuf};

/// What the row asks the window to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThemeCommand {
    /// A list entry was chosen: fill the window from it.
    Choose(String),
    /// Save the settings under the name of the selected saved theme.
    Save,
    /// Save the settings under a new name (already checked by the prompt).
    SaveAs(String),
    /// Delete the selected saved theme.
    Delete,
}

/// The state of the row of one window.
#[derive(Default)]
pub struct ThemeRow {
    /// Tells this row's Save As window from the ones of the other import windows.
    salt: &'static str,
    /// The theme folder; `None` hides the row (tests, and runs with no settings folder).
    dir: Option<PathBuf>,
    /// The saved names, as the store listed them when the window opened or last changed.
    names: Vec<String>,
    /// The entry shown in the list.
    selected: String,
    /// The text of the Save As name prompt while it is open.
    prompt: Option<String>,
    /// The last thing that went wrong, shown under the row.
    message: Option<String>,
    /// Set when a prompt just opened, so that its box takes the keyboard.
    focus_prompt: bool,
}

impl ThemeRow {
    /// A row for the window that `salt` names; every import window that can be open together
    /// takes its own, so that their Save As windows have their own ids.
    pub fn new(salt: &'static str) -> Self {
        ThemeRow { salt, selected: DEFAULT_LABEL.to_string(), ..Default::default() }
    }

    /// The id of the Save As window of this row.
    pub fn prompt_id(&self) -> egui::Id {
        egui::Id::new(("import_theme_prompt", self.salt))
    }

    /// Closes the Save As window without an answer (the window changed page).
    pub fn close_prompt(&mut self) {
        self.prompt = None;
        self.focus_prompt = false;
    }

    /// Whether the Save As window is open (for tests).
    #[cfg(test)]
    pub fn prompt_open(&self) -> bool {
        self.prompt.is_some()
    }

    /// The entry shown in the list (for tests).
    #[cfg(test)]
    pub fn selected(&self) -> &str {
        &self.selected
    }

    /// The saved names in the list (for tests).
    #[cfg(test)]
    pub fn names(&self) -> &[String] {
        &self.names
    }

    /// The last message of the row (for tests).
    #[cfg(test)]
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    fn is_built_in(entry: &str) -> bool {
        entry == DEFAULT_LABEL || entry == LAST_USED_LABEL
    }

    /// Shows the row; the answer is what the user asked for, if anything.
    pub fn ui(&mut self, ui: &mut egui::Ui) -> Option<ThemeCommand> {
        self.dir.as_ref()?;
        let mut command = None;
        ui.horizontal(|ui| {
            ui.label("Dialog Theme");
            let entries: Vec<String> = [DEFAULT_LABEL.to_string(), LAST_USED_LABEL.to_string()]
                .into_iter()
                .chain(self.names.clone())
                .collect();
            egui::ComboBox::from_id_salt("import_theme_list")
                .selected_text(self.selected.clone())
                .width(240.0)
                .show_ui(ui, |ui| {
                    for entry in &entries {
                        if ui.selectable_label(*entry == self.selected, entry).clicked() {
                            command = Some(ThemeCommand::Choose(entry.clone()));
                        }
                    }
                });
            let saved = !Self::is_built_in(&self.selected);
            ui.menu_button("Manage", |ui| {
                if ui.add_enabled(true, egui::Button::new("Save")).clicked() {
                    if saved {
                        command = Some(ThemeCommand::Save);
                    } else {
                        self.open_prompt(String::new());
                    }
                    ui.close();
                }
                if ui.button("Save As...").clicked() {
                    let start = if saved { self.selected.clone() } else { String::new() };
                    self.open_prompt(start);
                    ui.close();
                }
                if ui.add_enabled(saved, egui::Button::new("Delete")).clicked() {
                    command = Some(ThemeCommand::Delete);
                    ui.close();
                }
            })
            .response
            .on_hover_text("Save, Save As and Delete");
        });
        if let Some(message) = &self.message {
            ui.colored_label(egui::Color32::from_rgb(190, 40, 40), message);
        }
        command.or_else(|| self.prompt_ui(ui.ctx()))
    }

    fn open_prompt(&mut self, text: String) {
        self.prompt = Some(text);
        self.focus_prompt = true;
    }

    /// The Save As window: a name box with Save and Cancel.
    fn prompt_ui(&mut self, ctx: &egui::Context) -> Option<ThemeCommand> {
        let text = self.prompt.clone()?;
        let mut text = text;
        let mut answer = None;
        let mut close = false;
        let checked = themes::clean_name(&text);
        egui::Window::new("Save Dialog Theme As")
            .id(self.prompt_id())
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Theme name");
                    let edit = ui.add(egui::TextEdit::singleline(&mut text).desired_width(220.0));
                    if std::mem::take(&mut self.focus_prompt) {
                        edit.request_focus();
                    }
                    if edit.lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                        && let Ok(name) = &checked
                    {
                        answer = Some(ThemeCommand::SaveAs(name.clone()));
                    }
                });
                match &checked {
                    Err(e) if !text.trim().is_empty() => {
                        ui.label(RichText::new(e.to_string()).weak());
                    }
                    Ok(name) if self.names.iter().any(|n| n.to_lowercase() == name.to_lowercase()) => {
                        ui.label(RichText::new("A theme of this name is replaced.").weak());
                    }
                    _ => {}
                }
                ui.horizontal(|ui| {
                    if ui.add_enabled(checked.is_ok(), egui::Button::new("Save")).clicked()
                        && let Ok(name) = &checked
                    {
                        answer = Some(ThemeCommand::SaveAs(name.clone()));
                    }
                    if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        close = true;
                    }
                });
            });
        if answer.is_some() || close {
            self.prompt = None;
        } else {
            self.prompt = Some(text);
        }
        answer
    }
}

/// A window that carries a theme row. The three `*_theme` methods say how its controls map to the
/// stores; the rest is shared.
pub trait Themed {
    fn theme_row(&mut self) -> &mut ThemeRow;
    fn theme_row_ref(&self) -> &ThemeRow;
    /// The saved names of the window's store.
    fn list_names(&self, dir: &Path) -> Vec<String>;
    /// Fills the controls (not the file list or the target) from a list entry, without changing
    /// anything on an error.
    fn apply_entry(&mut self, dir: &Path, entry: &str) -> Result<(), ThemeError>;
    /// Stores the controls under a name; returns the name as stored.
    fn store_as(&self, dir: &Path, name: &str) -> Result<String, ThemeError>;
    /// Removes a saved theme.
    fn remove_theme(&self, dir: &Path, name: &str) -> Result<(), IoError>;
    /// Stores the controls as `<last used>`.
    fn record_theme(&self, dir: &Path) -> Result<(), IoError>;

    /// Points the window at the theme folder and starts it from `<last used>` when there is one,
    /// else from `<default>` (Sheaf's choice).
    fn attach_themes(&mut self, dir: Option<PathBuf>) {
        let Some(dir) = dir else {
            let salt = self.theme_row().salt;
            *self.theme_row() = ThemeRow::new(salt);
            return;
        };
        let names = self.list_names(&dir);
        let selected = if self.apply_entry(&dir, LAST_USED_LABEL).is_ok() { LAST_USED_LABEL } else { DEFAULT_LABEL };
        let row = self.theme_row();
        *row = ThemeRow::new(row.salt);
        row.dir = Some(dir);
        row.names = names;
        row.selected = selected.to_string();
    }

    /// Does what the row asked for.
    fn run_theme_command(&mut self, command: ThemeCommand) {
        let Some(dir) = self.theme_row_ref().dir.clone() else { return };
        self.theme_row().message = None;
        let outcome: Result<Option<String>, String> = match command {
            ThemeCommand::Choose(entry) => match self.apply_entry(&dir, &entry) {
                Ok(()) => Ok(Some(entry)),
                // Nothing has been confirmed yet: `<last used>` is the defaults.
                Err(ThemeError::NoLastUsed) => {
                    self.apply_entry(&dir, DEFAULT_LABEL).map(|()| Some(entry)).map_err(|e| e.to_string())
                }
                Err(e) => Err(e.to_string()),
            },
            ThemeCommand::Save => {
                let name = self.theme_row_ref().selected.clone();
                self.store_as(&dir, &name).map(Some).map_err(|e| e.to_string())
            }
            ThemeCommand::SaveAs(name) => self.store_as(&dir, &name).map(Some).map_err(|e| e.to_string()),
            ThemeCommand::Delete => {
                let name = self.theme_row_ref().selected.clone();
                match self.remove_theme(&dir, &name) {
                    Ok(()) => self
                        .apply_entry(&dir, DEFAULT_LABEL)
                        .map(|()| Some(DEFAULT_LABEL.to_string()))
                        .map_err(|e| e.to_string()),
                    Err(e) => Err(e.to_string()),
                }
            }
        };
        // The list is read again whatever happened: another instance may have changed it.
        let names = self.list_names(&dir);
        let row = self.theme_row();
        row.names = names;
        match outcome {
            Ok(Some(selected)) => row.selected = selected,
            Ok(None) => {}
            Err(e) => row.message = Some(e),
        }
    }

    /// OK was pressed: remember the settings as `<last used>`. A failure is returned as text.
    fn record_last_used(&mut self) -> Option<String> {
        let dir = self.theme_row_ref().dir.clone()?;
        self.record_theme(&dir).err().map(|e| format!("The import settings could not be saved as <last used>: {e}"))
    }
}
