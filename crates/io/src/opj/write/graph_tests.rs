//! Round-trip tests of the graph writer: a graph that is written and read back by the reader of
//! this crate comes back with everything the file holds, and what the file cannot hold is named in
//! the warnings. Origin itself opened the files that these tests make (spec section 13.9).

use super::*;
use crate::opj::{OpjImport, read_opj_from};
use sheaf_core::color::Color;
use sheaf_core::graph::*;
use sheaf_core::ids::{ColumnId, WindowId};
use sheaf_core::template::Template;
use std::io::Cursor;
use std::path::Path;

const TIME: f64 = 2_461_000.5;

fn nums(v: Vec<f64>) -> ColumnData {
    ColumnData::numeric(v)
}

/// A workbook "Src" with X (A), three Y columns (B, C, D), an error column (E) and a text column (F).
fn source() -> Project {
    let x: Vec<f64> = (0..11).map(|i| f64::from(i) * 0.5).collect();
    let mut s = Worksheet::new("Sheet1");
    s.columns = vec![
        Column::new(ColumnId(0), "A", Designation::X, nums(x.clone())).with_labels("Time", "s", ""),
        Column::new(ColumnId(0), "B", Designation::Y, nums(x.iter().map(|v| v * v).collect())),
        Column::new(ColumnId(0), "C", Designation::Y, nums(x.iter().map(|v| 50.0 - 4.0 * v).collect())),
        Column::new(ColumnId(0), "D", Designation::Y, nums(x.iter().map(|v| 10f64.powf(*v / 2.0)).collect())),
        Column::new(ColumnId(0), "E", Designation::YError, nums(vec![1.0; 11])),
        Column::new(
            ColumnId(0),
            "F",
            Designation::X,
            ColumnData::text(
                ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k"].iter().map(|s| s.to_string()).collect(),
            ),
        ),
    ];
    let mut b = Workbook::new(WindowId(0), "Src");
    b.sheets = vec![s];
    let mut p = Project::new();
    p.adopt_named_workbook(b);
    p
}

fn ids(p: &Project) -> Vec<ColumnId> {
    p.workbooks().next().unwrap().sheets[0].columns.iter().map(|c| c.id).collect()
}

fn graph_with(p: &mut Project, template: Template, plots: &[(usize, usize)], f: impl FnOnce(&mut Graph)) {
    let c = ids(p);
    let id = p.add_graph(template);
    if p.graph(id).unwrap().layers[0].legend.position == LegendPosition::Automatic {
        p.graph_mut(id).unwrap().layers[0].legend.position = LegendPosition::TopRight;
        p.graph_mut(id).unwrap().layers[0].legend.background = None;
    }
    for (x, y) in plots {
        p.add_plot_xy(id, 0, Some(c[*x]), c[*y], None).unwrap();
    }
    f(p.graph_mut(id).unwrap());
}

#[test]
fn stock_multi_y_legend_in_opj_keeps_selected_rows_and_their_order() {
    let mut p = source();
    let book = p.workbooks().next().unwrap().id;
    let c = ids(&p);
    let extra = p.add_column(book, 0, Designation::Y, nums(vec![1.0; 11])).unwrap();
    let last = p.add_column(book, 0, Designation::Y, nums(vec![2.0; 11])).unwrap();
    let id = p.plot_columns(Template::Multi3YYy, &[c[1], c[2], c[3], extra, last]).unwrap();
    p.graph_mut(id).unwrap().layers[1].plots[0].visible = false;

    let out = write_named(&p, "g", TIME).unwrap();
    assert!(out.warnings.iter().any(|w| w.contains("stock multi-Y legend's current rows")));
    let imported = read_opj_from(Cursor::new(&out.bytes), Path::new("stock-legend.opj")).unwrap();
    let text = imported.graphs[0].layers[0].legend.text.as_deref().unwrap();
    let rows: Vec<_> = text.lines().collect();
    assert_eq!(rows, [r"\l(1.1) %(1.1,@LG)", r"\l(3.1) %(3.1,@LG)", r"\l(1.2) %(1.2,@LG)"]);
    assert_eq!(imported.graphs[0].layers[0].legend.scope, LegendScope::Graph);
}

#[test]
fn an_underlined_title_is_written_with_origins_underline_markup() {
    // T293: as bold is written as \b(..).
    let mut p = source();
    graph_with(&mut p, Template::Line, &[(0, 1)], |g| {
        g.layers[0].x_axis.title = Some("Time".into());
        g.layers[0].x_axis.title_font.underline = true;
    });
    let out = write_named(&p, "g", TIME).unwrap();
    let pat: &[u8] = br"\u(Time)";
    assert!(out.bytes.windows(pat.len()).any(|w| w == pat), "the title is underlined");
}

#[test]
fn a_turned_title_keeps_its_angle_in_the_opj() {
    // T291: the title object's rotation is written and read back; the default angle stays default.
    let mut p = source();
    graph_with(&mut p, Template::Line, &[(0, 1)], |g| {
        g.layers[0].y_axis.title_rotation_deg = Some(45.0);
        g.layers[0].x_axis.title_offset_pt = (10.0, 4.0);
    });
    let (imp, _) = round_trip(&p);
    let layer = &imp.graphs[0].layers[0];
    assert_eq!(layer.y_axis.title_rotation_deg, Some(45.0));
    assert_eq!(layer.x_axis.title_rotation_deg, None, "an X title at 0 degrees is the default");
}

#[test]
fn window_comments_go_in_the_label_after_the_long_name() {
    // T283: Origin 2016 writes `long name CR LF comments` in a window's label.
    let mut p = source();
    graph_with(&mut p, Template::Line, &[(0, 1)], |g| g.long_name = "Fig".into());
    let book = p.workbooks().next().unwrap().id;
    let graph = p.graphs().next().unwrap().id;
    p.set_window_comments(book, "raw data\nday 2");
    p.set_window_comments(graph, "for the paper");
    let out = write_named(&p, "g", TIME).unwrap();
    let has = |pat: &[u8]| out.bytes.windows(pat.len()).any(|w| w == pat);
    assert!(has(b"\r\nraw data\r\nday 2"), "the book's comments, CR LF inside");
    assert!(has(b"Fig\r\nfor the paper"));
    assert!(!out.warnings.iter().any(|w| w.contains("omment")), "{:?}", out.warnings);
    let q = read(&out.bytes).into_project();
    let (b, g) = (q.workbooks().next().unwrap().id, q.graphs().next().unwrap().id);
    assert_eq!((q.window_comments(b), q.window_comments(g)), ("raw data\nday 2", "for the paper"));
    assert_eq!(q.graphs().next().unwrap().long_name, "Fig");
}

fn read(bytes: &[u8]) -> OpjImport {
    read_opj_from(Cursor::new(bytes.to_vec()), Path::new("g.opj")).unwrap()
}

fn round_trip(p: &Project) -> (OpjImport, Vec<String>) {
    let out = write_named(p, "g", TIME).unwrap();
    (read(&out.bytes), out.warnings)
}

/// The short name of the column that `id` (an id of the read workbooks) stands for.
fn name_of(imp: &OpjImport, id: ColumnId) -> String {
    imp.workbooks
        .iter()
        .flat_map(|b| b.sheets.iter())
        .flat_map(|s| s.columns.iter())
        .find(|c| c.id == id)
        .map(|c| c.short_name.clone())
        .unwrap_or_else(|| "?".into())
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6 * (1.0 + b.abs())
}

/// Positions are stored in 1/600 inch, so they come back to within one unit of that.
fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 2e-3
}

#[test]
fn a_full_graph_reads_back_with_everything_the_file_holds() {
    let mut p = source();
    graph_with(&mut p, Template::Line, &[(0, 1), (0, 2)], |g| {
        g.name = "Fig".into();
        g.long_name = "My figure".into();
        g.page = Page::inches(6.0, 4.0);
        let l = &mut g.layers[0];
        l.frame = Frame { left: 0.2, top: 0.1, width: 0.7, height: 0.6 };
        l.x_axis.min = Some(0.0);
        l.x_axis.max = Some(5.0);
        l.x_axis.major_step = Some(1.0);
        l.x_axis.minor_ticks = 3;
        l.x_axis.title = Some("Time (s)".into());
        l.x_axis.title_font.size_pt = 18.0;
        l.x_axis.tick_font.size_pt = 14.0;
        l.x_axis.major_tick = TickDirection::In;
        l.x_axis.minor_tick = TickDirection::None;
        l.x_axis.line_width_pt = 2.0;
        l.x_axis.show_opposite = true;
        l.y_axis.min = Some(-10.0);
        l.y_axis.max = Some(30.0);
        l.y_axis.major_step = Some(10.0);
        l.y_axis.title = Some("\\b(Current (mA))".into());
        l.y_axis.show_tick_labels = false;
        l.y_axis.major_tick = TickDirection::Both;
        l.plots[0].line.color = Color::rgb(255, 0, 0);
        l.plots[0].line.width_pt = 2.5;
        l.plots[0].line.dash = Dash::DashDot;
        l.plots[1].line.color = Color::rgb(12, 99, 200);
        l.plots[1].kind = PlotKind::LineSymbol;
        l.plots[1].symbol.shape = SymbolShape::TriangleUp;
        l.plots[1].symbol.size_pt = 8.0;
        l.plots[1].symbol.color = Color::rgb(0, 255, 0);
        l.plots[1].symbol.filled = false;
        l.legend.visible = true;
        l.legend.font.size_pt = 12.0;
        l.legend.position = LegendPosition::Manual { x: 0.7, y: 0.2 };
        l.labels.push(TextLabel {
            text: "peak".into(),
            x: 0.4,
            y: 0.3,
            font: Font::new("Arial", 20.0),
            rotation_deg: 0.0,
            table: false,
            group: None,
        });
    });
    let (imp, warnings) = round_trip(&p);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(imp.warnings.is_empty(), "{:?}", imp.warnings);
    assert_eq!(imp.graphs.len(), 1);
    let g = &imp.graphs[0];
    assert_eq!((g.name.as_str(), g.long_name.as_str()), ("Fig", "My figure"));
    assert!(close(g.page.width_pt, 432.0) && close(g.page.height_pt, 288.0));
    let l = &g.layers[0];
    let want = &p.graphs().next().unwrap().layers[0];
    assert!(
        close(l.frame.left, 0.2) && close(l.frame.top, 0.1) && close(l.frame.width, 0.7) && close(l.frame.height, 0.6)
    );
    for (got, want) in [(&l.x_axis, &want.x_axis), (&l.y_axis, &want.y_axis)] {
        assert_eq!((got.min, got.max, got.major_step), (want.min, want.max, want.major_step));
        assert_eq!(got.scale, want.scale);
        assert_eq!((got.major_tick, got.minor_tick), (want.major_tick, want.minor_tick));
        assert_eq!(got.show_tick_labels, want.show_tick_labels);
        assert_eq!(got.show_opposite, want.show_opposite);
        assert!(close(got.line_width_pt, want.line_width_pt));
        assert_eq!(got.title, want.title);
        assert!(got.show_title);
    }
    assert_eq!(l.x_axis.minor_ticks, 3);
    assert!(close(l.x_axis.tick_font.size_pt, 14.0) && close(l.x_axis.title_font.size_pt, 18.0));
    assert_eq!(l.plots.len(), 2);
    assert_eq!((name_of(&imp, l.plots[0].x.unwrap()), name_of(&imp, l.plots[0].y)), ("A".into(), "B".into()));
    assert_eq!((name_of(&imp, l.plots[1].x.unwrap()), name_of(&imp, l.plots[1].y)), ("A".into(), "C".into()));
    assert_eq!((l.plots[0].kind, l.plots[1].kind), (PlotKind::Line, PlotKind::LineSymbol));
    assert_eq!(l.plots[0].line.color, Color::rgb(255, 0, 0));
    assert!(close(l.plots[0].line.width_pt, 2.5));
    assert_eq!(l.plots[0].line.dash, Dash::DashDot);
    assert_eq!(l.plots[1].line.color, Color::rgb(12, 99, 200));
    assert_eq!(l.plots[1].symbol.shape, SymbolShape::TriangleUp);
    assert!(close(l.plots[1].symbol.size_pt, 8.0));
    assert_eq!(l.plots[1].symbol.color, Color::rgb(0, 255, 0));
    assert!(!l.plots[1].symbol.filled);
    assert!(l.legend.visible && close(l.legend.font.size_pt, 12.0));
    match l.legend.position {
        LegendPosition::Manual { x, y } => assert!(near(x, 0.7) && near(y, 0.2), "{x} {y}"),
        other => panic!("{other:?}"),
    }
    assert_eq!(l.labels.len(), 1);
    assert_eq!(l.labels[0].text, "peak");
    assert!(near(l.labels[0].x, 0.4) && near(l.labels[0].y, 0.3), "{} {}", l.labels[0].x, l.labels[0].y);
    assert!(close(l.labels[0].font.size_pt, 20.0));
}

#[test]
fn every_plot_kind_shape_dash_and_connection_comes_back() {
    let mut p = source();
    let kinds = [PlotKind::Line, PlotKind::Scatter, PlotKind::LineSymbol, PlotKind::Column, PlotKind::Area];
    for (k, kind) in kinds.iter().enumerate() {
        graph_with(&mut p, Template::Line, &[(0, 1)], |g| {
            g.name = format!("K{k}");
            let plot = &mut g.layers[0].plots[0];
            plot.kind = *kind;
            plot.line.color = Color::rgb(200, 10, 10);
            plot.symbol.color = Color::rgb(10, 10, 200);
        });
    }
    for (k, s) in SymbolShape::ALL.iter().enumerate() {
        graph_with(&mut p, Template::Scatter, &[(0, 1)], |g| {
            g.name = format!("S{k}");
            g.layers[0].plots[0].kind = PlotKind::Scatter;
            g.layers[0].plots[0].symbol.shape = *s;
        });
    }
    for (k, d) in Dash::ALL.iter().enumerate() {
        graph_with(&mut p, Template::Line, &[(0, 1)], |g| {
            g.name = format!("D{k}");
            g.layers[0].plots[0].line.dash = *d;
        });
    }
    for (k, c) in [
        Connect::Straight,
        Connect::Spline,
        Connect::StepHorizontal,
        Connect::StepVertical,
        Connect::BSpline,
        Connect::Bezier,
        Connect::Segment2,
        Connect::Segment3,
        Connect::StepHorizontalCenter,
        Connect::StepVerticalCenter,
    ]
    .iter()
    .enumerate()
    {
        graph_with(&mut p, Template::Line, &[(0, 1)], |g| {
            g.name = format!("C{k}");
            g.layers[0].plots[0].line.connect = *c;
        });
    }
    let (imp, warnings) = round_trip(&p);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(imp.warnings.is_empty(), "{:?}", imp.warnings);
    assert_eq!(imp.graphs.len(), 5 + 8 + 8 + 10);
    let by_name = |n: &str| imp.graphs.iter().find(|g| g.name == n).unwrap().layers[0].plots[0].clone();
    for (k, kind) in kinds.iter().enumerate() {
        let got = by_name(&format!("K{k}"));
        assert_eq!(got.kind, *kind);
        match kind {
            PlotKind::Line => assert_eq!(got.line.color, Color::rgb(200, 10, 10)),
            PlotKind::Scatter => assert_eq!(got.symbol.color, Color::rgb(10, 10, 200)),
            PlotKind::LineSymbol => {
                assert_eq!(got.line.color, Color::rgb(200, 10, 10));
                assert_eq!(got.symbol.color, Color::rgb(10, 10, 200));
            }
            PlotKind::Column
            | PlotKind::StackedColumn
            | PlotKind::Stacked100Column
            | PlotKind::Bar
            | PlotKind::FloatingColumn => assert_eq!(got.symbol.color, Color::rgb(10, 10, 200), "the fill of a bar"),
            _ => {
                assert_eq!(got.symbol.color, Color::rgb(10, 10, 200), "the fill of an area");
                assert_eq!(got.line.color, Color::rgb(200, 10, 10), "its border");
            }
        }
    }
    for (k, s) in SymbolShape::ALL.iter().enumerate() {
        assert_eq!(by_name(&format!("S{k}")).symbol.shape, *s);
    }
    for (k, d) in Dash::ALL.iter().enumerate() {
        assert_eq!(by_name(&format!("D{k}")).line.dash, *d);
    }
    for (k, c) in [
        Connect::Straight,
        Connect::Spline,
        Connect::StepHorizontal,
        Connect::StepVertical,
        Connect::BSpline,
        Connect::Bezier,
        Connect::Segment2,
        Connect::Segment3,
        Connect::StepHorizontalCenter,
        Connect::StepVerticalCenter,
    ]
    .iter()
    .enumerate()
    {
        assert_eq!(by_name(&format!("C{k}")).line.connect, *c);
    }
}

#[test]
fn logarithmic_axes_and_automatic_ranges() {
    let mut p = source();
    graph_with(&mut p, Template::Scatter, &[(0, 3)], |g| {
        g.layers[0].y_axis.scale = ScaleKind::Log10;
        g.layers[0].plots[0].kind = PlotKind::Scatter;
    });
    graph_with(&mut p, Template::Line, &[(0, 1)], |_| {});
    let (imp, _) = round_trip(&p);
    let log = &imp.graphs[0].layers[0];
    assert_eq!(log.y_axis.scale, ScaleKind::Log10);
    // the data run from 1 to 316: Origin's own range (8 % of the log span, then the next 1..9 tick)
    assert_eq!((log.y_axis.min, log.y_axis.max), (Some(0.6), Some(600.0)));
    assert_eq!(log.x_axis.scale, ScaleKind::Linear);
    let auto = &imp.graphs[1].layers[0];
    let (lo, hi) = (auto.y_axis.min.unwrap(), auto.y_axis.max.unwrap());
    assert!(lo <= 0.0 && hi >= 25.0, "the data 0..25 fit in {lo}..{hi}");
    let (xlo, xhi) = (auto.x_axis.min.unwrap(), auto.x_axis.max.unwrap());
    assert!(xlo <= 0.0 && xhi >= 5.0, "{xlo}..{xhi}");
    assert!(auto.x_axis.major_step.unwrap() > 0.0);
}

#[test]
fn nice_ranges_have_round_limits() {
    // Origin's own range for data 0..25 (graphing spec section 4): 8 % margin, rounded to half the step 2.
    let (a, b, s) = graph::nice_range(0.0, 25.0, false, AutoRange::Nice, 8.0);
    assert!((a, b, s) == (-2.0, 27.0, 2.0), "{a} {b} {s}");
    let (a, b, s) = graph::nice_range(-0.00042, 0.00043, false, AutoRange::Nice, 8.0);
    assert!(a <= -0.00042 && b >= 0.00043 && s > 0.0);
    let (a, b, _) = graph::nice_range(0.3, 20.0, true, AutoRange::Nice, 8.0);
    assert_eq!((a, b), (0.2, 30.0));
    let (a, b, _) = graph::nice_range(2.0, 2.0, false, AutoRange::Nice, 8.0);
    assert!(a < 2.0 && b > 2.0);
    let (a, b, _) = graph::nice_range(-3.0, 8.0, false, AutoRange::SymmetricZero, 8.0);
    assert!(a <= -8.0 && b >= 8.0 && close(a, -b));
    assert_eq!(graph::nice_range(1.0, 7.0, false, AutoRange::Tight, 8.0).0, 1.0);
}

#[test]
fn error_bars_text_x_and_two_layers() {
    let mut p = source();
    let c = ids(&p);
    let id = p.add_graph(Template::Line);
    p.graph_mut(id).unwrap().layers[0].legend.position = LegendPosition::TopRight;
    p.graph_mut(id).unwrap().layers[0].legend.background = None;
    p.add_plot_xy(id, 0, Some(c[5]), c[1], Some(c[4])).unwrap();
    p.graph_mut(id).unwrap().layers[0].plots[0].kind = PlotKind::Column;
    let mut second = p.graph(id).unwrap().layers[0].clone();
    second.plots.clear();
    second.frame = Frame { left: 0.5, top: 0.5, width: 0.3, height: 0.3 };
    p.graph_mut(id).unwrap().layers.push(second);
    p.add_plot_xy(id, 1, Some(c[0]), c[2], None).unwrap();
    let (imp, warnings) = round_trip(&p);
    assert!(warnings.is_empty(), "{warnings:?}");
    let g = &imp.graphs[0];
    assert_eq!(g.layers.len(), 2);
    let a = &g.layers[0].plots[0];
    assert_eq!((name_of(&imp, a.x.unwrap()), name_of(&imp, a.y)), ("F".into(), "B".into()));
    assert_eq!(a.y_error.map(|e| name_of(&imp, e)), Some("E".to_string()));
    let b = &g.layers[1].plots[0];
    assert_eq!(name_of(&imp, b.y), "C");
    // The file keeps positions in 1/600 inch, which on Origin's page is a step of 0.02 %.
    let near = |a: f64, b: f64| (a - b).abs() < 1e-3;
    assert!(near(g.layers[1].frame.left, 0.5) && near(g.layers[1].frame.width, 0.3));
    // the categories of a text column run from 0.5 to n + 0.5
    assert_eq!((g.layers[0].x_axis.min, g.layers[0].x_axis.max), (Some(0.5), Some(11.5)));
}

#[test]
fn what_the_file_cannot_hold_is_named() {
    let mut p = source();
    let other = Column::new(ColumnId(999_999), "Z", Designation::Y, nums(vec![1.0]));
    graph_with(&mut p, Template::LineSymbol, &[(0, 1), (0, 2), (0, 3)], |g| {
        g.name = "Odd".into();
        g.page.background = Color::rgb(250, 250, 200);
        let l = &mut g.layers[0];
        l.plots[0].rows = Some((2, 5));
        l.plots[0].symbol.every = 3;
        l.plots[1].visible = false;
        l.plots[2].y = other.id;
        l.y_axis.tick_format = TickFormat::DateTime(sheaf_core::datetime::DatePattern::Iso);
        l.x_axis.tick_label_divisor = 1000.0;
        l.x_axis.major_tick_len_pt = 12.0;
        l.y_axis.major_grid = Some(LineStyle::solid(Color::rgb(200, 200, 200), 0.5));
        l.y_axis.tick_font.family = "Times New Roman".into();
        l.legend.background = Some(Color::rgb(250, 250, 250));
        l.labels.push(TextLabel {
            text: "caf\u{e9} \u{3b1}".into(),
            x: 0.1,
            y: 0.1,
            font: {
                let mut f = Font::new("Courier New", 12.0);
                f.color = Color::rgb(3, 90, 40);
                f
            },
            rotation_deg: 0.0,
            table: false,
            group: None,
        });
    });
    let (imp, warnings) = round_trip(&p);
    let all = warnings.join(" | ");
    for needle in [
        "Graph 'Odd'",
        "page background",
        "row range",
        "hidden plot",
        "every n-th point",
        "not a worksheet column",
        "no counterpart in Origin",
        "divisor",
        "minor tick length",
        "grid lines",
        "font family",
        "legend background",
        "nearest listed colour",
        "'Courier New'",
    ] {
        assert!(all.contains(needle), "{needle}: {all}");
    }
    // the written graph still reads back, with the plots that could be written
    assert_eq!(imp.graphs[0].layers[0].plots.len(), 1);
}

#[test]
fn a_frame_outside_the_page_is_replaced_and_named() {
    let mut p = source();
    graph_with(&mut p, Template::LineSymbol, &[(0, 1)], |g| {
        g.name = "Edge".into();
        g.layers[0].frame = Frame { left: -1.0, top: 5.0, width: 0.0, height: 100.0 };
    });
    let (imp, warnings) = round_trip(&p);
    assert!(warnings.join(" | ").contains("layer frame"), "{warnings:?}");
    let f = imp.graphs[0].layers[0].frame;
    assert!(f.width > 0.0 && f.height > 0.0 && f.left >= 0.0 && f.left + f.width <= 1.0 + 1e-6, "{f:?}");
    assert!(!warnings.join(" | ").contains("position could not be read"), "{warnings:?}");
}

#[test]
fn a_graph_without_plots_and_a_second_graph_read_back_in_order() {
    let mut p = source();
    p.add_graph(Template::Line);
    graph_with(&mut p, Template::Line, &[(0, 1)], |g| g.layers[0].legend.visible = false);
    let (imp, warnings) = round_trip(&p);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(imp.graphs.len(), 2);
    assert!(imp.graphs[0].layers[0].plots.is_empty());
    assert!(!imp.graphs[1].layers[0].legend.visible);
    // one workbook and two graphs: three windows
    let proj = imp.into_project();
    assert_eq!(proj.workbooks().count(), 1);
    assert_eq!(proj.graphs().count(), 2);
    assert_eq!(proj.graphs().nth(1).unwrap().layers[0].plots.len(), 1);
}

#[test]
fn axis_layer_icons_survive_a_bare_opj_round_trip_per_graph() {
    let mut p = source();
    graph_with(&mut p, Template::Line, &[(0, 1)], |g| g.page.style.axis_layer_icons = true);
    graph_with(&mut p, Template::Line, &[(0, 1)], |_| {});
    let (imp, warnings) = round_trip(&p);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(imp.graphs[0].page.style.axis_layer_icons);
    assert!(!imp.graphs[1].page.style.axis_layer_icons);
}

#[test]
fn graph_grids_survive_a_bare_opj_round_trip_per_graph() {
    let mut p = source();
    graph_with(&mut p, Template::Line, &[(0, 1)], |g| g.page.style.grid = PageGrid::Object);
    graph_with(&mut p, Template::Line, &[(0, 1)], |g| {
        g.page.style.grid = PageGrid::Layer;
        g.page.style.axis_layer_icons = true;
    });
    graph_with(&mut p, Template::Line, &[(0, 1)], |_| {});
    let (imp, warnings) = round_trip(&p);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(imp.graphs[0].page.style.grid, PageGrid::Object);
    assert!(!imp.graphs[0].page.style.axis_layer_icons);
    assert_eq!(imp.graphs[1].page.style.grid, PageGrid::Layer);
    assert!(imp.graphs[1].page.style.axis_layer_icons);
    assert_eq!(imp.graphs[2].page.style.grid, PageGrid::Off);
}

#[test]
fn a_graph_before_its_workbook_still_finds_the_columns() {
    let mut p = source();
    let c = ids(&p);
    let id = p.add_graph(Template::Line);
    p.graph_mut(id).unwrap().layers[0].legend.position = LegendPosition::TopRight;
    p.graph_mut(id).unwrap().layers[0].legend.background = None;
    p.add_plot_xy(id, 0, Some(c[0]), c[2], None).unwrap();
    // move the graph in front of the workbook
    let g = p.windows.pop().unwrap();
    p.windows.insert(0, g);
    let (imp, warnings) = round_trip(&p);
    assert!(warnings.is_empty(), "{warnings:?}");
    let pl = &imp.graphs[0].layers[0].plots[0];
    assert_eq!(name_of(&imp, pl.y), "C");
}

#[test]
fn bad_pages_and_names_are_refused() {
    let mut p = source();
    graph_with(&mut p, Template::Line, &[(0, 1)], |g| g.page = Page::inches(200.0, 4.0));
    assert!(write_named(&p, "g", TIME).unwrap_err().to_string().contains("inches"));
    let mut p = source();
    graph_with(&mut p, Template::Line, &[(0, 1)], |g| g.name = "G@1".into());
    assert!(write_named(&p, "g", TIME).is_err());
}

#[test]
fn damaged_graph_files_never_panic_the_reader() {
    let mut p = source();
    graph_with(&mut p, Template::Line, &[(0, 1), (0, 2)], |g| {
        g.layers[0].labels.push(TextLabel {
            text: "t".into(),
            x: 0.5,
            y: 0.5,
            font: Font::new("Arial", 12.0),
            rotation_deg: 90.0,
            table: false,
            group: None,
        })
    });
    let bytes = write_named(&p, "g", TIME).unwrap().bytes;
    for cut in (0..bytes.len()).step_by(997) {
        let _ = read_opj_from(Cursor::new(bytes[..cut].to_vec()), Path::new("cut.opj"));
    }
}

#[test]
fn an_area_plot_is_written_and_read_back_without_a_warning() {
    let mut p = source();
    graph_with(&mut p, Template::Area, &[(0, 1)], |_| {});
    let (imp, warnings) = round_trip(&p);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(imp.graphs[0].layers[0].plots[0].kind, PlotKind::Area);
}

#[test]
fn rotated_tick_labels_are_named() {
    let mut p = source();
    graph_with(&mut p, Template::Line, &[(0, 1), (0, 2)], |g| g.layers[0].x_axis.tick_label_angle_deg = 45.0);
    let (_, warnings) = round_trip(&p);
    assert!(warnings.iter().any(|w| w.contains("rotated tick labels")), "{warnings:?}");
}

#[test]
fn drawn_objects_are_named_in_a_warning_and_not_written() {
    use sheaf_core::shape::{Shape, ShapeKind};
    let mut p = source();
    graph_with(&mut p, Template::Line, &[(0, 1)], |g| {
        g.layers[0].shapes = vec![
            Shape::new(ShapeKind::Arrow, (0.1, 0.1), (0.4, 0.4)),
            Shape::new(ShapeKind::Arrow, (0.1, 0.2), (0.4, 0.5)),
            Shape::new(ShapeKind::Ellipse, (0.5, 0.5), (0.9, 0.9)),
        ];
    });
    let (imp, warnings) = round_trip(&p);
    let w = warnings.iter().find(|w| w.contains("drawn objects")).unwrap_or_else(|| panic!("{warnings:?}"));
    assert!(w.contains("2 arrow") && w.contains("1 ellipse"), "{w}");
    assert!(imp.graphs[0].layers[0].shapes.is_empty());
    // A layer without drawn objects says nothing.
    let (_, quiet) = round_trip(&source());
    assert!(!quiet.iter().any(|w| w.contains("drawn objects")));
}

#[test]
fn significance_brackets_are_named_in_a_warning_and_not_written() {
    use sheaf_core::compplot::SignificanceBracket;
    let mut p = source();
    graph_with(&mut p, Template::Line, &[(0, 1)], |g| {
        let b = SignificanceBracket { group_a: 0, group_b: 1, p: 0.01, label: "**".into() };
        g.layers[0].brackets = vec![b];
    });
    let (imp, warnings) = round_trip(&p);
    let w = warnings.iter().find(|w| w.contains("significance bracket")).unwrap_or_else(|| panic!("{warnings:?}"));
    assert!(w.contains("layer 1 has 1 significance bracket ") && w.contains(".opj.sheaf"), "{w}");
    assert!(imp.graphs[0].layers[0].brackets.is_empty());
    let (_, quiet) = round_trip(&source());
    assert!(!quiet.iter().any(|w| w.contains("significance bracket")));
}

#[test]
fn slope_annotations_are_named_in_a_warning_and_not_written() {
    use sheaf_core::graph::SlopeAnnotation;
    let mut p = source();
    graph_with(&mut p, Template::Line, &[(0, 1)], |g| {
        let y = g.layers[0].plots[0].y;
        g.layers[0].slopes = vec![SlopeAnnotation::new(y, 0, 1), SlopeAnnotation::new(y, 1, 2)];
    });
    let (imp, warnings) = round_trip(&p);
    let w = warnings.iter().find(|w| w.contains("slope annotation")).unwrap_or_else(|| panic!("{warnings:?}"));
    assert!(w.contains("layer 1 has 2 slope annotations") && w.contains(".opj.sheaf"), "{w}");
    assert!(imp.graphs[0].layers[0].slopes.is_empty());
    let (_, quiet) = round_trip(&source());
    assert!(!quiet.iter().any(|w| w.contains("slope annotation")));
}

#[test]
fn straight_lines_are_named_in_a_warning_and_not_written() {
    use sheaf_core::straight_line::StraightLine;
    let mut p = source();
    graph_with(&mut p, Template::Line, &[(0, 1)], |g| {
        g.layers[0].straight_lines = vec![StraightLine::new(true, 1.0), StraightLine::new(false, 2.0)];
    });
    let (imp, warnings) = round_trip(&p);
    let w = warnings.iter().find(|w| w.contains("straight line")).unwrap_or_else(|| panic!("{warnings:?}"));
    assert!(w.contains("layer 1 has 2 straight lines") && w.contains(".opj.sheaf"), "{w}");
    assert!(imp.graphs[0].layers[0].straight_lines.is_empty());
    let (_, quiet) = round_trip(&source());
    assert!(!quiet.iter().any(|w| w.contains("straight line")));
}

#[test]
fn colour_and_bubble_scales_are_named_in_a_warning_and_not_written() {
    use sheaf_core::layer_scale::{LayerScale, LayerScaleKind};
    let mut p = source();
    graph_with(&mut p, Template::Line, &[(0, 1)], |g| {
        g.layers[0].scales = vec![LayerScale::new(LayerScaleKind::Color), LayerScale::new(LayerScaleKind::Bubble)];
    });
    let (imp, warnings) = round_trip(&p);
    let w = warnings.iter().find(|w| w.contains("bubble scale")).unwrap_or_else(|| panic!("{warnings:?}"));
    assert!(w.contains("layer 1 has 2 colour or bubble scales") && w.contains(".opj.sheaf"), "{w}");
    assert!(imp.graphs[0].layers[0].scales.is_empty());
    let (_, quiet) = round_trip(&source());
    assert!(!quiet.iter().any(|w| w.contains("bubble scale")));
}

#[test]
fn other_scale_types_survive_a_round_trip() {
    let mut p = source();
    graph_with(&mut p, Template::Scatter, &[(0, 3)], |g| {
        g.layers[0].y_axis.scale = ScaleKind::Ln;
        g.layers[0].x_axis.scale = ScaleKind::OffsetReciprocal;
    });
    let (imp, _) = round_trip(&p);
    let l = &imp.graphs[0].layers[0];
    assert_eq!(l.y_axis.scale, ScaleKind::Ln);
    assert_eq!(l.x_axis.scale, ScaleKind::OffsetReciprocal);
    assert!(l.y_axis.min.is_some_and(|m| m > 0.0));
}

#[test]
fn discrete_scales_survive_a_bare_opj_round_trip() {
    let mut p = source();
    graph_with(&mut p, Template::Line, &[(0, 1)], |g| {
        let layer = &mut g.layers[0];
        layer.x_axis.scale = ScaleKind::Discrete;
        layer.y_axis.scale = ScaleKind::Discrete;
    });
    let (imp, _) = round_trip(&p);
    let l = &imp.graphs[0].layers[0];
    assert_eq!(l.x_axis.scale, ScaleKind::Discrete);
    assert_eq!(l.y_axis.scale, ScaleKind::Discrete);
}

#[test]
fn the_major_tick_length_is_written_and_read_back() {
    let mut p = source();
    graph_with(&mut p, Template::LineSymbol, &[(0, 1)], |g| {
        let l = &mut g.layers[0];
        l.x_axis.major_tick_len_pt = 15.0;
        l.x_axis.minor_tick_len_pt = 7.5;
        l.y_axis.major_tick_len_pt = 3.0;
        l.y_axis.minor_tick_len_pt = 1.5;
    });
    let (imp, warnings) = round_trip(&p);
    assert!(warnings.iter().all(|w| !w.contains("tick length")), "{warnings:?}");
    let l = &imp.graphs[0].layers[0];
    assert_eq!((l.x_axis.major_tick_len_pt, l.x_axis.minor_tick_len_pt), (15.0, 7.5));
    assert_eq!((l.y_axis.major_tick_len_pt, l.y_axis.minor_tick_len_pt), (3.0, 1.5));
}

/// Writes `sheaf-ticks.opj` (X major 15 pt, Y 3 pt) into `SHEAF_OPJ_TICK_DIR`, for Origin to open and
/// save again (opj-format.md 10.3): `SHEAF_OPJ_TICK_DIR=<folder> cargo test -p sheaf-io -- --ignored
/// write_tick_lengths_for_origin`.
#[test]
#[ignore]
fn write_tick_lengths_for_origin() {
    let Ok(dir) = std::env::var("SHEAF_OPJ_TICK_DIR") else { return };
    let mut p = source();
    graph_with(&mut p, Template::LineSymbol, &[(0, 1)], |g| {
        let l = &mut g.layers[0];
        l.x_axis.major_tick_len_pt = 15.0;
        l.x_axis.minor_tick_len_pt = 7.5;
        l.y_axis.major_tick_len_pt = 3.0;
        l.y_axis.minor_tick_len_pt = 1.5;
    });
    std::fs::write(Path::new(&dir).join("sheaf-ticks.opj"), write_named(&p, "g", TIME).unwrap().bytes).unwrap();
}

#[test]
fn every_origin_symbol_shape_and_interior_comes_back() {
    // Shape byte 0x17 and interior byte 0x18 hold Origin's numbers, as read from a project Origin
    // 2016 saved with shapes 1-20 and interiors 0-11 (T323).
    let mut p = source();
    for (code, s) in SymbolShape::ORIGIN {
        graph_with(&mut p, Template::Scatter, &[(0, 1)], |g| {
            g.name = format!("S{code}");
            g.layers[0].plots[0].kind = PlotKind::Scatter;
            g.layers[0].plots[0].symbol.shape = s;
        });
    }
    for i in SymbolInterior::ALL {
        graph_with(&mut p, Template::Scatter, &[(0, 1)], |g| {
            g.name = format!("I{}", i.origin_code());
            g.layers[0].plots[0].kind = PlotKind::Scatter;
            g.layers[0].plots[0].symbol.filled = i == SymbolInterior::Solid;
            g.layers[0].plots[0].symbol.interior = (i != SymbolInterior::Solid).then_some(i);
        });
    }
    let (imp, warnings) = round_trip(&p);
    assert!(warnings.is_empty() && imp.warnings.is_empty(), "{warnings:?} {:?}", imp.warnings);
    let by_name = |n: &str| imp.graphs.iter().find(|g| g.name == n).unwrap().layers[0].plots[0].symbol.clone();
    for (code, s) in SymbolShape::ORIGIN {
        assert_eq!(by_name(&format!("S{code}")).shape, s);
    }
    for i in SymbolInterior::ALL {
        let got = by_name(&format!("I{}", i.origin_code()));
        assert_eq!(got.effective_interior(), i, "{i:?}");
    }
}
