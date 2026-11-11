//! Text measurement and the system font database (spec section 7).

use fontdb::{Database, Family, ID, Query, Stretch, Style, Weight};
use sheaf_core::graph::Font;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// Families tried after the one the model asks for, in this order.
pub const FALLBACK_FAMILIES: [&str; 3] = ["Liberation Sans", "Helvetica", "DejaVu Sans"];

/// Measures the width of text. Layout needs widths to right-align and
/// centre text; the measurer is a parameter so tests can use fixed metrics.
pub trait TextMeasurer {
    /// Width in points of `text` set in `font` (family, bold, italic) at `size_pt`.
    fn width(&self, text: &str, font: &Font, size_pt: f64) -> f64;
}

/// Every character is `0.55 * size` wide. Deterministic and independent of
/// the installed fonts, so it is what tests use.
#[derive(Clone, Copy, Debug, Default)]
pub struct FixedMetrics;

impl TextMeasurer for FixedMetrics {
    fn width(&self, text: &str, _font: &Font, size_pt: f64) -> f64 {
        text.chars().count() as f64 * 0.55 * size_pt
    }
}

/// The CSS `font-family` value for text set in `primary`: the model's
/// family first, then the fallbacks and the generic `sans-serif`.
pub fn css_family(primary: &str) -> String {
    let mut out = String::new();
    let mut seen: Vec<String> = Vec::new();
    let names = std::iter::once(primary).chain(FALLBACK_FAMILIES);
    for name in names {
        let name = name.trim();
        if name.is_empty() || seen.iter().any(|s| s.eq_ignore_ascii_case(name)) {
            continue;
        }
        seen.push(name.to_string());
        if !out.is_empty() {
            out.push_str(", ");
        }
        out.push('\'');
        out.push_str(&name.replace('\'', ""));
        out.push('\'');
    }
    if !out.is_empty() {
        out.push_str(", ");
    }
    out.push_str("sans-serif");
    out
}

#[derive(Default)]
struct Cache {
    faces: HashMap<(String, bool, bool), Option<ID>>,
    upem: HashMap<ID, f64>,
    advances: HashMap<(ID, char), Option<f64>>,
}

/// The installed fonts, loaded once, plus a width measurer on top of them.
///
/// Widths come from the advance widths of the first installed family of the
/// request (the model's family, then [`FALLBACK_FAMILIES`]), bold and italic
/// aware when such a face exists. Kerning is not applied.
pub struct SystemFonts {
    db: Arc<Database>,
    cache: Mutex<Cache>,
}

impl SystemFonts {
    /// The process-wide instance. System fonts are loaded on first use.
    pub fn shared() -> &'static SystemFonts {
        static SHARED: OnceLock<SystemFonts> = OnceLock::new();
        SHARED.get_or_init(|| {
            let mut db = Database::new();
            db.load_system_fonts();
            SystemFonts::from_database(db)
        })
    }

    /// Wraps an existing database (for example one holding only test fonts).
    pub fn from_database(mut db: Database) -> SystemFonts {
        // The generic `sans-serif` should land on a family that exists.
        let has = |db: &Database, name: &str| {
            db.faces().any(|f| f.families.iter().any(|(n, _)| n.eq_ignore_ascii_case(name)))
        };
        for name in ["Arial", "Liberation Sans", "Helvetica", "DejaVu Sans"] {
            if has(&db, name) {
                db.set_sans_serif_family(name);
                break;
            }
        }
        SystemFonts { db: Arc::new(db), cache: Mutex::new(Cache::default()) }
    }

    /// The font database, for the rasteriser.
    pub fn database(&self) -> &Arc<Database> {
        &self.db
    }

    fn resolve(&self, cache: &mut Cache, font: &Font) -> Option<ID> {
        let key = (font.family.clone(), font.bold, font.italic);
        if let Some(found) = cache.faces.get(&key) {
            return *found;
        }
        let weight = if font.bold { Weight::BOLD } else { Weight::NORMAL };
        let style = if font.italic { Style::Italic } else { Style::Normal };
        let candidates = std::iter::once(font.family.as_str()).chain(FALLBACK_FAMILIES);
        let mut found = None;
        for name in candidates {
            let families = [Family::Name(name)];
            let q = Query { families: &families, weight, stretch: Stretch::Normal, style };
            if let Some(id) = self.db.query(&q) {
                found = Some(id);
                break;
            }
        }
        if found.is_none() {
            let families = [Family::SansSerif];
            found = self.db.query(&Query { families: &families, weight, stretch: Stretch::Normal, style });
        }
        cache.faces.insert(key, found);
        found
    }

    /// Advance of `c` in em units, or `None` when the face lacks the glyph.
    fn advance(&self, cache: &mut Cache, id: ID, c: char) -> Option<f64> {
        if let Some(v) = cache.advances.get(&(id, c)) {
            return *v;
        }
        let measured = self.db.with_face_data(id, |data, index| {
            let face = ttf_parser::Face::parse(data, index).ok()?;
            let upem = f64::from(face.units_per_em());
            let adv = face.glyph_index(c).and_then(|g| face.glyph_hor_advance(g)).map(|a| f64::from(a) / upem);
            Some((upem, adv))
        });
        let adv = match measured.flatten() {
            Some((upem, adv)) => {
                cache.upem.insert(id, upem);
                adv
            }
            None => None,
        };
        cache.advances.insert((id, c), adv);
        adv
    }
}

impl TextMeasurer for SystemFonts {
    fn width(&self, text: &str, font: &Font, size_pt: f64) -> f64 {
        let Ok(mut cache) = self.cache.lock() else {
            return FixedMetrics.width(text, font, size_pt);
        };
        let Some(id) = self.resolve(&mut cache, font) else {
            return FixedMetrics.width(text, font, size_pt);
        };
        let mut em = 0.0;
        for c in text.chars() {
            em += self.advance(&mut cache, id, c).unwrap_or(0.55);
        }
        em * size_pt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_metrics_are_fifty_five_percent_per_character() {
        let w = FixedMetrics.width("abcd", &Font::new("Arial", 10.0), 10.0);
        assert!((w - 22.0).abs() < 1e-12);
    }

    #[test]
    fn family_list_starts_with_the_requested_family() {
        let s = css_family("Arial");
        assert!(s.starts_with("'Arial', 'Liberation Sans', 'Helvetica', 'DejaVu Sans'"));
        assert!(s.ends_with("sans-serif"));
        // No duplicate when the request is one of the fallbacks.
        assert_eq!(css_family("helvetica").matches("elvetica").count(), 1);
        assert_eq!(css_family("it's"), "'its', 'Liberation Sans', 'Helvetica', 'DejaVu Sans', sans-serif");
    }

    #[test]
    fn an_empty_database_falls_back_to_fixed_widths() {
        let fonts = SystemFonts::from_database(Database::new());
        let f = Font::new("Arial", 10.0);
        assert!((fonts.width("ab", &f, 10.0) - 11.0).abs() < 1e-12);
    }

    #[test]
    fn system_fonts_measure_something_plausible() {
        let fonts = SystemFonts::shared();
        let f = Font::new("Arial", 10.0);
        let w = fonts.width("Potential", &f, 10.0);
        assert!(w > 20.0 && w < 80.0, "width {w}");
        let mut bold = f.clone();
        bold.bold = true;
        assert!(fonts.width("Potential", &bold, 10.0) >= w - 1e-9);
    }
}
