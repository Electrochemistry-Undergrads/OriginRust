//! Lazy worksheet face registration. UI fonts and graph fonts are left intact.

use eframe::egui::{self, FontData, FontFamily};
use sheaf_core::cellformat::ResolvedStyle;
use std::collections::HashMap;
use std::sync::Arc;

pub struct CellFont {
    pub family: FontFamily,
    pub synthetic_bold: bool,
    pub synthetic_italic: bool,
}

pub fn cell_font(ctx: &egui::Context, style: &ResolvedStyle<'_>) -> CellFont {
    let name = style.font.unwrap_or("Arial");
    let family = ctx.data_mut(|d| {
        type Families = HashMap<String, [Option<FontFamily>; 4]>;
        let fonts = d.get_temp_mut_or_default::<Families>(egui::Id::new("worksheet_font_families"));
        if !fonts.contains_key(name) {
            fonts.insert(name.to_string(), Default::default());
        }
        let variants = fonts.get_mut(name).expect("cached family");
        variants[usize::from(style.bold) * 2 + usize::from(style.italic)]
            .get_or_insert_with(|| FontFamily::Name(format!("worksheet:{name}:{}:{}", style.bold, style.italic).into()))
            .clone()
    });
    let ready = ctx.fonts(|f| f.definitions().families.contains_key(&family));
    let face = sheaf_plot::worksheet_font(name, style.bold, style.italic);
    if !ready && let Some(face) = &face {
        let pass = ctx.cumulative_pass_nr();
        let current = ctx.fonts(|f| f.definitions().clone());
        let definitions = ctx.data_mut(|d| {
            let state = d.get_temp_mut_or_default::<(u64, Option<egui::FontDefinitions>)>(egui::Id::new(
                "worksheet_fonts_pending",
            ));
            if state.0 != pass || state.1.is_none() {
                state.0 = pass;
                state.1 = Some(current);
            }
            let fonts = state.1.as_mut().expect("pending font definitions");
            if fonts.families.contains_key(&family) {
                return None;
            }
            let mut data = FontData::from_owned(face.data.clone());
            data.index = face.index;
            let key = family.to_string();
            fonts.font_data.insert(key.clone(), Arc::new(data));
            let mut chain = vec![key];
            chain.extend(fonts.families.get(&FontFamily::Proportional).into_iter().flatten().cloned());
            fonts.families.insert(family.clone(), chain);
            Some(fonts.clone())
        });
        if let Some(definitions) = definitions {
            ctx.set_fonts(definitions);
            ctx.request_discard("worksheet font registered");
        }
    }
    CellFont {
        family: if ready { family } else { FontFamily::Proportional },
        synthetic_bold: style.bold && (!ready || face.as_ref().is_none_or(|f| !f.bold)),
        synthetic_italic: style.italic && (!ready || face.as_ref().is_none_or(|f| !f.italic)),
    }
}
