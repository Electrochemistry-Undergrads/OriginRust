use super::*;
use crate::scene::{Cap, Item, Join, LayerScene, Scene, Stroke};
use sheaf_core::graph::{Dash, LineStyle};
use sheaf_core::template::Template;
use sheaf_core::{Color, ColumnData, Designation};

fn project(style: Dash, width: f64) -> (Project, WindowId) {
    let mut p = Project::new();
    let book = p.add_workbook("Data");
    let x = p.add_column(book, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
    let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![1.0, 1.0])).unwrap();
    let g = p.add_graph(Template::Line);
    p.add_plot_xy(g, 0, Some(x), y, None).unwrap();
    let line = &mut p.graph_mut(g).unwrap().layers[0].plots[0].line;
    line.dash = style;
    line.width_pt = width;
    line.color = Color::BLUE;
    (p, g)
}

fn data_stroke(p: &Project, g: WindowId) -> Stroke {
    layout::build_scene(p, g, &FixedMetrics).unwrap().layers[0]
        .plots
        .iter()
        .find_map(|item| match item {
            Item::Path { stroke: Some(s), .. } if s.color == Color::BLUE => Some(s.clone()),
            _ => None,
        })
        .expect("the verified two-point vector produces a line")
}

#[test]
fn thin_dash_data_lines_and_legend_samples_use_the_same_measured_pattern() {
    for style in Dash::ALL.into_iter().skip(1) {
        for width in [0.5, 1.0] {
            let (p, g) = project(style, width);
            let svg = render_svg_with(&p, g, &FixedMetrics).unwrap();
            let doc = roxmltree::Document::parse(&svg).unwrap();
            let strokes: Vec<_> = doc
                .descendants()
                .filter(|n| n.has_tag_name("path") && n.attribute("stroke") == Some("#0000FF"))
                .collect();
            assert_eq!(strokes.len(), 2, "data line and legend sample: {style:?}, {width}");
            assert_eq!(strokes[0].attribute("stroke-dasharray"), strokes[1].attribute("stroke-dasharray"));
            let s = data_stroke(&p, g);
            assert_eq!(s.width, width);
            assert_eq!(s.cap, Cap::Butt);
            assert!(s.dash.iter().sum::<f64>() > style.pattern().iter().sum::<f64>() * width);
        }
    }
}

#[test]
fn thin_dash_raster_runs_match_origin_samples_at_both_resolutions() {
    // Visible on/off lengths, independently measured along verified blue horizontal
    // lines at 6400 px. Keep these at measurement precision rather than duplicating
    // the rounded interpolation table. A pixel at the lower resolution is 0.125 pt.
    let cases: &[(Dash, &[f64], &[f64])] = &[
        (Dash::Dash, &[3.243, 2.397], &[5.885, 4.315]),
        (Dash::Dot, &[1.323, 2.157], &[2.283, 3.837]),
        (Dash::DashDot, &[3.844, 0.958, 1.322, 1.077], &[6.837, 1.797, 2.288, 1.917]),
        (Dash::DashDotDot, &[3.001, 0.959, 1.323, 0.959, 1.322, 1.077], &[5.404, 1.794, 2.284, 1.798, 2.284, 1.914]),
        (Dash::ShortDash, &[1.803, 0.837], &[3.244, 1.436]),
        (Dash::ShortDot, &[0.963, 0.357], &[1.805, 0.475]),
        (Dash::ShortDashDot, &[2.641, 0.478, 0.963, 0.599], &[4.69, 0.84, 1.81, 0.95]),
    ];
    for &(style, half, one) in cases {
        for (width, measured) in [(0.5, half), (1.0, one)] {
            let (p, g) = project(style, width);
            let stroke = data_stroke(&p, g);
            let scene = Scene {
                width: 600.0,
                height: 12.0,
                background: Color::WHITE,
                layers: vec![LayerScene {
                    clip: Rect { x: 0.0, y: 0.0, w: 600.0, h: 12.0 },
                    grid: vec![],
                    plots: vec![Item::Path { d: "M0 6L600 6".into(), fill: None, stroke: Some(stroke) }],
                    overlay: vec![],
                    underlay_len: 0,
                    objects: vec![],
                }],
            };
            for scale in [8.0, 16.0] {
                let r = render_scene_raster(&scene, scale).unwrap();
                let y = (6.0 * scale) as usize;
                let row = &r.rgba[y * r.width as usize * 4..(y + 1) * r.width as usize * 4];
                let mut runs = Vec::new();
                let mut was_ink = true;
                let mut len = 0;
                for pixel in row.as_chunks::<4>().0 {
                    let ink = pixel[0] < 128 && pixel[1] < 128 && pixel[2] > 200;
                    if ink != was_ink {
                        runs.push(len);
                        was_ink = ink;
                        len = 0;
                    }
                    len += 1;
                }
                // Exclude both edge periods, which can be truncated by clipping.
                for (i, &expected) in measured.iter().enumerate() {
                    let samples: Vec<_> = runs.iter().skip(i + measured.len()).step_by(measured.len()).collect();
                    assert!(samples.len() > 10);
                    let actual = samples[..samples.len() - 1].iter().map(|v| **v as f64).sum::<f64>()
                        / (samples.len() - 1) as f64
                        / scale;
                    assert!(
                        (actual - expected).abs() < 0.14,
                        "{style:?}, {width} pt, {scale} px/pt, component {i}: {actual} vs {expected}"
                    );
                }
            }
        }
    }
}

#[test]
fn thin_dash_interpolation_is_continuous_and_keeps_normal_width_strokes() {
    for style in Dash::ALL {
        for width in [1.5, 2.0, 3.0, 5.0] {
            let line = LineStyle { dash: style, ..LineStyle::solid(Color::RED, width) };
            let s = plots::line_stroke(&line);
            assert_eq!(s.dash, style.pattern().iter().map(|m| m * width).collect::<Vec<_>>());
            assert_eq!(s.width, width);
            assert_eq!(s.color, Color::RED);
            assert_eq!(s.join, Join::Round);
            assert_eq!(s.cap, if style == Dash::Solid { Cap::Round } else { Cap::Butt });
        }
        for width in [0.5, 1.0, 1.5] {
            let before = dash::lengths(style, width - 1e-7);
            let after = dash::lengths(style, width + 1e-7);
            for (a, b) in before.iter().zip(after) {
                assert!((a - b).abs() < 1e-5);
                assert!(*a > 0.0);
            }
        }
        for width in [0.01, 0.25] {
            assert_eq!(
                dash::lengths(style, width),
                dash::lengths(style, 0.5).iter().map(|v| v * (width / 0.5)).collect::<Vec<_>>()
            );
        }
    }
}
