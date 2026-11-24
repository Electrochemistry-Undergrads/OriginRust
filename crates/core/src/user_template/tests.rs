use super::*;
use crate::color::Color;
use crate::column::{ColumnData, Designation};
use crate::graph::{Dash, Font, LayerLink, SymbolShape, TextLabel};
use crate::shape::{Shape, ShapeKind};
use crate::template::Template;

/// A book with one X column and `n` Y columns, named so that the long names tell them apart.
fn book(p: &mut Project, long: &str, n: usize) -> Vec<ColumnId> {
    let b = p.add_workbook(long);
    p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![0.0, 1.0, 2.0])).unwrap();
    (0..n)
        .map(|i| {
            let y =
                p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![i as f64, 1.0 + i as f64, 5.0])).unwrap();
            p.column_mut(y).unwrap().long_name = format!("{long}{i}");
            y
        })
        .collect()
}

/// A graph of two columns with a look that differs from every default.
fn styled(p: &mut Project, ys: &[ColumnId]) -> WindowId {
    let g = p.plot_columns(Template::Line, ys).unwrap();
    let graph = p.graph_mut(g).unwrap();
    graph.page.width_pt = 400.0;
    graph.page.background = Color::rgb(250, 240, 230);
    let l = &mut graph.layers[0];
    l.frame.left = 0.31;
    l.x_axis.title = Some("Potential (V)".into());
    l.x_axis.min = Some(-1.0);
    l.x_axis.max = Some(2.0);
    l.y_axis.major_step = Some(0.5);
    l.legend.text = Some("\\l(1) %(1)".into());
    l.labels.push(TextLabel::new("%(1) at 5 mV/s", 0.2, 0.2, Font::new("Arial", 17.0)));
    l.shapes.push(Shape::new(ShapeKind::Line, (0.1, 0.1), (0.5, 0.5)));
    l.plots[0].line.dash = Dash::Dash;
    l.plots[0].line.width_pt = 3.25;
    l.plots[0].symbol.shape = SymbolShape::TriangleUp;
    if let Some(second) = l.plots.get_mut(1) {
        second.line.color = Color::rgb(1, 2, 3);
        second.auto_color = false;
    }
    g
}

fn template_of(p: &Project, g: WindowId) -> UserTemplate {
    UserTemplate::from_graph("mine", p.graph(g).unwrap()).unwrap()
}

/// The style of a plot with the data references blanked, to compare looks.
fn look(plot: &crate::graph::Plot) -> crate::graph::Plot {
    let mut p = plot.clone();
    p.x = None;
    p.y = ColumnId(0);
    p.y_error = None;
    p.x_error = None;
    p
}

#[test]
fn a_template_keeps_no_column_and_numbers_its_plots_as_slots() {
    let mut p = Project::new();
    let ys = book(&mut p, "A", 3);
    let g = styled(&mut p, &ys[..2]);
    let t = template_of(&p, g);
    assert_eq!(t.plot_count(), 2);
    assert_eq!(t.slots(), vec![(0, 0), (0, 1)]);
    let mut seen = t.graph.referenced_columns();
    seen.sort();
    assert_eq!(seen, vec![ColumnId(1), ColumnId(2)]);
    assert!(t.graph.layers[0].plots.iter().all(|pl| pl.x.is_none() && pl.y_error.is_none() && pl.rows.is_none()));
}

#[test]
fn plotting_other_columns_keeps_every_look_setting() {
    let mut p = Project::new();
    let ys = book(&mut p, "A", 2);
    let g = styled(&mut p, &ys);
    let t = template_of(&p, g);
    let other = book(&mut p, "B", 2);
    let h = p.plot_with_user_template(&t, &other).unwrap();

    let (src, new) = (p.graph(g).unwrap(), p.graph(h).unwrap());
    assert_ne!(src.name, new.name);
    assert_eq!(new.long_name, "B");
    assert_eq!(new.page, src.page);
    assert_eq!(new.layers.len(), src.layers.len());
    let (a, b) = (&src.layers[0], &new.layers[0]);
    assert_eq!((&a.frame, &a.x_axis, &a.y_axis), (&b.frame, &b.x_axis, &b.y_axis));
    assert_eq!((&a.legend, &a.labels, &a.shapes), (&b.legend, &b.labels, &b.shapes));
    assert_eq!((&a.palette, &a.default_line, a.default_kind), (&b.palette, &b.default_line, b.default_kind));
    assert_eq!(b.plots.len(), 2);
    for (pa, pb) in a.plots.iter().zip(&b.plots) {
        assert_eq!(look(pa), look(pb));
    }
    assert_eq!([b.plots[0].y, b.plots[1].y], [other[0], other[1]]);
    let x = p.locate(other[0]).unwrap().1.columns[0].id;
    assert_eq!([b.plots[0].x, b.plots[1].x], [Some(x), Some(x)], "the X column is found from the sheet");
    assert_eq!(b.plots[1].line.color, Color::rgb(1, 2, 3));
}

#[test]
fn the_template_does_not_change_the_graph_it_came_from_or_the_project_data() {
    let mut p = Project::new();
    let ys = book(&mut p, "A", 2);
    let g = styled(&mut p, &ys);
    let before = p.graph(g).unwrap().clone();
    let t = template_of(&p, g);
    p.plot_with_user_template(&t, &ys).unwrap();
    assert_eq!(p.graph(g).unwrap(), &before);
    assert_eq!(p.numeric(ys[0]).unwrap(), [0.0, 1.0, 5.0]);
}

#[test]
fn a_two_plot_template_styles_two_three_and_five_datasets_in_turn() {
    let mut p = Project::new();
    let ys = book(&mut p, "A", 2);
    let g = styled(&mut p, &ys);
    let t = template_of(&p, g);
    let others = book(&mut p, "B", 5);
    for n in [2usize, 3, 5] {
        let h = p.plot_with_user_template(&t, &others[..n]).unwrap();
        let plots = &p.graph(h).unwrap().layers[0].plots;
        assert_eq!(plots.len(), n);
        for (i, pl) in plots.iter().enumerate() {
            assert_eq!(pl.y, others[i]);
            assert_eq!(pl.line.dash, t.graph.layers[0].plots[i % 2].line.dash, "plot {i} of {n}");
            assert_eq!(pl.line.width_pt, t.graph.layers[0].plots[i % 2].line.width_pt);
        }
        // Extra plots take the colours of the layer's palette in plot order.
        let layer = &p.graph(h).unwrap().layers[0];
        for (i, plot) in plots.iter().enumerate().skip(2) {
            assert_eq!(plot.line.color, layer.palette.color(i, n), "plot {i} of {n}");
        }
    }
}

#[test]
fn fewer_datasets_than_plots_leave_the_last_plots_out() {
    let mut p = Project::new();
    let ys = book(&mut p, "A", 3);
    let g = styled(&mut p, &ys);
    let t = template_of(&p, g);
    assert_eq!(t.plot_count(), 3);
    let h = p.plot_with_user_template(&t, &ys[..1]).unwrap();
    assert_eq!(p.graph(h).unwrap().layers[0].plots.len(), 1);
    let none = p.plot_with_user_template(&t, &[]).unwrap();
    assert!(p.graph(none).unwrap().layers[0].plots.is_empty());
}

#[test]
fn a_double_y_template_puts_the_columns_on_the_left_and_right_axes_in_turn() {
    let mut p = Project::new();
    let ys = book(&mut p, "A", 3);
    let g = p.plot_columns(Template::DoubleY, &ys[..2]).unwrap();
    p.graph_mut(g).unwrap().layers[1].y_axis.title = Some("right".into());
    let t = template_of(&p, g);
    assert_eq!(t.slots(), vec![(0, 0), (1, 0)]);
    let ys = book(&mut p, "B", 4);

    let three = p.plot_with_user_template(&t, &ys[..3]).unwrap();
    let layers = &p.graph(three).unwrap().layers;
    assert_eq!(layers.len(), 2);
    assert_eq!(layers[0].plots.iter().map(|q| q.y).collect::<Vec<_>>(), [ys[0], ys[2]]);
    assert_eq!(layers[1].plots.iter().map(|q| q.y).collect::<Vec<_>>(), [ys[1]]);
    assert_eq!(layers[1].y_axis.title.as_deref(), Some("right"));
    assert_eq!(layers[1].link.as_ref().unwrap().parent, 0);

    let one = p.plot_with_user_template(&t, &ys[..1]).unwrap();
    let layers = &p.graph(one).unwrap().layers;
    assert_eq!(layers.len(), 1, "the right layer has no plot and goes");
    assert_eq!(layers[0].plots.len(), 1);
}

#[test]
fn a_linked_layer_that_another_layer_follows_stays_when_empty() {
    let mut p = Project::new();
    let ys = book(&mut p, "A", 2);
    let g = p.plot_columns(Template::Line, &ys[..1]).unwrap();
    let graph = p.graph_mut(g).unwrap();
    let mut second = graph.layers[0].clone();
    second.plots.clear();
    second.link = Some(LayerLink { parent: 0, x: Default::default(), y: Default::default() });
    let mut third = second.clone();
    third.link = Some(LayerLink { parent: 1, x: Default::default(), y: Default::default() });
    graph.layers.extend([second, third]);
    let t = template_of(&p, g);
    let h = p.plot_with_user_template(&t, &ys[..1]).unwrap();
    // Both linked layers are empty and the last is not followed: it goes, then the one it followed.
    assert_eq!(p.graph(h).unwrap().layers.len(), 1);
}

#[test]
fn fixed_axis_ranges_stay_and_automatic_ones_follow_the_new_data() {
    let mut p = Project::new();
    let ys = book(&mut p, "A", 1);
    let g = styled(&mut p, &ys);
    let t = template_of(&p, g);
    let wide = book(&mut p, "B", 1);
    p.column_mut(wide[0]).unwrap().data = ColumnData::numeric(vec![-500.0, 0.0, 900.0]);
    let h = p.plot_with_user_template(&t, &wide).unwrap();
    let l = &p.graph(h).unwrap().layers[0];
    assert_eq!((l.x_axis.min, l.x_axis.max), (Some(-1.0), Some(2.0)), "fixed as saved");
    assert_eq!((l.y_axis.min, l.y_axis.max), (None, None), "automatic stays automatic");
    assert_eq!(l.y_axis.auto_range, t.graph.layers[0].y_axis.auto_range);
}

#[test]
fn text_with_substitutions_stays_text_and_a_plain_plot_name_is_not_kept() {
    let mut p = Project::new();
    let ys = book(&mut p, "A", 2);
    let g = styled(&mut p, &ys);
    {
        let l = &mut p.graph_mut(g).unwrap().layers[0];
        l.plots[0].label = Some("sample %(?X)".into());
        l.plots[1].label = Some("Sample A".into());
    }
    let t = template_of(&p, g);
    let others = book(&mut p, "B", 2);
    let h = p.plot_with_user_template(&t, &others).unwrap();
    let l = &p.graph(h).unwrap().layers[0];
    assert_eq!(l.labels[0].text, "%(1) at 5 mV/s");
    assert_eq!(l.legend.text.as_deref(), Some("\\l(1) %(1)"));
    assert_eq!(l.plots[0].label.as_deref(), Some("sample %(?X)"));
    assert_eq!(l.plots[1].label, None);
    assert_eq!(p.plot_label(&l.plots[1]), "B1");
}

#[test]
fn column_bound_settings_fall_back_to_automatic() {
    let mut p = Project::new();
    let ys = book(&mut p, "A", 1);
    let g = p.plot_columns(Template::Line, &ys).unwrap();
    {
        let l = &mut p.graph_mut(g).unwrap().layers[0];
        l.x_axis.tick_label_source = TickLabelSource::Column { column: ys[0] };
        l.x_axis.rug.plot = Some(ys[0]);
        l.plots[0].decor.labels.form = LabelForm::Column { column: ys[0] };
    }
    let t = template_of(&p, g);
    let l = &t.graph.layers[0];
    assert_eq!(l.x_axis.tick_label_source, TickLabelSource::Auto);
    assert_eq!(l.x_axis.rug.plot, None);
    assert_eq!(l.plots[0].decor.labels.form, LabelForm::Y);
    assert_eq!(t.graph.referenced_columns(), vec![ColumnId(1)]);
}

#[test]
fn stack_offsets_follow_their_slot_to_the_new_columns() {
    let mut p = Project::new();
    let ys = book(&mut p, "A", 2);
    let g = p.plot_columns(Template::Line, &ys).unwrap();
    {
        let l = &mut p.graph_mut(g).unwrap().layers[0];
        l.plot_offset_mut(ys[1]).y = 7.0;
        l.plot_offset_mut(ys[1]).follows = Some(ys[0]);
        l.plot_offset_mut(ColumnId(9999)).y = 1.0;
    }
    let t = template_of(&p, g);
    assert_eq!(t.graph.layers[0].plot_offsets.len(), 1, "an offset of a column no plot reads is dropped");
    let others = book(&mut p, "B", 4);
    let h = p.plot_with_user_template(&t, &others).unwrap();
    let l = &p.graph(h).unwrap().layers[0];
    let o: Vec<_> = l.plot_offsets.iter().map(|o| (o.column, o.y, o.follows)).collect();
    assert_eq!(o, vec![(others[1], 7.0, Some(others[0])), (others[3], 7.0, Some(others[2]))]);
}

#[test]
fn bubble_slots_find_the_z_column_of_each_new_column() {
    let mut p = Project::new();
    let b = p.add_workbook("A");
    p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![0.0, 1.0])).unwrap();
    let y = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
    let z = p.add_column(b, 0, Designation::Z, ColumnData::numeric(vec![3.0, 4.0])).unwrap();
    let g = p.plot_columns(Template::Bubble, &[y]).unwrap();
    assert_eq!(p.graph(g).unwrap().layers[0].plots[0].size_from, Some(z));
    let t = template_of(&p, g);
    assert_eq!(t.graph.layers[0].plots[0].size_from, Some(ColumnId(1)));

    let b2 = p.add_workbook("B");
    p.add_column(b2, 0, Designation::X, ColumnData::numeric(vec![0.0, 1.0])).unwrap();
    let y2 = p.add_column(b2, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
    let z2 = p.add_column(b2, 0, Designation::Z, ColumnData::numeric(vec![5.0, 6.0])).unwrap();
    let h = p.plot_with_user_template(&t, &[y2]).unwrap();
    assert_eq!(p.graph(h).unwrap().layers[0].plots[0].size_from, Some(z2));
    assert_eq!(p.graph(h).unwrap().layers[0].plots[0].color_from, None);
}

#[test]
fn a_template_of_an_empty_graph_plots_with_the_layer_defaults() {
    let mut p = Project::new();
    let ys = book(&mut p, "A", 2);
    let g = p.add_graph(Template::Scatter);
    let t = template_of(&p, g);
    assert_eq!(t.plot_count(), 0);
    let h = p.plot_with_user_template(&t, &ys).unwrap();
    let plots = &p.graph(h).unwrap().layers[0].plots;
    assert_eq!(plots.len(), 2);
    assert_eq!(plots[0].kind, PlotKind::Scatter);
}

#[test]
fn a_missing_column_is_an_error_and_adds_no_window() {
    let mut p = Project::new();
    let ys = book(&mut p, "A", 1);
    let g = p.plot_columns(Template::Line, &ys).unwrap();
    let t = template_of(&p, g);
    let windows = p.windows.len();
    assert_eq!(p.plot_with_user_template(&t, &[ys[0], ColumnId(777)]), Err(ProjectError::NoSuchColumn(ColumnId(777))));
    assert_eq!(p.windows.len(), windows);
}

#[test]
fn graphs_made_of_their_data_structure_cannot_be_templates() {
    let mut p = Project::new();
    let b = p.add_workbook("A");
    let cat = p.add_column(b, 0, Designation::X, ColumnData::text(vec!["a".into(), "b".into()])).unwrap();
    let val = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
    let tree = p.plot_treemap(&[cat], val).unwrap();
    assert!(matches!(UserTemplate::from_graph("t", p.graph(tree).unwrap()), Err(UserTemplateError::Unsupported(_))));
    let floating = p.plot_columns(Template::FloatingColumn, &[val, val]).unwrap();
    assert!(matches!(
        UserTemplate::from_graph("t", p.graph(floating).unwrap()),
        Err(UserTemplateError::Unsupported(_))
    ));
}

#[test]
fn a_template_survives_json_as_the_io_crate_stores_it() {
    let mut p = Project::new();
    let ys = book(&mut p, "A", 2);
    let g = styled(&mut p, &ys);
    let t = template_of(&p, g);
    let json = serde_json::to_string(&t).unwrap();
    let back: UserTemplate = serde_json::from_str(&json).unwrap();
    assert_eq!(back, t);
}
