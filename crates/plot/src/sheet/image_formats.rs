//! Worksheet export uses the same raster and vector writers as graph export.
use crate::{PlotError, RasterOptions, SystemFonts, emf, eps, psd, raster};
use sheaf_core::Worksheet;

/// The worksheet picture in a lowercase extension (without a dot), at `dpi` for raster formats.
/// EPS and EMF use the worksheet scene at its original size. Additional rasters use
/// the graph encoder defaults.
pub fn sheet_image_bytes(sheet: &Worksheet, ext: &str, dpi: f64) -> Result<Vec<u8>, PlotError> {
    match ext {
        "png" => super::sheet_image_png(sheet, dpi),
        "pdf" => super::sheet_image_pdf(sheet),
        "svg" => super::sheet_image_svg(sheet).map(String::into_bytes),
        "eps" => Ok(eps::write_eps(&super::sheet_image_scene(sheet)?, 1.0).bytes),
        "emf" => Ok(emf::write_emf(&super::sheet_image_scene(sheet)?, 1.0, SystemFonts::shared())),
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
            "worksheet pictures support PNG, PDF, SVG, EPS, EMF, JPEG, TIFF, BMP, GIF, TGA, PCX or PSD, not .{ext}"
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
    fn eps_and_emf_contain_worksheet_text_at_the_original_extent() {
        let sheet = sheet();
        let (width, height) = super::super::sheet_image_size(&sheet).unwrap();
        let eps = String::from_utf8(sheet_image_bytes(&sheet, "eps", 300.0).unwrap()).unwrap();
        assert!(eps.starts_with("%!PS-Adobe-3.0 EPSF-3.0\n"));
        assert!(eps.contains(&format!("%%BoundingBox: 0 0 {} {}", width.ceil(), height.ceil())));
        for text in ["Current", "mA", "2.5", "4.5"] {
            assert!(eps.contains(&format!("({text})")), "{text}");
        }
        assert!(!eps.contains("(A\\(Y\\))"));

        let emf = sheet_image_bytes(&sheet, "emf", 300.0).unwrap();
        let word = |offset| u32::from_le_bytes(emf[offset..offset + 4].try_into().unwrap());
        assert_eq!(word(0), 1, "EMR_HEADER");
        assert_eq!(&emf[40..44], b" EMF");
        assert_eq!(word(48) as usize, emf.len());
        let hundredths_mm = |points: f64| (points * 2540.0 / 72.0).round() as u32;
        assert_eq!((word(32), word(36)), (hundredths_mm(width), hundredths_mm(height)));
        for text in ["Current", "mA", "2.5", "4.5"] {
            let utf16: Vec<_> = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
            assert!(emf.windows(utf16.len()).any(|window| window == utf16), "{text}");
        }
        assert_eq!(emf, sheet_image_bytes(&sheet, "emf", 96.0).unwrap(), "vector size is independent of DPI");
    }

    #[test]
    fn worksheet_png_uses_the_saved_cell_measurements() {
        use sheaf_core::sheet_view::CellMeasurements;

        let mut sheet = sheet();
        let extent = |sheet: &Worksheet| {
            let png = sheet_image_bytes(sheet, "png", 96.0).unwrap();
            let reader = png::Decoder::new(Cursor::new(png)).read_info().unwrap();
            (reader.info().width, reader.info().height)
        };
        let before = extent(&sheet);
        sheet.view.cell_measurements = CellMeasurements { row_height: Some(200), column_width: Some(200) };
        let after = extent(&sheet);
        assert!((i64::from(after.0) - 2 * i64::from(before.0)).abs() <= 2);
        assert!(after.1 > before.1 && after.1 < 2 * before.1, "the label rows retain their height");

        sheet.view.cell_measurements.row_height = Some(10);
        let short = extent(&sheet);
        sheet.columns[0].data = ColumnData::numeric(Vec::new());
        assert!(short.1 > extent(&sheet).1, "rows still occupy space when their text is clipped");
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
        for ext in ["wmf", "ai", "dxf", "cgm", "unknown"] {
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
