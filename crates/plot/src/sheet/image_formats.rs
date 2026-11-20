//! Worksheet image formats share the graph raster encoders; sheet layout stays in the parent.
use crate::{PlotError, RasterOptions, SystemFonts, psd, raster};
use sheaf_core::Worksheet;

/// The worksheet picture in a lowercase extension (without a dot), at `dpi` for raster formats.
/// PNG/PDF/SVG keep their existing paths. Other rasters use the graph encoder defaults.
pub fn sheet_image_bytes(sheet: &Worksheet, ext: &str, dpi: f64) -> Result<Vec<u8>, PlotError> {
    match ext {
        "png" => super::sheet_image_png(sheet, dpi),
        "pdf" => super::sheet_image_pdf(sheet),
        "svg" => super::sheet_image_svg(sheet).map(String::into_bytes),
        e if raster::is_raster_ext(e) => {
            let max_side = match e {
                "psd" => Some(psd::MAX_DIMENSION),
                "jpg" | "jpeg" | "gif" | "tga" => Some(u32::from(u16::MAX)),
                // PCX stores an even bytes-per-plane row length in u16. Use a common safe side cap.
                "pcx" => Some(u32::from(u16::MAX - 1)),
                _ => None,
            };
            let svg = super::sheet_image_svg(sheet)?;
            let image = raster::rasterise_with_limit(&svg, dpi / 72.0, SystemFonts::shared(), max_side)?;
            raster::encode_raster(&image, e, dpi, &RasterOptions::default())
        }
        _ => Err(PlotError::Image(format!(
            "worksheet pictures support PNG, PDF, SVG, JPEG, TIFF, BMP, GIF, TGA, PCX or PSD, not .{ext}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sheaf_core::{Column, ColumnData, Designation, ids::ColumnId};
    use std::io::Cursor;

    fn sheet() -> Worksheet {
        let mut sheet = Worksheet::new("Images");
        let mut c = Column::new(ColumnId(1), "A", Designation::Y, ColumnData::numeric(vec![2.5, 4.5]));
        c.long_name = "Current".into();
        c.units = "mA".into();
        sheet.columns.push(c);
        sheet
    }

    #[test]
    fn worksheet_rasters_have_the_same_dimensions_as_the_existing_png() {
        let sheet = sheet();
        let png = super::super::sheet_image_png(&sheet, 96.).unwrap();
        let reader = png::Decoder::new(Cursor::new(&png)).read_info().unwrap();
        let expected = (reader.info().width, reader.info().height);
        for ext in ["jpg", "jpeg", "tif", "tiff", "bmp", "gif", "tga", "pcx", "psd"] {
            let bytes = sheet_image_bytes(&sheet, ext, 96.).unwrap();
            let le16 = |i| u32::from(u16::from_le_bytes(bytes[i..i + 2].try_into().unwrap()));
            let le32 = |i| u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap());
            let be32 = |i| u32::from_be_bytes(bytes[i..i + 4].try_into().unwrap());
            let actual = match ext {
                "jpg" | "jpeg" | "gif" => {
                    let image = image::load_from_memory(&bytes).unwrap();
                    (image.width(), image.height())
                }
                "tif" | "tiff" => tiff::decoder::Decoder::new(Cursor::new(&bytes)).unwrap().dimensions().unwrap(),
                "bmp" => {
                    assert!(bytes.starts_with(b"BM"));
                    (le32(18), le32(22))
                }
                "tga" => {
                    assert_eq!(bytes[2], 2, "uncompressed true-colour TGA");
                    (le16(12), le16(14))
                }
                "pcx" => {
                    assert_eq!(&bytes[..3], &[10, 5, 1]);
                    (le16(8) - le16(4) + 1, le16(10) - le16(6) + 1)
                }
                "psd" => {
                    assert!(bytes.starts_with(b"8BPS"));
                    (be32(18), be32(14))
                }
                _ => unreachable!(),
            };
            assert_eq!(actual, expected, "{ext} must keep the worksheet picture's extent");
        }
    }

    #[test]
    fn existing_png_pdf_and_svg_routes_stay_fixed() {
        let sheet = sheet();
        assert_eq!(sheet_image_bytes(&sheet, "png", 96.).unwrap(), super::super::sheet_image_png(&sheet, 96.).unwrap());
        assert_eq!(
            sheet_image_bytes(&sheet, "svg", 0.).unwrap(),
            super::super::sheet_image_svg(&sheet).unwrap().as_bytes()
        );
        assert!(sheet_image_bytes(&sheet, "pdf", 0.).unwrap().starts_with(b"%PDF"));
    }

    #[test]
    fn unsupported_types_and_bad_or_excessive_raster_sizes_fail_before_encoding() {
        let sheet = sheet();
        for ext in ["eps", "emf", "wmf", "ai", "dxf", "cgm", "unknown"] {
            assert!(sheet_image_bytes(&sheet, ext, 96.).is_err(), "{ext}");
        }
        for ext in ["png", "jpg", "tif", "bmp", "gif", "tga", "pcx", "psd"] {
            for dpi in [0., -1., f64::NAN, f64::INFINITY, 1e12] {
                assert!(sheet_image_bytes(&sheet, ext, dpi).is_err(), "{ext} at {dpi}");
            }
        }
        for ext in ["jpg", "gif", "tga", "pcx", "psd"] {
            let error = sheet_image_bytes(&sheet, ext, 1e9).unwrap_err().to_string();
            assert!(error.contains("per dimension"), "{ext}: {error}");
        }
    }
}
