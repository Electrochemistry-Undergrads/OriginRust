//! Flattened 8-bit PSD files, written from the published Adobe file-format specification.
//!
//! https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/
//! There are no layers, masks, alpha channels or compressed pixel streams in these exports.

use crate::PlotError;

/// The per-dimension limit of version-1 PSD (PSB is a separate format).
pub(crate) const MAX_DIMENSION: u32 = 30_000;

/// Opaque pixels in row order. RGB input is interleaved; PSD output is planar.
pub(crate) enum Pixels<'a> {
    Rgb(&'a [u8]),
    Grey(&'a [u8]),
    Indexed { palette: &'a [[u8; 3]], indices: &'a [u8] },
}

fn error(message: impl Into<String>) -> PlotError {
    PlotError::Image(format!("PSD: {}", message.into()))
}

fn put_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_be_bytes());
}

/// Encodes a flattened PSD with resolution in pixels per inch and display size in inches.
pub(crate) fn encode(width: u32, height: u32, dpi: f64, pixels: Pixels<'_>) -> Result<Vec<u8>, PlotError> {
    if !(1..=MAX_DIMENSION).contains(&width) || !(1..=MAX_DIMENSION).contains(&height) {
        return Err(error("width and height must be between 1 and 30000 pixels"));
    }
    // ResolutionInfo uses positive signed 16.16 Fixed values. Do not silently wrap a large
    // resolution or round a tiny positive resolution to zero.
    let resolution = (dpi * 65_536.0).round();
    if !dpi.is_finite() || dpi <= 0.0 || !(1.0..=f64::from(i32::MAX)).contains(&resolution) {
        return Err(error("resolution must be positive and representable in 16.16 fixed point"));
    }
    let resolution = resolution as u32;
    let count = usize::try_from(width)
        .ok()
        .and_then(|w| usize::try_from(height).ok().and_then(|h| w.checked_mul(h)))
        .ok_or_else(|| error("image dimensions overflow the pixel count"))?;
    let (channels, mode, data, palette) = match pixels {
        Pixels::Rgb(data) => (3u16, 3u16, data, None),
        Pixels::Grey(data) => (1, 1, data, None),
        Pixels::Indexed { palette, indices } => {
            if palette.is_empty() || palette.len() > 256 {
                return Err(error("an indexed palette must contain 1 to 256 colors"));
            }
            (1, 2, indices, Some(palette))
        }
    };
    let expected = count.checked_mul(usize::from(channels)).ok_or_else(|| error("pixel buffer length overflow"))?;
    if data.len() != expected {
        return Err(error(format!("pixel buffer has {} bytes; expected {expected}", data.len())));
    }
    if let Some(palette) = palette
        && data.iter().any(|&index| usize::from(index) >= palette.len())
    {
        return Err(error("an indexed pixel refers to a color outside its palette"));
    }
    let palette_length = if palette.is_some() { 768 } else { 0 };
    let capacity = 68usize
        .checked_add(palette_length)
        .and_then(|n| n.checked_add(data.len()))
        .ok_or_else(|| error("file length overflow"))?;
    let mut out = Vec::new();
    out.try_reserve_exact(capacity).map_err(|e| error(format!("cannot allocate the encoded image: {e}")))?;

    out.extend_from_slice(b"8BPS");
    put_u16(&mut out, 1);
    out.extend_from_slice(&[0; 6]);
    put_u16(&mut out, channels);
    put_u32(&mut out, height);
    put_u32(&mut out, width);
    put_u16(&mut out, 8);
    put_u16(&mut out, mode);

    // Indexed color tables contain 256 reds, then 256 greens, then 256 blues.
    put_u32(&mut out, palette_length as u32);
    if let Some(palette) = palette {
        for channel in 0..3 {
            for index in 0..256 {
                out.push(palette.get(index).map_or(0, |rgb| rgb[channel]));
            }
        }
    }

    // One image-resource block: 1005 (ResolutionInfo), with an empty Pascal name.
    put_u32(&mut out, 28);
    out.extend_from_slice(b"8BIM");
    put_u16(&mut out, 1005);
    out.extend_from_slice(&[0, 0]);
    put_u32(&mut out, 16);
    put_u32(&mut out, resolution);
    put_u16(&mut out, 1); // Horizontal resolution unit: pixels per inch.
    put_u16(&mut out, 1); // Width display unit: inches.
    put_u32(&mut out, resolution);
    put_u16(&mut out, 1); // Vertical resolution unit: pixels per inch.
    put_u16(&mut out, 1); // Height display unit: inches.

    put_u32(&mut out, 0); // No layer or mask records.
    put_u16(&mut out, 0); // Raw, uncompressed composite image.
    if channels == 3 {
        for channel in 0..3 {
            out.extend(data.as_chunks::<3>().0.iter().map(|rgb| rgb[channel]));
        }
    } else {
        out.extend_from_slice(data);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u16_at(bytes: &[u8], at: usize) -> u16 {
        u16::from_be_bytes(bytes[at..at + 2].try_into().unwrap())
    }

    fn u32_at(bytes: &[u8], at: usize) -> u32 {
        u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap())
    }

    #[test]
    fn rgb_header_and_asymmetric_pixels_follow_psd_channel_order() {
        let bytes = encode(2, 2, 300.0, Pixels::Rgb(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12])).unwrap();
        assert_eq!(&bytes[..12], b"8BPS\0\x01\0\0\0\0\0\0");
        assert_eq!((u16_at(&bytes, 12), u32_at(&bytes, 14), u32_at(&bytes, 18)), (3, 2, 2));
        assert_eq!((u16_at(&bytes, 22), u16_at(&bytes, 24)), (8, 3));
        assert_eq!(u32_at(&bytes, 26), 0, "RGB has no color-mode data");
        assert_eq!(u32_at(&bytes, 62), 0, "no layer/mask section");
        assert_eq!(u16_at(&bytes, 66), 0, "uncompressed image");
        assert_eq!(&bytes[68..], &[1, 4, 7, 10, 2, 5, 8, 11, 3, 6, 9, 12]);
        assert_eq!(bytes.len(), 80);
    }

    #[test]
    fn resolution_resource_preserves_fractional_dpi_and_inch_units() {
        let bytes = encode(1, 1, 150.5, Pixels::Grey(&[23])).unwrap();
        assert_eq!(u32_at(&bytes, 30), 28);
        assert_eq!(&bytes[34..38], b"8BIM");
        assert_eq!(u16_at(&bytes, 38), 1005);
        assert_eq!(&bytes[40..42], &[0, 0], "empty name uses two bytes");
        assert_eq!(u32_at(&bytes, 42), 16);
        assert_eq!(u32_at(&bytes, 46), 150 * 65_536 + 32_768);
        assert_eq!((u16_at(&bytes, 50), u16_at(&bytes, 52)), (1, 1));
        assert_eq!(u32_at(&bytes, 54), u32_at(&bytes, 46));
        assert_eq!((u16_at(&bytes, 58), u16_at(&bytes, 60)), (1, 1));
    }

    #[test]
    fn grayscale_pixels_are_row_order_without_padding() {
        let bytes = encode(3, 2, 72.0, Pixels::Grey(&[0, 17, 255, 91, 128, 201])).unwrap();
        assert_eq!((u16_at(&bytes, 12), u16_at(&bytes, 24)), (1, 1));
        assert_eq!((u32_at(&bytes, 14), u32_at(&bytes, 18)), (2, 3));
        assert_eq!(&bytes[68..], &[0, 17, 255, 91, 128, 201]);
    }

    #[test]
    fn indexed_palette_is_planar_and_unused_entries_are_zero() {
        let palette = [[10, 20, 30], [40, 50, 60], [70, 80, 90]];
        let bytes = encode(2, 2, 96.0, Pixels::Indexed { palette: &palette, indices: &[2, 0, 1, 2] }).unwrap();
        assert_eq!((u16_at(&bytes, 12), u16_at(&bytes, 24)), (1, 2));
        assert_eq!(u32_at(&bytes, 26), 768);
        assert_eq!(&bytes[30..33], &[10, 40, 70]);
        assert!(bytes[33..286].iter().all(|&x| x == 0));
        assert_eq!(&bytes[286..289], &[20, 50, 80]);
        assert!(bytes[289..542].iter().all(|&x| x == 0));
        assert_eq!(&bytes[542..545], &[30, 60, 90]);
        assert!(bytes[545..798].iter().all(|&x| x == 0));
        assert_eq!(u32_at(&bytes, 798), 28);
        assert_eq!(&bytes[836..], &[2, 0, 1, 2]);
        assert_eq!(bytes.len(), 840);
    }

    #[test]
    fn malformed_buffers_dimensions_palettes_and_indices_are_errors() {
        for (w, h) in [(0, 1), (1, 0), (30_001, 1), (1, 30_001), (u32::MAX, u32::MAX)] {
            assert!(encode(w, h, 72.0, Pixels::Grey(&[])).is_err(), "{w} x {h}");
        }
        for pixels in [Pixels::Rgb(&[1, 2]), Pixels::Rgb(&[1, 2, 3, 4]), Pixels::Grey(&[]), Pixels::Grey(&[1, 2])] {
            assert!(matches!(encode(1, 1, 72.0, pixels), Err(PlotError::Image(_))));
        }
        assert!(encode(1, 1, 72.0, Pixels::Indexed { palette: &[], indices: &[0] }).is_err());
        assert!(encode(1, 1, 72.0, Pixels::Indexed { palette: &[[0; 3]; 257], indices: &[0] }).is_err());
        assert!(encode(1, 1, 72.0, Pixels::Indexed { palette: &[[0; 3]], indices: &[1] }).is_err());
        assert!(encode(1, 1, 72.0, Pixels::Indexed { palette: &[[0; 3]], indices: &[] }).is_err());
        assert!(encode(1, 1, 72.0, Pixels::Indexed { palette: &[[0; 3]], indices: &[0, 0] }).is_err());
    }

    #[test]
    fn invalid_or_unrepresentable_resolutions_are_errors() {
        for dpi in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1.0e20, 1.0e-20, 32_768.0] {
            assert!(encode(1, 1, dpi, Pixels::Grey(&[0])).is_err(), "{dpi}");
        }
        let largest = f64::from(i32::MAX) / 65_536.0;
        assert!(encode(1, 1, largest, Pixels::Grey(&[0])).is_ok());
        assert!(encode(1, 1, 1.0 / 65_536.0, Pixels::Grey(&[0])).is_ok());
    }

    #[test]
    fn maximum_dimension_and_complete_palette_remain_valid() {
        let line = vec![19; 30_000];
        assert!(encode(30_000, 1, 300.0, Pixels::Grey(&line)).is_ok());
        assert!(encode(1, 30_000, 300.0, Pixels::Grey(&line)).is_ok());
        let bytes = encode(1, 1, 300.0, Pixels::Indexed { palette: &[[1, 2, 3]; 256], indices: &[255] }).unwrap();
        assert_eq!(bytes.last(), Some(&255));
    }
}
