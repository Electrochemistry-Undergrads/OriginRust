//! User graph templates (File > Save Template As): the look of a graph without its data.
//!
//! A [`UserTemplate`] is a graph whose plots no longer point at columns. Each plot is a numbered
//! slot: its `y` is `ColumnId(slot + 1)`, slots count plots layer by layer in order. A slot keeps
//! the plot's whole style; where the plot read bubble sizes or colours from the Z column the slot
//! says so with a number too. [`Project::plot_with_user_template`] plots columns into a copy of the
//! template: the first column takes the first slot, and so on, round and round when there are more
//! columns than slots. The rules are in `docs/spec/graphing.md` (section 24).

use crate::graph::{Graph, LabelForm, PlotKind, TickLabelSource};
use crate::ids::{ColumnId, WindowId};
use crate::project::{Project, ProjectError};
use serde::{Deserialize, Serialize};

/// A named graph look without data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UserTemplate {
    /// The name the template is listed under.
    pub name: String,
    /// The graph with its data references removed (see the module docs). Its id and name mean nothing.
    pub graph: Graph,
}

/// Why a graph cannot be made into a template.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum UserTemplateError {
    /// A plot's look is made of the columns it reads, so there is nothing a template could keep.
    #[error("a graph with {0} cannot be saved as a template yet")]
    Unsupported(&'static str),
}

/// The placeholder column of slot `slot` (0-based): slot numbers start at 1 so that no slot is `ColumnId(0)`.
fn placeholder(slot: usize) -> ColumnId {
    ColumnId(slot as u64 + 1)
}

fn slot_of(id: ColumnId) -> Option<usize> {
    usize::try_from(id.0).ok()?.checked_sub(1)
}

impl UserTemplate {
    /// Takes the look of `graph`: everything but the data. Plots keep their style in order; their
    /// columns, row ranges and the lower-edge column of floating bars are dropped, a legend text
    /// of a plot is kept only when it holds `%(` substitutions, a data label from a column goes
    /// back to the Y value, a tick label column back to automatic and the column a rug follows
    /// back to all plots. Text objects, the legend text, drawn objects, axis titles and fixed or
    /// automatic axis ranges stay as they are. Refused for plots that are made of their data
    /// structure: treemap, sunburst, grouped column and box plots of indexed data, interval plots and Pareto charts, parallel plot and parallel
    /// index, floating column and bar, and the fill between two curves.
    pub fn from_graph(name: &str, graph: &Graph) -> Result<UserTemplate, UserTemplateError> {
        for p in graph.layers.iter().flat_map(|l| &l.plots) {
            if p.indexed.is_some() {
                return Err(UserTemplateError::Unsupported("a grouped plot of indexed data"));
            }
            if p.stat.is_some() {
                return Err(UserTemplateError::Unsupported("an interval plot or Pareto chart"));
            }
            match p.kind {
                PlotKind::Treemap => return Err(UserTemplateError::Unsupported("a treemap")),
                PlotKind::Sunburst => return Err(UserTemplateError::Unsupported("a sunburst plot")),
                k if k.is_parallel() => return Err(UserTemplateError::Unsupported("a parallel plot")),
                k if k.is_floating() => return Err(UserTemplateError::Unsupported("floating bars")),
                _ => {}
            }
            if p.y_base.is_some() {
                return Err(UserTemplateError::Unsupported("a fill between two curves"));
            }
        }
        let mut g = graph.clone();
        g.id = WindowId(0);
        g.name = name.to_string();
        g.long_name.clear();
        let mut slot = 0;
        for layer in &mut g.layers {
            let first_slot = slot;
            let columns: Vec<ColumnId> = layer.plots.iter().map(|p| p.y).collect();
            for p in &mut layer.plots {
                p.x = None;
                p.y = placeholder(slot);
                p.y_error = None;
                p.x_error = None;
                p.rows = None;
                p.y_base = None;
                for from in [&mut p.size_from, &mut p.color_from] {
                    *from = from.map(|_| placeholder(slot));
                }
                if p.label.as_ref().is_some_and(|l| !l.contains("%(")) {
                    p.label = None;
                }
                if matches!(p.decor.labels.form, LabelForm::Column { .. }) {
                    p.decor.labels.form = LabelForm::Y;
                }
                p.parallel.color = None;
                p.treemap.categories.clear();
                p.treemap.order_column = None;
                p.sunburst.categories.clear();
                slot += 1;
            }
            // Offsets belong to the first plot of their column; the rest are dropped.
            let slot_of_column =
                |c: ColumnId| columns.iter().position(|y| *y == c).map(|i| placeholder(first_slot + i));
            layer.plot_offsets = layer
                .plot_offsets
                .iter()
                .filter_map(|o| {
                    let mut o = *o;
                    o.column = slot_of_column(o.column)?;
                    o.follows = o.follows.and_then(slot_of_column);
                    Some(o)
                })
                .collect();
            for axis in [&mut layer.x_axis, &mut layer.y_axis] {
                if matches!(axis.tick_label_source, TickLabelSource::Column { .. }) {
                    axis.tick_label_source = TickLabelSource::Auto;
                }
                axis.rug.plot = None;
                axis.rug_opposite.plot = None;
            }
        }
        Ok(UserTemplate { name: name.to_string(), graph: g })
    }

    /// The plots of the template as `(layer, plot)` pairs, layer by layer: slot `k` is the `k`-th pair.
    pub fn slots(&self) -> Vec<(usize, usize)> {
        self.graph.layers.iter().enumerate().flat_map(|(li, l)| (0..l.plots.len()).map(move |pi| (li, pi))).collect()
    }

    /// How many plots the template has (its slots).
    pub fn plot_count(&self) -> usize {
        self.graph.layers.iter().map(|l| l.plots.len()).sum()
    }
}

impl Project {
    /// Creates a graph in the look of `template` that plots the columns `ys` and returns its id.
    ///
    /// Column `i` takes slot `i % n` of the `n` slots of the template: its plot gets that slot's
    /// whole style, in the slot's layer, and finds its X, error and (for bubble and colour mapped
    /// plots) Z columns from the sheet as [`Project::add_plot`] does. Columns beyond the slots
    /// repeat the styles in turn and take their colours from the layer's palette in plot order;
    /// the layer's group increments are applied over all plots. When there are fewer columns than
    /// slots the unused slots are left out, and a linked layer left without plots (the right Y
    /// layer of a double-Y template given one column) is removed. A template without plots plots
    /// every column in the first layer with the layer's defaults. Axis ranges, text and drawn
    /// objects, the legend and the page are the template's. The graph's long name is that of the
    /// workbook of the first column.
    pub fn plot_with_user_template(
        &mut self,
        template: &UserTemplate,
        ys: &[ColumnId],
    ) -> Result<WindowId, ProjectError> {
        for y in ys {
            self.column(*y).ok_or(ProjectError::NoSuchColumn(*y))?;
        }
        let long_name = ys.first().and_then(|y| self.locate(*y)).map(|(b, _, _)| b.long_name.clone());
        let id = WindowId(self.fresh_id());
        let mut graph = template.graph.clone();
        graph.id = id;
        graph.name = self.unique_name("Graph");
        graph.long_name = long_name.unwrap_or_default();
        for layer in &mut graph.layers {
            layer.plots.clear();
            layer.plot_offsets.clear();
        }
        self.windows.push(crate::project::Window::Graph(graph));

        let slots = template.slots();
        for (i, y) in ys.iter().enumerate() {
            let Some(&(li, pi)) = slots.get(i % slots.len().max(1)) else {
                self.add_plot(id, 0, *y)?;
                continue;
            };
            let proto = &template.graph.layers[li].plots[pi];
            let ternary = matches!(template.graph.layers[li].coords, crate::graph::Coords::Ternary(_));
            let size_z =
                proto.size_from.and_then(|_| if ternary { self.ternary_z_for(*y) } else { self.z_column_for(*y) });
            let color_z = proto.color_from.and_then(|_| self.z_column_for(*y));
            self.add_plot(id, li, *y)?;
            let layer = &mut self.graph_mut(id).expect("the graph was just added").layers[li];
            let added = layer.plots.last_mut().expect("a plot was just added");
            let mut styled = proto.clone();
            styled.x = added.x;
            styled.y = *y;
            styled.y_error = added.y_error;
            styled.x_error = added.x_error;
            styled.size_from = size_z;
            styled.color_from = color_z;
            if i >= slots.len() {
                styled.auto_color = true;
            }
            *added = styled;

            // The placement of the slot's curve, for the column that takes it in this round.
            let slot = i % slots.len();
            let round_start = i - slot;
            let proto_layer = &template.graph.layers[li];
            if let Some(o) = proto_layer.plot_offsets.iter().find(|o| o.column == placeholder(slot)) {
                let mut o = *o;
                o.column = *y;
                o.follows = o.follows.and_then(slot_of).and_then(|j| ys.get(round_start + j).copied());
                layer.plot_offsets.push(o);
            }
        }

        let graph = self.graph_mut(id).expect("the graph was just added");
        for layer in &mut graph.layers {
            layer.apply_group_increments();
        }
        // Drop linked layers that have no plot and that no other layer follows, last first.
        for li in (1..graph.layers.len()).rev() {
            let followed = graph.layers.iter().any(|l| l.link.as_ref().is_some_and(|k| k.parent == li));
            if graph.layers[li].plots.is_empty() && graph.layers[li].link.is_some() && !followed {
                graph.layers.remove(li);
                for l in &mut graph.layers[li..] {
                    if let Some(k) = l.link.as_mut().filter(|k| k.parent > li) {
                        k.parent -= 1;
                    }
                }
            }
        }
        Ok(id)
    }
}

#[cfg(test)]
mod tests;
