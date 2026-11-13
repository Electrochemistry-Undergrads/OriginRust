//! Margin control for exported figures (spec graphing.md 9.2): which part of the page, or of
//! the page and what hangs over its edge, a figure shows. The modes of Origin's export settings
//! are the full page, the box round everything drawn, that box plus a border, and the box round
//! what is mostly on the page.

use crate::Rect;
use crate::fonts::TextMeasurer;
use crate::hit::item_bounds;
use crate::scene::{Item, Scene};

/// Origin's `Margin` setting of the export settings.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum Margin {
    /// -2: the global Copy Page settings. Origin resolves it to [`Margin::Border`] with the
    /// default border width (see [`Margin::resolved`]).
    Auto,
    /// 0: the bounding box of everything drawn, plus a border.
    Border,
    /// 1: the bounding box of everything drawn, including what is outside the page.
    Tight,
    /// 2: the whole page rectangle. The default of the library functions, as before margin control.
    #[default]
    Page,
    /// 3: the bounding box of the objects that are over 50 % inside the page, kept inside the page.
    TightInPage,
}

impl Margin {
    /// The mode `Auto` stands for; the other modes stand for themselves.
    pub fn resolved(self) -> Margin {
        if self == Margin::Auto { Margin::Border } else { self }
    }

    /// Parses a mode name: `auto`, `border`, `tight`, `page`, `tight-in-page` (also with `_` or a space).
    pub fn parse(s: &str) -> Option<Margin> {
        match s.trim().to_ascii_lowercase().replace(['_', ' '], "-").as_str() {
            "auto" => Some(Margin::Auto),
            "border" => Some(Margin::Border),
            "tight" => Some(Margin::Tight),
            "page" => Some(Margin::Page),
            "tight-in-page" | "tightinpage" => Some(Margin::TightInPage),
            _ => None,
        }
    }

    /// The name [`Margin::parse`] reads.
    pub fn name(self) -> &'static str {
        match self {
            Margin::Auto => "auto",
            Margin::Border => "border",
            Margin::Tight => "tight",
            Margin::Page => "page",
            Margin::TightInPage => "tight-in-page",
        }
    }
}

/// The border added in [`Margin::Border`] when none is given (Origin's `BorderWid` -1), in points.
/// Origin's own value is not documented; this is 5 pt, about 1.8 mm [INF].
pub const DEFAULT_BORDER_PT: f64 = 5.0;

/// Margin mode and border width of an export.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct ExportLayout {
    pub margin: Margin,
    /// The border width of [`Margin::Border`] in pixels of the export (at the export DPI for
    /// raster figures, 1 px = 1 pt for vector ones); `None` or a negative value is the default.
    pub border_px: Option<f64>,
}

impl ExportLayout {
    /// The part of the page to show, in page points (the page itself in mode `Page`). `dpi` is the
    /// resolution of a raster export, which gives the border width its unit.
    pub fn view(&self, scene: &Scene, measurer: &dyn TextMeasurer, dpi: Option<f64>) -> Rect {
        let page = Rect { x: 0.0, y: 0.0, w: scene.width, h: scene.height };
        let pick = |r: Option<Rect>| r.filter(|r| r.w > 0.0 && r.h > 0.0).unwrap_or(page);
        match self.margin.resolved() {
            Margin::Page | Margin::Auto => page,
            Margin::Tight => pick(drawn_bounds(scene, measurer, None)),
            Margin::TightInPage => pick(drawn_bounds(scene, measurer, Some(page)).and_then(|r| intersect(r, page))),
            Margin::Border => {
                let Some(b) = drawn_bounds(scene, measurer, None) else { return page };
                let pt = match self.border_px {
                    Some(px) if px >= 0.0 => px * 72.0 / dpi.filter(|d| *d > 0.0).unwrap_or(72.0),
                    _ => DEFAULT_BORDER_PT,
                };
                pick(Some(widen(b, pt)))
            }
        }
    }
}

fn intersect(a: Rect, b: Rect) -> Option<Rect> {
    let (x0, y0) = (a.x.max(b.x), a.y.max(b.y));
    let (x1, y1) = ((a.x + a.w).min(b.x + b.w), (a.y + a.h).min(b.y + b.h));
    (x1 > x0 && y1 > y0).then_some(Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 })
}

fn union(a: Rect, b: Rect) -> Rect {
    let (x0, y0) = (a.x.min(b.x), a.y.min(b.y));
    let (x1, y1) = ((a.x + a.w).max(b.x + b.w), (a.y + a.h).max(b.y + b.h));
    Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }
}

/// The box round everything the scene draws (strokes widened, text measured, data cut at its
/// layer's clip rectangle), not counting the page background. With `page`, only items that are at
/// least half inside it count.
pub fn drawn_bounds(scene: &Scene, m: &dyn TextMeasurer, page: Option<Rect>) -> Option<Rect> {
    let mut all: Option<Rect> = None;
    let mut add = |item: &Item, clip: Option<Rect>| {
        let Some(mut b) = visible_bounds(item, m) else { return };
        if let Some(c) = clip {
            match intersect(b, c) {
                Some(r) => b = r,
                None => return,
            }
        }
        if let Some(p) = page {
            let area = b.w * b.h;
            let inside = intersect(b, p).map_or(0.0, |r| r.w * r.h);
            let ok = if area > 1e-9 {
                inside >= 0.5 * area
            } else {
                let (cx, cy) = (b.x + b.w / 2.0, b.y + b.h / 2.0);
                cx >= p.x && cx <= p.x + p.w && cy >= p.y && cy <= p.y + p.h
            };
            if !ok {
                return;
            }
        }
        all = Some(all.map_or(b, |a| union(a, b)));
    };
    for layer in &scene.layers {
        layer.grid.iter().for_each(|it| add(it, Some(layer.clip)));
        layer.plots.iter().for_each(|it| add(it, Some(layer.clip)));
        layer.overlay.iter().for_each(|it| add(it, None));
    }
    all
}

/// The bounds of one item with its line width added.
fn visible_bounds(item: &Item, m: &dyn TextMeasurer) -> Option<Rect> {
    match item {
        Item::Rect { rect, fill, stroke } => {
            if fill.is_none() && stroke.is_none() {
                return None;
            }
            Some(widen(*rect, stroke.as_ref().map_or(0.0, |s| s.width / 2.0)))
        }
        Item::Path { d, stroke, .. } => {
            let b = of_points(path_extent(d))?;
            Some(widen(b, stroke.as_ref().map_or(0.0, |s| s.width / 2.0)))
        }
        Item::Text(t) => {
            if t.text.trim().is_empty() {
                return None;
            }
            item_bounds(item, m)
        }
    }
}

fn widen(r: Rect, by: f64) -> Rect {
    Rect { x: r.x - by, y: r.y - by, w: r.w + 2.0 * by, h: r.h + 2.0 * by }
}

fn of_points(points: Vec<(f64, f64)>) -> Option<Rect> {
    let mut it = points.into_iter().filter(|(x, y)| x.is_finite() && y.is_finite());
    let (x, y) = it.next()?;
    let (x0, y0, x1, y1) = it.fold((x, y, x, y), |(a, b, c, d), (x, y)| (a.min(x), b.min(y), c.max(x), d.max(y)));
    Some(Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 })
}

/// Points that cover the path data written by this crate (`M`, `L`, `C`, `A`, `Z`, absolute):
/// the vertices, points along curves and points along arcs.
pub(crate) fn path_extent(d: &str) -> Vec<(f64, f64)> {
    let mut pts: Vec<(f64, f64)> = Vec::new();
    let mut cmd = 'M';
    let mut nums: Vec<f64> = Vec::new();
    let mut cur = (0.0, 0.0);
    let flush = |cmd: char, nums: &mut Vec<f64>, cur: &mut (f64, f64), pts: &mut Vec<(f64, f64)>| {
        let per = match cmd {
            'M' | 'L' => 2,
            'C' => 6,
            'A' => 7,
            _ => 0,
        };
        if per > 0 {
            for seg in nums.chunks_exact(per) {
                match cmd {
                    'M' | 'L' => *cur = (seg[0], seg[1]),
                    'C' => {
                        let (p0, p1, p2, p3) = (*cur, (seg[0], seg[1]), (seg[2], seg[3]), (seg[4], seg[5]));
                        for k in 1..8 {
                            let t = f64::from(k) / 8.0;
                            let u = 1.0 - t;
                            let f = |a: f64, b: f64, c: f64, d: f64| {
                                u * u * u * a + 3.0 * u * u * t * b + 3.0 * u * t * t * c + t * t * t * d
                            };
                            pts.push((f(p0.0, p1.0, p2.0, p3.0), f(p0.1, p1.1, p2.1, p3.1)));
                        }
                        *cur = p3;
                    }
                    _ => {
                        arc_points(*cur, seg, pts);
                        *cur = (seg[5], seg[6]);
                    }
                }
                pts.push(*cur);
            }
        }
        nums.clear();
    };
    let mut token = String::new();
    let take = |token: &mut String, nums: &mut Vec<f64>| {
        if let Ok(v) = token.parse() {
            nums.push(v);
        }
        token.clear();
    };
    for c in d.chars() {
        match c {
            'M' | 'L' | 'C' | 'A' | 'Z' => {
                take(&mut token, &mut nums);
                flush(cmd, &mut nums, &mut cur, &mut pts);
                cmd = c;
            }
            ' ' | ',' => take(&mut token, &mut nums),
            '-' if !token.is_empty() && !token.ends_with('e') => {
                take(&mut token, &mut nums);
                token.push(c);
            }
            _ => token.push(c),
        }
    }
    take(&mut token, &mut nums);
    flush(cmd, &mut nums, &mut cur, &mut pts);
    pts
}

/// Points along the circular arc `A r r rot large sweep x y` (`seg` holds its 7 numbers) from
/// `from`. Only circles are written by this crate; the radius `ry` and the rotation are ignored.
fn arc_points(from: (f64, f64), seg: &[f64], pts: &mut Vec<(f64, f64)>) {
    let (large, sweep) = (seg[3] != 0.0, seg[4] != 0.0);
    let to = (seg[5], seg[6]);
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let chord = (dx * dx + dy * dy).sqrt();
    if chord < 1e-9 {
        return;
    }
    let r = seg[0].abs().max(chord / 2.0);
    let h = (r * r - chord * chord / 4.0).max(0.0).sqrt();
    let mid = ((from.0 + to.0) / 2.0, (from.1 + to.1) / 2.0);
    // The centre is on the perpendicular bisector; which side follows the two flags.
    let sign = if large == sweep { -1.0 } else { 1.0 };
    let c = (mid.0 + sign * h * dy / chord, mid.1 - sign * h * dx / chord);
    let a0 = (from.1 - c.1).atan2(from.0 - c.0);
    let a1 = (to.1 - c.1).atan2(to.0 - c.0);
    let tau = std::f64::consts::TAU;
    let mut delta = a1 - a0;
    if sweep {
        while delta < 0.0 {
            delta += tau;
        }
    } else {
        while delta > 0.0 {
            delta -= tau;
        }
    }
    for k in 1..32 {
        let a = a0 + delta * f64::from(k) / 32.0;
        pts.push((c.0 + r * a.cos(), c.1 + r * a.sin()));
    }
}
