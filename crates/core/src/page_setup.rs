//! Page Setup: the paper, its orientation and the margins a page is printed with, kept on the
//! project (and so saved with it), and the one function that decides where a graph page lands on
//! the paper. Printing and Print Preview both call [`placement`], so the preview shows exactly what
//! the printer gets. See `docs/spec/graphing.md`, section "Printing".

use serde::{Deserialize, Serialize};

/// Points per inch and per millimetre.
const PT_PER_IN: f64 = 72.0;
const PT_PER_MM: f64 = 72.0 / 25.4;

/// The unprintable edge of the fallback printable area, in points (measured in Origin 2016).
const FALLBACK_EDGE_X_PT: f64 = 12.0;
const FALLBACK_EDGE_Y_PT: f64 = 18.0;

/// The paper sizes Page Setup offers (the Windows paper list's common entries).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Paper {
    #[default]
    Letter,
    Legal,
    Tabloid,
    A3,
    A4,
    A5,
}

impl Paper {
    pub const ALL: [Paper; 6] = [Paper::Letter, Paper::Legal, Paper::Tabloid, Paper::A3, Paper::A4, Paper::A5];

    pub fn label(self) -> &'static str {
        match self {
            Paper::Letter => "Letter (8.5 x 11 in)",
            Paper::Legal => "Legal (8.5 x 14 in)",
            Paper::Tabloid => "Tabloid (11 x 17 in)",
            Paper::A3 => "A3 (297 x 420 mm)",
            Paper::A4 => "A4 (210 x 297 mm)",
            Paper::A5 => "A5 (148 x 210 mm)",
        }
    }

    /// Width and height in points, portrait.
    pub fn size_pt(self) -> (f64, f64) {
        match self {
            Paper::Letter => (8.5 * PT_PER_IN, 11.0 * PT_PER_IN),
            Paper::Legal => (8.5 * PT_PER_IN, 14.0 * PT_PER_IN),
            Paper::Tabloid => (11.0 * PT_PER_IN, 17.0 * PT_PER_IN),
            Paper::A3 => (297.0 * PT_PER_MM, 420.0 * PT_PER_MM),
            Paper::A4 => (210.0 * PT_PER_MM, 297.0 * PT_PER_MM),
            Paper::A5 => (148.0 * PT_PER_MM, 210.0 * PT_PER_MM),
        }
    }

    /// The Windows `DMPAPER_*` code of the size.
    pub fn dmpaper(self) -> i16 {
        match self {
            Paper::Letter => 1,
            Paper::Tabloid => 3,
            Paper::Legal => 5,
            Paper::A3 => 8,
            Paper::A4 => 9,
            Paper::A5 => 11,
        }
    }
}

/// The orientation of the paper. `Auto` follows the page being printed (landscape for a page that
/// is wider than tall), which is what Origin's Page Setup showed for a landscape graph page.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Orientation {
    #[default]
    Auto,
    Portrait,
    Landscape,
}

impl Orientation {
    pub const ALL: [Orientation; 3] = [Orientation::Auto, Orientation::Portrait, Orientation::Landscape];

    pub fn label(self) -> &'static str {
        match self {
            Orientation::Auto => "Automatic",
            Orientation::Portrait => "Portrait",
            Orientation::Landscape => "Landscape",
        }
    }

    /// Portrait or landscape for a page of `page_w` x `page_h` (never `Auto`).
    pub fn resolve(self, page_w: f64, page_h: f64) -> Orientation {
        match self {
            Orientation::Auto if page_w > page_h => Orientation::Landscape,
            Orientation::Auto => Orientation::Portrait,
            other => other,
        }
    }
}

/// Margins in points, measured in from each paper edge.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Margins {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

impl Margins {
    pub fn all(pt: f64) -> Margins {
        Margins { left: pt, top: pt, right: pt, bottom: pt }
    }
}

/// Paper, orientation and margins of the project's printing.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PageSetup {
    pub paper: Paper,
    pub orientation: Orientation,
    pub margins: Margins,
}

impl Default for PageSetup {
    /// Letter, orientation following the page, 1 inch margins: what Origin's Page Setup showed on
    /// a new graph (the printer's unprintable edge is respected on top of the margins).
    fn default() -> Self {
        PageSetup { paper: Paper::Letter, orientation: Orientation::Auto, margins: Margins::all(72.0) }
    }
}

/// An axis-aligned rectangle in points, measured from the paper's top left corner.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl PageSetup {
    /// The orientation a page of `page_w` x `page_h` points is printed in.
    pub fn orientation_for(&self, page_w: f64, page_h: f64) -> Orientation {
        self.orientation.resolve(page_w, page_h)
    }

    /// The paper's width and height in points as it lies on the printer (landscape swaps them),
    /// for a page of `page_w` x `page_h` points.
    pub fn paper_pt(&self, page_w: f64, page_h: f64) -> (f64, f64) {
        let (w, h) = self.paper.size_pt();
        match self.orientation_for(page_w, page_h) {
            Orientation::Landscape => (h, w),
            _ => (w, h),
        }
    }

    /// The page size Format > Fit Page To Printer gives when no printer can be asked: the paper in
    /// the page's orientation less the unprintable edges measured in Origin 2016 (12 pt at left and
    /// right, 18 pt at top and bottom). Letter landscape gives 768 x 576 pt, Origin's 6400 x 4800
    /// dots at 600 dpi.
    pub fn fallback_printable_pt(&self, page_w: f64, page_h: f64) -> (f64, f64) {
        let (w, h) = self.paper_pt(page_w, page_h);
        ((w - 2.0 * FALLBACK_EDGE_X_PT).max(1.0), (h - 2.0 * FALLBACK_EDGE_Y_PT).max(1.0))
    }

    /// Where a page of `page_w` x `page_h` points lands on this setup's paper when the whole paper
    /// can be printed on (the Print Preview case). See [`placement`].
    pub fn place(&self, page_w: f64, page_h: f64) -> Option<Rect> {
        let (pw, ph) = self.paper_pt(page_w, page_h);
        placement(page_w, page_h, (pw, ph), Rect { x: 0.0, y: 0.0, w: pw, h: ph }, &self.margins)
    }
}

/// Where a graph page of `page_w` x `page_h` points is drawn on a sheet of `paper` points, as a
/// rectangle measured from the paper's top left corner. The page is scaled (up or down, aspect
/// ratio kept) to fill the area inside `margins` that the printer can also reach (`printable`),
/// and centred in that area. `None` when nothing is left to draw on or the page has no size.
pub fn placement(page_w: f64, page_h: f64, paper: (f64, f64), printable: Rect, margins: &Margins) -> Option<Rect> {
    let left = (margins.left.max(0.0)).max(printable.x);
    let top = (margins.top.max(0.0)).max(printable.y);
    let right = (paper.0 - margins.right.max(0.0)).min(printable.x + printable.w);
    let bottom = (paper.1 - margins.bottom.max(0.0)).min(printable.y + printable.h);
    let (area_w, area_h) = (right - left, bottom - top);
    if !(page_w > 0.0 && page_h > 0.0 && area_w > 0.0 && area_h > 0.0) {
        return None;
    }
    let scale = (area_w / page_w).min(area_h / page_h);
    let (w, h) = (page_w * scale, page_h * scale);
    Some(Rect { x: left + (area_w - w) / 2.0, y: top + (area_h - h) / 2.0, w, h })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    fn a4(orientation: Orientation, m: f64) -> PageSetup {
        PageSetup { paper: Paper::A4, orientation, margins: Margins::all(m) }
    }

    #[test]
    fn the_fallback_printable_area_is_the_paper_less_the_measured_edges() {
        let letter = PageSetup::default();
        // A landscape page (Automatic follows it): 11 x 8.5 in less 12 pt and 18 pt edges.
        let (w, h) = letter.fallback_printable_pt(768.0, 576.0);
        assert!(close(w, 768.0) && close(h, 576.0));
        assert!(close(w / 72.0, 10.0 + 2.0 / 3.0) && close(h / 72.0, 8.0));
        // A portrait page gives the portrait area.
        let (w, h) = letter.fallback_printable_pt(576.0, 768.0);
        assert!(close(w, 8.5 * 72.0 - 24.0) && close(h, 11.0 * 72.0 - 36.0));
        // A forced orientation wins over the page's shape.
        let forced = PageSetup { orientation: Orientation::Portrait, ..letter };
        assert_eq!(forced.fallback_printable_pt(768.0, 576.0), (8.5 * 72.0 - 24.0, 11.0 * 72.0 - 36.0));
    }

    #[test]
    fn a_wide_page_on_portrait_a4_fills_the_width() {
        // 6.4 x 4.8 in graph page, A4 portrait, 36 pt margins: the width limits.
        let s = a4(Orientation::Portrait, 36.0);
        let r = s.place(6.4 * 72.0, 4.8 * 72.0).unwrap();
        let (pw, ph) = s.paper_pt(6.4 * 72.0, 4.8 * 72.0);
        assert!(pw < ph);
        assert!(close(r.w, pw - 72.0));
        assert!(close(r.h / r.w, 4.8 / 6.4));
        assert!(close(r.x, 36.0));
        assert!(close(r.y + r.h / 2.0, ph / 2.0));
    }

    #[test]
    fn automatic_orientation_follows_the_page() {
        let s = a4(Orientation::Auto, 36.0);
        let (pw, ph) = s.paper_pt(800.0, 600.0);
        assert!(pw > ph, "a wide page prints landscape");
        // A 4:3 page on A4 landscape: the height limits.
        let r = s.place(800.0, 600.0).unwrap();
        assert!(close(r.h, ph - 72.0));
        assert!(close(r.w, (ph - 72.0) * 4.0 / 3.0));
        assert!(close(r.x + r.w / 2.0, pw / 2.0));
        assert!(close(r.y, 36.0));
        let (pw, ph) = s.paper_pt(600.0, 800.0);
        assert!(pw < ph, "a tall page prints portrait");
        assert_eq!(s.orientation_for(500.0, 500.0), Orientation::Portrait);
    }

    #[test]
    fn a_set_orientation_overrides_the_page() {
        let s = a4(Orientation::Landscape, 36.0);
        let (pw, ph) = s.paper_pt(600.0, 800.0);
        assert!(pw > ph);
        // A tall page turned onto landscape paper: the height limits.
        let r = s.place(600.0, 800.0).unwrap();
        assert!(close(r.h, ph - 72.0));
        assert!(close(r.x + r.w / 2.0, pw / 2.0));
    }

    #[test]
    fn letter_with_unequal_margins() {
        let s = PageSetup {
            paper: Paper::Letter,
            orientation: Orientation::Portrait,
            margins: Margins { left: 72.0, top: 18.0, right: 36.0, bottom: 90.0 },
        };
        // A square page: the area is 504 wide (612-72-36) and 684 high (792-18-90); the width limits.
        let r = s.place(100.0, 100.0).unwrap();
        assert!(close(r.w, 504.0) && close(r.h, 504.0));
        assert!(close(r.x, 72.0));
        assert!(close(r.y, 18.0 + (684.0 - 504.0) / 2.0));
    }

    #[test]
    fn the_default_is_letter_with_inch_margins() {
        let s = PageSetup::default();
        assert_eq!(s.paper, Paper::Letter);
        assert_eq!(s.orientation, Orientation::Auto);
        assert_eq!(s.margins, Margins::all(72.0));
        // The 6.4 x 4.8 in default graph page on Letter landscape: 11 x 8.5 in, 1 in margins, so
        // the area is 9 x 6.5 in and the width limits (6.4/4.8 = 1.333 < 9/6.5 = 1.385: height).
        let r = s.place(6.4 * 72.0, 4.8 * 72.0).unwrap();
        assert!(close(r.h, 6.5 * 72.0));
        assert!(close(r.w, 6.5 * 72.0 * 6.4 / 4.8));
    }

    #[test]
    fn the_printers_unprintable_edge_wins_over_a_smaller_margin() {
        // Letter, margins of 5 pt, but the printer cannot reach the outer 12 pt.
        let printable = Rect { x: 12.0, y: 12.0, w: 612.0 - 24.0, h: 792.0 - 24.0 };
        let r = placement(600.0, 800.0, (612.0, 792.0), printable, &Margins::all(5.0)).unwrap();
        assert!(r.x >= 12.0 - 1e-9 && r.y >= 12.0 - 1e-9);
        assert!(r.x + r.w <= 600.0 + 1e-9 && r.y + r.h <= 780.0 + 1e-9);
        // A tall page: the height limits (768 = 780 - 12).
        assert!(close(r.h, 768.0));
    }

    #[test]
    fn small_pages_are_scaled_up_to_fit() {
        let s = PageSetup { paper: Paper::A5, ..PageSetup::default() };
        let r = s.place(10.0, 10.0).unwrap();
        assert!(r.w > 100.0);
    }

    #[test]
    fn nothing_to_draw_on_gives_none() {
        let s = PageSetup { margins: Margins::all(500.0), ..PageSetup::default() };
        assert!(s.place(100.0, 100.0).is_none());
        assert!(PageSetup::default().place(0.0, 10.0).is_none());
    }

    #[test]
    fn setup_round_trips_through_json() {
        let s = PageSetup {
            paper: Paper::Legal,
            orientation: Orientation::Landscape,
            margins: Margins { left: 1.0, top: 2.0, right: 3.0, bottom: 4.0 },
        };
        let back: PageSetup = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back, s);
    }
}
