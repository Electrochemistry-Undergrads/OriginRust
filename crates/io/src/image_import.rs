//! File > Import > Image (T204, matrices spec 7h): a picture file read into one matrix sheet of gray
//! levels. Origin imports pictures into matrix books too; it keeps the colours, while Sheaf keeps the
//! luminance (0 black to 255 white), the first matrix row being the picture's top row.

use std::path::Path;

/// A decoded picture: `rows` x `cols` gray levels, row by row from the top.
#[derive(Clone, Debug, PartialEq)]
pub struct GrayImage {
    pub rows: usize,
    pub cols: usize,
    pub values: Vec<f64>,
}

/// The picture formats the importer reads (lowercase extensions).
pub const EXTENSIONS: [&str; 6] = ["png", "jpg", "jpeg", "gif", "bmp", "dib"];

/// Decodes `bytes` (PNG, JPEG, GIF or BMP, found from the content) into gray levels.
pub fn decode_gray(bytes: &[u8]) -> Result<GrayImage, String> {
    let img = image::load_from_memory(bytes).map_err(|e| format!("not a picture Sheaf can read: {e}"))?;
    let gray = img.to_luma8();
    let (cols, rows) = (gray.width() as usize, gray.height() as usize);
    if rows == 0 || cols == 0 {
        return Err("the picture is empty".into());
    }
    Ok(GrayImage { rows, cols, values: gray.into_raw().into_iter().map(f64::from).collect() })
}

/// Reads and decodes a picture file.
pub fn read_gray(path: &Path) -> Result<GrayImage, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    decode_gray(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(width: u32, height: u32, rgb: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let enc = image::codecs::png::PngEncoder::new(&mut out);
        image::ImageEncoder::write_image(enc, rgb, width, height, image::ExtendedColorType::Rgb8).unwrap();
        out
    }

    #[test]
    fn a_png_becomes_gray_levels_top_row_first() {
        // 2 x 2: white, black / red, gray 128.
        let bytes = png(2, 2, &[255, 255, 255, 0, 0, 0, 255, 0, 0, 128, 128, 128]);
        let g = decode_gray(&bytes).unwrap();
        assert_eq!((g.rows, g.cols), (2, 2));
        assert_eq!(g.values[0], 255.0);
        assert_eq!(g.values[1], 0.0);
        assert!((g.values[2] - 54.0).abs() <= 1.0, "red's luminance: {}", g.values[2]);
        assert_eq!(g.values[3], 128.0);
        assert!(decode_gray(b"not a picture").is_err());
    }
}
