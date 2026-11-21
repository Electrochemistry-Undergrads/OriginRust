//! Scene to Windows Enhanced Metafile, written record by record from the public [MS-EMF]
//! specification.
//!
//! The file is vector: shapes are path records (`EMR_BEGINPATH` ... `EMR_FILLPATH`,
//! `EMR_STROKEPATH`, `EMR_STROKEANDFILLPATH`) or `EMR_RECTANGLE`, pens are geometric pens with
//! user dashes, text is `EMR_EXTTEXTOUTW` in the graph's own font (Word and PowerPoint draw it
//! with the installed face), layer clipping is `EMR_INTERSECTCLIPRECT` between `EMR_SAVEDC` and
//! `EMR_RESTOREDC`. Pictures are 24-bit `EMR_STRETCHDIBITS` bitmaps (T237 slice 9). One logical unit is 0.01 mm, which is also what the header's frame uses, so a
//! viewer shows the figure at the size of the page. The format has no transparency, so
//! translucent colours are blended onto the page background (see [`crate::vector::flatten`]).

use crate::fonts::TextMeasurer;
use crate::scene::{Anchor, Cap, Item, Join, Scene, Stroke, TextItem};
use crate::vector::{Seg, flatten, item_segs};
use sheaf_core::Color;
use sheaf_core::graph::Font;
use std::collections::HashMap;

/// Logical units (0.01 mm) per point.
const UNITS_PER_PT: f64 = 2540.0 / 72.0;

const EMR_HEADER: u32 = 1;
const EMR_POLYBEZIERTO: u32 = 5;
const EMR_POLYLINETO: u32 = 6;
const EMR_EOF: u32 = 14;
const EMR_SETBKMODE: u32 = 18;
const EMR_SETPOLYFILLMODE: u32 = 19;
const EMR_SETMAPMODE: u32 = 17;
const EMR_SETWINDOWORGEX: u32 = 10;
const EMR_SETTEXTALIGN: u32 = 22;
const EMR_SETTEXTCOLOR: u32 = 24;
const EMR_MOVETOEX: u32 = 27;
const EMR_INTERSECTCLIPRECT: u32 = 30;
const EMR_SAVEDC: u32 = 33;
const EMR_RESTOREDC: u32 = 34;
const EMR_SELECTOBJECT: u32 = 37;
const EMR_DELETEOBJECT: u32 = 40;
const EMR_CREATEBRUSHINDIRECT: u32 = 39;
const EMR_RECTANGLE: u32 = 43;
const EMR_SETMITERLIMIT: u32 = 58;
const EMR_BEGINPATH: u32 = 59;
const EMR_ENDPATH: u32 = 60;
const EMR_CLOSEFIGURE: u32 = 61;
const EMR_FILLPATH: u32 = 62;
const EMR_STROKEANDFILLPATH: u32 = 63;
const EMR_STROKEPATH: u32 = 64;
const EMR_STRETCHDIBITS: u32 = 81;
const EMR_EXTCREATEFONTINDIRECTW: u32 = 82;
const EMR_EXTTEXTOUTW: u32 = 84;
const EMR_EXTCREATEPEN: u32 = 95;

const STOCK: u32 = 0x8000_0000;
const NULL_BRUSH: u32 = STOCK | 5;
const NULL_PEN: u32 = STOCK | 8;
const SYSTEM_FONT: u32 = STOCK | 13;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Object {
    Pen { rgb: u32, width: u32, style: u32, dash: Vec<u32> },
    Brush { rgb: u32 },
    Font { face: String, height: i32, tenths: i32, bold: bool, italic: bool },
}

/// The device-context state that `EMR_SAVEDC` and `EMR_RESTOREDC` carry along.
#[derive(Clone, Copy, Default)]
struct State {
    pen: Option<u32>,
    brush: Option<u32>,
    font: Option<u32>,
    text_color: Option<u32>,
    align: Option<u32>,
}

struct Emf<'a> {
    buf: Vec<u8>,
    records: u32,
    handles: HashMap<Object, u32>,
    next_handle: u32,
    state: State,
    saved: Vec<State>,
    background: Color,
    scale: f64,
    measurer: &'a dyn TextMeasurer,
}

/// Writes `scene`, scaled by `scale` (1 is the page size), measuring text with `measurer` for
/// the character spacing that is stored with each string.
pub fn write_emf(scene: &Scene, scale: f64, measurer: &dyn TextMeasurer) -> Vec<u8> {
    write_emf_view(scene, scale, measurer, crate::Rect { x: 0.0, y: 0.0, w: scene.width, h: scene.height })
}

/// [`write_emf`] for the part `view` of the page (page points) only.
pub fn write_emf_view(scene: &Scene, scale: f64, measurer: &dyn TextMeasurer, view: crate::Rect) -> Vec<u8> {
    let mut e = Emf {
        buf: Vec::with_capacity(32 * 1024),
        records: 0,
        handles: HashMap::new(),
        next_handle: 1,
        state: State::default(),
        saved: Vec::new(),
        background: scene.background,
        scale,
        measurer,
    };
    let (w, h) = (e.u(view.w), e.u(view.h));
    e.header(w, h);
    e.rec(EMR_SETMAPMODE, &le(&[1])); // MM_TEXT: one logical unit is one device unit
    if view.x != 0.0 || view.y != 0.0 {
        // The view's corner is the top-left of the picture.
        e.rec(EMR_SETWINDOWORGEX, &le_i(&[e.u(view.x), e.u(view.y)]));
    }
    e.rec(EMR_SETBKMODE, &le(&[1])); // TRANSPARENT
    e.rec(EMR_SETPOLYFILLMODE, &le(&[2])); // WINDING, as SVG fills
    e.rec(EMR_SETMITERLIMIT, &le(&[4]));
    if scene.background.a > 0 {
        e.item(&Item::Rect { rect: view, fill: Some(scene.background), stroke: None });
    }
    for layer in &scene.layers {
        let (underlay, overlay) = layer.split_overlay();
        layer.grid.iter().for_each(|it| e.item(it));
        underlay.iter().for_each(|it| e.item(it));
        let c = layer.clip;
        e.rec(EMR_SAVEDC, &[]);
        e.saved.push(e.state);
        let (l, t, r, b) = (e.u(c.x), e.u(c.y), e.u(c.x + c.w), e.u(c.y + c.h));
        e.rec(EMR_INTERSECTCLIPRECT, &le_i(&[l, t, r, b]));
        layer.plots.iter().for_each(|it| e.item(it));
        e.rec(EMR_RESTOREDC, &le_i(&[-1]));
        e.state = e.saved.pop().unwrap_or_default();
        overlay.iter().for_each(|it| e.item(it));
    }
    // Release the GDI objects: select stock ones first so that none is still selected.
    e.rec(EMR_SELECTOBJECT, &le(&[NULL_PEN]));
    e.rec(EMR_SELECTOBJECT, &le(&[NULL_BRUSH]));
    e.rec(EMR_SELECTOBJECT, &le(&[SYSTEM_FONT]));
    let mut handles: Vec<u32> = e.handles.values().copied().collect();
    handles.sort_unstable();
    for hnd in handles {
        e.rec(EMR_DELETEOBJECT, &le(&[hnd]));
    }
    e.finish()
}

/// A picture as a metafile of `width_pt` x `height_pt` points: one bitmap filling it (a layout page
/// printed or exported as EMF, T237 slice 9). Transparent pixels are blended onto white.
pub(crate) fn write_emf_picture(width_pt: f64, height_pt: f64, picture: &crate::Raster) -> Vec<u8> {
    let mut e = Emf {
        buf: Vec::with_capacity(picture.rgba.len() * 3 / 4 + 1024),
        records: 0,
        handles: HashMap::new(),
        next_handle: 1,
        state: State::default(),
        saved: Vec::new(),
        background: Color::WHITE,
        scale: 1.0,
        measurer: crate::fonts::SystemFonts::shared(),
    };
    let (w, h) = (e.u(width_pt), e.u(height_pt));
    e.header(w, h);
    e.rec(EMR_SETMAPMODE, &le(&[1]));
    e.bitmap(crate::Rect { x: 0.0, y: 0.0, w: width_pt, h: height_pt }, picture.width, picture.height, &picture.rgba);
    e.finish()
}

/// Decodes a PNG to 8-bit RGBA.
fn png_rgba(png: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let mut d = png::Decoder::new(std::io::Cursor::new(png));
    d.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut r = d.read_info().ok()?;
    let mut buf = vec![0; r.output_buffer_size()?];
    let info = r.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width as usize, info.height as usize);
    let channels = match info.color_type {
        png::ColorType::Grayscale => 1,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        png::ColorType::Indexed => return None,
    };
    let mut out = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        let row = &buf[y * info.line_size..];
        for x in 0..w {
            let p = &row[x * channels..x * channels + channels];
            out.extend_from_slice(&match channels {
                1 => [p[0], p[0], p[0], 255],
                2 => [p[0], p[0], p[0], p[1]],
                3 => [p[0], p[1], p[2], 255],
                _ => [p[0], p[1], p[2], p[3]],
            });
        }
    }
    Some((info.width, info.height, out))
}

fn le(v: &[u32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn le_i(v: &[i32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn colorref(c: Color) -> u32 {
    u32::from(c.r) | u32::from(c.g) << 8 | u32::from(c.b) << 16
}

impl Emf<'_> {
    /// Page points to logical units, with the export scale applied.
    fn u(&self, pt: f64) -> i32 {
        (pt * self.scale * UNITS_PER_PT).round() as i32
    }

    /// Patches nBytes, nRecords and nHandles into the header after the end record.
    fn finish(mut self) -> Vec<u8> {
        self.rec(EMR_EOF, &le(&[0, 16, 20]));
        let total = self.buf.len() as u32;
        self.buf[48..52].copy_from_slice(&total.to_le_bytes());
        self.buf[52..56].copy_from_slice(&self.records.to_le_bytes());
        self.buf[56..58].copy_from_slice(&(self.next_handle as u16).to_le_bytes());
        self.buf
    }

    /// `EMR_STRETCHDIBITS`: an RGBA picture `w` x `h` pixels stretched over `rect` (page points), as
    /// a bottom-up 24-bit DIB; alpha is blended onto the background (white when that is clear).
    fn bitmap(&mut self, rect: crate::Rect, w: u32, h: u32, rgba: &[u8]) {
        if w == 0 || h == 0 || rgba.len() < (w as usize) * (h as usize) * 4 {
            return;
        }
        let bg = if self.background.a == 0 { Color::WHITE } else { self.background };
        let stride = (w as usize * 3).div_ceil(4) * 4;
        let mut bits = vec![0u8; stride * h as usize];
        for y in 0..h as usize {
            // DIB rows go from the bottom up.
            let row = &mut bits[(h as usize - 1 - y) * stride..];
            for x in 0..w as usize {
                let p = &rgba[(y * w as usize + x) * 4..][..4];
                let a = u32::from(p[3]);
                let mix = |c: u8, b: u8| ((u32::from(c) * a + u32::from(b) * (255 - a) + 127) / 255) as u8;
                row[x * 3] = mix(p[2], bg.b);
                row[x * 3 + 1] = mix(p[1], bg.g);
                row[x * 3 + 2] = mix(p[0], bg.r);
            }
        }
        let (l, t) = (self.u(rect.x), self.u(rect.y));
        let (cw, ch) = (self.u(rect.w).max(1), self.u(rect.h).max(1));
        let mut p = le_i(&[l, t, l + cw - 1, t + ch - 1]); // rclBounds
        p.extend(le_i(&[l, t, 0, 0, w as i32, h as i32])); // xDest yDest xSrc ySrc cxSrc cySrc
        // offBmiSrc, cbBmiSrc, offBitsSrc, cbBitsSrc (from the start of the record), DIB_RGB_COLORS,
        // SRCCOPY
        p.extend(le(&[80, 40, 120, bits.len() as u32, 0, 0x00CC_0020]));
        p.extend(le_i(&[cw, ch])); // cxDest cyDest
        // BITMAPINFOHEADER: 24 bits, BI_RGB.
        p.extend(le_i(&[40, w as i32, h as i32]));
        p.extend_from_slice(&1u16.to_le_bytes());
        p.extend_from_slice(&24u16.to_le_bytes());
        p.extend(le(&[0, bits.len() as u32, 0, 0, 0, 0]));
        p.extend(bits);
        self.rec(EMR_STRETCHDIBITS, &p);
    }

    fn rec(&mut self, ty: u32, payload: &[u8]) {
        debug_assert!(payload.len().is_multiple_of(4), "records are multiples of 4 bytes");
        self.buf.extend_from_slice(&ty.to_le_bytes());
        self.buf.extend_from_slice(&((payload.len() + 8) as u32).to_le_bytes());
        self.buf.extend_from_slice(payload);
        self.records += 1;
    }

    fn header(&mut self, w: i32, h: i32) {
        // "Sheaf", "Figure" as the application and picture names, UTF-16, double-NUL ended.
        let desc: Vec<u16> = "Sheaf\0Figure\0\0".encode_utf16().collect();
        let mut p: Vec<u8> = Vec::new();
        p.extend(le_i(&[0, 0, w - 1, h - 1])); // rclBounds, device units, inclusive
        p.extend(le_i(&[0, 0, w, h])); // rclFrame, 0.01 mm
        // signature, version, then nBytes, nRecords and nHandles+reserved, patched at the end
        p.extend(le(&[0x464D_4520, 0x0001_0000, 0, 0, 0]));
        p.extend(le(&[desc.len() as u32, 108]));
        p.extend(le(&[0])); // nPalEntries
        // The reference device is 10000 x 10000 pixels of 0.01 mm: 100 x 100 mm.
        p.extend(le_i(&[10_000, 10_000, 100, 100]));
        p.extend(le(&[0, 0, 0])); // cbPixelFormat, offPixelFormat, bOpenGL
        p.extend(le(&[100_000, 100_000])); // szlMicrometers
        p.extend(desc.iter().flat_map(|c| c.to_le_bytes()));
        while !p.len().is_multiple_of(4) {
            p.push(0);
        }
        self.rec(EMR_HEADER, &p);
    }

    fn object(&mut self, obj: Object) -> u32 {
        if let Some(h) = self.handles.get(&obj) {
            return *h;
        }
        let h = self.next_handle;
        self.next_handle += 1;
        match &obj {
            Object::Brush { rgb } => self.rec(EMR_CREATEBRUSHINDIRECT, &le(&[h, 0, *rgb, 0])),
            Object::Pen { rgb, width, style, dash } => {
                let mut p = le(&[h, 0, 0, 0, 0, *style, *width, 0, *rgb, 0, dash.len() as u32]);
                p.extend(le(dash));
                self.rec(EMR_EXTCREATEPEN, &p);
            }
            Object::Font { face, height, tenths, bold, italic } => {
                let mut p = le(&[h]);
                p.extend(le_i(&[*height, 0, *tenths, *tenths, if *bold { 700 } else { 400 }]));
                // italic, underline, strikeout, charset DEFAULT, out/clip precision, quality, pitch
                p.extend([u8::from(*italic), 0, 0, 1, 0, 0, 4, 0]);
                let mut name: Vec<u16> = face.encode_utf16().take(31).collect();
                name.resize(32, 0);
                p.extend(name.iter().flat_map(|c| c.to_le_bytes()));
                self.rec(EMR_EXTCREATEFONTINDIRECTW, &p);
            }
        }
        self.handles.insert(obj, h);
        h
    }

    fn select_pen(&mut self, stroke: Option<&Stroke>) {
        let Some(st) = stroke else {
            if self.state.pen != Some(NULL_PEN) {
                self.rec(EMR_SELECTOBJECT, &le(&[NULL_PEN]));
                self.state.pen = Some(NULL_PEN);
            }
            return;
        };
        let cap = match st.cap {
            Cap::Round => 0,
            Cap::Square => 0x100,
            Cap::Butt => 0x200,
        };
        let join = match st.join {
            Join::Round => 0,
            Join::Miter => 0x2000,
        };
        let mut dash: Vec<u32> = st.dash.iter().map(|d| self.u(*d).max(1) as u32).collect();
        if dash.len() % 2 == 1 {
            dash.extend(dash.clone());
        }
        let style = 0x0001_0000 | cap | join | if dash.is_empty() { 0 } else { 7 };
        let width = (self.u(st.width)).max(1) as u32;
        let rgb = colorref(flatten(st.color, self.background));
        let h = self.object(Object::Pen { rgb, width, style, dash });
        if self.state.pen != Some(h) {
            self.rec(EMR_SELECTOBJECT, &le(&[h]));
            self.state.pen = Some(h);
        }
    }

    fn select_brush(&mut self, fill: Option<Color>) {
        let h = match fill {
            None => NULL_BRUSH,
            Some(c) => self.object(Object::Brush { rgb: colorref(flatten(c, self.background)) }),
        };
        if self.state.brush != Some(h) {
            self.rec(EMR_SELECTOBJECT, &le(&[h]));
            self.state.brush = Some(h);
        }
    }

    fn item(&mut self, item: &Item) {
        let (fill, stroke) = match item {
            Item::Text(t) => return self.text(t),
            Item::Rect { fill, stroke, .. } | Item::Path { fill, stroke, .. } => (*fill, stroke),
            // Pictures (T216): bitmaps (T237 slice 9).
            Item::Image { rect, png } => {
                if let Some((w, h, rgba)) = png_rgba(png) {
                    self.bitmap(*rect, w, h, &rgba);
                }
                return;
            }
        };
        let fill = fill.filter(|c| c.a > 0);
        let stroke = stroke.as_ref().filter(|s| s.color.a > 0 && s.width > 0.0);
        if fill.is_none() && stroke.is_none() {
            return;
        }
        if let Item::Rect { rect, .. } = item {
            self.select_brush(fill);
            self.select_pen(stroke);
            let (l, t, r, b) = (self.u(rect.x), self.u(rect.y), self.u(rect.x + rect.w), self.u(rect.y + rect.h));
            self.rec(EMR_RECTANGLE, &le_i(&[l, t, r, b]));
            return;
        }
        let Some(segs) = item_segs(item) else { return };
        if !segs.iter().any(|s| matches!(s, Seg::Move(..))) {
            return;
        }
        self.select_brush(fill);
        self.select_pen(stroke);
        self.rec(EMR_BEGINPATH, &[]);
        self.path(&segs);
        self.rec(EMR_ENDPATH, &[]);
        let ty = match (fill.is_some(), stroke.is_some()) {
            (true, true) => EMR_STROKEANDFILLPATH,
            (true, false) => EMR_FILLPATH,
            _ => EMR_STROKEPATH,
        };
        let b = self.bounds(&segs);
        self.rec(ty, &le_i(&b));
    }

    fn bounds(&self, segs: &[Seg]) -> [i32; 4] {
        let mut b = [i32::MAX, i32::MAX, i32::MIN, i32::MIN];
        let mut add = |x: f64, y: f64| {
            let (x, y) = (self.u(x), self.u(y));
            b = [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)];
        };
        for s in segs {
            match *s {
                Seg::Move(x, y) | Seg::Line(x, y) => add(x, y),
                Seg::Cubic(c) => (0..3).for_each(|k| add(c[2 * k], c[2 * k + 1])),
                Seg::Close => {}
            }
        }
        if b[0] > b[2] { [0, 0, 0, 0] } else { b }
    }

    fn path(&mut self, segs: &[Seg]) {
        let mut open = false;
        let mut i = 0;
        while i < segs.len() {
            match segs[i] {
                Seg::Move(x, y) => {
                    let p = le_i(&[self.u(x), self.u(y)]);
                    self.rec(EMR_MOVETOEX, &p);
                    open = true;
                    i += 1;
                }
                Seg::Close => {
                    if open {
                        self.rec(EMR_CLOSEFIGURE, &[]);
                    }
                    i += 1;
                }
                Seg::Line(..) | Seg::Cubic(_) if !open => i += 1,
                Seg::Line(..) => {
                    let mut pts: Vec<i32> = Vec::new();
                    while let Some(Seg::Line(x, y)) = segs.get(i) {
                        pts.extend([self.u(*x), self.u(*y)]);
                        i += 1;
                    }
                    self.poly(EMR_POLYLINETO, &pts);
                }
                Seg::Cubic(_) => {
                    let mut pts: Vec<i32> = Vec::new();
                    while let Some(Seg::Cubic(c)) = segs.get(i) {
                        pts.extend(c.iter().map(|v| self.u(*v)));
                        i += 1;
                    }
                    self.poly(EMR_POLYBEZIERTO, &pts);
                }
            }
        }
    }

    /// A `EMR_POLYLINETO` / `EMR_POLYBEZIERTO` record: bounds, count, 32-bit points.
    fn poly(&mut self, ty: u32, pts: &[i32]) {
        let xs = pts.iter().step_by(2);
        let ys = pts.iter().skip(1).step_by(2);
        let b = [
            xs.clone().min().copied().unwrap_or(0),
            ys.clone().min().copied().unwrap_or(0),
            xs.max().copied().unwrap_or(0),
            ys.max().copied().unwrap_or(0),
        ];
        let mut p = le_i(&b);
        p.extend(le(&[(pts.len() / 2) as u32]));
        p.extend(le_i(pts));
        self.rec(ty, &p);
    }

    fn text(&mut self, t: &TextItem) {
        if t.text.is_empty() || t.color.a == 0 {
            return;
        }
        let text: String = t.text.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
        let tenths = (t.rotate_ccw * 10.0).round() as i32;
        let font = self.object(Object::Font {
            face: t.family.clone(),
            height: -self.u(t.size).max(1),
            tenths,
            bold: t.bold,
            italic: t.italic,
        });
        if self.state.font != Some(font) {
            self.rec(EMR_SELECTOBJECT, &le(&[font]));
            self.state.font = Some(font);
        }
        let rgb = colorref(flatten(t.color, self.background));
        if self.state.text_color != Some(rgb) {
            self.rec(EMR_SETTEXTCOLOR, &le(&[rgb]));
            self.state.text_color = Some(rgb);
        }
        // TA_BASELINE (24) with TA_LEFT (0), TA_CENTER (6) or TA_RIGHT (2); the position is fixed.
        let align = 24
            | match t.anchor {
                Anchor::Start => 0,
                Anchor::Middle => 6,
                Anchor::End => 2,
            };
        if self.state.align != Some(align) {
            self.rec(EMR_SETTEXTALIGN, &le(&[align]));
            self.state.align = Some(align);
        }
        // Advance of each UTF-16 unit, from the same measurer as the layout.
        let measure =
            Font { family: t.family.clone(), size_pt: t.size, bold: t.bold, italic: t.italic, color: t.color };
        let mut units: Vec<u16> = Vec::new();
        let mut dx: Vec<u32> = Vec::new();
        let mut total = 0.0;
        for c in text.chars() {
            let w = self.measurer.width(c.encode_utf8(&mut [0; 4]), &measure, t.size);
            total += w;
            let n = c.len_utf16();
            let mut buf = [0u16; 2];
            units.extend_from_slice(c.encode_utf16(&mut buf));
            dx.push(self.u(w).max(0) as u32);
            dx.extend(std::iter::repeat_n(0, n - 1));
        }
        // A rectangle around the text for the record's bounds field.
        let (x0, x1) = match t.anchor {
            Anchor::Start => (0.0, total),
            Anchor::Middle => (-total / 2.0, total / 2.0),
            Anchor::End => (-total, 0.0),
        };
        let (sin, cos) = t.rotate_ccw.to_radians().sin_cos();
        let corners = [(x0, -t.size), (x1, -t.size), (x1, t.size * 0.3), (x0, t.size * 0.3)];
        let moved: Vec<(i32, i32)> = corners
            .iter()
            .map(|(cx, cy)| (self.u(t.x + cx * cos + cy * sin), self.u(t.y - cx * sin + cy * cos)))
            .collect();
        let b = [
            moved.iter().map(|p| p.0).min().unwrap_or(0),
            moved.iter().map(|p| p.1).min().unwrap_or(0),
            moved.iter().map(|p| p.0).max().unwrap_or(0),
            moved.iter().map(|p| p.1).max().unwrap_or(0),
        ];
        let off_string = 76; // from the start of the record: 8 + bounds 16 + 12 + EMRTEXT 40
        let str_bytes = units.len() * 2;
        let str_padded = str_bytes.div_ceil(4) * 4;
        let mut p = le_i(&b);
        p.extend(le(&[1])); // GM_COMPATIBLE
        p.extend(1.0f32.to_le_bytes()); // exScale: logical unit = 0.01 mm
        p.extend(1.0f32.to_le_bytes()); // eyScale
        p.extend(le_i(&[self.u(t.x), self.u(t.y)])); // ptlReference
        p.extend(le(&[units.len() as u32, off_string as u32, 0])); // nChars, offString, fOptions
        p.extend(le_i(&[0, 0, 0, 0])); // rcl (unused without ETO_CLIPPED / ETO_OPAQUE)
        p.extend(le(&[(off_string + str_padded) as u32])); // offDx
        p.extend(units.iter().flat_map(|c| c.to_le_bytes()));
        p.resize(p.len() + (str_padded - str_bytes), 0);
        p.extend(le(&dx));
        self.rec(EMR_EXTTEXTOUTW, &p);
    }
}

#[cfg(test)]
mod picture_tests {
    use super::*;

    /// The records of a metafile: (type, offset, size).
    fn records(emf: &[u8]) -> Vec<(u32, usize, usize)> {
        let mut out = Vec::new();
        let mut at = 0;
        while at + 8 <= emf.len() {
            let ty = u32::from_le_bytes(emf[at..at + 4].try_into().unwrap());
            let size = u32::from_le_bytes(emf[at + 4..at + 8].try_into().unwrap()) as usize;
            out.push((ty, at, size));
            at += size;
        }
        out
    }

    #[test]
    fn a_picture_is_a_bottom_up_24_bit_stretchdibits_record() {
        // 2 x 2 pixels: red, green / blue, half-transparent black.
        let rgba = vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 0, 0, 0, 128];
        let emf = write_emf_picture(72.0, 36.0, &crate::Raster { width: 2, height: 2, rgba });
        let recs = records(&emf);
        assert_eq!(
            recs.iter().map(|r| r.0).collect::<Vec<_>>(),
            [EMR_HEADER, EMR_SETMAPMODE, EMR_STRETCHDIBITS, EMR_EOF]
        );
        assert_eq!(u32::from_le_bytes(emf[48..52].try_into().unwrap()) as usize, emf.len(), "nBytes");
        let (_, at, size) = recs[2];
        let r = &emf[at..at + size];
        let i = |o: usize| i32::from_le_bytes(r[o..o + 4].try_into().unwrap());
        assert_eq!((i(40), i(44)), (2, 2), "cxSrc cySrc");
        assert_eq!((i(72), i(76)), (2540, 1270), "the picture covers 1 x 0.5 in (0.01 mm units)");
        assert_eq!((i(48), i(52), i(56)), (80, 40, 120), "the header and the bits follow the fixed part");
        assert_eq!((i(80), i(84), i(88)), (40, 2, 2), "BITMAPINFOHEADER of 2 x 2");
        // Rows are padded to 8 bytes, the bottom row first: blue, then grey-on-white; then red, green.
        let bits = &r[120..];
        assert_eq!(&bits[..6], &[255, 0, 0, 127, 127, 127], "bottom row (BGR): blue, black at half over white");
        assert_eq!(&bits[8..14], &[0, 0, 255, 0, 255, 0], "top row (BGR): red, green");
    }

    #[test]
    fn a_scene_picture_is_written_to_the_metafile() {
        let mut png = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut png, 1, 1);
            enc.set_color(png::ColorType::Rgb);
            enc.set_depth(png::BitDepth::Eight);
            enc.write_header().unwrap().write_image_data(&[10, 20, 30]).unwrap();
        }
        let rect = crate::Rect { x: 10.0, y: 10.0, w: 20.0, h: 20.0 };
        let mut e = Emf {
            buf: Vec::new(),
            records: 0,
            handles: HashMap::new(),
            next_handle: 1,
            state: State::default(),
            saved: Vec::new(),
            background: Color::WHITE,
            scale: 1.0,
            measurer: crate::fonts::SystemFonts::shared(),
        };
        e.header(100, 100);
        e.item(&Item::Image { rect, png: std::sync::Arc::new(png) });
        let emf = e.finish();
        let recs = records(&emf);
        let (_, at, size) = *recs.iter().find(|r| r.0 == EMR_STRETCHDIBITS).expect("the picture is a bitmap");
        assert_eq!(&emf[at + 120..at + 123], &[30, 20, 10], "BGR");
        assert_eq!(size, 124, "one pixel row padded to 4 bytes");
    }
}
