//! Plot > Specialized > Time Series Explorer (T416; Origin 2024b's Time Series Explorer, Sheaf's own design): a
//! Zoom graph (T112) whose upper layer shows the whole series and whose region is a band the full height of the
//! layer covering one of `intervals` equal X intervals of the data; the lower layer shows that interval. The bar
//! above the graph steps the band to the previous or next interval, or plays through them.
//!
//! Sheaf's choices (Origin 2024b not measured here, Origin 2016 has none): the upper layer's X axis runs exactly
//! over the data's X extent so the intervals split the data evenly; 10 intervals to start with; the lower layer
//! keeps the whole Y range so successive intervals compare on one scale.

use crate::graph::Graph;
use crate::ids::{ColumnId, WindowId};
use crate::project::{Project, ProjectError};
use serde::{Deserialize, Serialize};

/// The number of intervals a new explorer splits the series into.
pub const DEFAULT_INTERVALS: usize = 10;

/// A graph's explorer (in [`Graph::explorer`]).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SeriesExplorer {
    /// How many X intervals the series is split into (at least 2; with a `length`, as many as it takes).
    pub intervals: usize,
    /// The interval shown in the lower layer, from 0.
    pub current: usize,
    /// Intervals of this X length (T453: 1 for a day of dates, 7 for a week) instead of equal parts; the last one
    /// may be shorter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length: Option<f64>,
}

impl SeriesExplorer {
    /// "Interval 3 of 10": what the bar above the graph says.
    pub fn caption(&self) -> String {
        format!("Interval {} of {}", self.current + 1, self.intervals)
    }
}

impl Graph {
    /// Puts the region band of an explorer graph on its current interval.
    fn place_explorer_band(&mut self) {
        let Some(e) = self.explorer else { return };
        let Some(k) = self.layers.get(1).and_then(|l| l.zoom) else { return };
        // The part of the frame one interval takes: a length over the X extent, else an equal part.
        let width = match (e.length, self.layers.first().map(|l| (l.x_axis.min, l.x_axis.max))) {
            (Some(len), Some((Some(lo), Some(hi)))) if hi > lo && len > 0.0 => len / (hi - lo),
            _ => 1.0 / e.intervals.max(1) as f64,
        };
        if let Some(r) = self.layers.first_mut().and_then(|l| l.shapes.get_mut(k)) {
            let start = e.current as f64 * width;
            (r.x1, r.x2) = (start.min(1.0), (start + width).min(1.0));
            (r.y1, r.y2) = (0.0, 1.0);
        }
    }
}

impl Project {
    /// Plots the Y columns `ys` (with their X columns) as a Time Series Explorer on the first of
    /// [`DEFAULT_INTERVALS`] intervals.
    pub fn plot_time_series_explorer(&mut self, ys: &[ColumnId]) -> Result<WindowId, ProjectError> {
        let id = self.plot_zoom(ys)?;
        // The upper X axis runs over the data's X extent exactly.
        let g = self.graph(id).expect("the graph was just added");
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for p in &g.layers[0].plots {
            let xs: Vec<f64> = match p.x {
                Some(x) => self.numeric(x).unwrap_or_default().to_vec(),
                None => (1..=self.numeric(p.y).map_or(0, |v| v.len())).map(|i| i as f64).collect(),
            };
            for v in xs.into_iter().filter(|v| v.is_finite()) {
                (lo, hi) = (lo.min(v), hi.max(v));
            }
        }
        let g = self.graph_mut(id).expect("the graph was just added");
        if hi > lo {
            g.layers[0].x_axis.min = Some(lo);
            g.layers[0].x_axis.max = Some(hi);
        }
        g.explorer = Some(SeriesExplorer { intervals: DEFAULT_INTERVALS, current: 0, length: None });
        g.place_explorer_band();
        Ok(id)
    }

    /// Shows interval `to` of graph `graph`'s explorer.
    pub fn explore_interval(&mut self, graph: WindowId, to: usize) -> Result<(), ProjectError> {
        let g = self.graph_mut(graph).ok_or(ProjectError::NoSuchWindow(graph))?;
        let e = g.explorer.as_mut().ok_or(ProjectError::NoExplorer)?;
        if to >= e.intervals {
            return Err(ProjectError::NoExplorer);
        }
        e.current = to;
        g.place_explorer_band();
        Ok(())
    }

    /// Splits graph `graph`'s series into intervals of X length `length` (T453), or with `None` back into equal
    /// parts (their last count). Too short a length (more than 1000 intervals) or a series without an X extent is
    /// refused. The interval that holds the start of the one shown before is shown.
    pub fn set_explorer_length(&mut self, graph: WindowId, length: Option<f64>) -> Result<(), ProjectError> {
        let g = self.graph_mut(graph).ok_or(ProjectError::NoSuchWindow(graph))?;
        let span = g.layers.first().and_then(|l| Some(l.x_axis.max? - l.x_axis.min?)).filter(|s| *s > 0.0);
        let e = g.explorer.as_mut().ok_or(ProjectError::NoExplorer)?;
        let old_width = match (e.length, span) {
            (Some(len), Some(s)) => len / s,
            _ => 1.0 / e.intervals.max(1) as f64,
        };
        let start = e.current as f64 * old_width;
        match length {
            Some(len) => {
                let span = span.ok_or(ProjectError::NoExplorer)?;
                let n = (span / len).ceil();
                if !(len > 0.0 && n.is_finite() && n <= 1000.0) {
                    return Err(ProjectError::ExplorerLength);
                }
                e.length = Some(len);
                e.intervals = (n as usize).max(1);
                e.current = ((start * span / len + 1e-9).floor() as usize).min(e.intervals - 1);
            }
            None => {
                e.length = None;
                e.intervals = e.intervals.clamp(2, 1000);
                e.current = ((start * e.intervals as f64 + 1e-9).floor() as usize).min(e.intervals - 1);
            }
        }
        g.place_explorer_band();
        Ok(())
    }

    /// Splits graph `graph`'s series into `intervals` (2 to 1000) and shows the interval that holds the start
    /// of the one shown before.
    pub fn set_explorer_intervals(&mut self, graph: WindowId, intervals: usize) -> Result<(), ProjectError> {
        let g = self.graph_mut(graph).ok_or(ProjectError::NoSuchWindow(graph))?;
        let e = g.explorer.as_mut().ok_or(ProjectError::NoExplorer)?;
        let intervals = intervals.clamp(2, 1000);
        e.current = (e.current * intervals / e.intervals.max(1)).min(intervals - 1);
        e.intervals = intervals;
        e.length = None;
        g.place_explorer_band();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::{ColumnData, Designation};
    use crate::template::Template;

    #[test]
    fn the_explorer_band_covers_one_interval_and_steps_through_them() {
        let mut p = Project::new();
        let b = p.add_workbook("");
        p.add_column(b, 0, Designation::X, ColumnData::numeric((0..=100).map(f64::from).collect())).unwrap();
        let y = p
            .add_column(b, 0, Designation::Y, ColumnData::numeric((0..=100).map(|i| f64::from(i).sin()).collect()))
            .unwrap();
        let g = p.plot_columns(Template::TimeSeriesExplorer, &[y]).unwrap();
        let graph = p.graph(g).unwrap();
        assert_eq!(graph.layers.len(), 2);
        assert_eq!((graph.layers[0].x_axis.min, graph.layers[0].x_axis.max), (Some(0.0), Some(100.0)));
        assert_eq!(graph.explorer, Some(SeriesExplorer { intervals: 10, current: 0, length: None }));
        let band = |p: &Project| {
            let s = &p.graph(g).unwrap().layers[0].shapes[0];
            (s.x1, s.x2, s.y1, s.y2)
        };
        assert_eq!(band(&p), (0.0, 0.1, 0.0, 1.0));
        p.explore_interval(g, 3).unwrap();
        let (x1, x2, ..) = band(&p);
        assert!((x1 - 0.3).abs() < 1e-12 && (x2 - 0.4).abs() < 1e-12);
        assert_eq!(p.graph(g).unwrap().explorer.unwrap().caption(), "Interval 4 of 10");
        assert!(p.explore_interval(g, 10).is_err());
        // Four intervals: the start of interval 4 of 10 (30 %) lies in interval 2 of 4.
        p.set_explorer_intervals(g, 4).unwrap();
        assert_eq!(p.graph(g).unwrap().explorer, Some(SeriesExplorer { intervals: 4, current: 1, length: None }));
        assert_eq!(band(&p), (0.25, 0.5, 0.0, 1.0));
        // It saves with the project.
        let back: Project = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!(back.graph(g).unwrap().explorer, p.graph(g).unwrap().explorer);
        // Intervals of a set length (T453): 30 X units over 0..100 make 4, the last one 10 long.
        p.set_explorer_length(g, Some(30.0)).unwrap();
        let e = p.graph(g).unwrap().explorer.unwrap();
        assert_eq!((e.intervals, e.current), (4, 0), "the start of 25..50 lies in 0..30");
        p.explore_interval(g, 3).unwrap();
        let (x1, x2, ..) = band(&p);
        assert!((x1 - 0.9).abs() < 1e-12 && (x2 - 1.0).abs() < 1e-12, "{x1} {x2}");
        assert!(p.set_explorer_length(g, Some(0.01)).is_err(), "too many intervals");
        p.set_explorer_length(g, None).unwrap();
        assert_eq!(p.graph(g).unwrap().explorer.unwrap().length, None);
        // A plain Zoom graph is no explorer.
        let z = p.plot_columns(Template::Zoom, &[y]).unwrap();
        assert!(p.explore_interval(z, 0).is_err());
    }
}
