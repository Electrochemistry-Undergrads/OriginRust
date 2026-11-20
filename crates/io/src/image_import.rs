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

/// A decoded picture as it should go into a matrix book (T218): one Gray sheet when the picture
/// has no colour, else Red, Green and Blue sheets (0 to 255 each).
#[derive(Clone, Debug, PartialEq)]
pub struct Picture {
    pub rows: usize,
    pub cols: usize,
    /// (sheet name, values row by row from the top).
    pub channels: Vec<(String, Vec<f64>)>,
}

/// Decodes `bytes` into a [`Picture`]: colour channels when any pixel has colour.
pub fn decode_picture(bytes: &[u8]) -> Result<Picture, String> {
    let img = image::load_from_memory(bytes).map_err(|e| format!("not a picture Sheaf can read: {e}"))?;
    let rgb = img.to_rgb8();
    let (cols, rows) = (rgb.width() as usize, rgb.height() as usize);
    if rows == 0 || cols == 0 {
        return Err("the picture is empty".into());
    }
    let colour = rgb.pixels().any(|p| p[0] != p[1] || p[1] != p[2]);
    let channels = if colour {
        (0..3)
            .map(|k| {
                let name = ["Red", "Green", "Blue"][k].to_string();
                (name, rgb.pixels().map(|p| f64::from(p[k])).collect())
            })
            .collect()
    } else {
        vec![("Gray".to_string(), rgb.pixels().map(|p| f64::from(p[0])).collect())]
    };
    Ok(Picture { rows, cols, channels })
}

/// Reads and decodes a picture file as a [`Picture`].
pub fn read_picture(path: &Path) -> Result<Picture, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    decode_picture(&bytes)
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

    #[test]
    fn a_colour_picture_keeps_three_channels_and_a_gray_one_keeps_one() {
        let colour = decode_picture(&png(2, 1, &[255, 0, 0, 0, 0, 255])).unwrap();
        let names: Vec<&str> = colour.channels.iter().map(|c| c.0.as_str()).collect();
        assert_eq!(names, ["Red", "Green", "Blue"]);
        assert_eq!(colour.channels[0].1, [255.0, 0.0]);
        assert_eq!(colour.channels[2].1, [0.0, 255.0]);
        let gray = decode_picture(&png(2, 1, &[7, 7, 7, 9, 9, 9])).unwrap();
        assert_eq!(gray.channels, vec![("Gray".to_string(), vec![7.0, 9.0])]);
    }
}
