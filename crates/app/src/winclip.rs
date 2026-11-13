//! Copy Page on the Windows clipboard: a bitmap and an enhanced metafile together, as Origin does
//! with `AddEMFInCopyPage=1`. A program that pastes picks the form it prefers (a vector editor the
//! metafile, a word processor either). Elsewhere there is no metafile and the picture goes through
//! egui's image copy.

/// A device-independent bitmap (`CF_DIB`: the 40-byte `BITMAPINFOHEADER` and the pixels) of straight
/// RGBA8 `rgba`, `width` x `height`. Pixels are bottom-up 24-bit BGR with rows padded to 4 bytes,
/// and laid on white, which is what a program that ignores transparency would show anyway.
pub fn dib_from_rgba(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let (w, h) = (width as usize, height as usize);
    let stride = (w * 3).div_ceil(4) * 4;
    let image = stride * h;
    let mut out = Vec::with_capacity(40 + image);
    out.extend_from_slice(&40u32.to_le_bytes()); // biSize
    out.extend_from_slice(&(width as i32).to_le_bytes());
    out.extend_from_slice(&(height as i32).to_le_bytes()); // positive: rows from the bottom
    out.extend_from_slice(&1u16.to_le_bytes()); // planes
    out.extend_from_slice(&24u16.to_le_bytes()); // bit count
    out.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB
    out.extend_from_slice(&(image as u32).to_le_bytes());
    out.extend_from_slice(&[0x13, 0x0B, 0, 0, 0x13, 0x0B, 0, 0]); // 2835 pixels per metre (72 dpi)
    out.extend_from_slice(&[0; 8]); // colours used, important
    for y in (0..h).rev() {
        let row = &rgba[y * w * 4..(y + 1) * w * 4];
        for px in row.as_chunks::<4>().0 {
            let a = u32::from(px[3]);
            let over_white = |c: u8| ((u32::from(c) * a + 255 * (255 - a) + 127) / 255) as u8;
            out.extend_from_slice(&[over_white(px[2]), over_white(px[1]), over_white(px[0])]);
        }
        out.resize(out.len() + (stride - w * 3), 0);
    }
    out
}

/// Puts the bitmap `dib` (see [`dib_from_rgba`]) and the enhanced metafile `emf` (the bytes of an
/// `.emf` file) on the clipboard in one go, replacing what was there.
#[cfg(windows)]
pub fn put_bitmap_and_emf(dib: &[u8], emf: &[u8]) -> Result<(), String> {
    use clipboard_win::{Clipboard, raw};

    // Safe to declare here: both are plain Win32 calls with the signatures below.
    #[link(name = "gdi32")]
    unsafe extern "system" {
        fn SetEnhMetaFileBits(size: u32, bits: *const u8) -> isize;
        fn DeleteEnhMetaFile(handle: isize) -> i32;
    }
    #[link(name = "user32")]
    unsafe extern "system" {
        fn SetClipboardData(format: u32, handle: isize) -> isize;
    }
    const CF_DIB: u32 = 8;
    const CF_ENHMETAFILE: u32 = 14;

    let _clipboard = Clipboard::new_attempts(10).map_err(|e| format!("the clipboard is busy: {e}"))?;
    raw::empty().map_err(|e| format!("could not clear the clipboard: {e}"))?;
    raw::set_without_clear(CF_DIB, dib).map_err(|e| format!("could not put the bitmap on the clipboard: {e}"))?;
    let size = u32::try_from(emf.len()).map_err(|_| "the metafile is too large".to_string())?;
    // SAFETY: `emf` is valid for `size` bytes; the call copies them into a new metafile handle. On
    // success the clipboard owns the handle, on failure it is deleted here.
    unsafe {
        let handle = SetEnhMetaFileBits(size, emf.as_ptr());
        if handle == 0 {
            return Err("Windows did not accept the metafile".to_string());
        }
        if SetClipboardData(CF_ENHMETAFILE, handle) == 0 {
            DeleteEnhMetaFile(handle);
            return Err("could not put the metafile on the clipboard".to_string());
        }
    }
    Ok(())
}

/// There is no metafile clipboard format off Windows.
#[cfg(not(windows))]
pub fn put_bitmap_and_emf(_dib: &[u8], _emf: &[u8]) -> Result<(), String> {
    Err("metafiles are only put on the clipboard on Windows".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_dib_is_bottom_up_bgr_on_white_with_padded_rows() {
        // 3 x 2: top row red, green, transparent; bottom row blue, half-transparent black, white.
        let rgba = [
            255, 0, 0, 255, 0, 255, 0, 255, 9, 9, 9, 0, //
            0, 0, 255, 255, 0, 0, 0, 128, 255, 255, 255, 255,
        ];
        let d = dib_from_rgba(3, 2, &rgba);
        assert_eq!(u32::from_le_bytes(d[0..4].try_into().unwrap()), 40);
        assert_eq!(i32::from_le_bytes(d[4..8].try_into().unwrap()), 3);
        assert_eq!(i32::from_le_bytes(d[8..12].try_into().unwrap()), 2);
        assert_eq!(u16::from_le_bytes(d[14..16].try_into().unwrap()), 24);
        let stride = 12; // 9 bytes padded to 12
        assert_eq!(u32::from_le_bytes(d[20..24].try_into().unwrap()) as usize, stride * 2);
        assert_eq!(d.len(), 40 + stride * 2);
        let px = &d[40..];
        // The first stored row is the bottom row: blue is B=255, G=0, R=0.
        assert_eq!(&px[0..3], &[255, 0, 0]);
        // Half-transparent black over white is mid grey.
        assert!(px[3..6].iter().all(|c| (126..=128).contains(c)), "{:?}", &px[3..6]);
        assert_eq!(&px[6..9], &[255, 255, 255]);
        assert_eq!(&px[9..12], &[0, 0, 0], "row padding");
        // The second stored row is the top row: red is B=0, G=0, R=255; transparent is white.
        assert_eq!(&px[12..15], &[0, 0, 255]);
        assert_eq!(&px[15..18], &[0, 255, 0]);
        assert_eq!(&px[18..21], &[255, 255, 255]);
    }

    /// Writes the real clipboard, so it is not run with the others: `cargo test -p sheaf-app
    /// -- --ignored clipboard_gets_both_forms`.
    #[cfg(windows)]
    #[test]
    #[ignore]
    fn clipboard_gets_both_forms() {
        let mut p = sheaf_core::Project::new();
        let g = p.add_graph(sheaf_core::template::Template::Line);
        let emf = sheaf_plot::export_bytes(&p, g, "emf", 72.0, sheaf_plot::ExportSize::default()).unwrap();
        let dib = dib_from_rgba(2, 2, &[255; 16]);
        put_bitmap_and_emf(&dib, &emf).unwrap();
        assert!(clipboard_win::is_format_avail(8), "CF_DIB");
        assert!(clipboard_win::is_format_avail(14), "CF_ENHMETAFILE");
    }
}
