//! Graph > Fit Page to Layers (Origin's `pfit2l`, graphing spec 15.7d): the page is cut or grown to
//! the box round what the graph draws, plus a border, and the layers keep their size and distance
//! from each other in points.

use crate::graph::Graph;

/// Origin's "Margin Control".
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FitMargin {
    /// "Border" (the default): this many percent of the page's old width (height) on the left and
    /// right (top and bottom). Origin's default is 5.
    Border(f64),
    /// "Tight": a hairline round the content (measured: about 13 dots at 600 dpi, 1.56 points).
    Tight,
}

/// Origin's "Adjust Direction".
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum FitDirection {
    /// "Height and Width" (the default).
    #[default]
    Both,
    /// "Height Only": the page keeps its width and the layers their left edges.
    Height,
    /// "Width Only".
    Width,
}

/// The margin of Tight, in points (13 dots at 600 dpi).
pub const TIGHT_MARGIN_PT: f64 = 13.0 * 72.0 / 600.0;

/// Why the page cannot be fitted.
#[derive(Debug, thiserror::Error, Clone, PartialEq)]
pub enum FitError {
    #[error("the graph draws nothing to fit the page to")]
    Empty,
    #[error("the border width must be from 0 to 50 percent")]
    Border,
}

impl Graph {
    /// Fits the page to `content`, the box `(x, y, width, height)` in points from the page's top left
    /// corner round everything the graph draws (`sheaf_plot::drawn_bounds` of its scene). Layers that
    /// stand on the page get new frames so that their size and place relative to the content stay the
    /// same in points; linked layers are fractions of their parents and need no change.
    pub fn fit_page_to_content(
        &mut self,
        content: (f64, f64, f64, f64),
        margin: FitMargin,
        direction: FitDirection,
    ) -> Result<(), FitError> {
        let (cx, cy, cw, ch) = content;
        if !(cw > 0.0 && ch > 0.0 && cx.is_finite() && cy.is_finite()) {
            return Err(FitError::Empty);
        }
        let (old_w, old_h) = (self.page.width_pt, self.page.height_pt);
        let (mx, my) = match margin {
            FitMargin::Border(b) => {
                if !(0.0..=50.0).contains(&b) {
                    return Err(FitError::Border);
                }
                (old_w * b / 100.0, old_h * b / 100.0)
            }
            FitMargin::Tight => (TIGHT_MARGIN_PT, TIGHT_MARGIN_PT),
        };
        let fit_w = direction != FitDirection::Height;
        let fit_h = direction != FitDirection::Width;
        let (new_w, dx) = if fit_w { (cw + 2.0 * mx, mx - cx) } else { (old_w, 0.0) };
        let (new_h, dy) = if fit_h { (ch + 2.0 * my, my - cy) } else { (old_h, 0.0) };
        for layer in self.layers.iter_mut().filter(|l| l.link.is_none()) {
            let f = &mut layer.frame;
            f.left = (f.left * old_w + dx) / new_w;
            f.width = f.width * old_w / new_w;
            f.top = (f.top * old_h + dy) / new_h;
            f.height = f.height * old_h / new_h;
        }
        // An invalidly linked layer stands on the page too (Graph::link_parent): move it as well.
        let invalid: Vec<usize> = (0..self.layers.len())
            .filter(|&i| self.layers[i].link.is_some() && self.link_parent(i).is_none())
            .collect();
        for i in invalid {
            let f = &mut self.layers[i].frame;
            f.left = (f.left * old_w + dx) / new_w;
            f.width = f.width * old_w / new_w;
            f.top = (f.top * old_h + dy) / new_h;
            f.height = f.height * old_h / new_h;
        }
        self.page.width_pt = new_w;
        self.page.height_pt = new_h;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Frame;
    use crate::ids::WindowId;
    use crate::template::Template;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    /// Origin's stock page in points (6432 x 4923 dots at 600 dpi) with its layer.
    fn stock() -> Graph {
        let mut g = Template::Line.new_graph(WindowId(1), "Graph1");
        g.page.width_pt = 6432.0 * 0.12;
        g.page.height_pt = 4923.0 * 0.12;
        g.layers[0].frame = Frame { left: 0.1791, top: 0.1158, width: 0.6816, height: 0.7183 };
        g
    }

    /// The layer frame in points.
    fn frame_pt(g: &Graph, i: usize) -> (f64, f64, f64, f64) {
        let f = g.layers[i].frame;
        (f.left * g.page.width_pt, f.top * g.page.height_pt, f.width * g.page.width_pt, f.height * g.page.height_pt)
    }

    #[test]
    fn the_border_is_a_share_of_the_old_page_and_the_layer_keeps_its_size() {
        // Origin: an empty stock layer counted as its frame plus 370 dots above; the page became
        // 5026 x 4398 dots with a border of 5 (321 and 246 dots).
        let mut g = stock();
        let (x, y, w, h) = frame_pt(&g, 0);
        let dot = 0.12;
        let content = (x, y - 370.0 * dot, w, h + 370.0 * dot);
        g.fit_page_to_content(content, FitMargin::Border(5.0), FitDirection::Both).unwrap();
        // Origin counts whole dots (a border of 321, not 321.6)
        assert!((g.page.width_pt / dot - 5026.0).abs() < 2.0, "{}", g.page.width_pt / dot);
        assert!((g.page.height_pt / dot - 4398.0).abs() < 2.0, "{}", g.page.height_pt / dot);
        let (_, _, w2, h2) = frame_pt(&g, 0);
        assert!(close(w, w2) && close(h, h2), "the layer keeps its size in points");
        // Origin: left 6.387, top 14.01 percent
        let f = g.layers[0].frame;
        assert!((100.0 * f.left - 6.387).abs() < 0.02 && (100.0 * f.top - 14.01).abs() < 0.02, "{f:?}");
    }

    #[test]
    fn height_only_keeps_the_width_and_the_lefts() {
        let mut g = stock();
        let before = g.layers[0].frame;
        let (x, y, w, h) = frame_pt(&g, 0);
        g.fit_page_to_content((x, y - 44.4, w, h + 44.4), FitMargin::Border(5.0), FitDirection::Height).unwrap();
        assert!(close(g.page.width_pt, 6432.0 * 0.12));
        assert!(close(g.layers[0].frame.left, before.left) && close(g.layers[0].frame.width, before.width));
        assert!((100.0 * g.layers[0].frame.top - 14.01).abs() < 0.01);
    }

    #[test]
    fn tight_and_zero_border_and_errors() {
        let mut g = stock();
        let (x, y, w, h) = frame_pt(&g, 0);
        let mut zero = g.clone();
        zero.fit_page_to_content((x, y, w, h), FitMargin::Border(0.0), FitDirection::Both).unwrap();
        assert!(close(zero.layers[0].frame.left, 0.0) && close(zero.layers[0].frame.width, 1.0));
        let mut tight = g.clone();
        tight.fit_page_to_content((x, y, w, h), FitMargin::Tight, FitDirection::Both).unwrap();
        assert!(close(tight.page.width_pt, w + 2.0 * TIGHT_MARGIN_PT));
        assert_eq!(g.fit_page_to_content((x, y, 0.0, h), FitMargin::Tight, FitDirection::Both), Err(FitError::Empty));
        assert_eq!(
            g.fit_page_to_content((x, y, w, h), FitMargin::Border(60.0), FitDirection::Both),
            Err(FitError::Border)
        );
    }

    #[test]
    fn a_linked_layer_is_left_as_fractions_of_its_parent() {
        let mut g = stock();
        let mut child = g.layers[0].clone();
        child.link = Some(crate::graph::LayerLink {
            parent: 0,
            x: crate::graph::ScaleLink::Straight,
            y: crate::graph::ScaleLink::None,
        });
        child.frame = Frame::WHOLE;
        g.layers.push(child);
        let (x, y, w, h) = frame_pt(&g, 0);
        g.fit_page_to_content((x, y, w, h), FitMargin::Border(5.0), FitDirection::Both).unwrap();
        assert_eq!(g.layers[1].frame, Frame::WHOLE);
    }
}
