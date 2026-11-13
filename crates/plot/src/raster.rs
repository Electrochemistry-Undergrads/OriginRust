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

/// Colour depth of a raster export (Origin's "Color Depth" list; the 1- and 4-bit modes are not offered).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum ColorDepth {
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
    matches!(ext, "png" | "tif" | "tiff" | "jpg" | "jpeg" | "bmp" | "gif" | "tga")
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
    let rgba: Vec<u8> = rgb.as_chunks::<3>().0.iter().flat_map(|p| [p[0], p[1], p[2], 255]).collect();
    let nq = color_quant::NeuQuant::new(10, 256, &rgba);
    let palette = nq.color_map_rgb().as_chunks::<3>().0.to_vec();
    let indices = rgba.as_chunks::<4>().0.iter().map(|p| nq.index_of(p) as u8).collect();
    Indexed { palette, indices }
}

fn grey_indexed(rgb: &[u8]) -> Indexed {
    Indexed { palette: (0..=255u8).map(|g| [g, g, g]).collect(), indices: luma(rgb) }
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
            let ix = quantise(&rgb);
            enc.set_color(png::ColorType::Indexed);
            enc.set_palette(ix.palette.concat());
            ix.indices
        };
        let mut writer = enc.write_header().map_err(|e| PlotError::Png(e.to_string()))?;
        writer.write_image_data(&data).map_err(|e| PlotError::Png(e.to_string()))?;
    }
    Ok(out)
}

/// Encodes the picture as a TIFF with the resolution recorded (24-bit RGB or 8-bit grey).
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
    let indexed = match o.depth {
        ColorDepth::Colors256 => Some(quantise(&rgb)),
        ColorDepth::Grey8 => Some(grey_indexed(&rgb)),
        ColorDepth::Bit24 => None,
    };
    let (w, h) = (r.width as usize, r.height as usize);
    let (bpp, pal_bytes) = if indexed.is_some() { (8usize, 1024usize) } else { (24, 0) };
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
    out.extend_from_slice(&(if indexed.is_some() { 256u32 } else { 0 }).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    if let Some(ix) = &indexed {
        for i in 0..256 {
            let c = ix.palette.get(i).copied().unwrap_or([0, 0, 0]);
            out.extend_from_slice(&[c[2], c[1], c[0], 0]);
        }
    }
    // Rows run bottom to top.
    for y in (0..h).rev() {
        let start = out.len();
        match &indexed {
            Some(ix) => out.extend_from_slice(&ix.indices[y * w..(y + 1) * w]),
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
        ColorDepth::Colors256 => {
            let ix = quantise(&rgb);
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

/// Encodes the picture as a GIF (256 colours, no transparency).
pub(crate) fn encode_gif(r: &Raster) -> Result<Vec<u8>, PlotError> {
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
        "gif" => encode_gif(r),
        "tga" => encode_tga(r, o),
        _ => Err(PlotError::UnknownFormat(ext.to_string())),
    }
}
