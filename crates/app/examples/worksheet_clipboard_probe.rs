//! Manual native clipboard round-trip probe. Run under the scratch PowerShell wrapper that
//! preserves the original clipboard data object; it intentionally copies synthetic values.
#[path = "../src/wks_clipboard.rs"]
mod wks_clipboard;

fn main() {
    let mut clipboard = wks_clipboard::WorksheetClipboard::default();
    let external = "0.30\tµ\r\n--\tβ";
    let full = "0.30000000000000004\tµ\n--\tβ\n";
    let mode = std::env::args().nth(1).unwrap_or_default();
    let read = mode == "read" || mode == "external";
    let paired = read || clipboard.write(&eframe::egui::Context::default(), external, full);
    if cfg!(windows) {
        assert!(paired, "Windows must publish both worksheet formats");
        let receiver = wks_clipboard::WorksheetClipboard::default();
        let normalized = external.replace("\r\n", "\n");
        if mode == "external" {
            assert_eq!(receiver.for_paste(external.into()), external);
            assert_eq!(receiver.for_paste(normalized.clone()), normalized);
        } else {
            assert_eq!(receiver.for_paste(external.into()), full);
            assert_eq!(receiver.for_paste(normalized), full);
        }
        assert_eq!(receiver.for_paste("changed external text".into()), "changed external text");
        println!("Native worksheet clipboard probe passed ({mode}).");
    } else {
        assert!(!paired, "other platforms retain the lossless text fallback");
    }
    clipboard.write_plain(&eframe::egui::Context::default(), "probe complete".into());
}
