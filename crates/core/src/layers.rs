//! Adding and deleting layers of a graph (graphing spec section 21, T61): the nine kinds of
//! Graph > New Layer(Axes), and the delete rule of Origin's `layer -d`.
//!
//! Both are pure edits of [`Graph::layers`] that keep the link rule of the model: a layer's link
//! goes to an earlier layer that is not linked itself, so no link points forward, to itself or in
//! a cycle, and every link is renumbered when a layer goes away.

use crate::graph::{AxisSide, Frame, Graph, LayerLink, LegendScope, ScaleLink};
use crate::template::right_y_layer;

/// The kinds of Graph > New Layer(Axes) of Origin 2016 (`layadd type:=`), in menu order. The
/// "Horizontal Axis Layer" and "Vertical Axis Layer" items of older documentation do not exist in
/// the reference version.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NewLayerKind {
    /// Bottom-X Left-Y: a layer of its own, not linked to any other.
    Normal,
    /// Top-X (Linked Y Scale and Dimension).
    TopX,
    /// Right-Y (Linked X Scale and Dimension).
    RightY,
    /// Left-Y (Linked X Scale and Dimension): a second left axis over the first.
    LeftY,
    /// Top-X Right-Y (Linked Dimension).
    TopXRightY,
    /// Bottom-X Right-Y (Linked Dimension).
    BottomXRightY,
    /// Inset (Linked Dimension): a small layer in the top right corner of the parent.
    Inset,
    /// Inset With Data (Linked Dimension): an inset that starts with the parent's scales and
    /// first plot.
    InsetWithData,
    /// No Axes (Linked XY Scale and Dimension).
    NoAxes,
}

impl NewLayerKind {
    /// All kinds, in the order of the menu.
    pub const ALL: [NewLayerKind; 9] = [
        NewLayerKind::Normal,
        NewLayerKind::TopX,
        NewLayerKind::RightY,
        NewLayerKind::LeftY,
        NewLayerKind::TopXRightY,
        NewLayerKind::BottomXRightY,
        NewLayerKind::Inset,
        NewLayerKind::InsetWithData,
        NewLayerKind::NoAxes,
    ];

    /// The `type:=` name of Origin's `layadd`.
    pub fn name(self) -> &'static str {
        match self {
            NewLayerKind::Normal => "normal",
            NewLayerKind::TopX => "topx",
            NewLayerKind::RightY => "righty",
            NewLayerKind::LeftY => "lefty",
            NewLayerKind::TopXRightY => "txry",
            NewLayerKind::BottomXRightY => "bxry",
            NewLayerKind::Inset => "inset",
            NewLayerKind::InsetWithData => "insetdata",
            NewLayerKind::NoAxes => "noxy",
        }
    }

    pub fn from_name(name: &str) -> Option<NewLayerKind> {
        NewLayerKind::ALL.into_iter().find(|k| k.name().eq_ignore_ascii_case(name))
    }

    /// The text of the item in Graph > New Layer(Axes).
    pub fn menu_label(self) -> &'static str {
        match self {
            NewLayerKind::Normal => "Bottom-X Left-Y",
            NewLayerKind::TopX => "Top-X (Linked Y Scale and Dimension)",
            NewLayerKind::RightY => "Right-Y (Linked X Scale and Dimension)",
            NewLayerKind::LeftY => "Left-Y (Linked X Scale and Dimension)",
            NewLayerKind::TopXRightY => "Top-X Right-Y (Linked Dimension)",
            NewLayerKind::BottomXRightY => "Bottom-X Right-Y (Linked Dimension)",
            NewLayerKind::Inset => "Inset (Linked Dimension)",
            NewLayerKind::InsetWithData => "Inset With Data (Linked Dimension)",
            NewLayerKind::NoAxes => "No Axes (Linked XY Scale and Dimension)",
        }
    }

    fn is_inset(self) -> bool {
        matches!(self, NewLayerKind::Inset | NewLayerKind::InsetWithData)
    }

    /// Presets that draw axes around the full frame of their linked parent.
    pub fn supports_auto_offset(self) -> bool {
        matches!(self, Self::TopX | Self::RightY | Self::LeftY | Self::TopXRightY | Self::BottomXRightY)
    }
}

/// Axis and link choices for the User Defined option in New Layer (Axes).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct UserDefinedLayer {
    pub bottom: bool,
    pub top: bool,
    pub left: bool,
    pub right: bool,
    pub parent: Option<usize>,
    pub x_link: ScaleLink,
    pub y_link: ScaleLink,
}

impl Default for UserDefinedLayer {
    fn default() -> Self {
        Self {
            bottom: true,
            top: false,
            left: true,
            right: false,
            parent: None,
            x_link: ScaleLink::None,
            y_link: ScaleLink::None,
        }
    }
}

impl UserDefinedLayer {
    /// Auto offset applies to visible axes on a parent's frame.
    pub fn supports_auto_offset(self) -> bool {
        self.parent.is_some() && (self.bottom || self.top || self.left || self.right)
    }
}

/// The undo text of Origin after adding a layer.
pub const ADD_LAYER_UNDO: &str = "Create";
/// The undo text of Origin after deleting a layer.
pub const DELETE_LAYER_UNDO: &str = "Undelete frame";

/// Where a layer that is not linked starts, relative to the layer it is added next to (page
/// fractions, from one measurement; Origin's own offset varied between runs, graphing spec 21.1).
const NORMAL_LAYER_SHIFT: (f64, f64) = (-0.0232, -0.0305);

/// The size of an inset as a share of its parent's, and the factor its axis fonts are scaled by.
const INSET_FRAME_SCALE: f64 = 0.25;

/// An axis of a new layer: the style of the parent's axis, but with a scale, title and grid of its own.
fn fresh_axis(from: &crate::graph::Axis, side: AxisSide) -> crate::graph::Axis {
    let mut a = from.clone();
    a.side = side;
    a.min = None;
    a.max = None;
    a.major_step = None;
    a.title = None;
    a.tick_label_divisor = 1.0;
    a.show_opposite = false;
    a.opposite_ticks = false;
    a.opposite_offset_pct = None;
    a.opposite_style = Default::default();
    a.major_grid = None;
    a.minor_grid = None;
    a.offset_pct = 0.0;
    a
}

impl Graph {
    /// The frame of layer `index` as fractions of the page: a linked layer's frame is given in its
    /// parent's frame. `None` for a missing layer.
    pub fn absolute_frame(&self, index: usize) -> Option<Frame> {
        let layer = self.layers.get(index)?;
        Some(match self.link_parent(index) {
            Some(parent) => layer.frame.within(self.layers[parent].frame),
            None => layer.frame,
        })
    }

    /// Adds a layer of `kind` as the last layer (Graph > New Layer(Axes)) and returns its index; the
    /// caller makes it the active layer. `active` is the active layer, the new layer's parent: a
    /// layer that is linked itself cannot be followed, so a link then goes to the root the active
    /// layer follows. `None` when the graph has no layer `active`.
    ///
    /// The new layer is empty. A linked layer covers its parent (0, 0, 100 and 100 % of it) and
    /// shares the scale that its name says; an inset is 25 % of the parent's size in its top right
    /// corner, with a bottom and a left axis and scales of its own; Inset With Data also starts with
    /// the parent's axis limits and a copy of its first plot. A Bottom-X Left-Y layer is linked to
    /// nothing and starts near the active layer's place.
    pub fn add_layer(&mut self, kind: NewLayerKind, active: usize) -> Option<usize> {
        if active >= self.layers.len() {
            return None;
        }
        let parent_index = self.root_of(active);
        let parent = self.layers[parent_index].clone();
        let mut layer = right_y_layer(&parent, parent_index);
        // The parent's colours again, as a new layer of Origin starts its own list.
        layer.palette = parent.palette.clone();
        layer.legend.visible = false;
        layer.legend.scope = LegendScope::Layer;
        let (xs, ys, xl, yl) = match kind {
            NewLayerKind::Normal => (AxisSide::Normal, AxisSide::Normal, ScaleLink::None, ScaleLink::None),
            NewLayerKind::TopX => (AxisSide::Opposite, AxisSide::Hidden, ScaleLink::None, ScaleLink::Straight),
            NewLayerKind::RightY => (AxisSide::Hidden, AxisSide::Opposite, ScaleLink::Straight, ScaleLink::None),
            NewLayerKind::LeftY => (AxisSide::Hidden, AxisSide::Normal, ScaleLink::Straight, ScaleLink::None),
            NewLayerKind::TopXRightY => (AxisSide::Opposite, AxisSide::Opposite, ScaleLink::None, ScaleLink::None),
            NewLayerKind::BottomXRightY => (AxisSide::Normal, AxisSide::Opposite, ScaleLink::None, ScaleLink::None),
            NewLayerKind::Inset | NewLayerKind::InsetWithData => {
                (AxisSide::Normal, AxisSide::Normal, ScaleLink::None, ScaleLink::None)
            }
            NewLayerKind::NoAxes => (AxisSide::Hidden, AxisSide::Hidden, ScaleLink::Straight, ScaleLink::Straight),
        };
        layer.x_axis = fresh_axis(&parent.x_axis, xs);
        layer.y_axis = fresh_axis(&parent.y_axis, ys);
        if kind == NewLayerKind::InsetWithData {
            // The parent's limits and ticks, but not linked to them; the first plot is drawn again.
            for (new, old) in [(&mut layer.x_axis, &parent.x_axis), (&mut layer.y_axis, &parent.y_axis)] {
                new.min = old.min;
                new.max = old.max;
                new.major_step = old.major_step;
            }
            layer.plots = parent.plots.first().cloned().into_iter().collect();
            // Origin keeps the parent's axis title text (graphing spec 21.1).
            layer.x_axis.title = parent.x_axis.title.clone();
            layer.y_axis.title = parent.y_axis.title.clone();
        }
        if kind.is_inset() {
            // Scale With Layer Frame: tick label and title fonts shrink with the inset (21.1). The
            // exact factor was not measured; Sheaf uses the inset's share of the parent's size.
            for axis in [&mut layer.x_axis, &mut layer.y_axis] {
                axis.tick_font.size_pt *= INSET_FRAME_SCALE;
                axis.title_font.size_pt *= INSET_FRAME_SCALE;
            }
        }
        if kind == NewLayerKind::Normal {
            layer.link = None;
            let base = self.absolute_frame(parent_index).unwrap_or(parent.frame);
            layer.frame = Frame {
                left: (base.left + NORMAL_LAYER_SHIFT.0).max(0.0),
                top: (base.top + NORMAL_LAYER_SHIFT.1).max(0.0),
                width: base.width,
                height: base.height,
            };
        } else {
            layer.link = Some(LayerLink { parent: parent_index, x: xl, y: yl });
            layer.frame = if kind.is_inset() {
                Frame { left: 1.0 - INSET_FRAME_SCALE, top: 0.0, width: INSET_FRAME_SCALE, height: INSET_FRAME_SCALE }
            } else {
                Frame::WHOLE
            };
        }
        self.layers.push(layer);
        Some(self.layers.len() - 1)
    }

    /// Add a preset and separate its axes from coincident axes on the same frame.
    pub fn add_layer_with_auto_offset(&mut self, kind: NewLayerKind, active: usize) -> Option<usize> {
        let index = self.add_layer(kind, active)?;
        if kind.supports_auto_offset() {
            self.offset_new_layer_axes(index);
        }
        Some(index)
    }

    /// Adds an empty layer with the requested axes, optionally covering an earlier unlinked layer.
    /// An invalid source or parent leaves the graph unchanged.
    pub fn add_user_defined_layer(
        &mut self,
        options: UserDefinedLayer,
        active: usize,
        auto_offset: bool,
    ) -> Option<usize> {
        self.layers.get(active)?;
        let (kind, source) = if let Some(parent) = options.parent {
            if self.layers.get(parent)?.link.is_some() {
                return None;
            }
            (NewLayerKind::NoAxes, parent)
        } else {
            (NewLayerKind::Normal, active)
        };
        let index = self.add_layer(kind, source)?;
        let layer = &mut self.layers[index];
        if let Some(link) = &mut layer.link {
            link.x = options.x_link;
            link.y = options.y_link;
        }
        for (axis, near, far) in
            [(&mut layer.x_axis, options.bottom, options.top), (&mut layer.y_axis, options.left, options.right)]
        {
            axis.side = match (near, far) {
                (true, _) => AxisSide::Normal,
                (false, true) => AxisSide::Opposite,
                (false, false) => AxisSide::Hidden,
            };
            axis.at_position = None;
            axis.show_tick_labels = near || far;
            axis.show_opposite = near && far;
            axis.opposite_ticks = near && far;
            axis.opposite_offset_pct = (near && far).then_some(0.0);
        }
        layer.display.show_x_axes = true;
        layer.display.show_y_axes = true;
        if auto_offset && options.supports_auto_offset() {
            self.offset_new_layer_axes(index);
        }
        Some(index)
    }

    fn offset_new_layer_axes(&mut self, index: usize) {
        let Some(frame) = self.absolute_frame(index) else { return };
        let layer = &self.layers[index];
        let x = self.new_axis_offset(index, frame, layer.x_axis.side, true);
        let y = self.new_axis_offset(index, frame, layer.y_axis.side, false);
        let x_opposite =
            layer.x_axis.opposite_offset_pct.map(|_| self.new_axis_offset(index, frame, AxisSide::Opposite, true));
        let y_opposite =
            layer.y_axis.opposite_offset_pct.map(|_| self.new_axis_offset(index, frame, AxisSide::Opposite, false));
        let layer = &mut self.layers[index];
        layer.x_axis.at_position = None;
        layer.y_axis.at_position = None;
        layer.x_axis.offset_pct = x;
        layer.y_axis.offset_pct = y;
        layer.x_axis.opposite_offset_pct = x_opposite;
        layer.y_axis.opposite_offset_pct = y_opposite;
    }

    fn new_axis_offset(&self, index: usize, frame: Frame, side: AxisSide, is_x: bool) -> f64 {
        if side == AxisSide::Hidden {
            return 0.0;
        }
        // Nominal page spacing fitted to Origin 2016's integer position readbacks (graphing 21.15).
        let (page_spacing, extent) = if is_x { (2.03, frame.height) } else { (1.5, frame.width) };
        if !extent.is_finite() || extent <= 0.0 {
            return 0.0;
        }
        let step = (page_spacing / extent).round().max(1.0);
        let mut on_edge = false;
        let mut outermost: f64 = 0.0;
        for (i, layer) in self.layers[..index].iter().enumerate() {
            let (show, axis) = if is_x {
                (layer.display.show_x_axes, &layer.x_axis)
            } else {
                (layer.display.show_y_axes, &layer.y_axis)
            };
            if !show || layer.display.hidden || axis.side == AxisSide::Hidden || self.absolute_frame(i) != Some(frame) {
                continue;
            }
            let offset = if axis.side == side {
                axis.at_position.is_none().then_some(axis.offset_pct)
            } else {
                (axis.show_opposite || axis.opposite_ticks).then_some(axis.opposite_offset_pct.unwrap_or_default())
            };
            if let Some(offset) = offset.filter(|offset| offset.is_finite()) {
                on_edge |= offset == 0.0;
                outermost = outermost.max(offset);
            }
        }
        let offset = outermost + step;
        if on_edge && offset.is_finite() { offset } else { 0.0 }
    }

    /// Gives the layers that follow layer `index` on a straight link their range as it is drawn now
    /// (`ranges[i]` is layer i's X and Y as (min, max, step)), so that they keep it when `index` goes
    /// away (graphing spec 21.4: x.from and x.to stay). Call before [`Graph::delete_layer`].
    pub fn freeze_linked_ranges(&mut self, index: usize, ranges: &[((f64, f64, f64), (f64, f64, f64))]) {
        for (i, layer) in self.layers.iter_mut().enumerate() {
            let Some(link) = layer.link.filter(|l| l.parent == index) else { continue };
            let Some(&(x, y)) = ranges.get(i) else { continue };
            for (axis, shared, (min, max, step)) in [(&mut layer.x_axis, link.x, x), (&mut layer.y_axis, link.y, y)] {
                if shared == ScaleLink::Straight && axis.min.is_none() && axis.max.is_none() {
                    axis.min = Some(min);
                    axis.max = Some(max);
                    axis.major_step.get_or_insert(step);
                }
            }
        }
    }

    /// Deletes layer `index` (Delete Layer of the layer icon, `layer -d`). Origin never refuses and
    /// never gives the children a new parent: every layer linked to the deleted one becomes an
    /// independent layer that keeps its place on the page (for a layer that covers its parent that
    /// is the parent's rectangle), and every link to a later layer is renumbered. The text of
    /// legends is not rewritten.
    ///
    /// Returns false, changing nothing, for a missing layer and for the only layer (a graph keeps
    /// at least one; Origin's behaviour there is unmeasured).
    pub fn delete_layer(&mut self, index: usize) -> bool {
        if index >= self.layers.len() || self.layers.len() == 1 {
            return false;
        }
        let places: Vec<Option<Frame>> = (0..self.layers.len()).map(|i| self.absolute_frame(i)).collect();
        for (i, layer) in self.layers.iter_mut().enumerate() {
            let Some(link) = layer.link else { continue };
            if i == index {
                continue;
            }
            if link.parent == index {
                layer.frame = places[i].unwrap_or(layer.frame);
                layer.link = None;
            } else if link.parent > index {
                layer.link = Some(LayerLink { parent: link.parent - 1, ..link });
            }
        }
        self.layers.remove(index);
        true
    }
}

/// How a layer index moves when layer `deleted` is removed: the layers after it move up by one and
/// the deleted layer itself has no new index. Used to repair selections.
pub fn index_after_delete(index: usize, deleted: usize) -> Option<usize> {
    match index.cmp(&deleted) {
        std::cmp::Ordering::Less => Some(index),
        std::cmp::Ordering::Equal => None,
        std::cmp::Ordering::Greater => Some(index - 1),
    }
}

/// The layer that becomes active when the active layer `deleted` is removed from a graph that has
/// `remaining` layers afterwards: the one before it, or the new first layer.
pub fn active_after_delete(deleted: usize, remaining: usize) -> usize {
    deleted.saturating_sub(1).min(remaining.saturating_sub(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::WindowId;
    use crate::template::Template;

    fn graph() -> Graph {
        Template::Line.new_graph(WindowId(1), "G")
    }

    /// Every link goes to an earlier layer that is not linked itself.
    fn well_formed(g: &Graph) -> bool {
        g.layers.iter().enumerate().all(|(i, l)| match l.link {
            None => true,
            Some(k) => k.parent < i && g.layers[k.parent].link.is_none(),
        })
    }

    #[test]
    fn a_new_layer_of_a_scatter_central_graph_starts_with_axes_at_offset_zero() {
        let mut g = Template::ScatterCentral.new_graph(WindowId(1), "G");
        assert_eq!((g.layers[0].x_axis.offset_pct, g.layers[0].y_axis.offset_pct), (-50.0, -50.0));
        g.add_layer(NewLayerKind::RightY, 0).unwrap();
        let l = &g.layers[1];
        assert_eq!((l.x_axis.offset_pct, l.y_axis.offset_pct), (0.0, 0.0));
    }

    #[test]
    fn user_defined_links_use_the_chosen_frame_and_reject_invalid_parents() {
        let mut g = graph();
        g.layers[0].frame.width = 0.6816;
        g.layers[0].frame.height = 0.7183;
        let linked = g.add_layer(NewLayerKind::Inset, 0).unwrap();
        let request = UserDefinedLayer { parent: Some(linked), ..UserDefinedLayer::default() };
        let before = g.layers.clone();
        assert_eq!(g.add_user_defined_layer(request, 0, true), None);
        assert_eq!(g.add_user_defined_layer(UserDefinedLayer::default(), 99, false), None);
        assert_eq!(g.layers, before);

        let request = UserDefinedLayer {
            parent: Some(0),
            x_link: ScaleLink::Straight,
            top: true,
            left: false,
            right: true,
            ..UserDefinedLayer::default()
        };
        let index = g.add_user_defined_layer(request, linked, true).unwrap();
        assert_eq!(&g.layers[..index], before);
        let layer = &g.layers[index];
        assert_eq!(layer.frame, Frame::WHOLE);
        assert_eq!(g.absolute_frame(index), Some(before[0].frame));
        assert_eq!(layer.link, Some(LayerLink { parent: 0, x: ScaleLink::Straight, y: ScaleLink::None }));
        assert_eq!((layer.x_axis.side, layer.y_axis.side), (AxisSide::Normal, AxisSide::Opposite));
        assert!(layer.x_axis.show_opposite && layer.x_axis.opposite_ticks);
        assert_eq!(layer.x_axis.offset_pct, 3.0);
        assert_eq!(layer.y_axis.offset_pct, 0.0);
    }

    #[test]
    fn paired_axes_count_the_offsets_on_each_edge_separately() {
        for far_edges_on_parent in [false, true] {
            let mut g = graph();
            let parent = &mut g.layers[0];
            parent.frame.width = 0.6816;
            parent.frame.height = 0.7183;
            for axis in [&mut parent.x_axis, &mut parent.y_axis] {
                axis.show_opposite = far_edges_on_parent;
                axis.opposite_ticks = false;
            }
            let request = UserDefinedLayer { top: true, right: true, parent: Some(0), ..UserDefinedLayer::default() };
            for (index, (x, y)) in [(3.0, 2.0), (6.0, 4.0)].into_iter().enumerate() {
                let saved = g.layers.clone();
                let i = g.add_user_defined_layer(request, 0, true).unwrap();
                assert_eq!(g.layers[..i], saved);
                let layer = &g.layers[i];
                assert_eq!((layer.x_axis.offset_pct, layer.y_axis.offset_pct), (x, y));
                let (top, right) = if far_edges_on_parent { (x, y) } else { (3.0 * index as f64, 2.0 * index as f64) };
                assert_eq!(
                    (layer.x_axis.opposite_offset_pct, layer.y_axis.opposite_offset_pct),
                    (Some(top), Some(right))
                );
            }
            let json = serde_json::to_string(&g).unwrap();
            assert_eq!(serde_json::from_str::<Graph>(&json).unwrap(), g);
        }
    }

    #[test]
    fn new_layer_offsets_match_origin_readbacks_at_several_frame_sizes() {
        // Origin 2016: two Bottom-X Right-Y layers, the second added with offset:=1.
        for (w, h, x, y) in [
            (19.99, 20.01, 10.0, 8.0),
            (25.0, 25.01, 8.0, 6.0),
            (30.01, 30.0, 7.0, 5.0),
            (40.0, 40.0, 5.0, 4.0),
            (50.0, 50.01, 4.0, 3.0),
            (60.0, 60.0, 3.0, 3.0),
            (80.01, 79.99, 3.0, 2.0),
            (90.0, 90.01, 2.0, 2.0),
        ] {
            let mut g = graph();
            g.layers[0].frame.width = w / 100.0;
            g.layers[0].frame.height = h / 100.0;
            g.add_layer(NewLayerKind::BottomXRightY, 0).unwrap();
            let before = g.layers.clone();
            let new = g.add_layer_with_auto_offset(NewLayerKind::BottomXRightY, 0).unwrap();
            let layer = &g.layers[new];
            assert_eq!((layer.x_axis.offset_pct, layer.y_axis.offset_pct), (x, y), "{w} × {h}");
            assert_eq!(g.layers[..new], before);
            assert_eq!(g.absolute_frame(new), Some(before[0].frame));
            assert_eq!(layer.link.unwrap().parent, 0);
        }
    }

    #[test]
    fn auto_offset_reuses_a_free_edge_then_moves_past_the_outermost_axis() {
        let mut g = graph();
        g.layers[0].frame.width = 0.6816;
        g.layers[0].y_axis.show_opposite = false;
        let far = g.add_layer(NewLayerKind::RightY, 0).unwrap();
        g.layers[far].y_axis.offset_pct = 8.0;

        let edge = g.add_layer_with_auto_offset(NewLayerKind::RightY, 0).unwrap();
        assert_eq!(g.layers[edge].y_axis.offset_pct, 0.0);
        let outer = g.add_layer_with_auto_offset(NewLayerKind::RightY, 0).unwrap();
        assert_eq!(g.layers[outer].y_axis.offset_pct, 10.0);
        assert_eq!(g.layers[far].y_axis.offset_pct, 8.0);
        assert_eq!(g.layers[edge].y_axis.offset_pct, 0.0);
    }

    #[test]
    fn auto_offset_counts_only_visible_axes_on_the_matching_frame() {
        let mut g = graph();
        g.layers[0].frame.height = 0.4499;
        g.layers[0].x_axis.show_opposite = false;
        let hidden = g.add_layer(NewLayerKind::TopX, 0).unwrap();
        g.layers[hidden].display.hidden = true;
        let separated = g.add_layer(NewLayerKind::TopX, 0).unwrap();
        g.layers[separated].frame.top = 0.1;
        let off = g.add_layer(NewLayerKind::TopX, 0).unwrap();
        g.layers[off].display.show_x_axes = false;

        let edge = g.add_layer_with_auto_offset(NewLayerKind::TopX, 0).unwrap();
        let outer = g.add_layer_with_auto_offset(NewLayerKind::TopX, 0).unwrap();
        assert_eq!(g.layers[edge].x_axis.offset_pct, 0.0);
        assert_eq!(g.layers[outer].x_axis.offset_pct, 5.0);
    }

    #[test]
    fn the_nine_kinds_have_the_measured_links_frames_and_axes() {
        use AxisSide::*;
        let table = [
            (NewLayerKind::TopX, Some((ScaleLink::None, ScaleLink::Straight)), Opposite, Hidden),
            (NewLayerKind::RightY, Some((ScaleLink::Straight, ScaleLink::None)), Hidden, Opposite),
            (NewLayerKind::LeftY, Some((ScaleLink::Straight, ScaleLink::None)), Hidden, Normal),
            (NewLayerKind::TopXRightY, Some((ScaleLink::None, ScaleLink::None)), Opposite, Opposite),
            (NewLayerKind::BottomXRightY, Some((ScaleLink::None, ScaleLink::None)), Normal, Opposite),
            (NewLayerKind::Inset, Some((ScaleLink::None, ScaleLink::None)), Normal, Normal),
            (NewLayerKind::NoAxes, Some((ScaleLink::Straight, ScaleLink::Straight)), Hidden, Hidden),
            (NewLayerKind::Normal, None, Normal, Normal),
        ];
        for (kind, links, xs, ys) in table {
            let mut g = graph();
            assert_eq!(g.add_layer(kind, 0), Some(1), "{kind:?}");
            let l = &g.layers[1];
            assert_eq!(l.link.map(|k| (k.x, k.y)), links, "{kind:?}");
            assert_eq!((l.x_axis.side, l.y_axis.side), (xs, ys), "{kind:?}");
            if links.is_some() {
                assert_eq!(l.link.unwrap().parent, 0);
                let want = if kind == NewLayerKind::Inset {
                    Frame { left: 0.75, top: 0.0, width: 0.25, height: 0.25 }
                } else {
                    Frame::WHOLE
                };
                assert_eq!(l.frame, want, "{kind:?}");
            }
            assert!(well_formed(&g));
        }
    }

    #[test]
    fn a_bottom_x_left_y_layer_is_unlinked_and_near_the_parent() {
        let mut g = graph();
        g.add_layer(NewLayerKind::Normal, 0).unwrap();
        let (a, b) = (g.layers[0].frame, g.layers[1].frame);
        assert!(g.layers[1].link.is_none());
        assert_eq!((a.width, a.height), (b.width, b.height));
        assert!(b.left < a.left && b.top < a.top && b.left >= 0.0 && b.top >= 0.0);
    }

    #[test]
    fn the_parent_is_the_active_layer_or_the_root_it_follows() {
        let mut g = graph();
        g.add_layer(NewLayerKind::Normal, 0).unwrap();
        // Layer 2 (index 1) is active: the right Y layer follows it.
        assert_eq!(g.add_layer(NewLayerKind::RightY, 1), Some(2));
        assert_eq!(g.layers[2].link.unwrap().parent, 1);
        // A linked layer cannot be a parent: an inset added while it is active follows its root.
        assert_eq!(g.add_layer(NewLayerKind::Inset, 2), Some(3));
        assert_eq!(g.layers[3].link.unwrap().parent, 1);
        assert!(well_formed(&g));
        assert_eq!(g.add_layer(NewLayerKind::TopX, 9), None);
    }

    #[test]
    fn an_inset_with_data_copies_the_limits_and_the_first_plot_but_does_not_link_them() {
        let mut p = crate::project::Project::new();
        let book = p.add_workbook("d");
        let id = p.add_graph(Template::Line);
        let mut cols = Vec::new();
        for _ in 0..2 {
            let c = p
                .add_column(book, 0, crate::column::Designation::Y, crate::column::ColumnData::numeric(vec![1.0, 2.0]))
                .unwrap();
            p.add_plot(id, 0, c).unwrap();
            cols.push(c);
        }
        let mut g = p.graph(id).unwrap().clone();
        g.layers[0].x_axis.min = Some(-2.5);
        g.layers[0].y_axis.max = Some(125.0);
        g.add_layer(NewLayerKind::InsetWithData, 0).unwrap();
        let l = &g.layers[1];
        assert_eq!((l.x_axis.min, l.y_axis.max), (Some(-2.5), Some(125.0)));
        assert_eq!(l.plots.len(), 1);
        assert_eq!(l.plots[0].y, cols[0]);
        assert_eq!(l.link.map(|k| (k.x, k.y)), Some((ScaleLink::None, ScaleLink::None)));
        g.add_layer(NewLayerKind::Inset, 0).unwrap();
        assert!(g.layers[2].plots.is_empty() && g.layers[2].x_axis.min.is_none());
    }

    #[test]
    fn deleting_a_parent_unlinks_its_children_which_keep_its_rectangle() {
        // Origin's Y-Y-Y: layers 2 and 3 both linked to layer 1, deleting layer 1.
        let mut g = graph();
        g.add_layer(NewLayerKind::RightY, 0).unwrap();
        g.add_layer(NewLayerKind::RightY, 0).unwrap();
        let rect = g.layers[0].frame;
        assert!(g.delete_layer(0));
        assert_eq!(g.layers.len(), 2);
        for l in &g.layers {
            assert!(l.link.is_none());
            assert_eq!(l.frame, rect);
        }
    }

    #[test]
    fn deleting_the_first_layer_keeps_later_groups_linked_and_renumbered() {
        let mut g = graph();
        g.add_layer(NewLayerKind::Normal, 0).unwrap(); // 1, a second root
        g.add_layer(NewLayerKind::RightY, 1).unwrap(); // 2 -> 1
        g.add_layer(NewLayerKind::Inset, 1).unwrap(); // 3 -> 1
        assert!(g.delete_layer(0));
        assert_eq!(g.layers.len(), 3);
        assert_eq!(g.layers[1].link.unwrap().parent, 0);
        assert_eq!(g.layers[2].link.unwrap().parent, 0);
        assert!(well_formed(&g));
    }

    #[test]
    fn deleting_the_last_layer_changes_nothing_else_and_a_middle_one_renumbers_later_links() {
        let mut g = graph();
        g.add_layer(NewLayerKind::RightY, 0).unwrap();
        g.add_layer(NewLayerKind::Normal, 0).unwrap();
        g.add_layer(NewLayerKind::TopX, 2).unwrap(); // 3 -> 2
        let before = g.clone();
        assert!(g.delete_layer(3));
        assert_eq!(g.layers, before.layers[..3]);
        let mut g = before;
        assert!(g.delete_layer(1));
        assert_eq!(g.layers.len(), 3);
        assert_eq!(g.layers[2].link.unwrap().parent, 1, "was linked to layer 3 of 4, now to layer 2 of 3");
        assert!(well_formed(&g));
    }

    #[test]
    fn deleting_a_parent_that_is_not_the_first_layer_gives_children_their_absolute_place() {
        let mut g = graph();
        g.add_layer(NewLayerKind::Normal, 0).unwrap();
        g.add_layer(NewLayerKind::Inset, 1).unwrap();
        let inset_place = g.absolute_frame(2).unwrap();
        assert!(g.delete_layer(1));
        assert_eq!(g.layers.len(), 2);
        assert!(g.layers[1].link.is_none());
        assert_eq!(g.layers[1].frame, inset_place, "the inset stays where it was drawn");
    }

    #[test]
    fn the_only_layer_and_missing_layers_are_not_deleted() {
        let mut g = graph();
        assert!(!g.delete_layer(0));
        assert!(!g.delete_layer(4));
        assert_eq!(g.layers.len(), 1);
    }

    #[test]
    fn dangling_links_are_cleared_or_left_to_the_model_which_draws_them_independent() {
        let mut g = graph();
        g.add_layer(NewLayerKind::RightY, 0).unwrap();
        g.add_layer(NewLayerKind::RightY, 0).unwrap();
        // A hand-edited file: layer 1 points forward to layer 2.
        g.layers[1].link = Some(LayerLink { parent: 2, x: ScaleLink::Straight, y: ScaleLink::None });
        assert!(g.delete_layer(2));
        assert_eq!(g.layers[1].link, None, "a link to the deleted layer is cleared");
        let mut g = graph();
        g.add_layer(NewLayerKind::RightY, 0).unwrap();
        g.add_layer(NewLayerKind::RightY, 0).unwrap();
        g.layers[1].link = Some(LayerLink { parent: 2, x: ScaleLink::Straight, y: ScaleLink::None });
        assert!(g.delete_layer(0));
        assert_eq!(g.link_parent(0), None);
    }

    #[test]
    fn indices_and_the_active_layer_follow_a_deletion() {
        assert_eq!(index_after_delete(0, 1), Some(0));
        assert_eq!(index_after_delete(1, 1), None);
        assert_eq!(index_after_delete(3, 1), Some(2));
        assert_eq!(active_after_delete(2, 3), 1);
        assert_eq!(active_after_delete(0, 2), 0);
        assert_eq!(active_after_delete(3, 3), 2);
    }

    #[test]
    fn names_round_trip() {
        for k in NewLayerKind::ALL {
            assert_eq!(NewLayerKind::from_name(k.name()), Some(k));
        }
        assert_eq!(NewLayerKind::from_name("horizontal"), None);
    }

    #[test]
    fn an_inset_scales_its_fonts_and_inset_with_data_keeps_the_titles() {
        let mut g = graph();
        g.layers[0].x_axis.title = Some("Potential (V)".into());
        let parent_tick = g.layers[0].x_axis.tick_font.size_pt;
        let i = g.add_layer(NewLayerKind::InsetWithData, 0).unwrap();
        assert!((g.layers[i].x_axis.tick_font.size_pt - parent_tick * 0.25).abs() < 1e-9);
        assert_eq!(g.layers[i].x_axis.title.as_deref(), Some("Potential (V)"));
        let j = g.add_layer(NewLayerKind::Inset, 0).unwrap();
        assert!(g.layers[j].x_axis.title.is_none());
        assert!(g.layers[j].y_axis.title_font.size_pt < g.layers[0].y_axis.title_font.size_pt);
    }

    #[test]
    fn deleting_a_parent_freezes_the_range_a_child_followed() {
        let mut g = graph();
        let i = g.add_layer(NewLayerKind::RightY, 0).unwrap();
        g.freeze_linked_ranges(0, &[((0.0, 1.0, 1.0), (0.0, 1.0, 1.0)), ((-1.0, 11.0, 2.0), (5.0, 6.0, 1.0))]);
        assert_eq!(i, 1);
        let c = &g.layers[1];
        assert_eq!((c.x_axis.min, c.x_axis.max, c.x_axis.major_step), (Some(-1.0), Some(11.0), Some(2.0)));
        assert_eq!(c.y_axis.min, None, "an own scale stays automatic");
        assert!(g.delete_layer(0));
        assert_eq!(g.layers[0].x_axis.min, Some(-1.0));
    }
}
