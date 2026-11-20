//! Scene to SVG text (spec section 6). The output is deterministic.

use crate::fonts::css_family;
use crate::scene::{Anchor, Cap, Item, Join, LayerScene, Scene, Stroke, push_num};
use sheaf_core::Color;
use std::fmt::Write;

/// Writes the scene as an SVG document.
pub fn write_svg(scene: &Scene) -> String {
    write_svg_view(scene, crate::Rect { x: 0.0, y: 0.0, w: scene.width, h: scene.height }, 1.0)
}

/// Writes the part `view` of the page (page points) as an SVG document drawn `scale` times as
/// large: the size on the root element is `view` x `scale`, the drawing keeps its units through
/// the `viewBox`, and the background fills the whole view even where it is off the page.
pub fn write_svg_view(scene: &Scene, view: crate::Rect, scale: f64) -> String {
    let mut s = String::with_capacity(16 * 1024);
    s.push_str("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"");
    push_num(&mut s, view.w * scale);
    s.push_str("pt\" height=\"");
    push_num(&mut s, view.h * scale);
    s.push_str("pt\" viewBox=\"");
    push_num(&mut s, view.x);
    s.push(' ');
    push_num(&mut s, view.y);
    s.push(' ');
    push_num(&mut s, view.w);
    s.push(' ');
    push_num(&mut s, view.h);
    s.push_str("\">\n");
    write_item(&mut s, &Item::Rect { rect: view, fill: Some(scene.background), stroke: None });
    for (i, layer) in scene.layers.iter().enumerate() {
        write_layer(&mut s, i, layer);
    }
    s.push_str("</svg>\n");
    s
}

fn write_layer(s: &mut String, index: usize, layer: &LayerScene) {
    let (underlay, overlay) = layer.split_overlay();
    let _ = writeln!(s, "<g id=\"layer{index}\">");
    if !layer.grid.is_empty() {
        s.push_str("<g class=\"grid\">\n");
        layer.grid.iter().for_each(|it| write_item(s, it));
        s.push_str("</g>\n");
    }
    if !underlay.is_empty() {
        s.push_str("<g class=\"underlay\">\n");
        underlay.iter().for_each(|it| write_item(s, it));
        s.push_str("</g>\n");
    }
    let c = layer.clip;
    let _ = write!(s, "<clipPath id=\"clip{index}\"><rect x=\"");
    push_num(s, c.x);
    s.push_str("\" y=\"");
    push_num(s, c.y);
    s.push_str("\" width=\"");
    push_num(s, c.w);
    s.push_str("\" height=\"");
    push_num(s, c.h);
    s.push_str("\"/></clipPath>\n");
    let _ = writeln!(s, "<g class=\"plots\" clip-path=\"url(#clip{index})\">");
    layer.plots.iter().for_each(|it| write_item(s, it));
    s.push_str("</g>\n<g class=\"overlay\">\n");
    overlay.iter().for_each(|it| write_item(s, it));
    s.push_str("</g>\n</g>\n");
}

fn paint(s: &mut String, attr: &str, color: Option<Color>) {
    match color {
        None => {
            let _ = write!(s, " {attr}=\"none\"");
        }
        Some(c) => {
            let _ = write!(s, " {attr}=\"{}\"", c.hex_rgb());
            if c.a != 255 {
                let _ = write!(s, " {attr}-opacity=\"");
                push_num(s, (c.opacity() * 1000.0).round() / 1000.0);
                s.push('"');
            }
        }
    }
}

fn stroke_attrs(s: &mut String, stroke: &Option<Stroke>) {
    let Some(st) = stroke else { return };
    paint(s, "stroke", Some(st.color));
    s.push_str(" stroke-width=\"");
    push_num(s, st.width);
    s.push('"');
    match st.cap {
        Cap::Butt => {}
        Cap::Round => s.push_str(" stroke-linecap=\"round\""),
        Cap::Square => s.push_str(" stroke-linecap=\"square\""),
    }
    if st.join == Join::Round {
        s.push_str(" stroke-linejoin=\"round\"");
    }
    if !st.dash.is_empty() {
        s.push_str(" stroke-dasharray=\"");
        for (i, d) in st.dash.iter().enumerate() {
            if i > 0 {
                s.push(' ');
            }
            push_num(s, *d);
        }
        s.push('"');
    }
}

fn write_item(s: &mut String, item: &Item) {
    match item {
        // A picture object (T216): the PNG embedded as a data URL, stretched over its rectangle.
        Item::Image { rect, png } => {
            s.push_str("<image x=\"");
            push_num(s, rect.x);
            s.push_str("\" y=\"");
            push_num(s, rect.y);
            s.push_str("\" width=\"");
            push_num(s, rect.w);
            s.push_str("\" height=\"");
            push_num(s, rect.h);
            s.push_str("\" preserveAspectRatio=\"none\" href=\"data:image/png;base64,");
            s.push_str(&sheaf_core::worksheet::encode_base64(png));
            s.push_str("\"/>\n");
        }
        Item::Rect { rect, fill, stroke } => {
            s.push_str("<rect x=\"");
            push_num(s, rect.x);
            s.push_str("\" y=\"");
            push_num(s, rect.y);
            s.push_str("\" width=\"");
            push_num(s, rect.w);
            s.push_str("\" height=\"");
            push_num(s, rect.h);
            s.push('"');
            paint(s, "fill", *fill);
            stroke_attrs(s, stroke);
            s.push_str("/>\n");
        }
        Item::Path { d, fill, stroke } => {
            s.push_str("<path d=\"");
            s.push_str(d);
            s.push('"');
            paint(s, "fill", *fill);
            stroke_attrs(s, stroke);
            s.push_str("/>\n");
        }
        Item::Text(t) => {
            s.push_str("<text x=\"");
            push_num(s, t.x);
            s.push_str("\" y=\"");
            push_num(s, t.y);
            s.push_str("\" font-family=\"");
            s.push_str(&css_family(&t.family).replace('&', "&amp;").replace('<', "&lt;").replace('"', "&quot;"));
            s.push_str("\" font-size=\"");
            push_num(s, t.size);
            s.push('"');
            paint(s, "fill", Some(t.color));
            if t.bold {
                s.push_str(" font-weight=\"bold\"");
            }
            if t.italic {
                s.push_str(" font-style=\"italic\"");
            }
            match t.anchor {
                Anchor::Start => {}
                Anchor::Middle => s.push_str(" text-anchor=\"middle\""),
                Anchor::End => s.push_str(" text-anchor=\"end\""),
            }
            if t.rotate_ccw != 0.0 {
                s.push_str(" transform=\"rotate(");
                push_num(s, -t.rotate_ccw);
                s.push(' ');
                push_num(s, t.x);
                s.push(' ');
                push_num(s, t.y);
                s.push_str(")\"");
            }
            if t.text.starts_with(char::is_whitespace) || t.text.ends_with(char::is_whitespace) {
                s.push_str(" xml:space=\"preserve\"");
            }
            s.push('>');
            escape_into(s, &t.text);
            s.push_str("</text>\n");
        }
    }
}

/// XML-escapes text content. Characters that XML 1.0 forbids are dropped.
fn escape_into(s: &mut String, text: &str) {
    for c in text.chars() {
        match c {
            '&' => s.push_str("&amp;"),
            '<' => s.push_str("&lt;"),
            '>' => s.push_str("&gt;"),
            '\t' | '\n' | '\r' => s.push(' '),
            c if (c as u32) < 0x20 || c == '\u{fffe}' || c == '\u{ffff}' => {}
            c => s.push(c),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_is_escaped_and_control_characters_dropped() {
        let mut s = String::new();
        escape_into(&mut s, "a<b & c>\u{1}d\te");
        assert_eq!(s, "a&lt;b &amp; c&gt;d e");
    }
}
