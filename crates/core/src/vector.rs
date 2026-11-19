//! Plot > Specialized > Vector XYAM and Vector XYXY (T107), as Origin 2016's, measured on 2026-11-16
//! (`docs/spec/graphing.md`, "Vector plots"):
//!
//! - **XYAM** takes X, Y, an angle column and a magnitude column (the three Y columns of the selection in
//!   order). Each arrow starts at (X, Y) by default (Position Tail) and points at the angle, counterclockwise from +X
//!   (Convention Polar). Origin's default angle unit ("System") is radians: an angle of 90 drew an arrow at
//!   116.6 degrees (90 rad). The length is magnitude x multiplier in points (Magnitude in real world space
//!   off); Origin chose the multiplier from the data (0.16 for magnitudes up to 100, 9.6 for magnitudes up to
//!   2), Sheaf takes 16 points for the largest magnitude.
//! - **XYXY** takes columns designated X, Y, X, Y and draws an arrow from (X1, Y1) to (X2, Y2); Origin
//!   refuses other designations ("you need to select columns designated as 'XYXY'"). The axes take in both
//!   ends.
//! - Both draw black 0.5 pt lines with a closed arrowhead of 30 degrees; the head is 45 (XYAM) or 65 (XYXY)
//!   in Origin's dialog, which Sheaf reads as tenths of a point (4.5 and 6.5 pt).

use crate::graph::PlotKind;
use crate::ids::{ColumnId, WindowId};
use crate::project::{Project, ProjectError};
use crate::template::Template;
use serde::{Deserialize, Serialize};

/// The part of an XYAM vector fixed at its data coordinate.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VectorPosition {
    #[default]
    Tail,
    Midpoint,
    Head,
}

/// The direction of zero and the rotation sense of the angles in an XYAM plot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VectorAngleConvention {
    #[default]
    Polar,
    Meteorological,
    Custom {
        offset_deg: f64,
        clockwise: bool,
    },
}

impl VectorAngleConvention {
    fn is_polar(&self) -> bool {
        *self == Self::Polar
    }

    /// The zero direction in degrees counterclockwise from +X, and whether data angles turn clockwise.
    pub fn offset_and_clockwise(self) -> (f64, bool) {
        match self {
            Self::Polar => (0.0, false),
            Self::Meteorological => (90.0, true),
            Self::Custom { offset_deg, clockwise } => (offset_deg, clockwise),
        }
    }
}

/// The extra columns and settings of a vector plot (in [`crate::graph::PlotDecor::vector`]).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct VectorSpec {
    /// XYAM: the angle column; XYXY: the X of the arrow's end.
    pub a: ColumnId,
    /// XYAM: the magnitude column; XYXY: the Y of the arrow's end.
    pub b: ColumnId,
    /// XYAM: length per unit of magnitude, in page points or data units.
    #[serde(default = "one")]
    pub multiplier: f64,
    /// XYAM: the angles are in degrees (Origin's "System" unit is radians).
    #[serde(default)]
    pub degrees: bool,
    #[serde(default, skip_serializing_if = "VectorAngleConvention::is_polar")]
    pub convention: VectorAngleConvention,
    /// XYAM: interpret the scaled magnitude in X and Y data units.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub real_world_space: bool,
    #[serde(default)]
    pub position: VectorPosition,
    /// The arrowhead's length in points and its opening angle in degrees.
    pub head_pt: f64,
    pub head_angle_deg: f64,
    #[serde(default)]
    pub open_head: bool,
}

impl VectorSpec {
    /// An XYAM angle in radians counterclockwise from +X.
    pub fn xyam_angle(&self, value: f64) -> f64 {
        let angle = if self.degrees { value.to_radians() } else { value };
        let (offset, clockwise) = self.convention.offset_and_clockwise();
        offset.to_radians() + if clockwise { -angle } else { angle }
    }
}

fn one() -> f64 {
    1.0
}

/// The multiplier that makes the longest arrow 16 points (Sheaf's rule; Origin's measured 0.16 for 100).
pub fn auto_multiplier(magnitudes: &[f64]) -> f64 {
    let max = magnitudes.iter().filter(|m| m.is_finite()).fold(0.0_f64, |a, m| a.max(m.abs()));
    if max > 0.0 { 16.0 / max } else { 1.0 }
}

impl Project {
    /// Plots vector arrows: XYAM from the Y columns `ys` taken three at a time (Y, angle, magnitude), XYXY
    /// from `ys` taken two at a time (Y1, Y2) with the X column of each. One graph, one plot per group.
    pub fn plot_vectors(&mut self, template: Template, ys: &[ColumnId]) -> Result<WindowId, ProjectError> {
        let xyam = template == Template::VectorXyam;
        let group = if xyam { 3 } else { 2 };
        if ys.len() < group {
            return Err(ProjectError::VectorColumns(if xyam {
                "a vector XYAM plot needs X, Y, angle and magnitude columns; select a Y column and two after it"
            } else {
                "a vector XYXY plot needs columns designated X, Y, X, Y"
            }));
        }
        let id = self.add_graph(template);
        for chunk in ys.chunks_exact(group) {
            let y = chunk[0];
            let (_, sheet, _) = self.locate(y).ok_or(ProjectError::NoSuchColumn(y))?;
            let x = sheet.column_index(y).and_then(|i| sheet.x_for(i)).map(|c| c.id);
            let spec = if xyam {
                let mags = self.numeric(chunk[2]).unwrap_or_default().to_vec();
                VectorSpec {
                    a: chunk[1],
                    b: chunk[2],
                    multiplier: auto_multiplier(&mags),
                    degrees: false,
                    convention: VectorAngleConvention::Polar,
                    real_world_space: false,
                    position: VectorPosition::Tail,
                    head_pt: 4.5,
                    head_angle_deg: 30.0,
                    open_head: false,
                }
            } else {
                let (_, s2, _) = self.locate(chunk[1]).ok_or(ProjectError::NoSuchColumn(chunk[1]))?;
                let x2 = s2
                    .column_index(chunk[1])
                    .and_then(|i| s2.columns[..i].iter().rev().find(|c| c.designation == crate::column::Designation::X))
                    .map(|c| c.id);
                match (x, x2) {
                    (Some(x1), Some(x2)) if x1 != x2 => {}
                    _ => {
                        return Err(ProjectError::VectorColumns(
                            "a vector XYXY plot needs columns designated X, Y, X, Y",
                        ));
                    }
                }
                VectorSpec {
                    a: x2.expect("checked"),
                    b: chunk[1],
                    multiplier: 1.0,
                    degrees: false,
                    convention: VectorAngleConvention::Polar,
                    real_world_space: false,
                    position: VectorPosition::Tail,
                    head_pt: 6.5,
                    head_angle_deg: 30.0,
                    open_head: false,
                }
            };
            self.add_plot_xy(id, 0, x, y, None)?;
            let plot = self.graph_mut(id).and_then(|g| g.layers[0].plots.last_mut()).expect("a plot was just added");
            plot.kind = if xyam { PlotKind::VectorXyam } else { PlotKind::VectorXyxy };
            plot.line.color = crate::Color::BLACK;
            plot.line.width_pt = 0.5;
            plot.auto_color = false;
            plot.decor.vector = Some(spec);
        }
        if let Some(name) = ys.first().and_then(|y| self.locate(*y)).map(|(b, _, _)| b.title_for_graph())
            && let Some(g) = self.graph_mut(id)
        {
            g.long_name = name;
        }
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::{ColumnData, Designation};

    #[test]
    fn old_vectors_keep_a_tail_anchor_and_closed_head() {
        let legacy = serde_json::json!({"a": 1, "b": 2, "head_pt": 4.5, "head_angle_deg": 30.0});
        let mut spec: VectorSpec = serde_json::from_value(legacy).unwrap();
        assert_eq!((spec.position, spec.open_head), (VectorPosition::Tail, false));
        assert_eq!(spec.convention, VectorAngleConvention::Polar);
        assert!(!spec.real_world_space);
        let serialized = serde_json::to_value(spec).unwrap();
        assert!(serialized.get("convention").is_none());
        assert!(serialized.get("real_world_space").is_none());
        spec.position = VectorPosition::Midpoint;
        spec.open_head = true;
        spec.real_world_space = true;
        assert_eq!(serde_json::from_value::<VectorSpec>(serde_json::to_value(spec).unwrap()).unwrap(), spec);
    }

    #[test]
    fn vector_angle_conventions_round_trip_and_use_the_data_angle_unit() {
        use std::f64::consts::FRAC_PI_2;
        let legacy = serde_json::json!({"a": 1, "b": 2, "head_pt": 4.5, "head_angle_deg": 30.0});
        let mut spec: VectorSpec = serde_json::from_value(legacy).unwrap();
        spec.convention = VectorAngleConvention::Meteorological;
        assert_eq!(spec.xyam_angle(0.0), FRAC_PI_2);
        assert_eq!(spec.xyam_angle(FRAC_PI_2), 0.0);
        spec.degrees = true;
        assert_eq!(spec.xyam_angle(90.0), 0.0);
        spec.convention = VectorAngleConvention::Custom { offset_deg: 30.0, clockwise: true };
        let restored: VectorSpec = serde_json::from_value(serde_json::to_value(spec).unwrap()).unwrap();
        assert_eq!(restored, spec);
        assert!((restored.xyam_angle(60.0).to_degrees() + 30.0).abs() < 1e-12);
        spec.convention = VectorAngleConvention::Custom { offset_deg: 30.0, clockwise: false };
        assert!((spec.xyam_angle(60.0).to_degrees() - 90.0).abs() < 1e-12);
    }

    #[test]
    fn xyam_takes_y_angle_magnitude_and_xyxy_needs_x_y_x_y() {
        let mut p = Project::new();
        let b = p.add_workbook("");
        let x = p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0, 3.0])).unwrap();
        let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0, 3.0])).unwrap();
        let a = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![0.0, 90.0, 45.0])).unwrap();
        let m = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![50.0, 100.0, 100.0])).unwrap();
        let g = p.plot_vectors(Template::VectorXyam, &[y, a, m]).unwrap();
        let plot = &p.graph(g).unwrap().layers[0].plots[0];
        assert_eq!((plot.kind, plot.x, plot.y), (PlotKind::VectorXyam, Some(x), y));
        let v = plot.decor.vector.unwrap();
        assert_eq!((v.a, v.b, v.degrees), (a, m, false), "radians, as Origin's System unit");
        assert!((v.multiplier - 0.16).abs() < 1e-12, "16 pt for the largest magnitude, as Origin's 0.16 for 100");
        assert_eq!((plot.line.width_pt, v.head_pt, v.head_angle_deg), (0.5, 4.5, 30.0));
        // XYXY with A(X) B(Y) C(Y) D(Y): no second X column.
        assert!(p.plot_vectors(Template::VectorXyxy, &[y, a]).is_err());
        p.column_mut(a).unwrap().designation = Designation::X;
        let g = p.plot_vectors(Template::VectorXyxy, &[y, m]).unwrap();
        let plot = &p.graph(g).unwrap().layers[0].plots[0];
        let v = plot.decor.vector.unwrap();
        assert_eq!((plot.x, plot.y, v.a, v.b, v.head_pt), (Some(x), y, a, m, 6.5));
    }
}
