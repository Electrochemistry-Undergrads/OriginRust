//! Scene to Encapsulated PostScript (language level 2).
//!
//! The figure is drawn with paths, `rectclip` clipping and text in the standard PostScript fonts
//! (Helvetica, Times and Courier, chosen from the graph's font by [`standard_family`]), which
//! every PostScript printer and interpreter has, so nothing is embedded. The fonts are
//! re-encoded to ISO Latin-1, which gives the degree sign, the micro sign, superscript digits
//! and the accented letters; Greek letters and a few mathematical signs are set in the `Symbol`
//! font; any other character is replaced by its nearest Latin-1 look-alike, or by `?`, and the
//! replacement is reported as a warning. PostScript has no transparency, so translucent colours
//! are blended onto the page background (see [`crate::vector::flatten`]).

use crate::scene::{Anchor, Cap, Item, Join, Scene, Stroke, TextItem, push_num};
use crate::vector::{Seg, flatten, item_segs, latin1_stand_in, ps_font_name, standard_family, symbol_byte};
use sheaf_core::Color;
use std::collections::BTreeSet;
use std::fmt::Write;

/// An EPS file and what could not be reproduced exactly in it.
#[derive(Clone, Debug, PartialEq)]
pub struct EpsOutput {
    /// The file, 7-bit ASCII.
    pub bytes: Vec<u8>,
    /// One line per kind of replacement made (characters with no PostScript font glyph).
    pub warnings: Vec<String>,
}

struct Writer {
    body: String,
    fonts: BTreeSet<&'static str>,
    height: f64,
    background: Color,
    replaced: BTreeSet<char>,
    dropped: BTreeSet<char>,
}

/// Writes `scene`, scaled by `scale` (1 is the page size).
pub fn write_eps(scene: &Scene, scale: f64) -> EpsOutput {
    let mut w = Writer {
        body: String::new(),
        fonts: BTreeSet::new(),
        height: scene.height,
        background: scene.background,
        replaced: BTreeSet::new(),
        dropped: BTreeSet::new(),
    };
    if scene.background.a > 0 {
        let rect = crate::Rect { x: 0.0, y: 0.0, w: scene.width, h: scene.height };
        w.item(&Item::Rect { rect, fill: Some(scene.background), stroke: None });
    }
    for layer in &scene.layers {
        layer.grid.iter().for_each(|it| w.item(it));
        let c = layer.clip;
        w.body.push_str("q ");
        w.nums(&[c.x, scene.height - c.y - c.h, c.w, c.h]);
        w.body.push_str("rectclip\n");
        layer.plots.iter().for_each(|it| w.item(it));
        w.body.push_str("Q\n");
        layer.overlay.iter().for_each(|it| w.item(it));
    }

    let (pw, ph) = (scene.width * scale, scene.height * scale);
    let mut s = String::with_capacity(w.body.len() + 2048);
    s.push_str("%!PS-Adobe-3.0 EPSF-3.0\n%%Creator: Sheaf\n%%Title: Sheaf figure\n");
    let _ = writeln!(s, "%%BoundingBox: 0 0 {} {}", pw.ceil() as i64, ph.ceil() as i64);
    s.push_str("%%HiResBoundingBox: 0 0 ");
    push_num(&mut s, pw);
    s.push(' ');
    push_num(&mut s, ph);
    s.push_str("\n%%LanguageLevel: 2\n%%Pages: 1\n");
    s.push_str("%%EndComments\n%%BeginProlog\n");
    s.push_str(PROLOG);
    for name in &w.fonts {
        let _ = writeln!(s, "/{name} /{name}-L1 reenc");
    }
    s.push_str("%%EndProlog\n%%Page: 1 1\nq\n");
    if scale != 1.0 {
        push_num(&mut s, scale);
        s.push(' ');
        push_num(&mut s, scale);
        s.push_str(" scale\n");
    }
    s.push_str("0 setlinewidth newpath\n");
    s.push_str(&w.body);
    s.push_str("Q\nshowpage\n%%EOF\n");

    let mut warnings = Vec::new();
    if !w.replaced.is_empty() {
        let list: String = w.replaced.iter().collect();
        warnings.push(format!(
            "EPS: characters with no glyph in the standard PostScript fonts were set as look-alikes: {list}"
        ));
    }
    if !w.dropped.is_empty() {
        let list: String = w.dropped.iter().collect();
        warnings.push(format!(
            "EPS: characters with no glyph in the standard PostScript fonts were replaced by '?': {list}"
        ));
    }
    EpsOutput { bytes: s.into_bytes(), warnings }
}

const PROLOG: &str = "\
/m {moveto} bind def /l {lineto} bind def /c {curveto} bind def /h {closepath} bind def
/rg {setrgbcolor} bind def /q {gsave} bind def /Q {grestore} bind def
/sf {exch findfont exch scalefont setfont} bind def
/L1 ISOLatin1Encoding 256 array copy def
L1 39 /quotesingle put L1 96 /grave put L1 45 /hyphen put
/reenc {exch findfont dup length dict begin
 {1 index /FID ne {def} {pop pop} ifelse} forall
 /Encoding L1 def currentdict end definefont pop} bind def
";

/// One stretch of text in one font.
struct Piece {
    font: String,
    text: Vec<u8>,
}

impl Writer {
    fn nums(&mut self, v: &[f64]) {
        for x in v {
            push_num(&mut self.body, *x);
            self.body.push(' ');
        }
    }

    fn point(&mut self, x: f64, y: f64) {
        self.nums(&[x, self.height - y]);
    }

    fn color(&mut self, c: Color) {
        let c = flatten(c, self.background);
        let _ = write!(self.body, "{} {} {} rg ", unit(c.r), unit(c.g), unit(c.b));
    }

    fn path(&mut self, segs: &[Seg]) {
        self.body.push_str("newpath ");
        let mut open = false;
        for seg in segs {
            match *seg {
                Seg::Move(x, y) => {
                    self.point(x, y);
                    self.body.push_str("m ");
                    open = true;
                }
                Seg::Line(x, y) if open => {
                    self.point(x, y);
                    self.body.push_str("l ");
                }
                Seg::Cubic(c) if open => {
                    for k in 0..3 {
                        self.point(c[2 * k], c[2 * k + 1]);
                    }
                    self.body.push_str("c ");
                }
                Seg::Close if open => self.body.push_str("h "),
                _ => {}
            }
        }
        self.body.push('\n');
    }

    fn stroke_setup(&mut self, st: &Stroke) {
        self.nums(&[st.width.max(0.0)]);
        let cap = match st.cap {
            Cap::Butt => 0,
            Cap::Round => 1,
            Cap::Square => 2,
        };
        let join = match st.join {
            Join::Miter => 0,
            Join::Round => 1,
        };
        let _ = write!(self.body, "setlinewidth {cap} setlinecap {join} setlinejoin 4 setmiterlimit [");
        let dash: Vec<f64> =
            if st.dash.len() % 2 == 1 { st.dash.iter().chain(&st.dash).copied().collect() } else { st.dash.clone() };
        self.nums(&dash);
        self.body.push_str("] 0 setdash ");
        self.color(st.color);
    }

    fn item(&mut self, item: &Item) {
        let (fill, stroke) = match item {
            Item::Text(t) => return self.text(t),
            Item::Rect { fill, stroke, .. } | Item::Path { fill, stroke, .. } => (*fill, stroke),
        };
        let fill = fill.filter(|c| c.a > 0);
        let stroke = stroke.as_ref().filter(|s| s.color.a > 0 && s.width > 0.0);
        if fill.is_none() && stroke.is_none() {
            return;
        }
        let Some(segs) = item_segs(item) else { return };
        if !segs.iter().any(|s| matches!(s, Seg::Move(..))) {
            return;
        }
        self.path(&segs);
        if let Some(c) = fill {
            if stroke.is_some() {
                self.body.push_str("q ");
            }
            self.color(c);
            self.body.push_str("fill ");
            if stroke.is_some() {
                self.body.push_str("Q ");
            }
        }
        if let Some(st) = stroke {
            self.body.push_str("q ");
            self.stroke_setup(st);
            self.body.push_str("stroke Q");
        }
        self.body.push('\n');
    }

    /// Splits text into pieces that each use one font, with Latin-1 bytes in the re-encoded
    /// font and the `Symbol` encoding for Greek and mathematical signs.
    fn pieces(&mut self, t: &TextItem) -> Vec<Piece> {
        let base = ps_font_name(standard_family(&t.family), t.bold, t.italic);
        self.fonts.insert(base);
        let main = format!("{base}-L1");
        let mut out: Vec<Piece> = Vec::new();
        let mut push = |font: &str, b: u8| match out.last_mut() {
            Some(p) if p.font == font => p.text.push(b),
            _ => out.push(Piece { font: font.to_string(), text: vec![b] }),
        };
        for c in t.text.chars() {
            let c = if c.is_control() { ' ' } else { c };
            let code = c as u32;
            if (0x20..0x7f).contains(&code) || (0xa0..=0xff).contains(&code) {
                push(&main, code as u8);
            } else if let Some(b) = symbol_byte(c) {
                push("Symbol", b);
            } else if let Some(r) = latin1_stand_in(c) {
                self.replaced.insert(c);
                push(&main, r as u8);
            } else {
                self.dropped.insert(c);
                push(&main, b'?');
            }
        }
        out
    }

    fn text(&mut self, t: &TextItem) {
        if t.text.is_empty() || t.color.a == 0 {
            return;
        }
        let pieces = self.pieces(t);
        self.body.push_str("q ");
        self.nums(&[t.x, self.height - t.y]);
        self.body.push_str("translate ");
        if t.rotate_ccw != 0.0 {
            self.nums(&[t.rotate_ccw]);
            self.body.push_str("rotate ");
        }
        self.color(t.color);
        if t.anchor != Anchor::Start {
            self.body.push_str("0 ");
            for p in &pieces {
                self.piece(p, t.size, "stringwidth pop add ");
            }
            self.body.push_str(if t.anchor == Anchor::Middle { "2 div neg 0 m " } else { "neg 0 m " });
        } else {
            self.body.push_str("0 0 m ");
        }
        for p in &pieces {
            self.piece(p, t.size, "show ");
        }
        self.body.push_str("Q\n");
    }

    fn piece(&mut self, p: &Piece, size: f64, op: &str) {
        let _ = write!(self.body, "/{} ", p.font);
        push_num(&mut self.body, size);
        self.body.push_str(" sf (");
        for &b in &p.text {
            match b {
                b'(' | b')' | b'\\' => {
                    self.body.push('\\');
                    self.body.push(b as char);
                }
                0x20..=0x7e => self.body.push(b as char),
                _ => {
                    let _ = write!(self.body, "\\{b:03o}");
                }
            }
        }
        self.body.push_str(") ");
        self.body.push_str(op);
    }
}

/// A colour channel as a PostScript number from 0 to 1.
fn unit(v: u8) -> String {
    let mut s = String::new();
    push_num(&mut s, f64::from(v) / 255.0);
    s
}
