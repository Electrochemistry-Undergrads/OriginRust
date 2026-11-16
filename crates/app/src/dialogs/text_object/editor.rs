//! Edit markup at character cursors, keeping the raw text outside the selection intact.
use eframe::egui::text::{CCursor, CCursorRange};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Format {
    Normal,
    Bold,
    Italic,
    Underline,
    Strike,
    Superscript,
    Subscript,
    Greek,
}

impl Format {
    pub const ALL: [Self; 8] = [
        Self::Normal,
        Self::Bold,
        Self::Italic,
        Self::Underline,
        Self::Strike,
        Self::Superscript,
        Self::Subscript,
        Self::Greek,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Bold => "Bold selection",
            Self::Italic => "Italic selection",
            Self::Underline => "Underline selection",
            Self::Strike => "Strikethrough selection",
            Self::Superscript => "Superscript",
            Self::Subscript => "Subscript",
            Self::Greek => "Greek",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Bold => "B",
            Self::Italic => "I",
            Self::Underline => "U",
            Self::Strike => "S",
            Self::Superscript => "x²",
            Self::Subscript => "x₂",
            Self::Greek => "α",
        }
    }

    pub fn tip(self) -> &'static str {
        match self {
            Self::Normal => "Remove inline formatting from the selection",
            Self::Bold => "Bold (Ctrl+B)",
            Self::Italic => "Italic (Ctrl+I)",
            Self::Underline => "Underline (Ctrl+U)",
            Self::Strike => "Strikethrough",
            Self::Superscript => "Superscript (Ctrl+Shift+=)",
            Self::Subscript => "Subscript (Ctrl+=)",
            Self::Greek => "Greek letters (Ctrl+G)",
        }
    }

    fn prefix(self) -> &'static str {
        match self {
            Self::Normal => "",
            Self::Bold => r"\b(",
            Self::Italic => r"\i(",
            Self::Underline => r"\u(",
            Self::Strike => r"\s(",
            Self::Superscript => r"\+(",
            Self::Subscript => r"\-(",
            Self::Greek => r"\g(",
        }
    }
}

/// The closing parenthesis belonging to the open parenthesis just before `body`.
fn closing(chars: &[char], body: usize) -> Option<usize> {
    let mut depth = 0;
    for (index, ch) in chars.iter().enumerate().skip(body) {
        match ch {
            '(' => depth += 1,
            ')' if depth == 0 => return Some(index),
            ')' => depth -= 1,
            _ => {}
        }
    }
    None
}

/// Recognize formatting prefixes for clearing, without interpreting their contents.
fn prefix_end(chars: &[char], start: usize) -> Option<usize> {
    if chars.get(start) != Some(&'\\') {
        return None;
    }
    let code = *chars.get(start + 1)?;
    let open = match code {
        'b' | 'i' | 'u' | 's' | '+' | '-' | 'g' => start + 2,
        'p' | 'c' => {
            let digits = chars.get(start + 2..)?.iter().take_while(|c| c.is_ascii_digit()).count();
            if !(1..=10).contains(&digits) {
                return None;
            }
            start + 2 + digits
        }
        'f' if chars.get(start + 2) == Some(&':') => {
            (start + 3..chars.len().min(start + 68)).find(|&i| chars[i] == '(')?
        }
        _ => return None,
    };
    (chars.get(open) == Some(&'(')).then_some(open + 1)
}

fn clear(chars: &[char], depth: usize) -> String {
    if depth == 24 {
        return chars.iter().collect();
    }
    let mut out = String::new();
    let mut index = 0;
    while index < chars.len() {
        if let Some(body) = prefix_end(chars, index)
            && let Some(end) = closing(chars, body)
        {
            out.push_str(&clear(&chars[body..end], depth + 1));
            index = end + 1;
        } else if chars.get(index) == Some(&'\\')
            && chars.get(index + 1) == Some(&'v')
            && chars.get(index + 2) == Some(&'(')
            && let Some(end) = closing(chars, index + 3)
        {
            // Literal blocks can themselves contain formatting-looking text.
            out.extend(chars[index..=end].iter());
            index = end + 1;
        } else {
            out.push(chars[index]);
            index += 1;
        }
    }
    out
}

/// Escape unmatched parentheses in a selected raw-text fragment. Unclosed markup stays untouched.
fn safe_body(chars: &[char]) -> Option<String> {
    let mut opens = Vec::new();
    let mut escape = vec![false; chars.len()];
    for (index, ch) in chars.iter().enumerate() {
        match ch {
            '(' => opens.push(index),
            ')' if opens.pop().is_none() => escape[index] = true,
            _ => {}
        }
    }
    for index in opens {
        if (0..index).any(|i| prefix_end(chars, i) == Some(index + 1))
            || (index >= 2 && chars[index - 2] == '\\' && matches!(chars[index - 1], 'v' | 'x'))
        {
            return None;
        }
        escape[index] = true;
    }
    let mut out = String::new();
    for (index, ch) in chars.iter().enumerate() {
        match (*ch, escape[index]) {
            ('(', true) => out.push_str(r"\x(0028)"),
            (')', true) => out.push_str(r"\x(0029)"),
            _ => out.push(*ch),
        }
    }
    Some(out)
}

/// Apply or remove one complete wrapper. Returned cursors select the body, not its markup.
pub(super) fn apply(text: &mut String, cursor: CCursorRange, format: Format) -> CCursorRange {
    let original = text.clone();
    let mut chars: Vec<char> = text.chars().collect();
    let backwards = cursor.primary.index < cursor.secondary.index;
    let range = cursor.as_sorted_char_range();
    let (mut start, mut end) = (range.start.min(chars.len()), range.end.min(chars.len()));
    let replacement;
    let body_len;
    let body_start;
    if format == Format::Normal {
        // Include wrappers immediately surrounding the selected body, as after a button click.
        loop {
            let outer =
                (0..start).rev().find(|&i| prefix_end(&chars, i) == Some(start) && closing(&chars, start) == Some(end));
            let Some(outer) = outer else { break };
            start = outer;
            end += 1;
        }
        replacement = clear(&chars[start..end], 0);
        body_len = replacement.chars().count();
        body_start = start;
    } else {
        let prefix: Vec<char> = format.prefix().chars().collect();
        let adjacent = start >= prefix.len()
            && chars[start - prefix.len()..start] == prefix
            && closing(&chars, start) == Some(end);
        let whole =
            chars[start..end].starts_with(&prefix) && closing(&chars, start + prefix.len()) == end.checked_sub(1);
        if adjacent {
            replacement = chars[start..end].iter().collect();
            start -= prefix.len();
            end += 1;
            body_len = replacement.chars().count();
            body_start = start;
        } else if whole {
            replacement = chars[start + prefix.len()..end - 1].iter().collect();
            body_len = replacement.chars().count();
            body_start = start;
        } else {
            let Some(body) = safe_body(&chars[start..end]) else { return cursor };
            body_len = body.chars().count();
            replacement = format!("{}{body})", format.prefix());
            body_start = start + prefix.len();
        }
    }
    chars.splice(start..end, replacement.chars());
    let candidate: String = chars.into_iter().collect();
    if !matches!(format, Format::Normal | Format::Greek)
        && sheaf_plot::markup::plain(&candidate) != sheaf_plot::markup::plain(&original)
    {
        return cursor;
    }
    *text = candidate;
    let (anchor, caret) =
        if backwards { (body_start + body_len, body_start) } else { (body_start, body_start + body_len) };
    CCursorRange { primary: CCursor::new(caret), secondary: CCursor::new(anchor), h_pos: None }
}

/// Legend symbols and row boundaries are outside text-format groups. Name substitutions stay raw.
pub(super) fn apply_legend(text: &mut String, cursor: CCursorRange, format: Format) -> CCursorRange {
    let chars: Vec<char> = text.chars().collect();
    let selected = cursor.as_sorted_char_range();
    let (start, end) = (selected.start.min(chars.len()), selected.end.min(chars.len()));
    if start == end {
        return apply(text, cursor, format);
    }
    let mut protected = vec![false; chars.len()];
    let mut i = 0;
    while i < chars.len() {
        if matches!(chars[i], '\n' | '\r') {
            protected[i] = true;
        }
        if chars[i] == '\\'
            && chars.get(i + 1).is_some_and(|c| matches!(c, 'l' | 'L'))
            && chars.get(i + 2) == Some(&'(')
            && let Some(offset) = chars[i + 3..].iter().position(|c| *c == ')')
        {
            let close = i + 3 + offset;
            let raw: String = chars[i + 3..close].iter().collect();
            let id = raw.split(',').next().unwrap_or_default().trim();
            if !id.is_empty() && id.chars().all(|c| c.is_ascii_digit() || c == '.') {
                let mut after = close + 1;
                while after < chars.len() && matches!(chars[after], ' ' | '\t') {
                    after += 1;
                }
                protected[i..after].fill(true);
                i = after;
                continue;
            }
        }
        i += 1;
    }
    if !protected[start..end].iter().any(|p| *p) {
        return apply(text, cursor, format);
    }
    let mut ranges = Vec::new();
    let mut i = start;
    while i < end {
        if protected[i] {
            i += 1;
            continue;
        }
        let begin = i;
        while i < end && !protected[i] {
            i += 1;
        }
        ranges.push(begin..i);
    }
    for range in ranges.into_iter().rev() {
        apply(text, CCursorRange::two(CCursor::new(range.start), CCursor::new(range.end)), format);
    }
    let new_len = text.chars().count();
    let end = new_len.saturating_sub(chars.len() - end);
    let start = start.min(new_len);
    let backwards = cursor.primary.index < cursor.secondary.index;
    let (caret, anchor) = if backwards { (start, end) } else { (end, start) };
    CCursorRange { primary: CCursor::new(caret), secondary: CCursor::new(anchor), h_pos: None }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_parenthesis_formatting_preserves_the_rendered_text() {
        for range in [3..4, 5..6] {
            let mut text = "cm (V)".to_string();
            apply(&mut text, CCursorRange::two(CCursor::new(range.start), CCursor::new(range.end)), Format::Bold);
            assert_eq!(sheaf_plot::markup::plain(&text), "cm (V)", "{text}");
        }
    }

    #[test]
    fn scripts_use_character_indices_and_retain_a_backwards_selection() {
        let mut text = "μA cm-2".to_string();
        let cursor = CCursorRange::two(CCursor::new(7), CCursor::new(5));
        let selected = apply(&mut text, cursor, Format::Superscript);
        assert_eq!(text, r"μA cm\+(-2)");
        assert_eq!(selected.as_sorted_char_range(), 8..10);
        assert_eq!(selected.primary.index, 8);
    }

    #[test]
    fn empty_caret_inserts_a_group_and_repeated_formatting_toggles_it() {
        let mut text = "A".to_string();
        let cursor = apply(&mut text, CCursorRange::one(CCursor::new(0)), Format::Greek);
        assert_eq!(text, r"\g()A");
        assert_eq!(cursor.primary.index, 3);
        let cursor = apply(&mut text, cursor, Format::Greek);
        assert_eq!(text, "A");
        assert_eq!(cursor.primary.index, 0);
    }

    #[test]
    fn toggling_a_complete_wrapper_keeps_nested_styles_and_neighbouring_text() {
        let mut text = r"A\b(\i(beta))C".to_string();
        let range = CCursorRange::two(CCursor::new(1), CCursor::new(text.chars().count() - 1));
        apply(&mut text, range, Format::Bold);
        assert_eq!(text, r"A\i(beta)C");
    }

    #[test]
    fn normal_removes_nested_formatting_and_keeps_substitutions_unicode_and_literals_raw() {
        let mut text = r"\b(\i(%(1) \v(\b(raw)) \x(03b8)))".to_string();
        let end = text.chars().count();
        apply(&mut text, CCursorRange::two(CCursor::new(0), CCursor::new(end)), Format::Normal);
        assert_eq!(text, r"%(1) \v(\b(raw)) \x(03b8)");
    }

    #[test]
    fn normal_clears_the_group_around_its_selected_body() {
        let mut text = r"cm\+(-2)".to_string();
        let cursor = apply(&mut text, CCursorRange::two(CCursor::new(5), CCursor::new(7)), Format::Normal);
        assert_eq!(text, "cm-2");
        assert_eq!(cursor.as_sorted_char_range(), 2..4);
    }

    #[test]
    fn normal_clears_published_rgb_colour_codes_and_keeps_literal_blocks() {
        let mut text = r"\c25249550(x)\v(\c4(raw))".to_string();
        let end = text.chars().count();
        apply(&mut text, CCursorRange::two(CCursor::new(0), CCursor::new(end)), Format::Normal);
        assert_eq!(text, r"x\v(\c4(raw))");
    }
}
