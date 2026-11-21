//! A layout page as one picture (T237 slice 3, layout-page spec): every graph or worksheet item is
//! rendered at the pixel size it takes on the page and laid, at its place, on a white page.

use crate::{PlotError, Raster, RasterOptions, raster};
use sheaf_core::layout_page::LayoutContent;
use sheaf_core::{Project, WindowId};

/// The picture of a layout item `pixel_width` pixels wide; `None` when what it shows is gone.
pub fn render_layout_item(
    project: &Project,
    content: &LayoutContent,
    pixel_width: u32,
) -> Option<Result<Raster, PlotError>> {
    let px = f64::from(pixel_width.max(1));
    match *content {
        LayoutContent::Graph { graph } => {
            let g = project.graph(graph)?;
            Some(crate::render_raster(project, graph, px / g.page.width_pt.max(1.0)))
        }
        LayoutContent::Sheet { book, sheet } => {
            let ws = project.workbook(book)?.sheets.get(sheet)?;
            Some(crate::sheet_image_size(ws).and_then(|(w, _)| crate::sheet_image_raster(ws, px / w.max(1.0))))
        }
    }
}

/// The width / height of what a layout item shows, for placing a new one; `None` when it is gone.
pub fn layout_item_aspect(project: &Project, content: &LayoutContent) -> Option<f64> {
    match *content {
        LayoutContent::Graph { graph } => {
            let g = project.graph(graph)?;
            Some(g.page.width_pt / g.page.height_pt.max(1.0))
        }
        LayoutContent::Sheet { book, sheet } => {
            let ws = project.workbook(book)?.sheets.get(sheet)?;
            crate::sheet_image_size(ws).ok().map(|(w, h)| w / h.max(1.0))
        }
    }
}

/// The layout page `layout` as a raster at `dpi` dots per inch. An item whose graph or sheet is gone
/// is left blank. A picture stretched by a resize is drawn stretched (nearest pixel).
pub fn render_layout_raster(project: &Project, layout: WindowId, dpi: f64) -> Result<Raster, PlotError> {
    let page = project
        .layout(layout)
        .ok_or_else(|| PlotError::Image(format!("there is no layout page with id {}", layout.0)))?;
    if !(dpi.is_finite() && dpi > 0.0) {
        return Err(PlotError::Image("the resolution must be a positive number".into()));
    }
    let width = (page.width_pt * dpi / 72.0).round().max(1.0) as u32;
    let height = (page.height_pt * dpi / 72.0).round().max(1.0) as u32;
    let total = u64::from(width) * u64::from(height);
    if total > 400_000_000 {
        return Err(PlotError::Image(format!("a {width} x {height} pixel page is too large")));
    }
    let mut rgba = vec![255u8; (width as usize) * (height as usize) * 4];
    for item in &page.items {
        let (x0, y0) = ((item.x * f64::from(width)).round() as i64, (item.y * f64::from(height)).round() as i64);
        let (iw, ih) = ((item.w * f64::from(width)).round() as i64, (item.h * f64::from(height)).round() as i64);
        if iw <= 0 || ih <= 0 {
            continue;
        }
        let Some(pic) = render_layout_item(project, &item.content, iw as u32) else { continue };
        let pic = pic?;
        if pic.width == 0 || pic.height == 0 {
            continue;
        }
        for dy in 0..ih {
            let py = y0 + dy;
            if py < 0 || py >= i64::from(height) {
                continue;
            }
            let sy = ((dy as f64 + 0.5) * f64::from(pic.height) / ih as f64) as usize;
            let sy = sy.min(pic.height as usize - 1);
            for dx in 0..iw {
                let px = x0 + dx;
                if px < 0 || px >= i64::from(width) {
                    continue;
                }
                let sx = ((dx as f64 + 0.5) * f64::from(pic.width) / iw as f64) as usize;
                let sx = sx.min(pic.width as usize - 1);
                let s = (sy * pic.width as usize + sx) * 4;
                let d = (py as usize * width as usize + px as usize) * 4;
                // Straight alpha over what is below.
                let a = f64::from(pic.rgba[s + 3]) / 255.0;
                for c in 0..3 {
                    let v = f64::from(pic.rgba[s + c]) * a + f64::from(rgba[d + c]) * (1.0 - a);
                    rgba[d + c] = v.round() as u8;
                }
                rgba[d + 3] = 255;
            }
        }
    }
    Ok(Raster { width, height, rgba })
}

/// The layout page encoded as `ext` (png, jpg, tif, bmp, ...) at `dpi`, with the graph exports'
/// default options.
pub fn render_layout_image(project: &Project, layout: WindowId, ext: &str, dpi: f64) -> Result<Vec<u8>, PlotError> {
    let ext = ext.to_ascii_lowercase();
    if !raster::is_raster_ext(&ext) {
        return Err(PlotError::Image(format!(
            "a layout page is exported as a picture (png, jpg, tif, bmp), not .{ext}"
        )));
    }
    let r = render_layout_raster(project, layout, dpi)?;
    raster::encode_raster(&r, &ext, dpi, &RasterOptions::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sheaf_core::template::Template;
    use sheaf_core::{ColumnData, Designation};

    #[test]
    fn graph_pictures_land_at_their_places_on_a_white_page() {
        let mut p = Project::new();
        let book = p.add_workbook("d");
        p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![0.0, 1.0, 2.0])).unwrap();
        let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![0.0, 2.0, 1.0])).unwrap();
        let g = p.add_graph(Template::Line);
        p.add_plot(g, 0, y).unwrap();
        let id = p.add_layout();
        p.layout_mut(id).unwrap().add_graph(g, 4.0 / 3.0);
        let r = render_layout_raster(&p, id, 36.0).unwrap();
        assert_eq!((r.width, r.height), (396, 306), "11 x 8.5 in at 36 dpi");
        let px = |x: u32, y: u32| {
            let k = ((y * r.width + x) * 4) as usize;
            [r.rgba[k], r.rgba[k + 1], r.rgba[k + 2]]
        };
        assert_eq!(px(r.width - 2, r.height - 2), [255, 255, 255], "the empty cell stays white");
        // Something other than white is drawn inside the first cell (axes, curve, text).
        let item = p.layout(id).unwrap().items[0];
        let (x0, y0) = ((item.x * 396.0) as u32, (item.y * 306.0) as u32);
        let (x1, y1) = (((item.x + item.w) * 396.0) as u32, ((item.y + item.h) * 306.0) as u32);
        let inked =
            (y0..y1).flat_map(|y| (x0..x1).map(move |x| (x, y))).filter(|&(x, y)| px(x, y) != [255, 255, 255]).count();
        assert!(inked > 100, "the graph picture is drawn: {inked} inked pixels");
        let png = render_layout_image(&p, id, "PNG", 36.0).unwrap();
        assert!(png.starts_with(&[0x89, b'P', b'N', b'G']));
        assert!(render_layout_image(&p, id, "svg", 36.0).is_err());
        // A worksheet item is drawn in the second cell; a gone sheet stays blank.
        p.layout_mut(id).unwrap().add_sheet(book, 0, 2.0);
        p.layout_mut(id).unwrap().add_sheet(book, 7, 2.0);
        let aspect = layout_item_aspect(&p, &LayoutContent::Sheet { book, sheet: 0 }).unwrap();
        assert!(aspect > 0.0, "{aspect}");
        assert!(layout_item_aspect(&p, &LayoutContent::Sheet { book, sheet: 7 }).is_none());
        let r = render_layout_raster(&p, id, 36.0).unwrap();
        let px = |x: u32, y: u32| {
            let k = ((y * r.width + x) * 4) as usize;
            [r.rgba[k], r.rgba[k + 1], r.rgba[k + 2]]
        };
        let ink = |it: &sheaf_core::layout_page::LayoutItem| {
            let (x0, y0) = ((it.x * 396.0) as u32, (it.y * 306.0) as u32);
            let (x1, y1) = (((it.x + it.w) * 396.0) as u32, ((it.y + it.h) * 306.0) as u32);
            (y0..y1).flat_map(|y| (x0..x1).map(move |x| (x, y))).filter(|&(x, y)| px(x, y) != [255, 255, 255]).count()
        };
        let items = p.layout(id).unwrap().items.clone();
        assert!(ink(&items[1]) > 20, "the sheet picture is drawn: {}", ink(&items[1]));
        assert_eq!(ink(&items[2]), 0, "a gone sheet stays blank");
    }
}
