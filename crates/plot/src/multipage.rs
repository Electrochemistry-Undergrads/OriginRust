//! Several graphs in one PDF, one graph per page (Origin's `expGraph` `multipage`, graphing.md
//! 9.6). Each page is converted on its own with svg2pdf's chunk API, exactly as the one-page
//! export converts it, and placed as a form XObject filling a page of the picture's size, so a
//! page looks the same as the one-graph PDF of that graph.

use crate::PlotError;
use crate::fonts::SystemFonts;
use crate::raster::parse_tree;
use pdf_writer::{Content, Filter, Finish, Name, Pdf, Rect, Ref, TextStr};
use std::collections::HashMap;

/// Writes one PDF page per SVG, in order. Each page is the size of its SVG (in points).
pub(crate) fn svgs_to_pdf(svgs: &[String], fonts: &SystemFonts) -> Result<Vec<u8>, PlotError> {
    if svgs.is_empty() {
        return Err(PlotError::Pdf("no graphs to put in the PDF".into()));
    }
    let mut alloc = Ref::new(1);
    let catalog = alloc.bump();
    let page_tree = alloc.bump();
    let info = alloc.bump();
    let mut pdf = Pdf::new();
    let mut kids = Vec::with_capacity(svgs.len());
    let name = Name(b"G1");
    for svg in svgs {
        let tree = parse_tree(svg, fonts).map_err(PlotError::Pdf)?;
        let (w, h) = (tree.size().width(), tree.size().height());
        if !(w.is_finite() && h.is_finite() && w > 0.0 && h > 0.0) {
            return Err(PlotError::Pdf("a page has no size".into()));
        }
        let (chunk, root) = svg2pdf::to_chunk(&tree, svg2pdf::ConversionOptions::default())
            .map_err(|e| PlotError::Pdf(e.to_string()))?;
        let mut map = HashMap::new();
        let chunk = chunk.renumber(|old| *map.entry(old).or_insert_with(|| alloc.bump()));
        let x_object = *map.get(&root).ok_or_else(|| PlotError::Pdf("the converted page has no root".into()))?;
        let page = alloc.bump();
        let contents = alloc.bump();
        kids.push(page);

        // The chunk's XObject is one point square; scale it to the page.
        let mut content = Content::new();
        content.save_state();
        content.transform([w, 0.0, 0.0, h, 0.0, 0.0]);
        content.x_object(name);
        content.restore_state();
        let stream = miniz_deflate(&content.finish());
        pdf.stream(contents, &stream).filter(Filter::FlateDecode);

        let mut p = pdf.page(page);
        p.media_box(Rect::new(0.0, 0.0, w, h));
        p.parent(page_tree);
        p.contents(contents);
        p.resources().x_objects().pair(name, x_object);
        p.group().transparency().isolated(true).knockout(false);
        p.finish();
        pdf.extend(&chunk);
    }
    pdf.catalog(catalog).pages(page_tree);
    pdf.pages(page_tree).count(kids.len() as i32).kids(kids);
    pdf.document_info(info).producer(TextStr("Sheaf"));
    Ok(pdf.finish())
}

/// The page's own content stream is a few operators; store it compressed as svg2pdf does.
fn miniz_deflate(data: &[u8]) -> Vec<u8> {
    miniz_oxide::deflate::compress_to_vec_zlib(data, 6)
}
