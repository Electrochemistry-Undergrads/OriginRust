//! Peak Analyzer themes (T169): named Peak Analyzer settings, saved from the Peak Analyzer window and
//! used by Analysis > Peaks and Baseline > Batch Peak Analysis Using Theme.... One JSON file,
//! `peak-themes.json` next to the autosave settings ([`default_path`]), holding every theme. Sheaf's own
//! file kind; Origin's dialog themes are never read or written. Rules in `docs/spec/analysis-tools.md`,
//! "Peak Analyzer themes and batch peak analysis".

use crate::recovery;
use sheaf_core::analysis::PeakChainSpec;
use std::path::{Path, PathBuf};

/// The file name of the store, inside the settings folder.
pub const FILE_NAME: &str = "peak-themes.json";
/// The most characters a theme name keeps (Sheaf's choice).
pub const MAX_NAME_CHARS: usize = 60;

/// The store's file: `peak-themes.json` beside the autosave settings. `None` when the environment gives
/// no folder.
pub fn default_path() -> Option<PathBuf> {
    recovery::default_settings_path().and_then(|p| p.parent().map(|d| d.join(FILE_NAME)))
}

/// One saved theme.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PeakTheme {
    pub name: String,
    pub spec: PeakChainSpec,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
struct Store {
    #[serde(default)]
    themes: Vec<serde_json::Value>,
}

/// The name as stored: white space runs become one space, the ends are trimmed, control characters
/// go and the name is cut to [`MAX_NAME_CHARS`]. `None` for an empty name.
pub fn clean_name(raw: &str) -> Option<String> {
    let joined: String =
        raw.split_whitespace().collect::<Vec<_>>().join(" ").chars().filter(|c| !c.is_control()).collect();
    let cut: String = joined.chars().take(MAX_NAME_CHARS).collect();
    let cut = cut.trim().to_string();
    (!cut.is_empty()).then_some(cut)
}

fn read(path: &Path) -> Store {
    std::fs::read_to_string(path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

/// The saved themes in the order they were first saved. A missing or damaged file gives none; a theme
/// that cannot be read (written by a different version) is left out and hides no other.
pub fn list(path: &Path) -> Vec<PeakTheme> {
    read(path).themes.into_iter().filter_map(|v| serde_json::from_value(v).ok()).collect()
}

/// The names of the saved themes, in the order of [`list`].
pub fn names(path: &Path) -> Vec<String> {
    list(path).into_iter().map(|t| t.name).collect()
}

/// The theme of this name (letter case does not matter).
pub fn load(path: &Path, name: &str) -> Option<PeakTheme> {
    let key = clean_name(name)?.to_lowercase();
    list(path).into_iter().find(|t| t.name.to_lowercase() == key)
}

fn write(path: &Path, store: &Store) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let text = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}

/// Saves `spec` under `name` (cleaned). A theme of the same name (letter case does not matter) is
/// replaced in its place and takes the new spelling. Returns the stored name and whether one was
/// replaced. Themes this version cannot read are kept in the file as they are.
pub fn save(path: &Path, name: &str, spec: &PeakChainSpec) -> Result<(String, bool), String> {
    let name = clean_name(name).ok_or("a theme needs a name")?;
    let mut store = read(path);
    let key = name.to_lowercase();
    let value =
        serde_json::to_value(PeakTheme { name: name.clone(), spec: spec.clone() }).map_err(|e| e.to_string())?;
    let same = |v: &serde_json::Value| v.get("name").and_then(|n| n.as_str()).is_some_and(|n| n.to_lowercase() == key);
    let replaced = match store.themes.iter().position(same) {
        Some(i) => {
            store.themes[i] = value;
            true
        }
        None => {
            store.themes.push(value);
            false
        }
    };
    write(path, &store)?;
    Ok((name, replaced))
}

/// Deletes the theme of this name; false when there was none.
pub fn delete(path: &Path, name: &str) -> Result<bool, String> {
    let Some(name) = clean_name(name) else { return Ok(false) };
    let key = name.to_lowercase();
    let mut store = read(path);
    let before = store.themes.len();
    store.themes.retain(|v| !v.get("name").and_then(|n| n.as_str()).is_some_and(|n| n.to_lowercase() == key));
    if store.themes.len() == before {
        return Ok(false);
    }
    write(path, &store)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sheaf_core::analysis::{ChainBaseline, ChainOptions, FindMethod, LimitMode};

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sheaf-io-peak-themes-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join("settings").join(FILE_NAME)
    }

    fn spec(npts: usize) -> PeakChainSpec {
        let mut chain = ChainOptions { baseline: ChainBaseline::EndPoints, ..ChainOptions::default() };
        chain.find.npts = npts;
        chain.find.method = FindMethod::LocalMax;
        chain.limits = LimitMode::HalfHeight;
        PeakChainSpec { chain, integrate: true, fit: None, centers: Vec::new() }
    }

    #[test]
    fn themes_are_saved_listed_replaced_and_deleted() {
        let path = scratch("round");
        assert!(list(&path).is_empty(), "no file: no themes");
        assert_eq!(save(&path, "  XPS   scans ", &spec(3)).unwrap(), ("XPS scans".into(), false));
        save(&path, "Raman", &spec(5)).unwrap();
        assert_eq!(names(&path), ["XPS scans", "Raman"]);
        assert_eq!(load(&path, "xps SCANS").unwrap().spec, spec(3));
        assert_eq!(save(&path, "RAMAN", &spec(7)).unwrap(), ("RAMAN".into(), true));
        assert_eq!(names(&path), ["XPS scans", "RAMAN"], "replaced in its place");
        assert_eq!(load(&path, "raman").unwrap().spec.chain.find.npts, 7);
        assert!(delete(&path, "xps scans").unwrap());
        assert!(!delete(&path, "xps scans").unwrap());
        assert_eq!(names(&path), ["RAMAN"]);
        assert!(save(&path, "   ", &spec(1)).is_err());
        let _ = std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap());
    }

    #[test]
    fn an_unreadable_theme_hides_no_other_and_survives_a_save() {
        let path = scratch("damaged");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, r#"{"themes":[{"name":"Future","spec":{"new":1}}]}"#).unwrap();
        assert!(list(&path).is_empty());
        save(&path, "Mine", &spec(2)).unwrap();
        assert_eq!(names(&path), ["Mine"]);
        assert!(std::fs::read_to_string(&path).unwrap().contains("Future"), "the unknown theme is kept");
        std::fs::write(&path, "not json").unwrap();
        assert!(list(&path).is_empty());
        let _ = std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap());
    }
}
