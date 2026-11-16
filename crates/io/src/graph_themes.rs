//! Graph themes (T102): formats saved with Save Format as Theme... and listed by Tools > Theme Organizer,
//! as Origin 2016's user graph themes. One JSON file per theme, `graph-themes/<name>.json` in the
//! settings folder ([`default_dir`]), holding the copied format (`sheaf_core::format_clip::FormatClip`),
//! the description and the time it was saved. Sheaf's own file kind; Origin's `.oth` files are never
//! read or written. Rules in `docs/spec/graphing.md`, "Graph themes".

use crate::recovery;
use crate::user_templates::{NameError, clean_name};
use sheaf_core::format_clip::FormatClip;
use std::path::{Path, PathBuf};

const FOLDER_NAME: &str = "graph-themes";
const EXT: &str = "json";

/// The folder of the theme files: `graph-themes` next to the autosave settings. `None` when the
/// environment gives no folder.
pub fn default_dir() -> Option<PathBuf> {
    recovery::default_settings_path().and_then(|p| p.parent().map(|d| d.join(FOLDER_NAME)))
}

/// A saved theme.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GraphTheme {
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// When it was saved, `YYYY-MM-DD HH:MM` (UTC).
    #[serde(default)]
    pub saved: String,
    pub format: FormatClip,
}

/// One theme of the folder as the Theme Organizer lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThemeEntry {
    pub name: String,
    pub path: PathBuf,
    /// The size of the file in bytes.
    pub size: u64,
    pub saved: String,
}

fn fold(s: &str) -> String {
    s.to_lowercase()
}

/// The themes of `dir`, by name (letter case ignored); files that cannot be read are left out.
pub fn list(dir: &Path) -> Vec<ThemeEntry> {
    let Ok(read) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<ThemeEntry> = read
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == EXT))
        .filter_map(|p| {
            let theme = load_path(&p).ok()?;
            let size = std::fs::metadata(&p).map_or(0, |m| m.len());
            Some(ThemeEntry { name: theme.name, path: p, size, saved: theme.saved })
        })
        .collect();
    out.sort_by(|a, b| fold(&a.name).cmp(&fold(&b.name)).then_with(|| a.name.cmp(&b.name)));
    out
}

/// The first `ThemeN` name not in use, as Origin's window offers ("Theme1").
pub fn next_name(dir: &Path) -> String {
    let taken: Vec<String> = list(dir).into_iter().map(|e| fold(&e.name)).collect();
    (1..).map(|n| format!("Theme{n}")).find(|n| !taken.contains(&fold(n))).expect("unbounded")
}

fn load_path(path: &Path) -> Result<GraphTheme, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// The theme of this name (letter case ignored).
pub fn load(dir: &Path, name: &str) -> Result<GraphTheme, String> {
    let key = fold(name.trim());
    let entry =
        list(dir).into_iter().find(|e| fold(&e.name) == key).ok_or_else(|| format!("there is no theme {name}"))?;
    load_path(&entry.path)
}

/// Saves `theme` under its (cleaned) name, replacing a theme of the same name. Returns the file.
pub fn save(dir: &Path, mut theme: GraphTheme) -> Result<PathBuf, String> {
    let name = clean_name(&theme.name).map_err(|e: NameError| e.to_string().replace("template", "theme"))?;
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for old in list(dir).into_iter().filter(|e| fold(&e.name) == fold(&name)) {
        let _ = std::fs::remove_file(old.path);
    }
    theme.name = name.clone();
    if theme.saved.is_empty() {
        theme.saved = now();
    }
    let path = dir.join(format!("{name}.{EXT}"));
    let text = serde_json::to_string_pretty(&theme).map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}

/// Deletes the theme of this name; false when there is none.
pub fn delete(dir: &Path, name: &str) -> bool {
    let key = fold(name);
    list(dir).into_iter().filter(|e| fold(&e.name) == key).fold(false, |_, e| std::fs::remove_file(e.path).is_ok())
}

/// `YYYY-MM-DD HH:MM` (UTC) of the system clock.
fn now() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs()) as i64;
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02} {:02}:{:02}", rem / 3600, rem % 3600 / 60)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sheaf_core::format_clip::{AxisPart, copy_axis};
    use sheaf_core::template::Template;

    fn dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("sheaf-graph-themes-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn axis_format() -> FormatClip {
        let g = Template::Line.new_graph(sheaf_core::WindowId(1), "Graph1");
        copy_axis(&g, 0, true, AxisPart::All).unwrap()
    }

    #[test]
    fn themes_are_saved_listed_replaced_and_deleted() {
        let d = dir("crud");
        assert_eq!(next_name(&d), "Theme1");
        let theme = GraphTheme {
            name: "Theme1".into(),
            description: "thick axes".into(),
            saved: String::new(),
            format: axis_format(),
        };
        let path = save(&d, theme.clone()).unwrap();
        assert!(path.ends_with("Theme1.json"));
        assert_eq!(next_name(&d), "Theme2");
        let entries = list(&d);
        assert_eq!(entries.len(), 1);
        assert!(entries[0].size > 0 && entries[0].saved.len() == 16, "{:?}", entries[0]);
        let back = load(&d, "theme1").unwrap();
        assert_eq!((back.description.as_str(), &back.format), ("thick axes", &theme.format));
        // Saving the same name (other case) replaces it.
        save(&d, GraphTheme { name: "THEME1".into(), ..theme }).unwrap();
        assert_eq!(list(&d).len(), 1);
        assert!(delete(&d, "Theme1"));
        assert!(list(&d).is_empty());
        assert!(
            save(
                &d,
                GraphTheme {
                    name: "  ".into(),
                    description: String::new(),
                    saved: String::new(),
                    format: axis_format()
                }
            )
            .is_err()
        );
        let _ = std::fs::remove_dir_all(&d);
    }
}
