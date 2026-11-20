//! Saved import settings ("Dialog Themes"): the settings of the last import and named sets of
//! settings, kept per importer kind in the settings folder so that they survive a restart and are
//! shared by every running instance and by the command line.
//!
//! One file per kind, `import-themes/<kind>.json` (see [`Kind`] and [`default_dir`]). A [`ThemeStore`]
//! holds the `<last used>` settings and the named themes. The settings are kept as JSON values, so a
//! theme written by a newer Sheaf (more fields) still loads, a theme from an older one (fewer fields)
//! is filled from the defaults, and one theme that cannot be read does not hide the others. Changes go
//! through [`update`], which takes a lock, re-reads the file, applies the change and writes the file
//! again, so two instances never wipe each other's themes.
//!
//! The typed settings of the three windows are [`AsciiSettings`], [`ChiSettings`] and
//! [`ExcelSettings`]; the functions that take a folder ([`last_used`], [`save_theme`], [`resolve`],
//! ...) work on any [`Settings`] type. The rules are in `docs/spec/import-ascii.md`, "Dialog themes".

use crate::batch::ImportMode;
use crate::chi::ChiOptions;
use crate::error::IoError;
use crate::excel::ExcelOptions;
use crate::{ImportKind, ascii, recent, recovery, save};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// The name of the built-in settings in the theme list.
pub const DEFAULT_LABEL: &str = "<default>";
/// The name of the settings of the last OK in the theme list.
pub const LAST_USED_LABEL: &str = "<last used>";
/// The name of the folder, inside the settings folder, that holds the files.
pub const FOLDER_NAME: &str = "import-themes";
/// The most characters a theme name keeps (Sheaf's choice, not measured in Origin).
pub const MAX_NAME_CHARS: usize = 60;

/// The folder of the theme files: `import-themes` in the folder of the autosave settings
/// (`recovery::default_settings_path`). `None` when the environment gives no folder.
pub fn default_dir() -> Option<PathBuf> {
    recovery::default_settings_path().and_then(|p| p.parent().map(|d| d.join(FOLDER_NAME)))
}

/// The importer a store belongs to. Each kind has its own file and its own themes.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    /// The text import window.
    Ascii,
    /// The CH Instruments options.
    Chi,
    /// The Excel import window.
    Excel,
}

impl Kind {
    pub const ALL: [Kind; 3] = [Kind::Ascii, Kind::Chi, Kind::Excel];

    /// The word that names the kind on the command line and in the file name.
    pub fn key(self) -> &'static str {
        match self {
            Kind::Ascii => "ascii",
            Kind::Chi => "chi",
            Kind::Excel => "excel",
        }
    }

    /// The file of this kind in the theme folder `dir`.
    pub fn file(self, dir: &Path) -> PathBuf {
        dir.join(format!("{}.json", self.key()))
    }
}

/// The settings of one import window, as the theme stores them. Every field has a default, so a
/// theme from an older Sheaf loads.
pub trait Settings: Serialize + DeserializeOwned + Default + Clone {
    /// The store these settings belong to.
    const KIND: Kind;

    /// The copy that is stored: leaves out what belongs to one run and not to a theme (the file
    /// list and the target are not here at all; this clears the rest).
    fn for_storage(&self) -> Self {
        self.clone()
    }
}

/// What the text import window keeps (everything except the file list and the target sheet).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AsciiSettings {
    /// Detect, CH Instruments or text.
    pub data_type: ImportKind,
    pub ascii: ascii::AsciiOptions,
    /// Apply the partial-import selection to every file but the first.
    pub partial_except_first: bool,
    /// The text of the "Other" delimiter box.
    pub other_delimiter: String,
    /// Remembers the combine choice while switching between a delimiter set and a whole string.
    pub combine_delimiters: bool,
    /// The text of the fixed column widths box.
    pub fixed_widths: String,
    pub mode: ImportMode,
    /// Make a graph of every imported file.
    pub plot: bool,
}

impl Default for AsciiSettings {
    fn default() -> Self {
        AsciiSettings {
            data_type: ImportKind::Auto,
            ascii: ascii::AsciiOptions::default(),
            partial_except_first: false,
            other_delimiter: "|".to_string(),
            combine_delimiters: false,
            fixed_widths: "10".to_string(),
            mode: ImportMode::NewBook,
            plot: true,
        }
    }
}

impl Settings for AsciiSettings {
    const KIND: Kind = Kind::Ascii;
}

/// What the CH Instruments options keep.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ChiSettings {
    pub chi: ChiOptions,
    pub mode: ImportMode,
    /// Make a graph of every imported file.
    pub plot: bool,
}

impl Default for ChiSettings {
    fn default() -> Self {
        ChiSettings { chi: ChiOptions::default(), mode: ImportMode::NewBook, plot: true }
    }
}

impl Settings for ChiSettings {
    const KIND: Kind = Kind::Chi;
}

/// What the Excel import window keeps. The ticked sheets belong to the files and are not kept.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExcelSettings {
    /// `options.sheets` is always empty here.
    pub options: ExcelOptions,
    /// Apply the partial-import selection to every file but the first.
    pub partial_except_first: bool,
    pub mode: ImportMode,
    /// Make a graph of the first sheet of each file.
    pub plot: bool,
}

impl Default for ExcelSettings {
    fn default() -> Self {
        ExcelSettings {
            options: ExcelOptions::default(),
            partial_except_first: false,
            mode: ImportMode::NewBook,
            plot: false,
        }
    }
}

impl Settings for ExcelSettings {
    const KIND: Kind = Kind::Excel;

    fn for_storage(&self) -> Self {
        let mut s = self.clone();
        s.options.sheets.clear();
        s
    }
}

/// Why a theme name is refused.
#[derive(Copy, Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum NameError {
    #[error("a theme needs a name")]
    Empty,
    #[error("names in angle brackets, such as <default> and <last used>, are the built-in entries")]
    Reserved,
}

/// What went wrong with a theme.
#[derive(Debug, thiserror::Error)]
pub enum ThemeError {
    #[error(transparent)]
    Name(#[from] NameError),
    #[error(transparent)]
    Io(#[from] IoError),
    /// No theme has this name; `available` lists the saved ones.
    #[error("no import theme named '{name}'{}", list_saved(.available))]
    Unknown { name: String, available: Vec<String> },
    /// `<last used>` was asked for before any import has been confirmed.
    #[error("no <last used> import settings are saved yet")]
    NoLastUsed,
    /// The theme exists but its settings cannot be read (for example written by a newer Sheaf).
    #[error("the import theme '{name}' cannot be read")]
    Unreadable { name: String },
}

fn list_saved(names: &[String]) -> String {
    if names.is_empty() { "; no themes are saved".to_string() } else { format!("; saved themes: {}", names.join(", ")) }
}

/// Makes a name fit: control characters go, runs of white space become one space, the ends are
/// trimmed and the name is cut to [`MAX_NAME_CHARS`]. An empty result and a name in angle brackets
/// (the built-in entries) are refused. Two names are the same theme when this form matches in lower case.
pub fn clean_name(raw: &str) -> Result<String, NameError> {
    let mut out = String::new();
    let mut pending_space = false;
    for c in raw.chars() {
        if c.is_whitespace() {
            pending_space = !out.is_empty();
        } else if !c.is_control() {
            if pending_space {
                out.push(' ');
                pending_space = false;
            }
            out.push(c);
        }
    }
    let mut out: String = out.chars().take(MAX_NAME_CHARS).collect();
    out.truncate(out.trim_end().len());
    if out.is_empty() {
        return Err(NameError::Empty);
    }
    if out.starts_with('<') && out.ends_with('>') {
        return Err(NameError::Reserved);
    }
    Ok(out)
}

fn fold(name: &str) -> String {
    name.to_lowercase()
}

/// One named theme.
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    pub name: String,
    /// The settings as JSON (an object); [`Settings`] types read it with defaults for missing fields.
    pub settings: Value,
}

/// The `<last used>` settings and the named themes of one kind.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ThemeStore {
    /// The settings of the last OK, or `None` before the first.
    pub last_used: Option<Value>,
    /// The saved themes in the order they were first saved; no two names match when folded to lower case.
    pub themes: Vec<Theme>,
}

impl ThemeStore {
    /// Reads the file. A missing, unreadable or damaged file gives an empty store; a damaged theme is
    /// skipped and the others are kept; fields this version does not know are ignored.
    pub fn load(path: &Path) -> ThemeStore {
        std::fs::read(path).ok().and_then(|b| parse(&b)).unwrap_or_default()
    }

    /// Writes the file (creating the folder) without taking the lock; use [`update`] to change the
    /// file while other instances run.
    pub fn save(&self, path: &Path) -> Result<(), IoError> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|e| IoError::io(parent, e))?;
        }
        let mut map = serde_json::Map::new();
        map.insert("version".into(), Value::from(VERSION));
        if let Some(last) = &self.last_used {
            map.insert("last_used".into(), last.clone());
        }
        let themes = self
            .themes
            .iter()
            .map(|t| serde_json::json!({ "name": t.name, "settings": t.settings }))
            .collect::<Vec<_>>();
        map.insert("themes".into(), Value::Array(themes));
        let json = serde_json::to_vec_pretty(&Value::Object(map)).map_err(|e| IoError::Json {
            path: path.to_path_buf(),
            member: "import themes".into(),
            source: e,
        })?;
        save::write_atomically(path, &json)
    }

    /// The names of the saved themes, in order.
    pub fn names(&self) -> Vec<String> {
        self.themes.iter().map(|t| t.name.clone()).collect()
    }

    /// The theme with this name (letter case does not matter).
    pub fn find(&self, name: &str) -> Option<&Theme> {
        let key = fold(&clean_name(name).ok()?);
        self.themes.iter().find(|t| fold(&t.name) == key)
    }

    /// Saves a theme under `name`. A theme of the same name (letter case does not matter) is replaced
    /// in its place and takes the new spelling. Returns the name as stored.
    pub fn put(&mut self, name: &str, settings: Value) -> Result<String, NameError> {
        let name = clean_name(name)?;
        let key = fold(&name);
        match self.themes.iter_mut().find(|t| fold(&t.name) == key) {
            Some(t) => {
                t.name.clone_from(&name);
                t.settings = settings;
            }
            None => self.themes.push(Theme { name: name.clone(), settings }),
        }
        Ok(name)
    }

    /// Removes a theme; false when there was none of that name.
    pub fn remove(&mut self, name: &str) -> bool {
        let Ok(name) = clean_name(name) else { return false };
        let key = fold(&name);
        let before = self.themes.len();
        self.themes.retain(|t| fold(&t.name) != key);
        self.themes.len() != before
    }
}

/// The layout version this Sheaf writes.
const VERSION: u64 = 1;

/// Reads the bytes of a theme file; `None` when they are not a JSON object or when `themes` is
/// there but is not a list.
fn parse(bytes: &[u8]) -> Option<ThemeStore> {
    let Value::Object(map) = serde_json::from_slice::<Value>(bytes).ok()? else { return None };
    if map.get("themes").is_some_and(|t| !t.is_array()) {
        return None;
    }
    let mut store =
        ThemeStore { last_used: map.get("last_used").filter(|v| v.is_object()).cloned(), themes: Vec::new() };
    if let Some(Value::Array(items)) = map.get("themes") {
        for item in items {
            let (Some(name), Some(settings)) = (item.get("name").and_then(Value::as_str), item.get("settings")) else {
                continue;
            };
            if settings.is_object() {
                // Names that do not fit or repeat an earlier one are dropped.
                if store.find(name).is_none() {
                    let _ = store.put(name, settings.clone());
                }
            }
        }
    }
    Some(store)
}

/// Whether the file says it was written by a newer layout than this Sheaf writes.
fn is_newer_layout(bytes: &[u8]) -> bool {
    serde_json::from_slice::<Value>(bytes)
        .ok()
        .and_then(|v| v.get("version").and_then(Value::as_u64))
        .is_some_and(|v| v > VERSION)
}

/// Keeps the file as `<name>.bad` before it is written again.
fn keep_as_bad(path: &Path) {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let _ = std::fs::rename(path, path.with_file_name(format!("{name}.bad")));
}

/// Changes the store in the file at `path`: waits for the file lock (up to ten seconds, then goes on
/// without it), reads the file as it is now, applies `change`, writes the file and returns the new
/// store. Themes another instance added in the meantime are kept. A missing file is an empty store. A
/// file that is damaged, or that a newer layout wrote (`version` above 1), is kept as `<name>.bad`
/// before the new one is written. A file that cannot be read for another reason (no permission, a
/// sharing violation) is an error and is left as it is.
pub fn update<T>(path: &Path, change: impl FnOnce(&mut ThemeStore) -> T) -> Result<(ThemeStore, T), IoError> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|e| IoError::io(parent, e))?;
    }
    let _lock = recent::lock(path);
    let mut store = match std::fs::read(path) {
        Ok(bytes) => {
            let parsed = parse(&bytes);
            if parsed.is_none() || is_newer_layout(&bytes) {
                keep_as_bad(path);
            }
            parsed.unwrap_or_default()
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => ThemeStore::default(),
        Err(e) => return Err(IoError::io(path, e)),
    };
    let out = change(&mut store);
    store.save(path)?;
    Ok((store, out))
}

fn to_value<S: Settings>(settings: &S) -> Value {
    serde_json::to_value(settings.for_storage()).unwrap_or(Value::Null)
}

fn from_value<S: Settings>(value: &Value) -> Option<S> {
    serde_json::from_value(value.clone()).ok()
}

/// The `<last used>` settings, or `None` when there are none or they cannot be read.
pub fn last_used<S: Settings>(dir: &Path) -> Option<S> {
    let store = ThemeStore::load(&S::KIND.file(dir));
    store.last_used.as_ref().and_then(from_value)
}

/// Records the settings of an OK as `<last used>`.
pub fn record_last_used<S: Settings>(dir: &Path, settings: &S) -> Result<(), IoError> {
    let value = to_value(settings);
    update(&S::KIND.file(dir), |s| s.last_used = Some(value)).map(|_| ())
}

/// The names of the saved themes of this kind.
pub fn names<S: Settings>(dir: &Path) -> Vec<String> {
    ThemeStore::load(&S::KIND.file(dir)).names()
}

/// A saved theme, or `None` when there is none of that name or it cannot be read.
pub fn theme<S: Settings>(dir: &Path, name: &str) -> Option<S> {
    let store = ThemeStore::load(&S::KIND.file(dir));
    store.find(name).and_then(|t| from_value(&t.settings))
}

/// Saves the settings as a named theme (Save and Save As). Returns the name as stored.
pub fn save_theme<S: Settings>(dir: &Path, name: &str, settings: &S) -> Result<String, ThemeError> {
    let name = clean_name(name)?;
    let value = to_value(settings);
    let (_, stored) = update(&S::KIND.file(dir), |s| s.put(&name, value))?;
    Ok(stored?)
}

/// Deletes a theme; false when there was none of that name.
pub fn delete_theme<S: Settings>(dir: &Path, name: &str) -> Result<bool, IoError> {
    update(&S::KIND.file(dir), |s| s.remove(name)).map(|(_, removed)| removed)
}

/// Finds the settings that a theme list entry stands for: `<default>` gives the defaults,
/// `<last used>` the settings of the last OK, any other text a saved theme (letter case does not
/// matter). The error for an unknown name lists the saved themes.
pub fn resolve<S: Settings>(dir: &Path, entry: &str) -> Result<S, ThemeError> {
    let wanted = entry.trim();
    if wanted.eq_ignore_ascii_case(DEFAULT_LABEL) {
        return Ok(S::default());
    }
    let store = ThemeStore::load(&S::KIND.file(dir));
    if wanted.eq_ignore_ascii_case(LAST_USED_LABEL) {
        let value = store.last_used.as_ref().ok_or(ThemeError::NoLastUsed)?;
        return from_value(value).ok_or(ThemeError::NoLastUsed);
    }
    match store.find(wanted) {
        Some(t) => from_value(&t.settings).ok_or_else(|| ThemeError::Unreadable { name: t.name.clone() }),
        None => Err(ThemeError::Unknown { name: wanted.to_string(), available: store.names() }),
    }
}

#[cfg(test)]
mod tests;
