//! Installed faces for worksheet cells. Kept separate from graph text parsing and layout.

use crate::SystemFonts;
use fontdb::{Family, Query, Stretch, Style, Weight};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// A face loaded from the system, never bundled in a project or distributed with Sheaf.
#[derive(Debug)]
pub struct WorksheetFont {
    pub data: Vec<u8>,
    pub index: u32,
    pub family: String,
    pub bold: bool,
    pub italic: bool,
    pub line_height_em: f64,
}

type Cache = HashMap<String, HashMap<(bool, bool), Option<Arc<WorksheetFont>>>>;

/// Resolve once per family/style, sharing bytes across cells, windows and egui contexts.
/// A missing family uses the system sans-serif face, as worksheet exports do.
pub fn worksheet_font(family: &str, bold: bool, italic: bool) -> Option<Arc<WorksheetFont>> {
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    let mut cache = CACHE.get_or_init(Mutex::default).lock().unwrap_or_else(|e| e.into_inner());
    if let Some(face) = cache.get(family).and_then(|styles| styles.get(&(bold, italic))) {
        return face.clone();
    }
    cache
        .entry(family.to_string())
        .or_default()
        .entry((bold, italic))
        .or_insert_with(|| {
            let db = SystemFonts::shared().database();
            let families = [Family::Name(family), Family::SansSerif];
            let id = db.query(&Query {
                families: &families,
                weight: if bold { Weight::BOLD } else { Weight::NORMAL },
                style: if italic { Style::Italic } else { Style::Normal },
                stretch: Stretch::Normal,
            })?;
            let face = db.face(id)?;
            db.with_face_data(id, |data, index| {
                Arc::new(WorksheetFont {
                    data: data.to_vec(),
                    index,
                    family: face.families.first().map_or_else(|| family.to_string(), |(n, _)| n.clone()),
                    bold: face.weight >= Weight::SEMIBOLD,
                    italic: face.style != Style::Normal,
                    line_height_em: ttf_parser::Face::parse(data, index).ok().map_or(1.2, |f| {
                        f64::from(i32::from(f.ascender()) - i32::from(f.descender()) + i32::from(f.line_gap()))
                            / f64::from(f.units_per_em())
                    }),
                })
            })
        })
        .clone()
}
