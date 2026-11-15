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
    rasterise_with_limit(svg, pixels_per_point, fonts, None)
}

/// Rasterises with an optional format-specific dimension limit, checked before allocating pixels.
pub(crate) fn rasterise_with_limit(
    svg: &str,
    pixels_per_point: f64,
    fonts: &SystemFonts,
    max_side: Option<u32>,
) -> Result<Raster, PlotError> {
    if !(pixels_per_point.is_finite() && pixels_per_point > 0.0) {
        return Err(PlotError::Png(format!("the resolution must be a positive number, not {pixels_per_point}")));
    }
    let tree = parse_tree(svg, fonts).map_err(PlotError::Png)?;
    let size = tree.size();
    let (w, h) =
        (pixels(f64::from(size.width()), pixels_per_point), pixels(f64::from(size.height()), pixels_per_point));
    if let Some(max) = max_side
        && (w > max || h > max)
    {
        return Err(PlotError::Image(format!(
            "an image of {w} x {h} pixels exceeds this format's limit of {max} pixels per dimension; lower the resolution or image size"
        )));
    }
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

/// Colour depth of a raster export (Origin's "Color Depth" list).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum ColorDepth {
    /// Black and white, one bit per pixel where the format allows it.
    Mono,
    /// An optimised palette of 16 colours, four bits per pixel where the format allows it.
    Colors16,
    /// An optimised palette of at most 256 colours.
    Colors256,
    /// 24-bit colour (PNG keeps its alpha channel).
    #[default]
    Bit24,
    /// 8-bit grey.
    Grey8,
}

/// Compression of a TIFF export.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum TiffCompression {
    /// No compression.
    None,
    /// PackBits run-length coding.
    PackBits,
    /// LZW.
    #[default]
    Lzw,
}

/// Choices that only concern raster formats.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RasterOptions {
    /// Colour depth; a format that cannot store it uses the nearest one it can.
    pub depth: ColorDepth,
    /// TIFF compression.
    pub tiff_compression: TiffCompression,
    /// JPEG quality, 1 (smallest) to 100 (best).
    pub jpeg_quality: u8,
}

impl Default for RasterOptions {
    fn default() -> Self {
        RasterOptions { depth: ColorDepth::Bit24, tiff_compression: TiffCompression::Lzw, jpeg_quality: 90 }
    }
}

/// The raster formats that can be written, by file extension.
pub(crate) fn is_raster_ext(ext: &str) -> bool {
    matches!(ext, "png" | "tif" | "tiff" | "jpg" | "jpeg" | "bmp" | "gif" | "tga" | "pcx" | "psd")
}

/// Straight RGBA laid on white, as RGB. Many programs mishandle an alpha channel in these formats.
fn flatten_rgb(r: &Raster) -> Vec<u8> {
    let mut rgb = Vec::with_capacity(r.rgba.len() / 4 * 3);
    for px in r.rgba.as_chunks::<4>().0 {
        let a = u32::from(px[3]);
        for &c in &px[..3] {
            rgb.push(((u32::from(c) * a + 255 * (255 - a) + 127) / 255) as u8);
        }
    }
    rgb
}

fn luma(rgb: &[u8]) -> Vec<u8> {
    rgb.as_chunks::<3>()
        .0
        .iter()
        .map(|p| ((u32::from(p[0]) * 299 + u32::from(p[1]) * 587 + u32::from(p[2]) * 114 + 500) / 1000) as u8)
        .collect()
}

/// A palette (RGB triples, at most 256) and one index per pixel.
struct Indexed {
    palette: Vec<[u8; 3]>,
    indices: Vec<u8>,
}

fn quantise(rgb: &[u8]) -> Indexed {
    quantise_to(rgb, 256)
}

/// An optimised palette of at most `colours` colours (NeuQuant) and the nearest entry per pixel.
fn quantise_to(rgb: &[u8], colours: usize) -> Indexed {
    let rgba: Vec<u8> = rgb.as_chunks::<3>().0.iter().flat_map(|p| [p[0], p[1], p[2], 255]).collect();
    let nq = color_quant::NeuQuant::new(10, colours, &rgba);
    let palette = nq.color_map_rgb().as_chunks::<3>().0.to_vec();
    let indices = rgba.as_chunks::<4>().0.iter().map(|p| nq.index_of(p) as u8).collect();
    Indexed { palette, indices }
}

fn grey_indexed(rgb: &[u8]) -> Indexed {
    Indexed { palette: (0..=255u8).map(|g| [g, g, g]).collect(), indices: luma(rgb) }
}

/// The 8 x 8 ordered-dither (Bayer) matrix, thresholds 0 to 63.
pub(crate) const BAYER8: [[u8; 8]; 8] = [
    [0, 32, 8, 40, 2, 34, 10, 42],
    [48, 16, 56, 24, 50, 18, 58, 26],
    [12, 44, 4, 36, 14, 46, 6, 38],
    [60, 28, 52, 20, 62, 30, 54, 22],
    [3, 35, 11, 43, 1, 33, 9, 41],
    [51, 19, 59, 27, 49, 17, 57, 25],
    [15, 47, 7, 39, 13, 45, 5, 37],
    [63, 31, 55, 23, 61, 29, 53, 21],
];

/// Black and white by ordered dithering of the grey level, as Origin's Monochrome export does
/// (observed: the standard 8 x 8 Bayer matrix anchored at the top-left pixel; a pixel is white
/// where `4 t + 2` is below its grey level, `t` the matrix entry; graphing.md 9.4).
fn mono_indexed(rgb: &[u8], width: usize) -> Indexed {
    let w = width.max(1);
    let indices = luma(rgb)
        .iter()
        .enumerate()
        .map(|(i, &g)| u8::from(4 * u16::from(BAYER8[(i / w) % 8][(i % w) % 8]) + 2 < u16::from(g)))
        .collect();
    Indexed { palette: vec![[0, 0, 0], [255, 255, 255]], indices }
}

/// Mono indices of an RGB buffer of the given width (for tests).
#[cfg(test)]
pub(crate) fn mono_dither_for_test(rgb: &[u8], width: usize) -> Vec<u8> {
    mono_indexed(rgb, width).indices
}

/// The palette image of an indexed depth (None for 24-bit).
fn indexed_for(depth: ColorDepth, rgb: &[u8], width: u32) -> Option<Indexed> {
    match depth {
        ColorDepth::Mono => Some(mono_indexed(rgb, width as usize)),
        // Origin builds 16 colours from the picture too (observed: an adaptive palette on a 5-bit
        // grid, no dithering of solid colours); its exact quantiser is not reproduced.
        ColorDepth::Colors16 => Some(quantise_to(rgb, 16)),
        ColorDepth::Colors256 => Some(quantise(rgb)),
        ColorDepth::Grey8 => Some(grey_indexed(rgb)),
        ColorDepth::Bit24 => None,
    }
}

/// Bits per pixel of a packed palette image: 1 for mono, 4 for 16 colours, 8 otherwise.
fn packed_bits(depth: ColorDepth) -> usize {
    match depth {
        ColorDepth::Mono => 1,
        ColorDepth::Colors16 => 4,
        _ => 8,
    }
}

/// Packs one row of palette indices into `bits` per pixel, high bits first, padded to a byte.
pub(crate) fn pack_row(indices: &[u8], bits: usize) -> Vec<u8> {
    if bits == 8 {
        return indices.to_vec();
    }
    let per = 8 / bits;
    indices
        .chunks(per)
        .map(|c| c.iter().enumerate().fold(0u8, |acc, (k, &v)| acc | (v << (8 - bits * (k + 1)))))
        .collect()
}

fn dpm(dpi: f64) -> u32 {
    (dpi / 0.0254).round().clamp(1.0, f64::from(u32::MAX)) as u32
}

/// Encodes the picture as PNG with the resolution recorded; `Bit24` keeps the alpha channel.
pub(crate) fn encode_png_with(r: &Raster, dpi: f64, o: &RasterOptions) -> Result<Vec<u8>, PlotError> {
    if o.depth == ColorDepth::Bit24 {
        return encode_png(r, dpi);
    }
    let rgb = flatten_rgb(r);
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, r.width, r.height);
        enc.set_depth(png::BitDepth::Eight);
        let ppm = dpm(dpi);
        enc.set_pixel_dims(Some(png::PixelDimensions { xppu: ppm, yppu: ppm, unit: png::Unit::Meter }));
        let data = if o.depth == ColorDepth::Grey8 {
            enc.set_color(png::ColorType::Grayscale);
            luma(&rgb)
        } else {
            let ix = indexed_for(o.depth, &rgb, r.width).unwrap_or_else(|| quantise(&rgb));
            let bits = packed_bits(o.depth);
            enc.set_depth(match bits {
                1 => png::BitDepth::One,
                4 => png::BitDepth::Four,
                _ => png::BitDepth::Eight,
            });
            enc.set_color(png::ColorType::Indexed);
            enc.set_palette(ix.palette.concat());
            let w = r.width as usize;
            ix.indices.chunks(w.max(1)).flat_map(|row| pack_row(row, bits)).collect()
        };
        let mut writer = enc.write_header().map_err(|e| PlotError::Png(e.to_string()))?;
        writer.write_image_data(&data).map_err(|e| PlotError::Png(e.to_string()))?;
    }
    Ok(out)
}

/// Encodes a TIFF with resolution recorded: 24-bit RGB, indexed 256 colors or 8-bit grey.
pub(crate) fn encode_tiff_with(r: &Raster, dpi: f64, o: &RasterOptions) -> Result<Vec<u8>, PlotError> {
    use tiff::encoder::{Compression, colortype};
    let err = |e: tiff::TiffError| PlotError::Tiff(e.to_string());
    let rgb = flatten_rgb(r);
    let hundredths = (dpi * 100.0).round().clamp(1.0, f64::from(u32::MAX)) as u32;
    let res = tiff::encoder::Rational { n: hundredths, d: 100 };
    let compression = match o.tiff_compression {
        TiffCompression::None => Compression::Uncompressed,
        TiffCompression::PackBits => Compression::Packbits,
        TiffCompression::Lzw => Compression::Lzw,
    };
    let mut out = std::io::Cursor::new(Vec::new());
    {
        let mut enc = tiff::encoder::TiffEncoder::new(&mut out).map_err(err)?.with_compression(compression);
        if o.depth == ColorDepth::Grey8 {
            let mut image = enc.new_image::<colortype::Gray8>(r.width, r.height).map_err(err)?;
            image.resolution(tiff::tags::ResolutionUnit::Inch, res);
            image.write_data(&luma(&rgb)).map_err(err)?;
        } else if let Some(ix) = indexed_for(o.depth, &rgb, r.width) {
            // Mono and 16 colours are stored as 8-bit indices into a 2- or 16-colour table padded
            // to the 256 entries an 8-bit TIFF palette needs.
            // Palette images store one 8-bit index per pixel, like Gray8's sample layout.
            // Override its interpretation and attach TIFF's planar 16-bit RGB color table.
            let mut image = enc.new_image::<colortype::Gray8>(r.width, r.height).map_err(err)?;
            image.resolution(tiff::tags::ResolutionUnit::Inch, res);
            image
                .encoder()
                .write_tag(
                    tiff::tags::Tag::PhotometricInterpretation,
                    tiff::tags::PhotometricInterpretation::RGBPalette.to_u16(),
                )
                .map_err(err)?;
            let palette = &ix.palette;
            let map: Vec<u16> = (0..3)
                .flat_map(|channel| {
                    (0..256).map(move |i| palette.get(i).map_or(0, |rgb| u16::from(rgb[channel]) * 257))
                })
                .collect();
            image.encoder().write_tag(tiff::tags::Tag::ColorMap, map.as_slice()).map_err(err)?;
            image.write_data(&ix.indices).map_err(err)?;
        } else {
            let mut image = enc.new_image::<colortype::RGB8>(r.width, r.height).map_err(err)?;
            image.resolution(tiff::tags::ResolutionUnit::Inch, res);
            image.write_data(&rgb).map_err(err)?;
        }
    }
    Ok(out.into_inner())
}

/// Encodes the picture as a baseline JPEG (24-bit or 8-bit grey) with the resolution recorded.
pub(crate) fn encode_jpeg(r: &Raster, dpi: f64, o: &RasterOptions) -> Result<Vec<u8>, PlotError> {
    use image::ExtendedColorType;
    use image::codecs::jpeg::{JpegEncoder, PixelDensity};
    let rgb = flatten_rgb(r);
    let mut out = Vec::new();
    let mut enc = JpegEncoder::new_with_quality(&mut out, o.jpeg_quality.clamp(1, 100));
    enc.set_pixel_density(PixelDensity::dpi(dpi.round().clamp(1.0, 65535.0) as u16));
    let res = if o.depth == ColorDepth::Grey8 {
        enc.encode(&luma(&rgb), r.width, r.height, ExtendedColorType::L8)
    } else {
        enc.encode(&rgb, r.width, r.height, ExtendedColorType::Rgb8)
    };
    res.map_err(|e| PlotError::Image(e.to_string()))?;
    Ok(out)
}

/// Encodes the picture as an uncompressed Windows bitmap: 8-bit with a palette (256 colours or
/// grey) or 24-bit, with the resolution recorded.
pub(crate) fn encode_bmp(r: &Raster, dpi: f64, o: &RasterOptions) -> Result<Vec<u8>, PlotError> {
    let rgb = flatten_rgb(r);
    let indexed = indexed_for(o.depth, &rgb, r.width);
    let (w, h) = (r.width as usize, r.height as usize);
    let bits = packed_bits(o.depth);
    let colours = 1usize << bits;
    let (bpp, pal_bytes) = if indexed.is_some() { (bits, 4 * colours) } else { (24, 0) };
    let stride = (w * bpp).div_ceil(32) * 4;
    let data_len = stride * h;
    let offset = 14 + 40 + pal_bytes;
    let too_big = || PlotError::Image("the picture is too large for a BMP file".to_string());
    let file_len = u32::try_from(offset + data_len).map_err(|_| too_big())?;
    let mut out = Vec::with_capacity(offset + data_len);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&file_len.to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&(offset as u32).to_le_bytes());
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(r.width as i32).to_le_bytes());
    out.extend_from_slice(&(r.height as i32).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&(bpp as u16).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(data_len as u32).to_le_bytes());
    out.extend_from_slice(&dpm(dpi).to_le_bytes());
    out.extend_from_slice(&dpm(dpi).to_le_bytes());
    out.extend_from_slice(&(if indexed.is_some() { colours as u32 } else { 0 }).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    if let Some(ix) = &indexed {
        for i in 0..colours {
            let c = ix.palette.get(i).copied().unwrap_or([0, 0, 0]);
            out.extend_from_slice(&[c[2], c[1], c[0], 0]);
        }
    }
    // Rows run bottom to top.
    for y in (0..h).rev() {
        let start = out.len();
        match &indexed {
            Some(ix) => out.extend_from_slice(&pack_row(&ix.indices[y * w..(y + 1) * w], bits)),
            None => {
                for p in rgb[y * w * 3..(y + 1) * w * 3].as_chunks::<3>().0 {
                    out.extend_from_slice(&[p[2], p[1], p[0]]);
                }
            }
        }
        out.resize(start + stride, 0);
    }
    Ok(out)
}

/// Encodes the picture as an uncompressed Truevision TGA file: 24-bit colour, 8-bit grey, or 256
/// colours with a palette, top row first. TGA has no resolution field.
pub(crate) fn encode_tga(r: &Raster, o: &RasterOptions) -> Result<Vec<u8>, PlotError> {
    let too_big = || PlotError::Image("the picture is too large for a TGA file".to_string());
    let w = u16::try_from(r.width).map_err(|_| too_big())?;
    let h = u16::try_from(r.height).map_err(|_| too_big())?;
    let rgb = flatten_rgb(r);
    let (image_type, has_map, bpp, body, map): (u8, bool, u8, Vec<u8>, Vec<u8>) = match o.depth {
        ColorDepth::Bit24 => {
            let body = rgb.as_chunks::<3>().0.iter().flat_map(|p| [p[2], p[1], p[0]]).collect();
            (2, false, 24, body, Vec::new())
        }
        ColorDepth::Grey8 => (3, false, 8, luma(&rgb), Vec::new()),
        // TGA has no 1- or 4-bit colour-mapped form: mono and 16 colours use an 8-bit map.
        ColorDepth::Colors256 | ColorDepth::Mono | ColorDepth::Colors16 => {
            let ix = indexed_for(o.depth, &rgb, r.width).unwrap_or_else(|| quantise(&rgb));
            let map = (0..256).flat_map(|i| ix.palette.get(i).map_or([0, 0, 0], |c| [c[2], c[1], c[0]])).collect();
            (1, true, 8, ix.indices, map)
        }
    };
    let mut out = Vec::with_capacity(18 + map.len() + body.len());
    out.push(0); // no image id
    out.push(u8::from(has_map));
    out.push(image_type);
    out.extend_from_slice(&0u16.to_le_bytes()); // first palette entry
    out.extend_from_slice(&(if has_map { 256u16 } else { 0 }).to_le_bytes());
    out.push(if has_map { 24 } else { 0 });
    out.extend_from_slice(&0u16.to_le_bytes()); // x origin
    out.extend_from_slice(&0u16.to_le_bytes()); // y origin
    out.extend_from_slice(&w.to_le_bytes());
    out.extend_from_slice(&h.to_le_bytes());
    out.push(bpp);
    out.push(0x20); // rows run top to bottom
    out.extend_from_slice(&map);
    out.extend_from_slice(&body);
    Ok(out)
}

/// Encodes the picture as a run-length PCX file (version 5): 24-bit colour as three planes, or 8 bits
/// with a 256-entry palette at the end (256 colours or grey), with the resolution recorded.
pub(crate) fn encode_pcx(r: &Raster, dpi: f64, o: &RasterOptions) -> Result<Vec<u8>, PlotError> {
    let too_big = || PlotError::Image("the picture is too large for a PCX file".to_string());
    let (w, h) = (r.width as usize, r.height as usize);
    let xmax = u16::try_from(w.saturating_sub(1)).map_err(|_| too_big())?;
    let ymax = u16::try_from(h.saturating_sub(1)).map_err(|_| too_big())?;
    let line = w + w % 2; // bytes per plane line, even
    let line16 = u16::try_from(line).map_err(|_| too_big())?;
    let rgb = flatten_rgb(r);
    // Mono and 16 colours are written as 8-bit indices with the palette at the end.
    let indexed = indexed_for(o.depth, &rgb, r.width);
    let planes: u8 = if indexed.is_some() { 1 } else { 3 };
    let dpi16 = dpi.round().clamp(1.0, f64::from(u16::MAX)) as u16;
    let mut out = Vec::with_capacity(128 + w * h * usize::from(planes) + 769);
    out.extend_from_slice(&[0x0A, 5, 1, 8]);
    for v in [0, 0, xmax, ymax, dpi16, dpi16] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out.extend_from_slice(&[0; 48]); // 16-colour palette, unused
    out.push(0);
    out.push(planes);
    out.extend_from_slice(&line16.to_le_bytes());
    let grey = matches!(o.depth, ColorDepth::Grey8);
    out.extend_from_slice(&(if grey { 2u16 } else { 1 }).to_le_bytes());
    out.extend_from_slice(&[0; 58]);
    debug_assert_eq!(out.len(), 128);
    let mut row = vec![0u8; line];
    for y in 0..h {
        for p in 0..usize::from(planes) {
            for (x, b) in row.iter_mut().enumerate().take(w) {
                *b = match &indexed {
                    Some(ix) => ix.indices[y * w + x],
                    None => rgb[(y * w + x) * 3 + p],
                };
            }
            pcx_rle(&row, &mut out);
        }
    }
    if let Some(ix) = &indexed {
        out.push(0x0C);
        for i in 0..256 {
            out.extend_from_slice(&ix.palette.get(i).copied().unwrap_or([0, 0, 0]));
        }
    }
    Ok(out)
}

/// PCX run-length encoding of one plane line: runs of up to 63 equal bytes are written as
/// `0xC0 | count` and the byte; a single byte below `0xC0` is written as it is.
fn pcx_rle(line: &[u8], out: &mut Vec<u8>) {
    let mut i = 0;
    while i < line.len() {
        let b = line[i];
        let mut n = 1;
        while i + n < line.len() && line[i + n] == b && n < 63 {
            n += 1;
        }
        if n > 1 || b >= 0xC0 {
            out.push(0xC0 | n as u8);
        }
        out.push(b);
        i += n;
    }
}

/// Encodes the picture as a GIF (no transparency): an optimised 256-colour palette, or the exact
/// palette of the mono, 16-colour and grey depths.
pub(crate) fn encode_gif(r: &Raster, o: &RasterOptions) -> Result<Vec<u8>, PlotError> {
    if matches!(o.depth, ColorDepth::Mono | ColorDepth::Colors16 | ColorDepth::Grey8) {
        let ix = indexed_for(o.depth, &flatten_rgb(r), r.width).unwrap_or_else(|| quantise(&flatten_rgb(r)));
        let err = |e: gif::EncodingError| PlotError::Image(e.to_string());
        let too_big = || PlotError::Image("the picture is too large for a GIF file".to_string());
        let (w, h) = (u16::try_from(r.width).map_err(|_| too_big())?, u16::try_from(r.height).map_err(|_| too_big())?);
        let palette = ix.palette.concat();
        let mut out = Vec::new();
        {
            let mut enc = gif::Encoder::new(&mut out, w, h, &palette).map_err(err)?;
            let frame = gif::Frame {
                width: w,
                height: h,
                buffer: std::borrow::Cow::Borrowed(&ix.indices),
                ..gif::Frame::default()
            };
            enc.write_frame(&frame).map_err(err)?;
        }
        return Ok(out);
    }
    use image::codecs::gif::GifEncoder;
    use image::{Delay, Frame, RgbaImage};
    let rgb = flatten_rgb(r);
    let rgba: Vec<u8> = rgb.as_chunks::<3>().0.iter().flat_map(|p| [p[0], p[1], p[2], 255]).collect();
    let img = RgbaImage::from_raw(r.width, r.height, rgba)
        .ok_or_else(|| PlotError::Image("the picture size does not match its pixels".to_string()))?;
    let mut out = Vec::new();
    {
        let mut enc = GifEncoder::new_with_speed(&mut out, 10);
        enc.encode_frame(Frame::from_parts(img, 0, 0, Delay::from_numer_denom_ms(0, 1)))
            .map_err(|e| PlotError::Image(e.to_string()))?;
    }
    Ok(out)
}

/// Encodes the picture in the raster format named by `ext` (see [`is_raster_ext`]).
pub(crate) fn encode_raster(r: &Raster, ext: &str, dpi: f64, o: &RasterOptions) -> Result<Vec<u8>, PlotError> {
    match ext {
        "png" => encode_png_with(r, dpi, o),
        "tif" | "tiff" => encode_tiff_with(r, dpi, o),
        "jpg" | "jpeg" => encode_jpeg(r, dpi, o),
        "bmp" => encode_bmp(r, dpi, o),
        "gif" => encode_gif(r, o),
        "tga" => encode_tga(r, o),
        "pcx" => encode_pcx(r, dpi, o),
        "psd" => encode_psd(r, dpi, o),
        _ => Err(PlotError::UnknownFormat(ext.to_string())),
    }
}

/// A flattened PSD; transparency is composited on white like the other opaque raster exports.
fn encode_psd(r: &Raster, dpi: f64, o: &RasterOptions) -> Result<Vec<u8>, PlotError> {
    let rgb = flatten_rgb(r);
    match o.depth {
        ColorDepth::Bit24 => crate::psd::encode(r.width, r.height, dpi, crate::psd::Pixels::Rgb(&rgb)),
        ColorDepth::Grey8 => {
            let grey = luma(&rgb);
            crate::psd::encode(r.width, r.height, dpi, crate::psd::Pixels::Grey(&grey))
        }
        ColorDepth::Colors256 | ColorDepth::Mono | ColorDepth::Colors16 => {
            let ix = indexed_for(o.depth, &rgb, r.width).unwrap_or_else(|| quantise(&rgb));
            crate::psd::encode(
                r.width,
                r.height,
                dpi,
                crate::psd::Pixels::Indexed { palette: &ix.palette, indices: &ix.indices },
            )
        }
    }
}
