//! Merge Graph Windows (Origin's `merge_graph`, graphing spec section 15.11): finished graphs are
//! copied, layer by layer, into the cells of one new graph. The sources keep their plots, axes,
//! labels, legends, shapes and styles; only the frames are moved and, as Origin does, the text may be
//! scaled down to fit smaller cells.
//!
//! Measured in Origin 2016 (2026-11-15, `re-workspace/scratch/t60`): the page is 11.41 x 7.973 in,
//! landscape; cells follow the Arrange Layers formula with gaps 5 and margins 15 (left), 10 (right
//! and top) and 15 (bottom); rows and columns default to `ceil(sqrt(n))` and `ceil(n / rows)`; the
//! text factor (see [`auto_text_factor`]); a multi-layer source keeps its links; "Link Source
//! Graphs" links the layers to the first one in position only; "Show Axes Frame" hides the X tick
//! labels and ticks of a panel that touches the one below it.

use crate::graph::{Frame, Graph, LayerLink, Page, ScaleLink, TickDirection};
use crate::ids::WindowId;
use crate::panels::{PanelError, PanelLayout, PanelOrder, PanelPage};
use crate::project::{Project, Window};

/// Origin's merged page: 11.41 x 7.973 in (6846 x 4784 dots at 600 dpi), landscape.
pub const MERGE_PAGE_IN: (f64, f64) = (11.41, 7.973);

/// How the text of the merged layers is sized.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MergeTextScale {
    /// Origin's "Auto" (and "Scale with Layer Frame", which gave the same factors in every measured
    /// case): see [`auto_text_factor`].
    Auto,
    /// Origin's "Fixed Factor": every merged layer's text is multiplied by this factor.
    Fixed(f64),
    /// Keep every point size (a Sheaf choice; also what Origin's factor 1 gives).
    Keep,
}

/// The options of Merge Graph Windows. Lengths are percent of the page, as in [`PanelLayout`].
#[derive(Clone, Debug, PartialEq)]
pub struct MergeOptions {
    /// The graphs to merge, in the order they fill the cells (row by row from the top left).
    pub sources: Vec<WindowId>,
    /// Rows and columns of the grid; `None` lets them follow the number of graphs (Origin's
    /// automatic choice, see [`auto_grid`]).
    pub rows: Option<usize>,
    pub cols: Option<usize>,
    pub h_gap: f64,
    pub v_gap: f64,
    pub margin_left: f64,
    pub margin_right: f64,
    pub margin_top: f64,
    pub margin_bottom: f64,
    /// Keep the source graphs (Origin's default). When false they are deleted after the merge.
    pub keep_sources: bool,
    /// A portrait page: width and height of [`MergeOptions::page_in`] swapped.
    pub portrait: bool,
    /// The page size in inches, landscape (the longer side first); `None` is Origin's 11.41 x 7.973.
    pub page_in: Option<(f64, f64)>,
    /// "Keep Layer Aspect Ratio": a source keeps the width-to-height ratio of its layers (in percent
    /// of its page) and is drawn at the top left of its cell, as large as fits.
    pub keep_aspect: bool,
    /// "Link Source Graphs": the layers of the second and later sources follow the first merged
    /// layer in position (Origin's "% of linked layer"); their scales stay their own.
    pub link: bool,
    /// Sheaf's addition to `link`: the linked layers also share the first layer's X scale (one to
    /// one), so a vertical stack has one X axis. Implies `link`.
    pub share_x: bool,
    /// "Show Axes Frame": a panel that touches the panel below it (no vertical gap) shows no X tick
    /// labels, title or ticks, and one that touches the panel on its right no Y ones on that side.
    pub hide_inner: bool,
    pub text_scale: MergeTextScale,
}

impl Default for MergeOptions {
    /// Origin 2016's defaults (merge_graph's definition file and measured runs).
    fn default() -> Self {
        MergeOptions {
            sources: Vec::new(),
            rows: None,
            cols: None,
            h_gap: 5.0,
            v_gap: 5.0,
            margin_left: 15.0,
            margin_right: 10.0,
            margin_top: 10.0,
            margin_bottom: 15.0,
            keep_sources: true,
            portrait: false,
            page_in: None,
            keep_aspect: false,
            link: false,
            share_x: false,
            hide_inner: false,
            text_scale: MergeTextScale::Auto,
        }
    }
}

/// Why graphs cannot be merged.
#[derive(Debug, thiserror::Error, Clone, PartialEq)]
pub enum MergeError {
    #[error("Merge Graph Windows needs at least two graphs")]
    TooFew,
    #[error("window {0:?} is not a graph of this project")]
    NotAGraph(WindowId),
    #[error("the graph '{0}' is listed more than once")]
    Duplicate(String),
    #[error("the graph '{0}' has no layers")]
    Empty(String),
    #[error("{rows} x {cols} cells cannot hold {graphs} graphs")]
    TooManyGraphs { rows: usize, cols: usize, graphs: usize },
    #[error("the text factor must be a number from 0.05 to 20")]
    Factor,
    #[error("the page must be from 0.5 to 200 inches on each side")]
    Page,
    #[error(transparent)]
    Layout(#[from] PanelError),
}

/// Origin's automatic grid for `n` graphs: `rows = ceil(sqrt(n))`, `cols = ceil(n / rows)` (2 gave
/// 2 x 1, 3 and 4 gave 2 x 2, 5 gave 3 x 2). One fixed side gives the other as the fewest that hold
/// `n` graphs.
pub fn auto_grid(n: usize, rows: Option<usize>, cols: Option<usize>) -> (usize, usize) {
    let n = n.max(1);
    match (rows, cols) {
        (Some(r), Some(c)) => (r, c),
        (Some(r), None) => (r, n.div_ceil(r.max(1))),
        (None, Some(c)) => (n.div_ceil(c.max(1)), c),
        (None, None) => {
            let mut r = (n as f64).sqrt().ceil() as usize;
            while r > 1 && (r - 1) * (r - 1) >= n {
                r -= 1;
            }
            while r * r < n {
                r += 1;
            }
            (r, n.div_ceil(r))
        }
    }
}

/// The constants of Origin's automatic text factor, in percent of the page: the stock Line
/// template's layer (68.16 x 71.83) and the size the factor is measured against (fitted to the
/// measured merges, between 75.4 and 75.9).
const TEXT_FACTOR_FULL: (f64, f64) = (68.16, 71.83);
const TEXT_FACTOR_SIZE: f64 = 75.6;

/// Origin's automatic text factor for a cell of `cell` (width, height in percent of the merged
/// page): 1 when the cell is at least as wide or as tall as the stock Line template's layer (68.16 x
/// 71.83 percent), otherwise `sqrt(w h) / 75.6` rounded to two decimals. Measured in Origin 2016 on
/// 25 merges: 2 x 2 0.46, 3 x 2 and 2 x 3 0.36, 3 x 3 0.29, 4 x 4 0.20, a 65 x 21.68 cell 0.50 but
/// a 70 x 21.68 one 1, a 21.66 x 70 cell 0.52 but a 21.66 x 74 one 1. Neither the page size nor
/// the size of the source's own layer changes it (sources of 30 x 30, 50 x 50 and 80 x 40 percent
/// gave the same factors).
pub fn auto_text_factor(cell: (f64, f64)) -> f64 {
    let (w, h) = cell;
    if w >= TEXT_FACTOR_FULL.0 || h >= TEXT_FACTOR_FULL.1 || !(w > 0.0 && h > 0.0) {
        return 1.0;
    }
    let f = ((w * h).sqrt() / TEXT_FACTOR_SIZE).min(1.0);
    ((f * 100.0).round() / 100.0).max(0.01)
}

impl MergeOptions {
    /// The layout of the cells for `n` graphs.
    pub fn layout(&self, n: usize) -> PanelLayout {
        let (rows, cols) = auto_grid(n, self.rows, self.cols);
        PanelLayout {
            rows,
            cols,
            h_gap: self.h_gap,
            v_gap: self.v_gap,
            margin_left: self.margin_left,
            margin_right: self.margin_right,
            margin_top: self.margin_top,
            margin_bottom: self.margin_bottom,
            order: PanelOrder::TopDown,
            link_x: false,
            link_y: false,
            page: PanelPage::Keep,
        }
    }

    /// The merged page size in points.
    fn page_pt(&self) -> Result<(f64, f64), MergeError> {
        let (a, b) = self.page_in.unwrap_or(MERGE_PAGE_IN);
        if !(0.5..=200.0).contains(&a) || !(0.5..=200.0).contains(&b) {
            return Err(MergeError::Page);
        }
        let (w, h) = if self.portrait { (a.min(b), a.max(b)) } else { (a.max(b), a.min(b)) };
        Ok((w * 72.0, h * 72.0))
    }
}

/// The smallest frame holding every frame of `frames`.
fn bounds(frames: &[Frame]) -> Frame {
    let left = frames.iter().map(|f| f.left).fold(f64::INFINITY, f64::min);
    let top = frames.iter().map(|f| f.top).fold(f64::INFINITY, f64::min);
    let right = frames.iter().map(|f| f.left + f.width).fold(f64::NEG_INFINITY, f64::max);
    let bottom = frames.iter().map(|f| f.top + f.height).fold(f64::NEG_INFINITY, f64::max);
    Frame { left, top, width: right - left, height: bottom - top }
}

/// Multiplies every font size of a layer (axis titles and tick labels, the legend and the text
/// labels) by `factor`: the text factor of a merged layer.
fn scale_text(layer: &mut crate::graph::Layer, factor: f64) {
    if (factor - 1.0).abs() < 1e-12 {
        return;
    }
    for axis in [&mut layer.x_axis, &mut layer.y_axis] {
        axis.title_font.size_pt *= factor;
        axis.tick_font.size_pt *= factor;
    }
    layer.legend.font.size_pt *= factor;
    for label in &mut layer.labels {
        label.font.size_pt *= factor;
    }
}

/// Builds the merged graph (id and name not yet set) from the source graphs, in cell order.
pub fn merge_graphs(sources: &[&Graph], opts: &MergeOptions) -> Result<Graph, MergeError> {
    if sources.len() < 2 {
        return Err(MergeError::TooFew);
    }
    if let MergeTextScale::Fixed(f) = opts.text_scale
        && !(0.05..=20.0).contains(&f)
    {
        return Err(MergeError::Factor);
    }
    let layout = opts.layout(sources.len());
    let cells = layout.frames()?;
    if sources.len() > cells.len() {
        return Err(MergeError::TooManyGraphs { rows: layout.rows, cols: layout.cols, graphs: sources.len() });
    }
    let (page_w, page_h) = opts.page_pt()?;
    let first = sources[0];
    let mut g = Graph {
        id: WindowId(0),
        name: String::new(),
        long_name: String::new(),
        page: Page { width_pt: page_w, height_pt: page_h, ..first.page.clone() },
        layers: Vec::new(),
    };
    let link = opts.link || opts.share_x;
    for (k, (src, &cell)) in sources.iter().zip(&cells).enumerate() {
        if src.layers.is_empty() {
            return Err(MergeError::Empty(src.name.clone()));
        }
        let base = g.layers.len();
        // Layers standing on the page (roots, and layers whose link is not valid) are placed by
        // their part of the source's bounding box; a validly linked layer keeps its frame, which is
        // fractions of its parent's.
        let roots: Vec<usize> = (0..src.layers.len()).filter(|&i| src.link_parent(i).is_none()).collect();
        let bbox = bounds(&roots.iter().map(|&i| src.layers[i].frame).collect::<Vec<_>>());
        let target = if opts.keep_aspect && bbox.width > 0.0 && bbox.height > 0.0 {
            // the ratio is kept in percent of each page, as Origin does
            let ratio = bbox.width / bbox.height;
            let width = cell.width.min(cell.height * ratio);
            Frame { left: cell.left, top: cell.top, width, height: width / ratio }
        } else {
            cell
        };
        let factor = match opts.text_scale {
            MergeTextScale::Auto => auto_text_factor((100.0 * target.width, 100.0 * target.height)),
            MergeTextScale::Fixed(f) => f,
            MergeTextScale::Keep => 1.0,
        };
        for (i, layer) in src.layers.iter().enumerate() {
            let mut l = layer.clone();
            match src.link_parent(i) {
                Some(p) => {
                    if let Some(lk) = &mut l.link {
                        lk.parent = base + p;
                    }
                }
                None => {
                    l.link = None;
                    l.frame = if bbox.width > 0.0 && bbox.height > 0.0 {
                        layer.frame.relative_to(bbox).within(target)
                    } else {
                        target
                    };
                }
            }
            scale_text(&mut l, factor);
            g.layers.push(l);
        }
        // Link Source Graphs: the roots of the later sources follow the first merged layer. A root
        // that other layers follow (the left layer of a double-Y source) stays on the page, since
        // a linked layer cannot have layers linked to it.
        if link && k > 0 {
            for i in base..g.layers.len() {
                if g.layers[i].link.is_none() && !g.has_children(i) {
                    let frame = g.layers[i].frame.relative_to(g.layers[0].frame);
                    let x = if opts.share_x { ScaleLink::Straight } else { ScaleLink::None };
                    g.layers[i].link = Some(LayerLink { parent: 0, x, y: ScaleLink::None });
                    g.layers[i].frame = frame;
                }
            }
        }
    }
    if opts.hide_inner {
        hide_inner_axes(&mut g, &layout, sources);
    }
    Ok(g)
}

/// Origin's "Show Axes Frame": in a grid without a vertical gap, the layers of a cell with a filled
/// cell right below it get no X tick labels, title or ticks; without a horizontal gap, a cell with a
/// filled cell on its left gets no Y ones (Origin was measured for the rows only).
fn hide_inner_axes(g: &mut Graph, layout: &PanelLayout, sources: &[&Graph]) {
    let n = sources.len();
    let filled = |r: usize, c: usize| r < layout.rows && c < layout.cols && r * layout.cols + c < n;
    let mut start = 0;
    for (k, src) in sources.iter().enumerate() {
        let (r, c) = layout.cell(k);
        let below = layout.v_gap == 0.0 && filled(r + 1, c);
        let left = layout.h_gap == 0.0 && c > 0 && filled(r, c - 1);
        for layer in &mut g.layers[start..start + src.layers.len()] {
            if below {
                let x = &mut layer.x_axis;
                x.show_tick_labels = false;
                x.show_title = false;
                x.major_tick = TickDirection::None;
                x.minor_tick = TickDirection::None;
            }
            if left {
                let y = &mut layer.y_axis;
                y.show_tick_labels = false;
                y.show_title = false;
                y.major_tick = TickDirection::None;
                y.minor_tick = TickDirection::None;
            }
        }
        start += src.layers.len();
    }
}

impl Project {
    /// Graph > Merge Graph Windows: a new graph made of copies of the graphs `opts.sources`, one per
    /// cell. The copies plot the same columns. The sources are deleted when `opts.keep_sources` is
    /// false. Returns the new graph's id.
    pub fn merge_graphs(&mut self, opts: &MergeOptions) -> Result<WindowId, MergeError> {
        let mut seen = std::collections::HashSet::new();
        for &id in &opts.sources {
            let g = self.graph(id).ok_or(MergeError::NotAGraph(id))?;
            if !seen.insert(id) {
                return Err(MergeError::Duplicate(g.name.clone()));
            }
        }
        let sources: Vec<&Graph> = opts.sources.iter().filter_map(|&id| self.graph(id)).collect();
        let mut g = merge_graphs(&sources, opts)?;
        let id = WindowId(self.fresh_id());
        g.id = id;
        g.name = self.unique_name("Graph");
        self.windows.push(Window::Graph(g));
        if !opts.keep_sources {
            for &src in &opts.sources {
                self.remove_window(src);
            }
        }
        Ok(id)
    }
}

#[cfg(test)]
mod tests;
