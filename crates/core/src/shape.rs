//! Drawn objects of a graph layer: lines, arrows, rectangles and ellipses (Origin's Line, Arrow,
//! Rectangle and Circle tools). Their places are fractions of the layer frame, like a text label
//! (graphing spec section 16.2): `(0, 0)` is the top left corner of the frame, `(1, 1)` the
//! bottom right, values outside 0 to 1 are allowed.

use crate::Color;
use crate::graph::Dash;
use serde::{Deserialize, Serialize};

/// What a drawn object is.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ShapeKind {
    /// A straight line from `(x1, y1)` to `(x2, y2)`.
    #[default]
    Line,
    /// A line with an arrow head at its end `(x2, y2)` (and at its start, see `Shape::head_at_start`).
    Arrow,
    /// The rectangle with corners `(x1, y1)` and `(x2, y2)`.
    Rectangle,
    /// The ellipse inscribed in that rectangle.
    Ellipse,
}

impl ShapeKind {
    pub const ALL: [ShapeKind; 4] = [ShapeKind::Line, ShapeKind::Arrow, ShapeKind::Rectangle, ShapeKind::Ellipse];

    /// A line or an arrow (has two end points, not a box).
    pub fn is_linear(self) -> bool {
        matches!(self, ShapeKind::Line | ShapeKind::Arrow)
    }

    /// Origin's name of the tool.
    pub fn name(self) -> &'static str {
        match self {
            ShapeKind::Line => "Line",
            ShapeKind::Arrow => "Arrow",
            ShapeKind::Rectangle => "Rectangle",
            ShapeKind::Ellipse => "Circle",
        }
    }
}

/// One drawn object of a layer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Shape {
    pub kind: ShapeKind,
    /// First point (fractions of the layer frame): the start of a line, one corner of a box.
    pub x1: f64,
    pub y1: f64,
    /// Second point: the end of a line, the opposite corner of a box.
    pub x2: f64,
    pub y2: f64,
    pub color: Color,
    pub width_pt: f64,
    #[serde(default)]
    pub dash: Dash,
    /// Fill of a rectangle or ellipse; `None` leaves it empty (the default).
    #[serde(default)]
    pub fill: Option<Color>,
    /// Arrow head length in points, measured along the line (arrows only).
    #[serde(default = "default_head")]
    pub head_pt: f64,
    /// The arrow also has a head at its start.
    #[serde(default)]
    pub head_at_start: bool,
}

fn default_head() -> f64 {
    Shape::DEFAULT_HEAD_PT
}

impl Shape {
    /// Arrow head length of a new arrow (Sheaf's choice; Origin's default is unmeasured).
    pub const DEFAULT_HEAD_PT: f64 = 9.0;
    /// Line width of a new object.
    pub const DEFAULT_WIDTH_PT: f64 = 1.0;

    /// A new black object of `kind` with the given corners.
    pub fn new(kind: ShapeKind, (x1, y1): (f64, f64), (x2, y2): (f64, f64)) -> Shape {
        Shape {
            kind,
            x1,
            y1,
            x2,
            y2,
            color: Color::BLACK,
            width_pt: Self::DEFAULT_WIDTH_PT,
            dash: Dash::Solid,
            fill: None,
            head_pt: Self::DEFAULT_HEAD_PT,
            head_at_start: false,
        }
    }

    /// Moves the whole object by `(dx, dy)` fractions.
    pub fn translate(&mut self, dx: f64, dy: f64) {
        self.x1 += dx;
        self.x2 += dx;
        self.y1 += dy;
        self.y2 += dy;
    }
}

/// `(x2, y2)` after Shift is held while dragging from `(x1, y1)`, in page points `(w, h)` of the
/// layer frame (fractions are not isotropic): a line or arrow snaps to the nearest of horizontal,
/// vertical and 45 degrees; a rectangle becomes a square and an ellipse a circle (the longer side
/// wins).
pub fn constrain(kind: ShapeKind, from: (f64, f64), to: (f64, f64), frame: (f64, f64)) -> (f64, f64) {
    let (dx, dy) = ((to.0 - from.0) * frame.0, (to.1 - from.1) * frame.1);
    let (nx, ny) = if kind.is_linear() {
        let angle = dy.atan2(dx).to_degrees();
        let snapped = (angle / 45.0).round() * 45.0;
        let len = dx.hypot(dy);
        match snapped.rem_euclid(180.0) as i32 {
            0 => (len * snapped.to_radians().cos().signum(), 0.0),
            90 => (0.0, len * snapped.to_radians().sin().signum()),
            _ => {
                let s = len / std::f64::consts::SQRT_2;
                (s * dx.signum(), s * dy.signum())
            }
        }
    } else {
        let s = dx.abs().max(dy.abs());
        (s * dx.signum(), s * dy.signum())
    };
    (from.0 + nx / frame.0, from.1 + ny / frame.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shift_snaps_lines_to_axes_and_diagonals() {
        let frame = (400.0, 200.0);
        // Nearly horizontal (about 5 degrees): horizontal, same length.
        let (x, y) = constrain(ShapeKind::Line, (0.1, 0.5), (0.6, 0.55), frame);
        assert!((y - 0.5).abs() < 1e-12);
        let len = (0.5f64 * 400.0).hypot(0.05 * 200.0);
        assert!(((x - 0.1) * 400.0 - len).abs() < 1e-9);
        // Nearly vertical, upwards.
        let (x, y) = constrain(ShapeKind::Arrow, (0.5, 0.5), (0.51, 0.1), frame);
        assert!((x - 0.5).abs() < 1e-12 && y < 0.1 + 1e-3);
        // Diagonal: equal page distances.
        let (x, y) = constrain(ShapeKind::Line, (0.2, 0.2), (0.4, 0.5), frame);
        assert!((((x - 0.2) * 400.0).abs() - ((y - 0.2) * 200.0).abs()).abs() < 1e-9);
        assert!(x > 0.2 && y > 0.2);
    }

    #[test]
    fn shift_makes_boxes_square_in_page_points() {
        let (x, y) = constrain(ShapeKind::Rectangle, (0.1, 0.1), (0.5, 0.2), (400.0, 200.0));
        // dx = 160 pt, dy = 20 pt: the longer side, 160 pt, wins on both axes.
        assert!((x - 0.5).abs() < 1e-12);
        assert!(((y - 0.1) * 200.0 - 160.0).abs() < 1e-9);
        let (x, y) = constrain(ShapeKind::Ellipse, (0.5, 0.5), (0.4, 0.9), (400.0, 200.0));
        assert!(((x - 0.5) * 400.0).abs() - ((y - 0.5) * 200.0).abs() < 1e-9);
        assert!(x < 0.5 && y > 0.5);
    }

    #[test]
    fn a_shape_round_trips_and_old_json_gets_defaults() {
        let mut s = Shape::new(ShapeKind::Arrow, (0.1, 0.2), (0.7, 0.3));
        s.head_at_start = true;
        s.fill = Some(Color::BLACK);
        s.dash = Dash::Dash;
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(serde_json::from_str::<Shape>(&json).unwrap(), s);
        let minimal = r##"{"kind":"rectangle","x1":0,"y1":0,"x2":1,"y2":1,
            "color":"#000000","width_pt":2}"##;
        let r: Shape = serde_json::from_str(minimal).unwrap();
        assert_eq!((r.fill, r.dash, r.head_pt, r.head_at_start), (None, Dash::Solid, 9.0, false));
    }

    #[test]
    fn a_layer_without_drawn_objects_saves_no_key_and_old_files_load_unchanged() {
        use crate::template::Template;
        let mut project = crate::Project::default();
        let id = project.add_graph(Template::Line);
        let layer = &project.graph(id).unwrap().layers[0];
        let mut value = serde_json::to_value(layer).unwrap();
        assert!(value.get("shapes").is_none(), "an empty list is not written");
        // A file written before drawn objects existed has no "shapes" key.
        let old: crate::graph::Layer = serde_json::from_value(value.clone()).unwrap();
        assert!(old.shapes.is_empty());
        assert_eq!(serde_json::to_value(&old).unwrap(), value);
        // With objects the key appears and round trips.
        let mut with = old.clone();
        with.shapes.push(Shape::new(ShapeKind::Line, (0.0, 0.0), (1.0, 1.0)));
        value = serde_json::to_value(&with).unwrap();
        assert_eq!(value["shapes"].as_array().map(Vec::len), Some(1));
        assert_eq!(serde_json::from_value::<crate::graph::Layer>(value).unwrap().shapes, with.shapes);
    }
}
