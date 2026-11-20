//! T97: file-extension routing for the additional worksheet raster formats.
use super::*;

fn output_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sheaf-wks-formats-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn worksheet_writer_keeps_requested_format_case_alias_and_filename() {
    let h = start(vec![data_file("worksheet-formats")]);
    let source = project(&h).workbooks().next().unwrap().sheet();
    let dir = output_dir("aliases");
    for (ext, signature) in [
        ("JPEG", &[0xff, 0xd8][..]),
        ("jpg", &[0xff, 0xd8][..]),
        ("TIFF", &b"II"[..]),
        ("tif", &b"II"[..]),
        ("BmP", &b"BM"[..]),
        ("GiF", &b"GIF"[..]),
        ("PCX", &[10, 5, 1][..]),
        ("PsD", &b"8BPS"[..]),
    ] {
        let path = dir.join(format!("worksheet-Ω.{ext}"));
        assert_eq!(write_worksheet_image(source, &path).unwrap(), path);
        assert!(std::fs::read(&path).unwrap().starts_with(signature), "{ext}");
    }
    let path = dir.join("worksheet.TGA");
    assert_eq!(write_worksheet_image(source, &path).unwrap(), path);
    let bytes = std::fs::read(path).unwrap();
    assert_eq!((bytes[2], bytes[16]), (2, 24));
}

#[test]
fn worksheet_export_rejects_unsupported_formats_without_replacing_existing_files() {
    let h = start(vec![data_file("worksheet-format-errors")]);
    let source = project(&h).workbooks().next().unwrap().sheet();
    let dir = output_dir("errors");
    for ext in ["ai", "cgm", "dxf", "eps", "emf", "wmf", "invalid"] {
        let path = dir.join(format!("existing.{ext}"));
        std::fs::write(&path, b"retain previous export").unwrap();
        assert!(write_worksheet_image(source, &path).is_err(), "{ext}");
        assert_eq!(std::fs::read(path).unwrap(), b"retain previous export");
    }
    let path = dir.join("parent-file");
    std::fs::write(&path, b"retain parent").unwrap();
    assert!(write_worksheet_image(source, &path.join("child.bmp")).is_err());
    assert_eq!(std::fs::read(path).unwrap(), b"retain parent");
}

#[test]
fn extensionless_worksheet_export_still_defaults_to_the_original_png() {
    let h = start(vec![data_file("worksheet-format-default")]);
    let source = project(&h).workbooks().next().unwrap().sheet();
    let dir = output_dir("default");
    let path = write_worksheet_image(source, &dir.join("default")).unwrap();
    assert_eq!(path, dir.join("default.png"));
    assert_eq!(std::fs::read(path).unwrap(), sheaf_plot::sheet_image_png(source, 300.).unwrap());
}
