//! Graph > Layer Management (T60): arranging, sizing, moving, swapping, aligning and linking the
//! layers of one graph, as Origin 2016's Layer Management window does on its Arrange, Size/Position
//! and Link tabs. The logic is here; the dialog only collects the numbers.
//!
//! Frames are fractions of the page for a layer standing on the page and fractions of the parent's
//! frame for a validly linked layer ([`Graph::link_parent`]). Every operation therefore works on page
//! frames ([`Graph::page_frame`]) and writes them back in the layer's own unit. An operation either
//! succeeds as a whole or leaves the graph as it was.

use crate::graph::{Frame, Graph, ScaleLink, TickDirection};
use crate::panels::{PanelError, PanelLayout, PanelOrder, PanelPage};

/// The smallest and largest size of a layer, as fractions of the page.
const SIZE_RANGE: (f64, f64) = (0.001, 10.0);
/// How far a layer may lie from the top left corner, as fractions of the page.
const OFFSET_RANGE: (f64, f64) = (-10.0, 10.0);

/// Why a Layer Management operation was refused.
#[derive(Debug, thiserror::Error, Clone, PartialEq)]
pub enum LayerError {
    #[error(transparent)]
    Panel(#[from] PanelError),
    #[error("the graph has no layers")]
    NoLayers,
    #[error("select at least one layer")]
    NoSelection,
    #[error("there is no layer {}", .0 + 1)]
    NoSuchLayer(usize),
    #[error("{layers} layer(s) do not fit the {cells} cell(s) of the grid")]
    TooManyLayers { cells: usize, layers: usize },
    #[error("layer {0} would get a size or position outside the page limits")]
    Bounds(usize),
    #[error("a number is not valid")]
    Number,
    #[error("choose two different layers")]
    SameLayer,
    #[error("layer {0} cannot be a parent: it is linked to another layer itself")]
    ParentLinked(usize),
    #[error(
        "layer {layer} cannot be linked to layer {parent}: a layer follows an earlier layer that is not linked itself, and a layer that others follow cannot be linked"
    )]
    Link { layer: usize, parent: usize },
    #[error("swapping the order would put a parent layer after a layer linked to it")]
    ReorderLinks,
}

/// In which order Arrange Selected Layers fills the cells.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum ArrangeOrder {
    /// By layer number.
    #[default]
    LayerIndex,
    /// By position on the page: top to bottom, then left to right.
    CurrentPosition,
}

/// Layer Management > Arrange tab.
#[derive(Clone, Debug, PartialEq)]
pub struct ArrangeOptions {
    pub order: ArrangeOrder,
    pub rows: usize,
    pub cols: usize,
    /// Keep the width to height ratio of each layer (inside its cell, at the cell's top left).
    pub keep_aspect: bool,
    /// A layer and the layers linked to it count as one unit.
    pub group_linked: bool,
    /// Link the other units to the first one, for position only (no scale is shared).
    pub link: bool,
    /// "Show Axes Frame": hide the X tick labels, title and ticks of a unit with another one right
    /// below it, and the Y ones of a unit with another one on its left, when there is no gap.
    pub hide_inner: bool,
    /// Gaps and margins in percent of the page.
    pub h_gap: f64,
    pub v_gap: f64,
    pub margin_left: f64,
    pub margin_right: f64,
    pub margin_top: f64,
    pub margin_bottom: f64,
}

impl Default for ArrangeOptions {
    /// The defaults of the Layer Management window (the layarrange X-Function uses 15 on every
    /// margin; [`PanelLayout::arrange`] keeps those).
    fn default() -> Self {
        ArrangeOptions {
            order: ArrangeOrder::LayerIndex,
            rows: 1,
            cols: 1,
            keep_aspect: false,
            group_linked: false,
            link: false,
            hide_inner: false,
            h_gap: 5.0,
            v_gap: 5.0,
            margin_left: 15.0,
            margin_right: 10.0,
            margin_top: 10.0,
            margin_bottom: 15.0,
        }
    }
}

impl ArrangeOptions {
    /// The grid these options describe.
    pub fn layout(&self) -> PanelLayout {
        PanelLayout {
            rows: self.rows,
            cols: self.cols,
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
}

/// The unit a length of the Size/Position tab is given in.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum SizeUnit {
    PagePercent,
    Inch,
    Cm,
    Mm,
    Pixel,
    Point,
    /// Percent of the reference layer's width (or height); for a position, measured from the
    /// reference layer's left (top) edge.
    #[default]
    ReferencePercent,
}

impl SizeUnit {
    pub const ALL: [SizeUnit; 7] = [
        SizeUnit::PagePercent,
        SizeUnit::Inch,
        SizeUnit::Cm,
        SizeUnit::Mm,
        SizeUnit::Pixel,
        SizeUnit::Point,
        SizeUnit::ReferencePercent,
    ];

    pub fn name(self) -> &'static str {
        match self {
            SizeUnit::PagePercent => "% of Page",
            SizeUnit::Inch => "inch",
            SizeUnit::Cm => "cm",
            SizeUnit::Mm => "mm",
            SizeUnit::Pixel => "pixel",
            SizeUnit::Point => "point",
            SizeUnit::ReferencePercent => "% of Reference Layer",
        }
    }

    /// Points per unit for the absolute units (pixels at 96 per inch).
    fn points(self) -> Option<f64> {
        match self {
            SizeUnit::Inch => Some(72.0),
            SizeUnit::Cm => Some(72.0 / 2.54),
            SizeUnit::Mm => Some(72.0 / 25.4),
            SizeUnit::Pixel => Some(0.75),
            SizeUnit::Point => Some(1.0),
            SizeUnit::PagePercent | SizeUnit::ReferencePercent => None,
        }
    }

    /// `value` as a fraction of a page side of `page_pt` points; `reference` is the reference
    /// layer's size on that side as a fraction of the page.
    fn fraction(self, value: f64, page_pt: f64, reference: f64) -> f64 {
        match self {
            SizeUnit::PagePercent => value / 100.0,
            SizeUnit::ReferencePercent => value / 100.0 * reference,
            _ => value * self.points().unwrap_or(1.0) / page_pt,
        }
    }
}

/// The edge Align moves to the reference layer's.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum AlignEdge {
    #[default]
    Left,
    Top,
    Right,
    Bottom,
}

impl AlignEdge {
    pub const ALL: [AlignEdge; 4] = [AlignEdge::Left, AlignEdge::Top, AlignEdge::Right, AlignEdge::Bottom];

    pub fn name(self) -> &'static str {
        match self {
            AlignEdge::Left => "Left",
            AlignEdge::Top => "Top",
            AlignEdge::Right => "Right",
            AlignEdge::Bottom => "Bottom",
        }
    }
}

impl Graph {
    /// The sorted, distinct layers of `selection`, all of which must exist.
    fn checked_selection(&self, selection: &[usize]) -> Result<Vec<usize>, LayerError> {
        let mut sel = selection.to_vec();
        sel.sort_unstable();
        sel.dedup();
        match sel.iter().find(|&&i| i >= self.layers.len()) {
            Some(&i) => Err(LayerError::NoSuchLayer(i)),
            None if sel.is_empty() => Err(LayerError::NoSelection),
            None => Ok(sel),
        }
    }

    /// Runs `change` and puts the layers back as they were when it fails.
    fn atomic<T>(&mut self, change: impl FnOnce(&mut Graph) -> Result<T, LayerError>) -> Result<T, LayerError> {
        let backup = self.layers.clone();
        let result = change(self);
        if result.is_err() {
            self.layers = backup;
        }
        result
    }

    /// Gives layers new page frames (`(layer, frame)`, in page fractions) and stores each in its
    /// layer's own unit. A layer that follows another one gets its frame as fractions of the parent's
    /// frame as it is after the move, so the targets are written parents first. Layers that are not
    /// listed keep their stored frame: a linked layer moves and grows with its parent.
    fn write_page_frames(&mut self, targets: &[(usize, Frame)]) -> Result<(), LayerError> {
        let mut targets = targets.to_vec();
        targets.sort_by_key(|&(i, _)| i);
        for &(i, f) in &targets {
            let values = [f.left, f.top, f.width, f.height];
            if !values.iter().all(|v| v.is_finite()) {
                return Err(LayerError::Number);
            }
            let sizes = SIZE_RANGE.0..=SIZE_RANGE.1;
            let offsets = OFFSET_RANGE.0..=OFFSET_RANGE.1;
            let size_ok = sizes.contains(&f.width) && sizes.contains(&f.height);
            let at_ok = offsets.contains(&f.left) && offsets.contains(&f.top);
            if !(size_ok && at_ok) {
                return Err(LayerError::Bounds(i + 1));
            }
        }
        for (i, f) in targets {
            self.layers[i].frame = match self.link_parent(i) {
                Some(p) => f.relative_to(self.page_frame(p)),
                None => f,
            };
        }
        Ok(())
    }

    /// The page frame of every layer.
    fn page_frames(&self) -> Vec<Frame> {
        (0..self.layers.len()).map(|i| self.page_frame(i)).collect()
    }

    /// Layer Management > Arrange > Apply: places the selected layers (all when `selection` is empty)
    /// in the cells of a grid, in layer order or in order of their position on the page. With
    /// `group_linked` a layer and the layers linked to it are one unit that takes one cell (its root
    /// gets the cell and the linked layers follow); without it every layer is a unit, and a linked
    /// layer gets the cell as fractions of its parent's frame. Surplus cells stay empty; more units
    /// than cells are refused.
    pub fn arrange_layers(&mut self, selection: &[usize], opts: &ArrangeOptions) -> Result<(), LayerError> {
        if self.layers.is_empty() {
            return Err(LayerError::NoLayers);
        }
        let layout = opts.layout();
        let cells = layout.frames()?;
        let sel =
            if selection.is_empty() { (0..self.layers.len()).collect() } else { self.checked_selection(selection)? };
        let mut units: Vec<Vec<usize>> = Vec::new();
        if opts.group_linked {
            let mut roots = Vec::new();
            for &i in &sel {
                let root = self.root_of(i);
                if !roots.contains(&root) {
                    roots.push(root);
                    units.push(self.group(root));
                }
            }
        } else {
            units = sel.iter().map(|&i| vec![i]).collect();
        }
        if units.len() > cells.len() {
            return Err(LayerError::TooManyLayers { cells: cells.len(), layers: units.len() });
        }
        let now = self.page_frames();
        match opts.order {
            ArrangeOrder::LayerIndex => units.sort_by_key(|u| u[0]),
            ArrangeOrder::CurrentPosition => {
                // Rows are told apart to a tenth of a percent, so that layers a hair off sit in one row.
                units.sort_by_key(|u| {
                    let f = now[u[0]];
                    ((f.top * 1000.0).round() as i64, (f.left * 1000.0).round() as i64)
                });
            }
        }
        self.atomic(|g| {
            let targets: Vec<(usize, Frame)> = units
                .iter()
                .zip(&cells)
                .map(|(u, &cell)| {
                    let cur = now[u[0]];
                    let target = if opts.keep_aspect && cur.width > 0.0 && cur.height > 0.0 {
                        let ratio = cur.width / cur.height;
                        let width = cell.width.min(cell.height * ratio);
                        Frame { left: cell.left, top: cell.top, width, height: width / ratio }
                    } else {
                        cell
                    };
                    (u[0], target)
                })
                .collect();
            g.write_page_frames(&targets)?;
            if opts.hide_inner {
                g.hide_inner_axes(&units, &layout);
            }
            if opts.link {
                g.link_units(&units)?;
            }
            Ok(())
        })
    }

    /// "Show Axes Frame" for `units` placed by `layout` (see [`ArrangeOptions::hide_inner`]).
    fn hide_inner_axes(&mut self, units: &[Vec<usize>], layout: &PanelLayout) {
        let n = units.len();
        let filled = |r: usize, c: usize| r < layout.rows && c < layout.cols && r * layout.cols + c < n;
        for (k, unit) in units.iter().enumerate() {
            let (r, c) = layout.cell(k);
            let below = layout.v_gap == 0.0 && filled(r + 1, c);
            let left = layout.h_gap == 0.0 && c > 0 && filled(r, c - 1);
            for &i in unit {
                let layer = &mut self.layers[i];
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
        }
    }

    /// Links the roots of the units to the first unit (the one with the lowest layer number), for
    /// position only. A layer that others follow stays on the page.
    fn link_units(&mut self, units: &[Vec<usize>]) -> Result<(), LayerError> {
        let Some(first) = units.iter().map(|u| u[0]).min() else { return Ok(()) };
        if self.layers[first].link.is_some() {
            return Err(LayerError::ParentLinked(first + 1));
        }
        let mut rest: Vec<usize> = units.iter().map(|u| u[0]).filter(|&i| i != first).collect();
        rest.sort_unstable();
        for i in rest {
            if !self.has_children(i) && !self.set_link(i, Some(first)) {
                return Err(LayerError::Link { layer: i + 1, parent: first + 1 });
            }
        }
        Ok(())
    }

    /// The page frame of the reference layer and the page's size in points.
    fn reference(&self, reference: usize) -> Result<(Frame, f64, f64), LayerError> {
        if reference >= self.layers.len() {
            return Err(LayerError::NoSuchLayer(reference));
        }
        Ok((self.page_frame(reference), self.page.width_pt, self.page.height_pt))
    }

    /// Size/Position > Resize > Apply: sets the width and height of the selected layers; their left
    /// and top stay. `reference` matters for [`SizeUnit::ReferencePercent`].
    pub fn resize_layers(
        &mut self,
        selection: &[usize],
        reference: usize,
        unit: SizeUnit,
        width: f64,
        height: f64,
    ) -> Result<(), LayerError> {
        let sel = self.checked_selection(selection)?;
        let (r, pw, ph) = self.reference(reference)?;
        let (w, h) = (unit.fraction(width, pw, r.width), unit.fraction(height, ph, r.height));
        let targets: Vec<(usize, Frame)> =
            sel.iter().map(|&i| (i, Frame { width: w, height: h, ..self.page_frame(i) })).collect();
        self.atomic(|g| g.write_page_frames(&targets))
    }

    /// Size/Position > Move > Apply: puts the selected layers' top left corner at (`left`, `top`),
    /// measured from the page corner, or from the reference layer's corner in
    /// [`SizeUnit::ReferencePercent`].
    pub fn move_layers(
        &mut self,
        selection: &[usize],
        reference: usize,
        unit: SizeUnit,
        left: f64,
        top: f64,
    ) -> Result<(), LayerError> {
        let sel = self.checked_selection(selection)?;
        let (r, pw, ph) = self.reference(reference)?;
        let (mut x, mut y) = (unit.fraction(left, pw, r.width), unit.fraction(top, ph, r.height));
        if unit == SizeUnit::ReferencePercent {
            x += r.left;
            y += r.top;
        }
        let targets: Vec<(usize, Frame)> =
            sel.iter().map(|&i| (i, Frame { left: x, top: y, ..self.page_frame(i) })).collect();
        self.atomic(|g| g.write_page_frames(&targets))
    }

    /// Size/Position > Align > Apply: moves the chosen edge of the selected layers to the same edge
    /// of the reference layer. The reference layer itself stays.
    pub fn align_layers(&mut self, selection: &[usize], reference: usize, edge: AlignEdge) -> Result<(), LayerError> {
        let sel = self.checked_selection(selection)?;
        let (r, _, _) = self.reference(reference)?;
        let targets: Vec<(usize, Frame)> = sel
            .iter()
            .filter(|&&i| i != reference)
            .map(|&i| {
                let f = self.page_frame(i);
                let moved = match edge {
                    AlignEdge::Left => Frame { left: r.left, ..f },
                    AlignEdge::Top => Frame { top: r.top, ..f },
                    AlignEdge::Right => Frame { left: r.left + r.width - f.width, ..f },
                    AlignEdge::Bottom => Frame { top: r.top + r.height - f.height, ..f },
                };
                (i, moved)
            })
            .collect();
        self.atomic(|g| g.write_page_frames(&targets))
    }

    /// Size/Position > Swap > Apply: the layers `a` and `b` trade places on the page (their whole
    /// frames). With `reorder` they also trade their numbers in the layer list, links to them
    /// follow; this is refused when a layer would then come before the parent it is linked to.
    pub fn swap_layers(&mut self, a: usize, b: usize, reorder: bool) -> Result<(), LayerError> {
        for i in [a, b] {
            if i >= self.layers.len() {
                return Err(LayerError::NoSuchLayer(i));
            }
        }
        if a == b {
            return Err(LayerError::SameLayer);
        }
        self.atomic(|g| {
            let (fa, fb) = (g.page_frame(a), g.page_frame(b));
            let valid_before: Vec<bool> = (0..g.layers.len()).map(|i| g.link_parent(i).is_some()).collect();
            g.write_page_frames(&[(a, fb), (b, fa)])?;
            if reorder {
                g.layers.swap(a, b);
                for layer in &mut g.layers {
                    if let Some(link) = &mut layer.link {
                        if link.parent == a {
                            link.parent = b;
                        } else if link.parent == b {
                            link.parent = a;
                        }
                    }
                }
                // The layer that was at a is now at b, and the other way round.
                let broken = (0..g.layers.len()).any(|i| {
                    let before = if i == a {
                        b
                    } else if i == b {
                        a
                    } else {
                        i
                    };
                    valid_before[before] && g.link_parent(i).is_none()
                });
                if broken {
                    return Err(LayerError::ReorderLinks);
                }
            }
            Ok(())
        })
    }

    /// Link > Apply: links every selected layer to `parent` (or unlinks it with `None`). The layers
    /// keep their place on the page; a new link shares no scale, and `x` and `y` are then set for
    /// every selected layer. Parents come before their layers and are not linked themselves.
    pub fn link_layers(
        &mut self,
        selection: &[usize],
        parent: Option<usize>,
        x: ScaleLink,
        y: ScaleLink,
    ) -> Result<(), LayerError> {
        let sel = self.checked_selection(selection)?;
        if let Some(p) = parent
            && p >= self.layers.len()
        {
            return Err(LayerError::NoSuchLayer(p));
        }
        self.atomic(|g| {
            for &i in &sel {
                if !g.set_link(i, parent) {
                    return Err(LayerError::Link { layer: i + 1, parent: parent.map_or(0, |p| p + 1) });
                }
                if let Some(link) = &mut g.layers[i].link {
                    link.x = x;
                    link.y = y;
                }
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests;
