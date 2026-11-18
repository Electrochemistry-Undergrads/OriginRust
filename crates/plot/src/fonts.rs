//! Text measurement and the system font database (spec section 7).

use fontdb::{Database, Family, ID, Query, Stretch, Style, Weight};
use sheaf_core::graph::Font;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, OnceLock};

/// Families tried after the one the model asks for, in this order.
pub const FALLBACK_FAMILIES: [&str; 3] = ["Liberation Sans", "Helvetica", "DejaVu Sans"];

/// Position and thickness of a text decoration, relative to its font size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextLineMetrics {
    /// Stroke centre relative to the baseline in em units, positive down the page.
    pub center: f64,
    /// Stroke thickness in em units.
    pub thickness: f64,
}

/// Measures text and its decorations. Tests can provide fixed metrics independently of fonts.
pub trait TextMeasurer {
    /// Width in points of `text` set in `font` (family, bold, italic) at `size_pt`.
    fn width(&self, text: &str, font: &Font, size_pt: f64) -> f64;

    /// Expands a unique installed-family prefix in an inline font switch.
    fn expand_inline_family(&self, _name: &str) -> Option<String> {
        None
    }

    /// Underline geometry in em units. Defaults are used when no font metrics are available.
    fn underline_metrics(&self, _font: &Font) -> TextLineMetrics {
        TextLineMetrics { center: 0.14, thickness: 0.07 }
    }

    /// Strikethrough geometry in em units, with the same baseline as the glyphs.
    fn strikeout_metrics(&self, _font: &Font) -> TextLineMetrics {
        TextLineMetrics { center: -0.23, thickness: 0.05 }
    }
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
    decorations: HashMap<ID, (Option<TextLineMetrics>, Option<TextLineMetrics>)>,
}

/// The installed fonts, loaded once, plus a width measurer on top of them.
///
/// Widths come from the advance widths of the first installed family of the
/// request (the model's family, then [`FALLBACK_FAMILIES`]), bold and italic
/// aware when such a face exists. Kerning is not applied.
pub struct SystemFonts {
    db: Arc<Database>,
    family_names: BTreeMap<String, String>,
    cache: Mutex<Cache>,
}

fn unique_family_prefix(names: &BTreeMap<String, String>, name: &str) -> Option<String> {
    let prefix = name.to_lowercase();
    // The one-letter special defaults and Default labels require separate configuration.
    if name.chars().take(2).count() < 2 || prefix == "default" || prefix.starts_with("default:") {
        return None;
    }
    let mut matches = names.range(prefix.clone()..).take_while(|(key, _)| key.starts_with(&prefix));
    let (key, family) = matches.next()?;
    (key != &prefix && matches.next().is_none()).then(|| family.clone())
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
        let family_names =
            db.faces().flat_map(|face| &face.families).map(|(name, _)| (name.to_lowercase(), name.clone())).collect();
        SystemFonts { db: Arc::new(db), family_names, cache: Mutex::new(Cache::default()) }
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

    fn decoration(&self, font: &Font, strikeout: bool) -> Option<TextLineMetrics> {
        let mut cache = self.cache.lock().ok()?;
        let id = self.resolve(&mut cache, font)?;
        let metrics = *cache.decorations.entry(id).or_insert_with(|| {
            self.db
                .with_face_data(id, |data, index| {
                    let face = ttf_parser::Face::parse(data, index).ok()?;
                    let em = f64::from(face.units_per_em());
                    let line = |m: ttf_parser::LineMetrics| {
                        (m.thickness > 0).then(|| TextLineMetrics {
                            // OpenType post/OS2 positions specify the top of the stroke; scene
                            // paths need its centre, in a frame whose positive Y points down.
                            center: (-f64::from(m.position) + f64::from(m.thickness) / 2.0) / em,
                            thickness: f64::from(m.thickness) / em,
                        })
                    };
                    Some((face.underline_metrics().and_then(line), face.strikeout_metrics().and_then(line)))
                })
                .flatten()
                .unwrap_or((None, None))
        });
        if strikeout { metrics.1 } else { metrics.0 }
    }
}

impl TextMeasurer for SystemFonts {
    fn expand_inline_family(&self, name: &str) -> Option<String> {
        unique_family_prefix(&self.family_names, name)
    }

    fn underline_metrics(&self, font: &Font) -> TextLineMetrics {
        self.decoration(font, false).unwrap_or_else(|| FixedMetrics.underline_metrics(font))
    }

    fn strikeout_metrics(&self, font: &Font) -> TextLineMetrics {
        self.decoration(font, true).unwrap_or_else(|| FixedMetrics.strikeout_metrics(font))
    }

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
        assert_eq!(fonts.underline_metrics(&f), FixedMetrics.underline_metrics(&f));
        assert_eq!(fonts.strikeout_metrics(&f), FixedMetrics.strikeout_metrics(&f));
        assert_eq!(fonts.expand_inline_family("Cour New"), None);
    }

    #[test]
    fn inline_names_expand_only_for_a_unique_installed_prefix() {
        let families = ["Arial", "ARIAL", "Arial Narrow", "Courier New", "Courgette", "Symbol", "Default Sans"]
            .into_iter()
            .map(|name| (name.to_lowercase(), name.to_owned()))
            .collect();
        assert_eq!(unique_family_prefix(&families, "couRI"), Some("Courier New".to_owned()));
        assert_eq!(unique_family_prefix(&families, "SYM"), Some("Symbol".to_owned()));
        for name in ["Arial", "Ari", "Cour", "Missing", "S", "Default", "Default: Arial"] {
            assert_eq!(unique_family_prefix(&families, name), None, "{name}");
        }
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

    #[test]
    fn system_decoration_metrics_are_finite_and_cached_without_style_collisions() {
        let fonts = SystemFonts::shared();
        for (bold, italic) in [(false, false), (true, false), (false, true), (true, true)] {
            let mut font = Font::new("Arial", 20.0);
            font.bold = bold;
            font.italic = italic;
            let underline = fonts.underline_metrics(&font);
            let strike = fonts.strikeout_metrics(&font);
            assert!(underline.center.is_finite() && strike.center.is_finite());
            assert!((0.0..1.0).contains(&underline.thickness));
            assert!((0.0..1.0).contains(&strike.thickness));
            assert_eq!(fonts.underline_metrics(&font), underline);
            assert_eq!(fonts.strikeout_metrics(&font), strike);
        }
    }
}
