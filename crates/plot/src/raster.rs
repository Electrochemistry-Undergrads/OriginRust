//! SVG to pixels (screen and PNG) and to PDF.

use crate::fonts::SystemFonts;
use crate::{PlotError, Raster};
use resvg::tiny_skia;

/// Largest raster, in pixels, that will be attempted (guards against a
/// mistyped DPI exhausting memory).
const MAX_PIXELS: u64 = 400_000_000;

/// Parses SVG text with the system fonts. One user unit is one point.
pub(crate) fn parse_tree(svg: &str, fonts: &SystemFonts) -> Result<usvg::Tree, String> {
    let opt = usvg::Options { dpi: 72.0, fontdb: fonts.database().clone(), ..usvg::Options::default() };
    usvg::Tree::from_str(svg, &opt).map_err(|e| format!("the generated SVG could not be read back: {e}"))
}

/// Pixel size of a page of `pt` points at `scale` pixels per point.
fn pixels(pt: f64, scale: f64) -> u32 {
    // The small tolerance (the SVG size is an f32) keeps e.g. 3.6 in at 300 dpi at 1080 instead of 1081.
    ((pt * scale) - 1e-3).ceil().max(1.0) as u32
}

/// Rasterises an SVG at `pixels_per_point`.
pub(crate) fn rasterise(svg: &str, pixels_per_point: f64, fonts: &SystemFonts) -> Result<Raster, PlotError> {
    if !(pixels_per_point.is_finite() && pixels_per_point > 0.0) {
        return Err(PlotError::Png(format!("the resolution must be a positive number, not {pixels_per_point}")));
    }
    let tree = parse_tree(svg, fonts).map_err(PlotError::Png)?;
    let size = tree.size();
    let (w, h) =
        (pixels(f64::from(size.width()), pixels_per_point), pixels(f64::from(size.height()), pixels_per_point));
    if u64::from(w) * u64::from(h) > MAX_PIXELS {
        return Err(PlotError::Png(format!("an image of {w} x {h} pixels is too large; lower the resolution")));
    }
    let mut pixmap = tiny_skia::Pixmap::new(w, h)
        .ok_or_else(|| PlotError::Png(format!("cannot allocate an image of {w} x {h} pixels")))?;
    let sx = w as f32 / size.width();
    let sy = h as f32 / size.height();
    resvg::render(&tree, tiny_skia::Transform::from_scale(sx, sy), &mut pixmap.as_mut());
    let mut rgba = Vec::with_capacity(pixmap.data().len());
    for p in pixmap.pixels() {
        let c = p.demultiply();
        rgba.extend_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
    }
    Ok(Raster { width: w, height: h, rgba })
}

/// Encodes straight RGBA8 as PNG with the resolution recorded in a `pHYs` chunk.
pub(crate) fn encode_png(r: &Raster, dpi: f64) -> Result<Vec<u8>, PlotError> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, r.width, r.height);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let ppm = (dpi / 0.0254).round().clamp(1.0, f64::from(u32::MAX)) as u32;
        enc.set_pixel_dims(Some(png::PixelDimensions { xppu: ppm, yppu: ppm, unit: png::Unit::Meter }));
        let mut writer = enc.write_header().map_err(|e| PlotError::Png(e.to_string()))?;
        writer.write_image_data(&r.rgba).map_err(|e| PlotError::Png(e.to_string()))?;
    }
    Ok(out)
}

/// Converts an SVG to a one-page PDF whose page is the SVG size in points.
pub(crate) fn to_pdf(svg: &str, fonts: &SystemFonts) -> Result<Vec<u8>, PlotError> {
    let tree = parse_tree(svg, fonts).map_err(PlotError::Pdf)?;
    svg2pdf::to_pdf(&tree, svg2pdf::ConversionOptions::default(), svg2pdf::PageOptions::default())
        .map_err(|e| PlotError::Pdf(e.to_string()))
}

/// Encodes the picture as an LZW-compressed RGB TIFF with the resolution recorded. Transparent
/// pixels are laid on white, since many programs that take TIFF figures mishandle an alpha channel.
pub(crate) fn encode_tiff(r: &Raster, dpi: f64) -> Result<Vec<u8>, PlotError> {
    let err = |e: tiff::TiffError| PlotError::Tiff(e.to_string());
    let mut rgb = Vec::with_capacity(r.rgba.len() / 4 * 3);
    for px in r.rgba.as_chunks::<4>().0 {
        let a = u32::from(px[3]);
        for &c in &px[..3] {
            rgb.push(((u32::from(c) * a + 255 * (255 - a) + 127) / 255) as u8);
        }
    }
    let mut out = std::io::Cursor::new(Vec::new());
    {
        let mut enc =
            tiff::encoder::TiffEncoder::new(&mut out).map_err(err)?.with_compression(tiff::encoder::Compression::Lzw);
        let mut image = enc.new_image::<tiff::encoder::colortype::RGB8>(r.width, r.height).map_err(err)?;
        let hundredths = (dpi * 100.0).round().clamp(1.0, f64::from(u32::MAX)) as u32;
        image.resolution(tiff::tags::ResolutionUnit::Inch, tiff::encoder::Rational { n: hundredths, d: 100 });
        image.write_data(&rgb).map_err(err)?;
    }
    Ok(out.into_inner())
}
