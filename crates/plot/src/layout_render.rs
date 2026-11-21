//! A layout page as one picture (T237 slice 3, layout-page spec): every graph or worksheet item is
//! rendered at the pixel size it takes on the page and laid, at its place, on a white page.

use crate::fonts::SystemFonts;
use crate::{PlotError, Raster, RasterOptions, raster};
use sheaf_core::layout_page::LayoutContent;
use sheaf_core::{Project, WindowId};

/// The font of layout text (T237 slice 8).
const TEXT_FAMILY: &str = "Arial";

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Layout text as an SVG document and its natural size in points: the lines in Arial at `size`
/// points, 1.2 sizes apart, with a margin of 0.3 size all round.
pub fn layout_text_svg(text: &str, size: u16) -> (String, (f64, f64)) {
    use std::fmt::Write as _;
    let s = f64::from(size.max(1));
    let font = sheaf_core::graph::Font::new(TEXT_FAMILY, s);
    let m = SystemFonts::shared();
    let lines: Vec<&str> = if text.is_empty() { vec![""] } else { text.lines().collect() };
    let pad = 0.3 * s;
    let widest = lines.iter().map(|l| crate::fonts::TextMeasurer::width(m, l, &font, s)).fold(0.0, f64::max);
    let (w, h) = (widest.max(s) + 2.0 * pad, 1.2 * s * lines.len() as f64 + 2.0 * pad);
    let mut out = String::new();
    let _ = write!(
        out,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}pt\" height=\"{h}pt\" viewBox=\"0 0 {w} {h}\">\
         <text font-family=\"{TEXT_FAMILY}\" font-size=\"{s}\" fill=\"rgb(0,0,0)\">"
    );
    for (k, line) in lines.iter().enumerate() {
        let y = pad + 1.2 * s * k as f64 + 0.95 * s;
        let _ = write!(out, "<tspan x=\"{pad}\" y=\"{y}\">{}</tspan>", xml_escape(line));
    }
    out.push_str("</text></svg>\n");
    (out, (w, h))
}

/// The picture of a layout item `pixel_width` pixels wide; `None` when what it shows is gone.
pub fn render_layout_item(
    project: &Project,
    content: &LayoutContent,
    pixel_width: u32,
) -> Option<Result<Raster, PlotError>> {
    let px = f64::from(pixel_width.max(1));
    match content {
        LayoutContent::Graph { graph } => {
            let g = project.graph(*graph)?;
            Some(crate::render_raster(project, *graph, px / g.page.width_pt.max(1.0)))
        }
        LayoutContent::Sheet { book, sheet } => {
            let ws = project.workbook(*book)?.sheets.get(*sheet)?;
            Some(crate::sheet_image_size(ws).and_then(|(w, _)| crate::sheet_image_raster(ws, px / w.max(1.0))))
        }
        LayoutContent::Text { text, size } => {
            let (svg, (w, _)) = layout_text_svg(text, *size);
            Some(raster::rasterise(&svg, px / w.max(1.0), SystemFonts::shared()))
        }
    }
}

/// The width / height of what a layout item shows, for placing a new one; `None` when it is gone.
pub fn layout_item_aspect(project: &Project, content: &LayoutContent) -> Option<f64> {
    match content {
        LayoutContent::Graph { graph } => {
            let g = project.graph(*graph)?;
            Some(g.page.width_pt / g.page.height_pt.max(1.0))
        }
        LayoutContent::Sheet { book, sheet } => {
            let ws = project.workbook(*book)?.sheets.get(*sheet)?;
            crate::sheet_image_size(ws).ok().map(|(w, h)| w / h.max(1.0))
        }
        LayoutContent::Text { text, size } => {
            let (_, (w, h)) = layout_text_svg(text, *size);
            Some(w / h.max(1.0))
        }
    }
}

/// The layout page `layout` as a raster at `dpi` dots per inch, on the page colour. An item whose graph or sheet is gone
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
    let bg = page.background;
    let mut rgba = [bg.r, bg.g, bg.b, 255].repeat((width as usize) * (height as usize));
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
    match ext.as_str() {
        "svg" => return Ok(render_layout_svg(project, layout)?.into_bytes()),
        "emf" => return render_layout_emf(project, layout, dpi),
        "pdf" => return raster::to_pdf(&render_layout_svg(project, layout)?, SystemFonts::shared()),
        _ => {}
    }
    if !raster::is_raster_ext(&ext) {
        return Err(PlotError::Image(format!(
            "a layout page is exported as png, jpg, tif, bmp, svg, pdf or emf, not .{ext}"
        )));
    }
    let r = render_layout_raster(project, layout, dpi)?;
    raster::encode_raster(&r, &ext, dpi, &RasterOptions::default())
}

/// The layout page as an Enhanced Metafile (T237 slice 9): the page picture at `dpi` as one bitmap
/// the size of the page, for printing on Windows and for EMF export.
pub fn render_layout_emf(project: &Project, layout: WindowId, dpi: f64) -> Result<Vec<u8>, PlotError> {
    let page = project
        .layout(layout)
        .ok_or_else(|| PlotError::Image(format!("there is no layout page with id {}", layout.0)))?;
    let r = render_layout_raster(project, layout, dpi)?;
    Ok(crate::emf::write_emf_picture(page.width_pt, page.height_pt, &r))
}

/// The vector picture of what a layout item shows; `None` when it is gone.
fn item_svg(project: &Project, content: &LayoutContent) -> Option<Result<String, PlotError>> {
    match content {
        LayoutContent::Graph { graph } => {
            project.graph(*graph)?;
            Some(crate::render_svg(project, *graph))
        }
        LayoutContent::Sheet { book, sheet } => {
            let ws = project.workbook(*book)?.sheets.get(*sheet)?;
            Some(crate::sheet_image_svg(ws))
        }
        LayoutContent::Text { text, size } => Some(Ok(layout_text_svg(text, *size).0)),
    }
}

/// Makes the ids of a nested picture its own (`k` is the item's index), so two graphs' clip paths
/// do not meet.
fn own_ids(svg: &str, k: usize) -> String {
    svg.replace(" id=\"", &format!(" id=\"i{k}-"))
        .replace("url(#", &format!("url(#i{k}-"))
        .replace("href=\"#", &format!("href=\"#i{k}-"))
}

/// T237 slice 7: the layout page as an SVG document in points, on the page colour, with each item's
/// own SVG (the graph's or the worksheet image's) nested at its place and stretched to its rectangle.
/// An item whose graph or sheet is gone is left out.
pub fn render_layout_svg(project: &Project, layout: WindowId) -> Result<String, PlotError> {
    use std::fmt::Write as _;
    let page = project
        .layout(layout)
        .ok_or_else(|| PlotError::Image(format!("there is no layout page with id {}", layout.0)))?;
    let (w, h) = (page.width_pt, page.height_pt);
    let bg = page.background;
    let mut s = String::new();
    let _ = writeln!(
        s,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}pt\" height=\"{h}pt\" viewBox=\"0 0 {w} {h}\">"
    );
    let _ =
        writeln!(s, "<rect x=\"0\" y=\"0\" width=\"{w}\" height=\"{h}\" fill=\"rgb({},{},{})\"/>", bg.r, bg.g, bg.b);
    for (k, item) in page.items.iter().enumerate() {
        let Some(inner) = item_svg(project, &item.content) else { continue };
        let inner = inner?;
        // The nested root: its viewBox kept, placed and sized on the page.
        let Some(open) = inner.find("<svg") else { continue };
        let Some(close) = inner[open..].find('>').map(|e| open + e) else { continue };
        let tag = &inner[open..close];
        let view = tag
            .find("viewBox=\"")
            .map(|v| v + "viewBox=\"".len())
            .and_then(|v| tag[v..].find('"').map(|e| &tag[v..v + e]))
            .unwrap_or("0 0 1 1");
        let _ = write!(
            s,
            "<svg x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" viewBox=\"{view}\" preserveAspectRatio=\"none\">",
            item.x * w,
            item.y * h,
            item.w * w,
            item.h * h
        );
        s.push_str(&own_ids(&inner[close + 1..], k));
        s.push('\n');
    }
    s.push_str("</svg>\n");
    Ok(s)
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
        let item = p.layout(id).unwrap().items[0].clone();
        let (x0, y0) = ((item.x * 396.0) as u32, (item.y * 306.0) as u32);
        let (x1, y1) = (((item.x + item.w) * 396.0) as u32, ((item.y + item.h) * 306.0) as u32);
        let inked =
            (y0..y1).flat_map(|y| (x0..x1).map(move |x| (x, y))).filter(|&(x, y)| px(x, y) != [255, 255, 255]).count();
        assert!(inked > 100, "the graph picture is drawn: {inked} inked pixels");
        let png = render_layout_image(&p, id, "PNG", 36.0).unwrap();
        assert!(png.starts_with(&[0x89, b'P', b'N', b'G']));
        assert!(render_layout_image(&p, id, "eps", 36.0).is_err());
        let emf = render_layout_image(&p, id, "emf", 36.0).unwrap();
        assert_eq!(&emf[40..44], b" EMF", "an Enhanced Metafile");
        assert!(emf.len() > 396 * 306 * 3, "the page bitmap is in it");
        // The vector page: the graph's own SVG nested at its place, its ids made its own.
        let svg = String::from_utf8(render_layout_image(&p, id, "SVG", 36.0).unwrap()).unwrap();
        assert!(svg.starts_with("<svg xmlns") && svg.contains("width=\"792pt\""), "{}", &svg[..200]);
        assert_eq!(svg.matches("<svg").count(), 2, "the page and one graph");
        assert!(svg.contains("id=\"i0-clip0\"") && svg.contains("url(#i0-clip0)"));
        assert!(!svg.contains("url(#clip"), "no id left shared");
        let pdf = render_layout_image(&p, id, "pdf", 36.0).unwrap();
        assert!(pdf.starts_with(b"%PDF"));
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
        let svg = render_layout_svg(&p, id).unwrap();
        assert_eq!(svg.matches("<svg").count(), 3, "the page, the graph and the sheet; the gone sheet left out");
        assert!(svg.contains("url(#i0-clip0)"));
        // Text: drawn in its box, written as SVG text (escaped).
        let (_, size) = layout_text_svg("A < B", 24);
        p.layout_mut(id).unwrap().add_text("A < B".into(), 24, size);
        let r = render_layout_raster(&p, id, 72.0).unwrap();
        let t = p.layout(id).unwrap().items[3].clone();
        let (x0, y0) = ((t.x * 792.0) as u32, (t.y * 612.0) as u32);
        let (x1, y1) = (((t.x + t.w) * 792.0) as u32, ((t.y + t.h) * 612.0) as u32);
        let k = |x: u32, y: u32| ((y * r.width + x) * 4) as usize;
        let dark = (y0..y1).flat_map(|y| (x0..x1).map(move |x| (x, y))).filter(|&(x, y)| r.rgba[k(x, y)] < 128).count();
        assert!(dark > 30, "the text is drawn: {dark} dark pixels");
        let svg = render_layout_svg(&p, id).unwrap();
        assert!(svg.contains(">A &lt; B</tspan>"), "escaped text");
    }
}
