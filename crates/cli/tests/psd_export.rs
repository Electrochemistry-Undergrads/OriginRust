//! PSD follows the same output-extension, resolution, sizing and margin path as other figures.

use std::path::PathBuf;
use std::process::Command;

fn fixture(tag: &str) -> (PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("sheaf-cli-psd-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let data = dir.join("data.csv");
    std::fs::write(&data, "x,y\n0,1\n1,3\n2,2\n").unwrap();
    (dir, data)
}

#[test]
fn plot_writes_psd_with_requested_pixel_size_and_fractional_dpi() {
    let (dir, data) = fixture("write");
    for name in ["figure.psd", "figure.PSD"] {
        let out = dir.join(name);
        let run = Command::new(env!("CARGO_BIN_EXE_sheaf"))
            .arg("plot")
            .arg(&data)
            .arg("-o")
            .arg(&out)
            .args(["--dpi", "150.5", "--image-width", "72px", "--margin", "page"])
            .output()
            .unwrap();
        assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
        let psd = std::fs::read(out).unwrap();
        assert_eq!(&psd[..6], b"8BPS\0\x01");
        assert_eq!(u32::from_be_bytes(psd[18..22].try_into().unwrap()), 72);
        assert_eq!(u16::from_be_bytes(psd[24..26].try_into().unwrap()), 3, "RGB");
        let resource = psd.windows(6).position(|s| s == b"8BIM\x03\xed").expect("ResolutionInfo resource");
        let resolution = u32::from_be_bytes(psd[resource + 12..resource + 16].try_into().unwrap());
        assert_eq!(f64::from(resolution) / 65_536.0, 150.5);
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn plot_rejects_oversized_psd_without_writing_a_partial_file() {
    let (dir, data) = fixture("oversize");
    let out = dir.join("large.psd");
    let run = Command::new(env!("CARGO_BIN_EXE_sheaf"))
        .arg("plot")
        .arg(data)
        .arg("-o")
        .arg(&out)
        .args(["--image-width", "30001px"])
        .output()
        .unwrap();
    assert!(!run.status.success());
    assert!(String::from_utf8_lossy(&run.stderr).contains("30000 pixels per dimension"));
    assert!(!out.exists());
    let _ = std::fs::remove_dir_all(dir);
}
