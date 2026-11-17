//! Reusable opaque graph-color wells. Collection storage/UI are Sheaf policies (graphing 7.12).
use eframe::egui;
use sheaf_core::Color;
use std::cell::RefCell;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const MAX_COLORS: usize = 64;
const MAX_BYTES: u64 = 64 * 1024;
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Store {
    colors: Vec<Color>,
    file: Option<PathBuf>,
    failed: bool,
}
thread_local! {
    static STORE: RefCell<Store> =
        const { RefCell::new(Store { colors: Vec::new(), file: None, failed: false }) };
}

pub fn default_path() -> Option<PathBuf> {
    sheaf_io::recovery::default_settings_path().map(|p| p.with_file_name("custom-colors.json"))
}

pub fn initialize(file: Option<PathBuf>) {
    let colors = file.as_deref().map(load).unwrap_or_default();
    STORE.with(|s| *s.borrow_mut() = Store { colors, file, failed: false });
}

fn load(path: &Path) -> Vec<Color> {
    let parse = || -> Option<Vec<Color>> {
        let mut bytes = Vec::new();
        File::open(path).ok()?.take(MAX_BYTES + 1).read_to_end(&mut bytes).ok()?;
        if bytes.len() as u64 > MAX_BYTES {
            return None;
        }
        let mut v: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
        if v.get("version")?.as_u64() != Some(1) {
            return None;
        }
        let colors: Vec<Color> = serde_json::from_value(v.get_mut("colors")?.take()).ok()?;
        if colors.len() > MAX_COLORS || colors.iter().any(|c| c.a != 255) {
            return None;
        }
        let mut unique = Vec::new();
        for color in colors {
            if !unique.contains(&color) {
                unique.push(color);
            }
        }
        Some(unique)
    };
    parse().unwrap_or_default()
}

fn save(path: &Path, colors: &[Color]) -> io::Result<()> {
    let bytes = serde_json::to_vec(&serde_json::json!({"version":1,"colors":colors})).map_err(io::Error::other)?;
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    for _ in 0..16 {
        let seq = NEXT.fetch_add(1, Ordering::Relaxed);
        let tmp = path.with_extension(format!("{}.{seq}.tmp", std::process::id()));
        let mut file = match OpenOptions::new().create_new(true).write(true).open(&tmp) {
            Ok(file) => file,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        };
        let result = (|| {
            file.write_all(&bytes)?;
            file.sync_all()?;
            drop(file);
            fs::rename(&tmp, path)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&tmp);
        }
        return result;
    }
    Err(io::Error::other("no temporary custom-color file was available"))
}

fn colors() -> Vec<Color> {
    STORE.with(|s| s.borrow().colors.clone())
}

fn persist(s: &mut Store) {
    s.failed = s.file.as_deref().is_some_and(|p| save(p, &s.colors).is_err());
}

fn add(color: Color) -> bool {
    STORE.with(|s| {
        let mut s = s.borrow_mut();
        if color.a != 255 || s.colors.len() == MAX_COLORS || s.colors.contains(&color) {
            return false;
        }
        s.colors.push(color);
        persist(&mut s);
        true
    })
}

fn remove(color: Color) {
    STORE.with(|s| {
        let mut s = s.borrow_mut();
        let n = s.colors.len();
        s.colors.retain(|c| *c != color);
        if s.colors.len() != n {
            persist(&mut s);
        }
    });
}

fn error(ui: &mut egui::Ui) {
    if STORE.with(|s| s.borrow().failed) {
        ui.label("Custom colors are available for this session, but could not be saved.");
    }
}

pub fn add_button(ui: &mut egui::Ui, color: Option<Color>, prefix: &str) {
    let values = colors();
    let enabled = color.is_some_and(|c| c.a == 255 && values.len() < MAX_COLORS && !values.contains(&c));
    let response = ui.add_enabled(enabled, egui::Button::new("Add to Custom"));
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, format!("{prefix}: Add to Custom"))
    });
    if response.clicked()
        && let Some(color) = color
    {
        add(color);
    }
    error(ui);
}

pub fn menu(ui: &mut egui::Ui, prefix: &str) -> Option<Color> {
    let values = colors();
    let enabled = !values.is_empty();
    let mut chosen = None;
    let response = ui
        .add_enabled_ui(enabled, |ui| {
            let contents = |ui: &mut egui::Ui| {
                let mut removed = None;
                egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                    for color in values {
                        let response = ui
                            .horizontal(|ui| {
                                let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                                ui.painter().rect_filled(rect, 0.0, egui::Color32::from_rgb(color.r, color.g, color.b));
                                ui.button(format!("{prefix}: Custom {},{},{}", color.r, color.g, color.b))
                            })
                            .inner;
                        if response.clicked() {
                            chosen = Some(color);
                            ui.close();
                        }
                        // A memory-backed context popup replaces the toolbar's root popup.
                        // Keep this child open locally so its parent continues to be drawn.
                        let context_id = response.id.with("custom_color_remove");
                        let mut context_open = ui.data_mut(|d| d.get_temp::<bool>(context_id)).unwrap_or(false);
                        context_open |= response.secondary_clicked();
                        egui::Popup::context_menu(&response)
                            .open_bool(&mut context_open)
                            .at_position(response.rect.right_top())
                            .show(|ui| {
                                if ui.button("Remove custom color").clicked() {
                                    removed = Some(color);
                                    ui.close();
                                }
                            });
                        ui.data_mut(|d| d.insert_temp(context_id, context_open));
                    }
                });
                if let Some(color) = removed {
                    remove(color);
                    ui.close();
                }
                error(ui);
            };
            let label = format!("{prefix}: Custom colors");
            let config =
                egui::containers::menu::MenuConfig::new().close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside);
            if egui::containers::menu::is_in_menu(ui) {
                egui::containers::menu::SubMenuButton::new(label).config(config).ui(ui, contents).0
            } else {
                egui::containers::menu::MenuButton::new(label).config(config).ui(ui, contents).0
            }
        })
        .inner;
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, format!("{prefix}: Custom colors"))
    });
    chosen
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "sheaf-custom-colors-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir).unwrap();
        dir.join("custom-colors.json")
    }

    #[test]
    fn replacement_round_trip_preserves_order_and_leaves_no_temporary_files() {
        let path = file();
        for values in [vec![Color::RED, Color::BLUE], vec![Color::rgb(17, 131, 213)]] {
            save(&path, &values).unwrap();
            assert_eq!(load(&path), values);
            assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
        }
        fs::remove_file(&path).unwrap();
        fs::remove_dir(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn bad_records_are_bounded_and_never_rewritten_on_load() {
        let path = file();
        assert!(load(&path).is_empty());
        let mut alpha = Color::RED;
        alpha.a = 1;
        let bad = [
            b"broken".to_vec(),
            serde_json::to_vec(&serde_json::json!({"version":2,"colors":[Color::RED]})).unwrap(),
            serde_json::to_vec(&serde_json::json!({"version":1,"colors":[alpha]})).unwrap(),
            serde_json::to_vec(&serde_json::json!({"version":1,"colors":vec![Color::RED; MAX_COLORS+1]})).unwrap(),
        ];
        for bytes in bad {
            fs::write(&path, &bytes).unwrap();
            assert!(load(&path).is_empty());
            assert_eq!(fs::read(&path).unwrap(), bytes);
        }
        File::create(&path).unwrap().set_len(MAX_BYTES + 1).unwrap();
        assert!(load(&path).is_empty());
        assert_eq!(fs::metadata(&path).unwrap().len(), MAX_BYTES + 1);
        fs::remove_file(&path).unwrap();
        fs::remove_dir(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn capacity_duplicates_alpha_and_removal_preserve_the_collection_contract() {
        initialize(None);
        let mut alpha = Color::RED;
        alpha.a = 0;
        assert!(!add(alpha));
        for r in 0..MAX_COLORS as u8 {
            assert!(add(Color::rgb(r, 17, 213)));
        }
        assert!(!add(Color::rgb(0, 17, 213)));
        assert!(!add(Color::WHITE));
        let before = colors();
        remove(Color::WHITE);
        assert_eq!(colors(), before);
        remove(before[0]);
        assert!(add(Color::WHITE));
        assert_eq!(&colors()[..MAX_COLORS - 1], &before[1..]);
        assert_eq!(colors().last(), Some(&Color::WHITE));
        assert!(!STORE.with(|s| s.borrow().failed));
        initialize(None);
        assert!(colors().is_empty());
    }

    #[test]
    fn load_deduplicates_and_failed_save_retains_memory_and_the_existing_destination() {
        let path = file();
        save(&path, &[Color::BLUE, Color::RED, Color::BLUE]).unwrap();
        initialize(Some(path.clone()));
        assert_eq!(colors(), [Color::BLUE, Color::RED]);
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();
        assert!(add(Color::WHITE));
        assert_eq!(colors(), [Color::BLUE, Color::RED, Color::WHITE]);
        assert!(STORE.with(|s| s.borrow().failed));
        assert!(path.is_dir());
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
        fs::remove_dir(&path).unwrap();
        remove(Color::RED);
        assert!(!STORE.with(|s| s.borrow().failed));
        assert_eq!(load(&path), [Color::BLUE, Color::WHITE]);
        initialize(None);
        fs::remove_file(&path).unwrap();
        fs::remove_dir(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn concurrent_replacements_always_expose_one_complete_collection() {
        let path = file();
        let a = vec![Color::RED, Color::BLUE];
        let b = vec![Color::WHITE];
        save(&path, &a).unwrap();
        let workers: Vec<_> = [a.clone(), b.clone()]
            .into_iter()
            .map(|values| {
                let path = path.clone();
                std::thread::spawn(move || {
                    for _ in 0..12 {
                        save(&path, &values).unwrap();
                    }
                })
            })
            .collect();
        while workers.iter().any(|w| !w.is_finished()) {
            let loaded = load(&path);
            assert!(loaded == a || loaded == b);
        }
        for worker in workers {
            worker.join().unwrap();
        }
        let loaded = load(&path);
        assert!(loaded == a || loaded == b);
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
        fs::remove_file(&path).unwrap();
        fs::remove_dir(path.parent().unwrap()).unwrap();
    }
}
