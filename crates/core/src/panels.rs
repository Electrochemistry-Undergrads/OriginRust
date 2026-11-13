//! Multi-panel graphs (graphing spec section 15): several layers on one page, placed in the cells
//! of a grid, each with its own axes, plots and legend. Panels that share an axis are linked
//! layers (ADR 0009): every panel after the first is linked to the first one, so a vertical stack
//! of panels shares one X scale.
//!
//! A [`PanelLayout`] is an instruction, not something stored in the graph: applying it writes the
//! frames of the layers (and, when a panel graph is made, their links and label settings).

use crate::graph::{Frame, Graph, TickDirection};

/// Where the first panel goes.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PanelOrder {
    /// Row by row from the top left (Origin's 4 Panel and 9 Panel templates, Merge Graph).
    TopDown,
    /// Row by row from the bottom left: the first panel is at the bottom (Origin's Stack command
    /// and its Vertical 2 Panel template).
    BottomUp,
}

/// The page a new panel graph gets.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PanelPage {
    /// The template's page as it is.
    Keep,
    /// The template's page turned upright (width and height swapped when it is wider than tall),
    /// as Origin's Stack command does.
    Upright,
    /// The template's page repeated: `cols` times its width by `rows` times its height, so that
    /// every panel gets as much room as a single graph of the template.
    Tiled,
}

/// How panels are placed on the page. Lengths are in percent of the page: margins and the
/// horizontal gap of its width, the vertical gap and the top and bottom margins of its height.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct PanelLayout {
    pub rows: usize,
    pub cols: usize,
    /// Gap between neighbouring columns.
    pub h_gap: f64,
    /// Gap between neighbouring rows.
    pub v_gap: f64,
    pub margin_left: f64,
    pub margin_right: f64,
    pub margin_top: f64,
    pub margin_bottom: f64,
    pub order: PanelOrder,
    /// Panels after the first are linked to the first one with the same X scale.
    pub link_x: bool,
    /// Panels after the first are linked to the first one with the same Y scale.
    pub link_y: bool,
    pub page: PanelPage,
}

/// The most panels one layout may have.
pub const MAX_PANELS: usize = 64;

/// Why a panel layout or a panel graph cannot be made.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum PanelError {
    #[error("a panel grid needs at least 1 row and 1 column and at most {MAX_PANELS} panels (got {rows} x {cols})")]
    GridSize { rows: usize, cols: usize },
    #[error("panel gaps must be from 0 to 50 percent of the page")]
    Gap,
    #[error("panel margins must be from 0 to 90 percent of the page")]
    Margin,
    #[error("the margins and gaps leave no room for the panels")]
    NoRoom,
    #[error("there is nothing to put in the panels")]
    NoGroups,
    #[error("the layout has {cells} cell(s) but the graph has {panels} panels")]
    TooManyPanels { cells: usize, panels: usize },
    #[error("the '{0}' template has more than one layer and cannot be used for panels")]
    Template(&'static str),
    #[error("this panel does not share the X axis of the first panel, so it cannot get a right Y axis")]
    RightAxis,
}

impl PanelLayout {
    /// Origin's Stack command for `n` panels: one column, no gap, margins of 15 (left), 10
    /// (right, top and bottom) percent, the first panel at the bottom, X linked, an upright page.
    pub fn stack(n: usize) -> PanelLayout {
        PanelLayout {
            rows: n,
            cols: 1,
            h_gap: 0.0,
            v_gap: 0.0,
            margin_left: 15.0,
            margin_right: 10.0,
            margin_top: 10.0,
            margin_bottom: 10.0,
            order: PanelOrder::BottomUp,
            link_x: true,
            link_y: false,
            page: PanelPage::Upright,
        }
    }

    /// Sheaf's grid of `rows` x `cols` panels for a template whose layer sits at `frame` on its
    /// page: the page is the template's page repeated (`PanelPage::Tiled`) and each panel sits in
    /// its cell where the template puts its layer, so every panel has the room of a single graph
    /// and its labels fit as they do there. The margins are the template's margins divided by the
    /// number of columns (rows), and the gaps the sum of the two margins they separate. One column
    /// fills from the bottom, as Origin's Vertical 2 Panel; several columns fill row by row from
    /// the top left, as its 4 Panel and 9 Panel. No axes are linked.
    pub fn grid(rows: usize, cols: usize, frame: &Frame) -> PanelLayout {
        let pct = |v: f64| (100.0 * v).max(0.0);
        let (left, top) = (pct(frame.left), pct(frame.top));
        let right = pct(1.0 - frame.left - frame.width);
        let bottom = pct(1.0 - frame.top - frame.height);
        let (r, c) = (rows.max(1) as f64, cols.max(1) as f64);
        PanelLayout {
            rows,
            cols,
            h_gap: (left + right) / c,
            v_gap: (top + bottom) / r,
            margin_left: left / c,
            margin_right: right / c,
            margin_top: top / r,
            margin_bottom: bottom / r,
            order: if cols == 1 { PanelOrder::BottomUp } else { PanelOrder::TopDown },
            link_x: false,
            link_y: false,
            page: PanelPage::Tiled,
        }
    }

    /// Origin's Arrange Layers defaults for `rows` x `cols` cells: gaps of 5 and margins of
    /// 15 percent on every side, row by row from the top left, no links, the page kept.
    pub fn arrange(rows: usize, cols: usize) -> PanelLayout {
        PanelLayout {
            rows,
            cols,
            h_gap: 5.0,
            v_gap: 5.0,
            margin_left: 15.0,
            margin_right: 15.0,
            margin_top: 15.0,
            margin_bottom: 15.0,
            order: PanelOrder::TopDown,
            link_x: false,
            link_y: false,
            page: PanelPage::Keep,
        }
    }

    /// The number of cells.
    pub fn cells(&self) -> usize {
        self.rows.saturating_mul(self.cols)
    }

    /// Checks the grid size, gaps and margins.
    pub fn validate(&self) -> Result<(), PanelError> {
        if self.rows == 0 || self.cols == 0 || self.cells() > MAX_PANELS {
            return Err(PanelError::GridSize { rows: self.rows, cols: self.cols });
        }
        let within = |v: f64, hi: f64| v.is_finite() && (0.0..=hi).contains(&v);
        if !(within(self.h_gap, 50.0) && within(self.v_gap, 50.0)) {
            return Err(PanelError::Gap);
        }
        let margins = [self.margin_left, self.margin_right, self.margin_top, self.margin_bottom];
        if !margins.iter().all(|&m| within(m, 90.0)) {
            return Err(PanelError::Margin);
        }
        Ok(())
    }

    /// The row (counted from the top) and the column of panel `k`.
    pub fn cell(&self, k: usize) -> (usize, usize) {
        let cols = self.cols.max(1);
        let row = k / cols;
        match self.order {
            PanelOrder::TopDown => (row, k % cols),
            PanelOrder::BottomUp => (self.rows.saturating_sub(1).saturating_sub(row), k % cols),
        }
    }

    /// The panel in the cell at `row` (from the top) and `col`, when it is one of the first
    /// `count` panels.
    fn panel_at(&self, row: usize, col: usize, count: usize) -> Option<usize> {
        if row >= self.rows || col >= self.cols {
            return None;
        }
        let r = match self.order {
            PanelOrder::TopDown => row,
            PanelOrder::BottomUp => self.rows - 1 - row,
        };
        let k = r * self.cols + col;
        (k < count).then_some(k)
    }

    /// The frame of every cell, in panel order, as fractions of the page. With w and h the size of
    /// a cell in percent, `w = (100 - left - right - (cols - 1) h_gap) / cols` and
    /// `h = (100 - top - bottom - (rows - 1) v_gap) / rows`; the cell in row r and column c starts
    /// at `left + c (w + h_gap)` and `top + r (h + v_gap)`.
    pub fn frames(&self) -> Result<Vec<Frame>, PanelError> {
        self.validate()?;
        let (rows, cols) = (self.rows as f64, self.cols as f64);
        let w = (100.0 - self.margin_left - self.margin_right - (cols - 1.0) * self.h_gap) / cols;
        let h = (100.0 - self.margin_top - self.margin_bottom - (rows - 1.0) * self.v_gap) / rows;
        if !(w >= 1.0 && h >= 1.0) {
            return Err(PanelError::NoRoom);
        }
        Ok((0..self.cells())
            .map(|k| {
                let (r, c) = self.cell(k);
                Frame {
                    left: (self.margin_left + c as f64 * (w + self.h_gap)) / 100.0,
                    top: (self.margin_top + r as f64 * (h + self.v_gap)) / 100.0,
                    width: w / 100.0,
                    height: h / 100.0,
                }
            })
            .collect())
    }

    /// Sets the label settings of panels that touch, for the first `count` panels of a graph whose
    /// panel k is layer k (spec 15.4, steps 5 and 6). In a row without a gap whose panels share X,
    /// a panel with another one right below it shows no X tick labels and no X title, and its X
    /// ticks point in; a panel with another one right above it leaves out its top Y label. Columns
    /// without a gap whose panels share Y work the same way across.
    fn join_labels(&self, g: &mut Graph) {
        let count = g.layers.len();
        let rows_joined = self.v_gap == 0.0 && self.link_x;
        let cols_joined = self.h_gap == 0.0 && self.link_y;
        for (k, layer) in g.layers.iter_mut().enumerate() {
            let (r, c) = self.cell(k);
            let below = r.checked_add(1).and_then(|r| self.panel_at(r, c, count)).is_some();
            let above = r.checked_sub(1).and_then(|r| self.panel_at(r, c, count)).is_some();
            let left = c.checked_sub(1).and_then(|c| self.panel_at(r, c, count)).is_some();
            let right = c.checked_add(1).and_then(|c| self.panel_at(r, c, count)).is_some();
            if rows_joined && below {
                let x = &mut layer.x_axis;
                x.show_tick_labels = false;
                x.show_title = false;
                x.major_tick = inward(x.major_tick);
                x.minor_tick = inward(x.minor_tick);
            }
            if rows_joined && above {
                layer.y_axis.hide_end_label = true;
            }
            if cols_joined && left {
                let y = &mut layer.y_axis;
                y.show_tick_labels = false;
                y.show_title = false;
                y.major_tick = inward(y.major_tick);
                y.minor_tick = inward(y.minor_tick);
            }
            if cols_joined && right {
                layer.x_axis.hide_end_label = true;
            }
        }
    }
}

/// A tick direction that does not reach out of the frame: `Out` and `Both` become `In`.
fn inward(d: TickDirection) -> TickDirection {
    match d {
        TickDirection::Out | TickDirection::Both => TickDirection::In,
        other => other,
    }
}

/// `f` (fractions of the page) as fractions of the frame `parent` (fractions of the page).
fn relative(f: Frame, parent: Frame) -> Frame {
    Frame {
        left: (f.left - parent.left) / parent.width,
        top: (f.top - parent.top) / parent.height,
        width: f.width / parent.width,
        height: f.height / parent.height,
    }
}

/// `f` (fractions of the frame `parent`) as fractions of the page.
fn absolute(f: Frame, parent: Frame) -> Frame {
    Frame {
        left: parent.left + f.left * parent.width,
        top: parent.top + f.top * parent.height,
        width: f.width * parent.width,
        height: f.height * parent.height,
    }
}

impl Graph {
    /// Places the panels of the graph ([`Graph::panels`]) in the cells of `layout`, in order
    /// (Origin's Arrange Layers). A panel linked to another layer gets its frame as fractions of
    /// that layer's frame, as linked layers store it; layers drawn in the frame of a panel (its
    /// right Y axis) move with it. Links, axes and the page are not changed.
    pub fn arrange(&mut self, layout: &PanelLayout) -> Result<(), PanelError> {
        let cells = layout.frames()?;
        let panels: Vec<Vec<usize>> = self.panels().into_iter().map(|p| self.same_frame(p)).collect();
        if panels.len() > cells.len() {
            return Err(PanelError::TooManyPanels { cells: cells.len(), panels: panels.len() });
        }
        self.place_panels(&panels, &cells);
        Ok(())
    }

    /// Puts panel k (the layers `panels[k]`, the panel itself first) into `cells[k]`, a frame in
    /// page fractions. A root gets the cell as its frame; a linked panel gets the cell as
    /// fractions of its root's new frame. The panels must come in layer order.
    fn place_panels(&mut self, panels: &[Vec<usize>], cells: &[Frame]) {
        // Page frames of the roots after the move (a root comes before the layers linked to it).
        let mut page_frames: Vec<Option<Frame>> = vec![None; self.layers.len()];
        for (layers, &cell) in panels.iter().zip(cells) {
            let Some(&panel) = layers.first() else { continue };
            match self.link_parent(panel) {
                None => {
                    self.layers[panel].frame = cell;
                    page_frames[panel] = Some(cell);
                }
                Some(root) => {
                    let parent = page_frames[root].unwrap_or(self.layers[root].frame);
                    let place = relative(cell, parent);
                    for &i in layers {
                        self.layers[i].frame = place;
                    }
                }
            }
        }
    }

    /// The frame of layer `index` as fractions of the page (a linked layer's frame is stored as
    /// fractions of its parent's frame).
    pub fn page_frame(&self, index: usize) -> Frame {
        match self.link_parent(index) {
            Some(root) => absolute(self.layers[index].frame, self.layers[root].frame),
            None => self.layers[index].frame,
        }
    }
}

/// Builds the layers of a panel graph: `count` copies of `base` (a template's layer without
/// plots), linked to the first one as the layout asks, placed in its cells, with the label
/// settings of panels that touch. The page is not changed here.
pub(crate) fn panel_layers(
    g: &mut Graph,
    base: &crate::graph::Layer,
    count: usize,
    layout: &PanelLayout,
) -> Result<(), PanelError> {
    use crate::graph::{LayerLink, ScaleLink};
    let cells = layout.frames()?;
    if count > cells.len() {
        return Err(PanelError::TooManyPanels { cells: cells.len(), panels: count });
    }
    let link = |on: bool| if on { ScaleLink::Straight } else { ScaleLink::None };
    g.layers = (0..count)
        .map(|k| {
            let mut l = base.clone();
            if k > 0 && (layout.link_x || layout.link_y) {
                l.link = Some(LayerLink { parent: 0, x: link(layout.link_x), y: link(layout.link_y) });
            }
            l
        })
        .collect();
    // The copies all have the same frame, so they are placed by index, not by `Graph::panels`.
    let panels: Vec<Vec<usize>> = (0..count).map(|k| vec![k]).collect();
    g.place_panels(&panels, &cells);
    layout.join_labels(g);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{AxisSide, Page, ScaleLink};
    use crate::ids::WindowId;
    use crate::template::{Template, right_y_layer};

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    fn assert_frame(f: &Frame, left: f64, top: f64, width: f64, height: f64) {
        let tol = 1e-12;
        assert!(
            close(f.left, left, tol)
                && close(f.top, top, tol)
                && close(f.width, width, tol)
                && close(f.height, height, tol),
            "{f:?} is not ({left}, {top}, {width}, {height})"
        );
    }

    #[test]
    fn a_stack_of_three_follows_origins_stack_command() {
        let frames = PanelLayout::stack(3).frames().unwrap();
        assert_eq!(frames.len(), 3);
        let h = 0.8 / 3.0;
        assert_frame(&frames[0], 0.15, 0.10 + 2.0 * h, 0.75, h);
        assert_frame(&frames[1], 0.15, 0.10 + h, 0.75, h);
        assert_frame(&frames[2], 0.15, 0.10, 0.75, h);
        // Origin's Stack gave (14.99, 63.33 / 36.67 / 9.99, 75, 26.68) percent.
        for (f, top) in frames.iter().zip([0.6333, 0.3667, 0.0999]) {
            assert!(close(f.top, top, 0.0002) && close(f.left, 0.1499, 0.0002) && close(f.height, 0.2668, 0.0002));
        }
    }

    #[test]
    fn a_stack_of_two_with_a_gap_of_5() {
        let layout = PanelLayout { v_gap: 5.0, ..PanelLayout::stack(2) };
        let frames = layout.frames().unwrap();
        // Origin: height 37.5, tops 52.51 and 9.99.
        assert_frame(&frames[0], 0.15, 0.525, 0.75, 0.375);
        assert_frame(&frames[1], 0.15, 0.10, 0.75, 0.375);
    }

    #[test]
    fn arrange_layers_defaults_match_origin() {
        let frames = PanelLayout::arrange(4, 1).frames().unwrap();
        // Origin: (15, 14.99 / 33.76 / 52.51 / 71.26, 69.99, 13.75).
        for (f, top) in frames.iter().zip([0.15, 0.3375, 0.525, 0.7125]) {
            assert_frame(f, 0.15, top, 0.70, 0.1375);
        }
        // Merge Graph's 2016 defaults for four graphs (gaps 5, margins 15/10/10/15) give a 2 x 2 grid
        // at (15, 10), (55, 10), (15, 50) and (55, 50), each 35 x 35.
        let merge = PanelLayout { margin_right: 10.0, margin_top: 10.0, ..PanelLayout::arrange(2, 2) };
        let frames = merge.frames().unwrap();
        for (f, (left, top)) in frames.iter().zip([(0.15, 0.10), (0.55, 0.10), (0.15, 0.50), (0.55, 0.50)]) {
            assert_frame(f, left, top, 0.35, 0.35);
        }
        // Five graphs: 3 rows x 2 columns, height 21.67, tops 10, 36.67 and 63.33.
        let merge = PanelLayout { rows: 3, ..merge };
        let frames = merge.frames().unwrap();
        let h = 0.65 / 3.0;
        assert_frame(&frames[4], 0.15, 0.10 + 2.0 * (h + 0.05), 0.35, h);
        assert_frame(&frames[1], 0.55, 0.10, 0.35, h);
    }

    #[test]
    fn a_grid_gives_every_panel_the_room_of_a_single_graph() {
        let line = Template::Line.new_graph(WindowId(1), "G");
        let frame = line.layers[0].frame;
        let layout = PanelLayout::grid(2, 2, &frame);
        assert_eq!(
            (layout.order, layout.page, layout.link_x, layout.link_y),
            (PanelOrder::TopDown, PanelPage::Tiled, false, false)
        );
        let frames = layout.frames().unwrap();
        // Each cell is half the page; the panel sits where the template puts its layer.
        for (f, (cx, cy)) in frames.iter().zip([(0.0, 0.0), (0.5, 0.0), (0.0, 0.5), (0.5, 0.5)]) {
            assert_frame(f, cx + frame.left / 2.0, cy + frame.top / 2.0, frame.width / 2.0, frame.height / 2.0);
        }
        // One column fills from the bottom, as Origin's Vertical 2 Panel.
        let vertical = PanelLayout::grid(2, 1, &frame);
        assert_eq!(vertical.order, PanelOrder::BottomUp);
        let frames = vertical.frames().unwrap();
        assert!(frames[0].top > frames[1].top);
        assert_frame(&frames[1], frame.left, frame.top / 2.0, frame.width, frame.height / 2.0);
    }

    #[test]
    fn layouts_that_do_not_fit_are_refused() {
        let wide = PanelLayout { margin_left: 50.0, margin_right: 50.0, ..PanelLayout::stack(2) };
        assert_eq!(wide.frames(), Err(PanelError::NoRoom));
        assert_eq!(PanelError::NoRoom.to_string(), "the margins and gaps leave no room for the panels");
        assert_eq!(PanelLayout { v_gap: 60.0, ..PanelLayout::stack(2) }.frames(), Err(PanelError::Gap));
        assert_eq!(PanelLayout { h_gap: f64::NAN, ..PanelLayout::stack(2) }.frames(), Err(PanelError::Gap));
        assert_eq!(PanelLayout { margin_top: -1.0, ..PanelLayout::stack(2) }.frames(), Err(PanelError::Margin));
        assert_eq!(PanelLayout::stack(65).frames(), Err(PanelError::GridSize { rows: 65, cols: 1 }));
        assert_eq!(PanelLayout::arrange(0, 2).frames(), Err(PanelError::GridSize { rows: 0, cols: 2 }));
        assert_eq!(
            PanelLayout::arrange(usize::MAX, 2).validate(),
            Err(PanelError::GridSize { rows: usize::MAX, cols: 2 })
        );
        assert!(PanelLayout::stack(64).frames().is_ok());
        // Many thin panels: each one must be at least 1 percent high.
        assert_eq!(PanelLayout { v_gap: 2.0, ..PanelLayout::stack(40) }.frames(), Err(PanelError::NoRoom));
    }

    fn graph_with(layers: usize) -> Graph {
        let mut g = Template::Line.new_graph(WindowId(1), "G");
        let base = g.layers[0].clone();
        g.layers = vec![base; layers];
        g
    }

    #[test]
    fn linked_panels_store_their_frames_as_fractions_of_the_first_panel_and_can_be_arranged_again() {
        let base = graph_with(1).layers[0].clone();
        let mut g = graph_with(1);
        panel_layers(&mut g, &base, 3, &PanelLayout::stack(3)).unwrap();
        let h = 0.8 / 3.0;
        assert_frame(&g.layers[0].frame, 0.15, 0.10 + 2.0 * h, 0.75, h);
        // Stored as fractions of layer 0's frame: one and two panel heights higher.
        assert_frame(&g.layers[1].frame, 0.0, -1.0, 1.0, 1.0);
        assert_frame(&g.layers[2].frame, 0.0, -2.0, 1.0, 1.0);
        assert_frame(&g.page_frame(2), 0.15, 0.10, 0.75, h);
        assert_eq!(g.panels(), vec![0, 1, 2]);
        // Arranged again with gaps of 5 and a top margin of 5: height 25, tops 65, 35 and 5 percent.
        g.arrange(&PanelLayout { v_gap: 5.0, margin_top: 5.0, ..PanelLayout::stack(3) }).unwrap();
        assert_frame(&g.layers[0].frame, 0.15, 0.65, 0.75, 0.25);
        assert_frame(&g.page_frame(1), 0.15, 0.35, 0.75, 0.25);
        assert_frame(&g.page_frame(2), 0.15, 0.05, 0.75, 0.25);
        assert_frame(&g.layers[2].frame, 0.0, -2.4, 1.0, 1.0);
        assert_eq!(g.arrange(&PanelLayout::stack(2)), Err(PanelError::TooManyPanels { cells: 2, panels: 3 }));
        assert_eq!(
            panel_layers(&mut g, &base, 3, &PanelLayout::stack(2)),
            Err(PanelError::TooManyPanels { cells: 2, panels: 3 })
        );
    }

    #[test]
    fn a_right_y_layer_moves_with_its_panel_and_is_not_a_panel_of_its_own() {
        let mut g = graph_with(1);
        let right = right_y_layer(&g.layers[0], 0);
        g.layers.push(right);
        let mut second = g.layers[0].clone();
        second.link = Some(crate::graph::LayerLink { parent: 0, x: ScaleLink::Straight, y: ScaleLink::None });
        g.layers.push(second);
        assert_eq!(g.panels(), vec![0, 2]);
        assert_eq!(g.same_frame(0), vec![0, 1]);
        assert_eq!(g.right_y_member(0), Some(1));
        g.arrange(&PanelLayout::stack(2)).unwrap();
        assert_eq!(g.layers[1].frame, Frame::WHOLE, "the right Y layer still covers layer 0");
        assert_frame(&g.layers[2].frame, 0.0, -1.0, 1.0, 1.0);
        // The second panel has no right Y axis of its own.
        assert_eq!(g.right_y_member(2), None);
        assert_eq!(g.layers[1].y_axis.side, AxisSide::Opposite);
    }

    #[test]
    fn panels_that_touch_hide_the_labels_at_the_joint() {
        let base = graph_with(1).layers[0].clone();
        let mut g = graph_with(1);
        panel_layers(&mut g, &base, 3, &PanelLayout::stack(3)).unwrap();
        // Panel 0 is at the bottom: it keeps its X labels and title.
        let (x0, x1, x2) = (&g.layers[0].x_axis, &g.layers[1].x_axis, &g.layers[2].x_axis);
        assert!(x0.show_tick_labels && x0.show_title);
        assert_eq!(x0.major_tick, TickDirection::Out);
        for x in [x1, x2] {
            assert!(!x.show_tick_labels && !x.show_title);
            assert_eq!((x.major_tick, x.minor_tick), (TickDirection::In, TickDirection::In));
        }
        let hidden: Vec<bool> = g.layers.iter().map(|l| l.y_axis.hide_end_label).collect();
        assert_eq!(hidden, vec![true, true, false], "the top panel keeps its top label");
        assert!(g.layers.iter().all(|l| !l.x_axis.hide_end_label && l.y_axis.show_tick_labels));
        // With a gap, every panel shows everything, as Origin does from a gap of 1.
        let mut g = graph_with(1);
        panel_layers(&mut g, &base, 3, &PanelLayout { v_gap: 1.0, ..PanelLayout::stack(3) }).unwrap();
        assert!(g.layers.iter().all(|l| l.x_axis.show_tick_labels && l.x_axis.show_title && !l.y_axis.hide_end_label));
        // Top down: panel 0 is at the top and the last panel shows the X axis.
        let mut g = graph_with(1);
        let layout = PanelLayout { order: PanelOrder::TopDown, ..PanelLayout::stack(2) };
        panel_layers(&mut g, &base, 2, &layout).unwrap();
        assert!(!g.layers[0].x_axis.show_tick_labels && g.layers[1].x_axis.show_tick_labels);
        assert!(!g.layers[0].y_axis.hide_end_label && g.layers[1].y_axis.hide_end_label);
        assert_frame(&g.layers[1].frame, 0.0, 1.0, 1.0, 1.0);
    }

    #[test]
    fn columns_without_a_gap_that_share_y_hide_the_inner_y_labels() {
        let base = graph_with(1).layers[0].clone();
        let mut g = graph_with(1);
        let layout = PanelLayout { h_gap: 0.0, link_y: true, ..PanelLayout::arrange(1, 3) };
        panel_layers(&mut g, &base, 3, &layout).unwrap();
        let shown: Vec<bool> = g.layers.iter().map(|l| l.y_axis.show_tick_labels && l.y_axis.show_title).collect();
        assert_eq!(shown, vec![true, false, false]);
        let hidden: Vec<bool> = g.layers.iter().map(|l| l.x_axis.hide_end_label).collect();
        assert_eq!(hidden, vec![true, true, false]);
        assert!(g.layers.iter().all(|l| l.x_axis.show_tick_labels && !l.y_axis.hide_end_label));
        let link = g.layers[2].link.unwrap();
        assert_eq!((link.parent, link.x, link.y), (0, ScaleLink::None, ScaleLink::Straight));
        // Without the Y link nothing is hidden.
        let mut g = graph_with(1);
        panel_layers(&mut g, &base, 3, &PanelLayout { h_gap: 0.0, ..PanelLayout::arrange(1, 3) }).unwrap();
        assert!(g.layers.iter().all(|l| l.y_axis.show_tick_labels && l.link.is_none()));
    }

    #[test]
    fn page_frames_of_a_graph_without_links_are_the_frames() {
        let mut g = graph_with(2);
        g.page = Page::inches(6.4, 4.8);
        g.arrange(&PanelLayout::arrange(1, 2)).unwrap();
        assert_eq!(g.page_frame(1), g.layers[1].frame);
        assert_eq!(g.panels(), vec![0, 1]);
    }
}
