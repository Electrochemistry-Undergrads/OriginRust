//! Worksheet text plus a private lossless format on Windows. Other platforms retain the
//! existing lossless text fallback. Tests use an in-memory clipboard, independent of the desktop.

use eframe::egui;

#[cfg(any(windows, test))]
const MAGIC: &[u8; 8] = b"SHEAFWK1";
#[cfg(any(windows, test))]
const HEADER: usize = 24;
#[cfg(any(windows, test))]
const MAX_PAYLOAD: usize = 128 * 1024 * 1024;
#[cfg(all(windows, not(test)))]
const FORMAT: &str = "OriginRust.Sheaf.Worksheet.v1";

#[derive(Default)]
pub struct WorksheetClipboard {
    #[cfg(test)]
    memory: Option<Vec<u8>>,
}

impl WorksheetClipboard {
    /// Returns true when both external text and the private lossless form were published.
    pub fn write(&mut self, ctx: &egui::Context, external: &str, lossless: &str) -> bool {
        // egui suppresses empty clipboard paste events. A blank line keeps an empty-cell
        // copy usable, while the private form identifies the exact table contents.
        let external = if external.is_empty() { "\n" } else { external };
        #[cfg(test)]
        {
            self.memory = encode(external, lossless);
            if self.memory.is_some() {
                ctx.copy_text(external.to_string());
                return true;
            }
        }
        #[cfg(all(windows, not(test)))]
        if native_write(external, lossless).is_some() {
            // Letting eframe copy text afterwards would clear the private format.
            return true;
        }
        #[cfg(all(not(windows), not(test)))]
        let _ = external;
        ctx.copy_text(lossless.to_string());
        false
    }

    /// An external paste, a changed clipboard, or an invalid private payload uses the supplied
    /// text unchanged. Nothing is inferred by comparing against old in-process copied text.
    pub fn for_paste(&self, external: String) -> String {
        #[cfg(test)]
        if let Some((plain, full)) = self.memory.as_deref().and_then(contents)
            && event_matches(plain, &external)
        {
            return full.to_string();
        }
        #[cfg(all(windows, not(test)))]
        if let Some(full) = native_read(&external) {
            return full;
        }
        external
    }

    /// Plain label-cell copies replace all worksheet formats at the end of the frame.
    pub fn write_plain(&mut self, ctx: &egui::Context, text: String) {
        #[cfg(test)]
        {
            self.memory = None;
        }
        ctx.copy_text(text);
    }

    #[cfg(test)]
    pub fn external_text(&self) -> Option<&str> {
        self.memory.as_deref().and_then(contents).map(|(plain, _)| plain)
    }
}

#[cfg(any(windows, test))]
fn encode(external: &str, lossless: &str) -> Option<Vec<u8>> {
    let size = HEADER.checked_add(external.len())?.checked_add(lossless.len())?;
    if size > MAX_PAYLOAD {
        return None;
    }
    let mut data = Vec::new();
    data.try_reserve_exact(size).ok()?;
    data.extend_from_slice(MAGIC);
    data.extend_from_slice(&(external.len() as u64).to_le_bytes());
    data.extend_from_slice(&(lossless.len() as u64).to_le_bytes());
    data.extend_from_slice(external.as_bytes());
    data.extend_from_slice(lossless.as_bytes());
    Some(data)
}

#[cfg(any(windows, test))]
fn lengths(data: &[u8]) -> Option<(usize, usize)> {
    if data.get(..8)? != MAGIC {
        return None;
    }
    let plain = usize::try_from(u64::from_le_bytes(data.get(8..16)?.try_into().ok()?)).ok()?;
    let full = usize::try_from(u64::from_le_bytes(data.get(16..24)?.try_into().ok()?)).ok()?;
    let end = HEADER.checked_add(plain)?.checked_add(full)?;
    (end <= MAX_PAYLOAD).then_some((plain, end))
}

#[cfg(any(windows, test))]
fn contents(data: &[u8]) -> Option<(&str, &str)> {
    let (plain, end) = lengths(data)?;
    let split = HEADER.checked_add(plain)?;
    let stored = std::str::from_utf8(data.get(HEADER..split)?).ok()?;
    // Windows may pad the allocation, so only the declared payload is read.
    Some((stored, std::str::from_utf8(data.get(split..end)?).ok()?))
}

#[cfg(any(windows, test))]
fn decode<'a>(data: &'a [u8], external: &str) -> Option<&'a str> {
    let (plain, full) = contents(data)?;
    (plain == external).then_some(full)
}

#[cfg(any(windows, test))]
fn event_matches(plain: &str, event: &str) -> bool {
    fn normalized(text: &str) -> impl Iterator<Item = u8> + '_ {
        let mut bytes = text.bytes().peekable();
        std::iter::from_fn(move || {
            let b = bytes.next()?;
            if b == b'\r' && bytes.peek() == Some(&b'\n') { bytes.next() } else { Some(b) }
        })
    }
    // egui applies this replacement once; normalizing the event again would lose a
    // literal CR in source text containing CR-CR-LF.
    plain == event || normalized(plain).eq(event.bytes())
}

#[cfg(all(windows, not(test)))]
fn native_write(external: &str, lossless: &str) -> Option<()> {
    use clipboard_win::{Clipboard, options::NoClear, raw};
    let data = encode(external, lossless)?;
    let format = raw::register_format(FORMAT)?.get();
    let _clipboard = Clipboard::new_attempts(10).ok()?;
    raw::empty().ok()?;
    raw::set_string_with(external, NoClear).ok()?;
    raw::set_without_clear(format, &data).ok()?;
    Some(())
}

#[cfg(all(windows, not(test)))]
fn native_read(external: &str) -> Option<String> {
    use clipboard_win::{Clipboard, raw};
    let format = raw::register_format(FORMAT)?.get();
    let _clipboard = Clipboard::new_attempts(10).ok()?;
    let mut actual = Vec::new();
    raw::get_string(&mut actual).ok()?;
    let actual = std::str::from_utf8(&actual).ok()?;
    if !event_matches(actual, external) {
        return None;
    }
    let mut header = [0; HEADER];
    if raw::get(format, &mut header).ok()? < HEADER {
        return None;
    }
    let (_, size) = lengths(&header)?;
    let mut data = Vec::new();
    data.try_reserve_exact(size).ok()?;
    data.resize(size, 0);
    if raw::get(format, &mut data).ok()? < size {
        return None;
    }
    decode(&data, actual).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paired_text_preserves_unicode_and_exact_values_and_checks_the_external_form() {
        let full = "0.30000000000000004\t\"alpha,β\"\n--\tµ\n";
        let mut data = encode("0.30\t\"alpha,β\"\r\n--\tµ", full).unwrap();
        assert_eq!(decode(&data, "0.30\t\"alpha,β\"\r\n--\tµ"), Some(full));
        assert_eq!(decode(&data, "0.31"), None);
        data.extend_from_slice(&[0; 8]);
        assert_eq!(decode(&data, "0.30\t\"alpha,β\"\r\n--\tµ"), Some(full));
    }

    #[test]
    fn invalid_version_lengths_and_utf8_are_not_used_for_paste() {
        let data = encode("0.3", "0.30000000000000004\n").unwrap();
        for length in 0..data.len() {
            assert_eq!(decode(&data[..length], "0.3"), None);
        }
        let mut bad = data.clone();
        bad[0] = b'X';
        assert_eq!(decode(&bad, "0.3"), None);
        let mut bad = data.clone();
        bad[8..16].copy_from_slice(&u64::MAX.to_le_bytes());
        assert_eq!(decode(&bad, "0.3"), None);
        let mut bad = data;
        *bad.last_mut().unwrap() = 0xff;
        assert_eq!(decode(&bad, "0.3"), None);
    }

    #[test]
    fn memory_backend_keeps_the_payload_with_its_external_text_only() {
        let ctx = egui::Context::default();
        let mut clipboard = WorksheetClipboard::default();
        assert!(clipboard.write(&ctx, "0.3", "0.30000000000000004\n"));
        assert_eq!(clipboard.for_paste("0.3".into()), "0.30000000000000004\n");
        assert_eq!(clipboard.for_paste("external replacement".into()), "external replacement");
        assert!(clipboard.write(&ctx, "new", "new\n"));
        assert_eq!(clipboard.for_paste("0.3".into()), "0.3");
    }

    #[test]
    fn normalized_events_empty_cells_and_plain_replacement_keep_the_right_payload() {
        let ctx = egui::Context::default();
        let mut clipboard = WorksheetClipboard::default();
        assert!(clipboard.write(&ctx, "0.30\r\n--", "0.30000000000000004\n--\n"));
        assert_eq!(clipboard.for_paste("0.30\n--".into()), "0.30000000000000004\n--\n");
        assert_eq!(clipboard.for_paste("0.30\n--\n".into()), "0.30\n--\n");
        assert!(clipboard.write(&ctx, "", "\"\"\n"));
        assert_eq!(clipboard.for_paste("\n".into()), "\"\"\n");
        clipboard.write_plain(&ctx, "\n".into());
        assert_eq!(clipboard.for_paste("\n".into()), "\n");
        assert!(!event_matches("a\r", "a"));
        assert!(event_matches("a\r\r\nb", "a\r\nb"));
        assert!(!event_matches("a\r\r\nb", "a\nb"));
    }
}
