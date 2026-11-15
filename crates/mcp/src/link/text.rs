//! Text that came over the wire and is about to be shown by the interface.

/// True for the Unicode format characters (category Cf) that can change how text is laid out or hide
/// characters: bidi controls, zero-width characters, the soft hyphen, tag characters and the like.
fn is_format(c: char) -> bool {
    matches!(c,
        '\u{00AD}'
        | '\u{0600}'..='\u{0605}'
        | '\u{061C}'
        | '\u{06DD}'
        | '\u{070F}'
        | '\u{0890}'..='\u{0891}'
        | '\u{08E2}'
        | '\u{180E}'
        | '\u{200B}'..='\u{200F}'
        | '\u{202A}'..='\u{202E}'
        | '\u{2060}'..='\u{2064}'
        | '\u{2066}'..='\u{206F}'
        | '\u{FEFF}'
        | '\u{FFF9}'..='\u{FFFB}'
        | '\u{110BD}'
        | '\u{110CD}'
        | '\u{13430}'..='\u{1343F}'
        | '\u{1BCA0}'..='\u{1BCA3}'
        | '\u{1D173}'..='\u{1D17A}'
        | '\u{E0001}'
        | '\u{E0020}'..='\u{E007F}')
}

/// Cleans a string that came over the wire before the interface shows it. Control characters, bidi
/// controls and the other format characters, and the line and paragraph separators (U+2028, U+2029) are
/// removed; tab and line breaks count as white space; runs of white space become one space; the ends are
/// trimmed; and a text over `max_chars` characters is cut at a character boundary, with `...` in place
/// of the last three characters. The result never has more than `max_chars` characters.
pub fn sanitize(text: &str, max_chars: usize) -> String {
    let mut out = String::with_capacity(text.len().min(max_chars.saturating_mul(4)));
    let mut space = false;
    for c in text.chars() {
        if matches!(c, '\t' | '\n' | '\r' | '\u{0B}' | '\u{0C}') {
            space = true;
            continue;
        }
        if c.is_control() || is_format(c) || matches!(c, '\u{2028}' | '\u{2029}') {
            continue;
        }
        if c.is_whitespace() {
            space = true;
            continue;
        }
        if space && !out.is_empty() {
            out.push(' ');
        }
        space = false;
        out.push(c);
    }
    let count = out.chars().count();
    if count <= max_chars {
        return out;
    }
    if max_chars <= 3 {
        return out.chars().take(max_chars).collect();
    }
    let mut cut: String = out.chars().take(max_chars - 3).collect();
    let kept = cut.trim_end().len();
    cut.truncate(kept);
    cut.push_str("...");
    cut
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_characters_are_removed_and_line_breaks_become_spaces() {
        assert_eq!(sanitize("a\u{0}b\u{7}c\u{1B}[31m", 40), "abc[31m");
        assert_eq!(sanitize("one\ntwo\r\nthree\tfour", 40), "one two three four");
        assert_eq!(sanitize("x\u{85}y\u{7F}z", 40), "xyz");
    }

    #[test]
    fn bidi_and_other_format_characters_are_removed() {
        assert_eq!(sanitize("txt\u{202E}gpj.exe", 40), "txtgpj.exe");
        assert_eq!(sanitize("a\u{2066}b\u{2069}c\u{200B}d\u{200F}e\u{FEFF}f\u{AD}g", 40), "abcdefg");
        assert_eq!(sanitize("tag\u{E0041}\u{E007F}s", 40), "tags");
        assert_eq!(sanitize("\u{202A}\u{202B}\u{202C}\u{202D}", 40), "");
    }

    #[test]
    fn line_and_paragraph_separators_are_removed() {
        assert_eq!(sanitize("a\u{2028}b\u{2029}c", 40), "abc");
    }

    #[test]
    fn runs_of_white_space_collapse_and_the_ends_are_trimmed() {
        assert_eq!(sanitize("  a   b \u{A0}\u{3000} c  ", 40), "a b c");
        assert_eq!(sanitize("   ", 40), "");
        assert_eq!(sanitize("", 40), "");
    }

    #[test]
    fn a_long_text_is_cut_on_a_character_boundary_with_dots() {
        let long = "é".repeat(100);
        let cut = sanitize(&long, 10);
        assert_eq!(cut.chars().count(), 10);
        assert_eq!(cut, format!("{}...", "é".repeat(7)));
        assert_eq!(sanitize("exactly ten", 11), "exactly ten");
        assert_eq!(sanitize("a b c d e f", 8), "a b c...");
        // The cut does not leave a space before the dots.
        assert_eq!(sanitize("abcd efgh ijkl", 8), "abcd...");
        assert_eq!(sanitize("abcdef", 2), "ab");
        assert_eq!(sanitize("日本語のテキスト", 5), "日本...");
    }

    #[test]
    fn text_that_is_clean_and_short_is_unchanged() {
        assert_eq!(sanitize("AI: plot Book1 B(Y)", 60), "AI: plot Book1 B(Y)");
    }
}
