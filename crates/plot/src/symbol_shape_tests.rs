//! Origin 2016's symbol shapes 9-20, interiors and fill colour (graphing 5.4, measured in T323).
use super::*;
use crate::scene::{Item, Scene};
use sheaf_core::graph::{PlotKind, SymbolInterior, SymbolShape, SymbolStyle};
use sheaf_core::template::Template;
use sheaf_core::{Color, ColumnData, Designation};

const BACKGROUND: Color = Color::rgb(255, 255, 0);
const R: f64 = 15.0;

/// One scatter plot of `n` points at y = 1, x = 1..n, blue 30 pt symbols on a yellow layer.
fn project(n: usize, symbol: impl FnOnce(&mut SymbolStyle)) -> (Project, WindowId) {
    let mut p = Project::new();
    let b = p.add_workbook("Data");
    p.add_column(b, 0, Designation::X, ColumnData::numeric((1..=n).map(|i| i as f64).collect())).unwrap();
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0; n])).unwrap();
    let g = p.add_graph(Template::Scatter);
    p.add_plot(g, 0, y).unwrap();
    let layer = &mut p.graph_mut(g).unwrap().layers[0];
    layer.background = Some(BACKGROUND);
    layer.legend.visible = false;
    layer.x_axis.min = Some(0.0);
    layer.x_axis.max = Some(n as f64 + 1.0);
    layer.y_axis.min = Some(0.0);
    layer.y_axis.max = Some(2.0);
    let plot = &mut layer.plots[0];
    plot.kind = PlotKind::Scatter;
    plot.symbol.color = Color::BLUE;
    plot.symbol.shape = SymbolShape::Circle;
    plot.symbol.filled = true;
    plot.symbol.size_pt = 2.0 * R;
    plot.symbol.edge_width_pt = 1.5;
    symbol(&mut plot.symbol);
    (p, g)
}

/// The colour at `(dx, dy)` pt from the centre of the first symbol.
fn at(p: &Project, g: WindowId, d: &[(f64, f64)]) -> Vec<[u8; 3]> {
    let geom = &layout_geometry(p, g).unwrap()[0];
    let (cx, cy) = geom.data_to_page(1.0, 1.0);
    let r = render_raster(p, g, 2.0).unwrap();
    d.iter()
        .map(|(dx, dy)| {
            let i = ((((cy + dy) * 2.0) as u32 * r.width + ((cx + dx) * 2.0) as u32) * 4) as usize;
            [r.rgba[i], r.rgba[i + 1], r.rgba[i + 2]]
        })
        .collect()
}

const BLUE: [u8; 3] = [0, 0, 255];
const WHITE: [u8; 3] = [255, 255, 255];
const YELLOW: [u8; 3] = [255, 255, 0];

fn with(interior: SymbolInterior, shape: SymbolShape) -> (Project, WindowId) {
    project(1, |s| {
        s.shape = shape;
        s.filled = false;
        s.interior = Some(interior);
    })
}

#[test]
fn open_is_white_inside_and_hollow_shows_what_is_beneath() {
    // The edge is anti-aliased: mostly blue.
    let edge = |c: [u8; 3]| c[2] == 255 && c[0] < 128 && c[1] < 128;
    let (p, g) = with(SymbolInterior::Open, SymbolShape::Circle);
    let c = at(&p, g, &[(0.0, 0.0), (R, 0.0), (R + 3.0, 0.0)]);
    assert!(c[0] == WHITE && edge(c[1]) && c[2] == YELLOW, "{c:?}");
    let (p, g) = with(SymbolInterior::Hollow, SymbolShape::Circle);
    let c = at(&p, g, &[(0.0, 0.0), (R, 0.0)]);
    assert!(c[0] == YELLOW && edge(c[1]), "{c:?}");
    // A plain unfilled symbol (no interior) stays as before: an outline over the background.
    let (p, g) = project(1, |s| s.filled = false);
    assert_eq!(at(&p, g, &[(0.0, 0.0)]), [YELLOW]);
    // The fill colour paints the open part.
    let (mut p, g) = with(SymbolInterior::Open, SymbolShape::Square);
    p.graph_mut(g).unwrap().layers[0].plots[0].symbol.fill_color = Some(Color::rgb(0, 255, 0));
    assert_eq!(at(&p, g, &[(0.0, 0.0)]), [[0, 255, 0]]);
}

#[test]
fn halves_fill_their_side_of_the_shape_in_the_edge_colour() {
    let h = R / 2.0;
    for (shape, interior, filled, open) in [
        (SymbolShape::Square, SymbolInterior::HalfUp, (0.0, -h), (0.0, h)),
        (SymbolShape::Square, SymbolInterior::HalfDown, (0.0, h), (0.0, -h)),
        (SymbolShape::Circle, SymbolInterior::HalfLeft, (-h, 0.0), (h, 0.0)),
        (SymbolShape::Circle, SymbolInterior::HalfRight, (h, 0.0), (-h, 0.0)),
        (SymbolShape::Hexagon, SymbolInterior::HalfRight, (h, 0.0), (-h, 0.0)),
        (SymbolShape::TriangleUp, SymbolInterior::HalfUp, (0.0, -0.5 * h), (0.0, 0.6 * h)),
    ] {
        let (p, g) = with(interior, shape);
        assert_eq!(at(&p, g, &[filled, open]), [BLUE, WHITE], "{shape:?} {interior:?}");
    }
}

#[test]
fn centre_marks_reach_the_edge_and_the_x_reaches_the_corners() {
    let (p, g) = with(SymbolInterior::PlusCenter, SymbolShape::Circle);
    assert_eq!(at(&p, g, &[(R / 2.0, 0.0), (0.0, R / 2.0), (R / 2.0, R / 2.0)]), [BLUE, BLUE, WHITE]);
    let (p, g) = with(SymbolInterior::XCenter, SymbolShape::Circle);
    assert_eq!(at(&p, g, &[(0.9 * R, 0.9 * R), (R / 2.0, 0.0)]), [BLUE, WHITE], "beyond the circle, as Origin");
    let (p, g) = with(SymbolInterior::MinusCenter, SymbolShape::Square);
    assert_eq!(at(&p, g, &[(R / 2.0, 0.0), (0.0, R / 2.0)]), [BLUE, WHITE]);
    let (p, g) = with(SymbolInterior::BarCenter, SymbolShape::Square);
    assert_eq!(at(&p, g, &[(0.0, R / 2.0), (R / 2.0, 0.0)]), [BLUE, WHITE]);
    let (p, g) = with(SymbolInterior::DotCenter, SymbolShape::Circle);
    assert_eq!(at(&p, g, &[(0.0, 0.0), (R / 3.0, 0.0)]), [BLUE, WHITE]);
}

#[test]
fn the_new_shapes_have_their_outlines() {
    let solid = |shape| project(1, |s| s.shape = shape);
    let (p, g) = solid(SymbolShape::TriangleLeft);
    assert_eq!(at(&p, g, &[(-R / 2.0, 0.0), (R / 4.0, -0.9 * R)]), [BLUE, YELLOW]);
    let (p, g) = solid(SymbolShape::TriangleRight);
    assert_eq!(at(&p, g, &[(R / 2.0, 0.0), (-R / 4.0, -0.9 * R)]), [BLUE, YELLOW]);
    let (p, g) = solid(SymbolShape::Hexagon);
    assert_eq!(
        at(&p, g, &[(0.0, -0.9 * R), (0.8 * R, 0.0), (0.0, 0.0), (0.9 * R, -0.9 * R)]),
        [BLUE, BLUE, BLUE, YELLOW]
    );
    let (p, g) = solid(SymbolShape::Pentagon);
    assert_eq!(at(&p, g, &[(0.0, -0.9 * R), (0.0, 0.0), (-0.9 * R, 0.9 * R)]), [BLUE, BLUE, YELLOW]);
    let (p, g) = solid(SymbolShape::FivePointStar);
    assert_eq!(
        at(&p, g, &[(0.0, 0.0), (0.0, -0.9 * R), (0.0, 0.6 * R)]),
        [BLUE, BLUE, YELLOW],
        "notch between the lower points"
    );
    let (p, g) = solid(SymbolShape::HBar);
    assert_eq!(at(&p, g, &[(0.8 * R, 0.0), (0.0, R / 2.0)]), [BLUE, YELLOW]);
    let (p, g) = solid(SymbolShape::VBar);
    assert_eq!(at(&p, g, &[(0.0, 0.8 * R), (R / 2.0, 0.0)]), [BLUE, YELLOW]);
    // The sphere: coloured, lighter towards the upper left.
    let (p, g) = solid(SymbolShape::Sphere);
    let c = at(&p, g, &[(0.6 * R, 0.6 * R), (-0.32 * R, -0.32 * R), (1.2 * R, 0.0)]);
    assert_eq!((c[0], c[2]), (BLUE, YELLOW));
    assert!(c[1][0] > 150 && c[1][2] == 255, "highlight {:?}", c[1]);
}

fn texts(s: &Scene) -> Vec<String> {
    s.layers[0]
        .plots
        .iter()
        .chain(&s.layers[0].overlay)
        .filter_map(|i| match i {
            Item::Text(t) if t.color == Color::BLUE => Some(t.text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn text_shapes_write_the_row_number_or_letter_and_the_legend_shows_one_or_a() {
    for (shape, want) in [
        (SymbolShape::Number, ["1", "2", "3"]),
        (SymbolShape::CapitalLetter, ["A", "B", "C"]),
        (SymbolShape::SmallLetter, ["a", "b", "c"]),
    ] {
        let (mut p, g) = project(3, |s| s.shape = shape);
        assert_eq!(texts(&graph_scene(&p, g).unwrap()), want, "{shape:?}");
        p.graph_mut(g).unwrap().layers[0].legend.visible = true;
        let all = texts(&graph_scene(&p, g).unwrap());
        assert_eq!(all.len(), 4);
        assert_eq!(all[3], if shape == SymbolShape::Number { "1" } else { "a" }, "Origin 2016's legend sample");
    }
    assert_eq!(SymbolShape::SmallLetter.text_at(26), Some("a".into()), "letters start again after z");
}
