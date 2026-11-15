//! The body of the axis dialog: a list at the left to choose the horizontal or the vertical axis of
//! the layer, and the tabs Scale, Tick Labels, Title, Grids, Line and Ticks, Special Ticks and
//! Breaks, and Rug (T41). As in Origin the list names Horizontal and Vertical on the Scale and Breaks tabs and the
//! four edges (Bottom, Top, Left, Right) on the others; the Top and Right pages only hold what Sheaf
//! keeps for the far edge (its line and ticks). "Apply To..." copies the look of the shown axis.

use eframe::egui::{self, DragValue};
use sheaf_core::graph::*;
use sheaf_core::{Color, ColumnId};
use sheaf_plot::ticks::ResolvedAxis;

use super::graph_props::{
    color_button, dash_ui, font_ui, grid, group_box, optional_number, tab_row, tick_direction_ui,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AxisTab {
    Scale,
    TickLabels,
    Title,
    Grids,
    LineAndTicks,
    SpecialTicks,
    Breaks,
    Rug,
}

impl AxisTab {
    /// Origin's order.
    const ALL: [(AxisTab, &'static str, bool); 8] = [
        (AxisTab::Scale, "Scale", true),
        (AxisTab::TickLabels, "Tick Labels", true),
        (AxisTab::Title, "Title", true),
        (AxisTab::Grids, "Grids", true),
        (AxisTab::LineAndTicks, "Line and Ticks", true),
        (AxisTab::SpecialTicks, "Special Ticks", true),
        (AxisTab::Breaks, "Breaks", true),
        (AxisTab::Rug, "Rug", true),
    ];
}

/// The list at the left, the tab row and the page of the chosen tab.
pub fn dialog_body(
    ui: &mut egui::Ui,
    vertical: &mut bool,
    tab: &mut AxisTab,
    graph: &mut Graph,
    li: usize,
    resolved: Option<&ResolvedAxis>,
    columns: &[(ColumnId, String)],
) {
    // Which side of the axis the Bottom/Top/Left/Right list has chosen (kept while the window is open).
    let side_id = egui::Id::new("axis_dialog_far_side");
    let mut far: bool = ui.data(|d| d.get_temp(side_id)).unwrap_or(false);
    let by_edge = !matches!(*tab, AxisTab::Scale | AxisTab::Breaks);
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width(92.0);
            let entries: &[(bool, bool, &str)] = if by_edge {
                &[(false, false, "Bottom"), (false, true, "Top"), (true, false, "Left"), (true, true, "Right")]
            } else {
                &[(false, false, "Horizontal"), (true, false, "Vertical")]
            };
            for (is_vertical, is_far, name) in entries {
                let chosen = *vertical == *is_vertical && (!by_edge || far == *is_far);
                if ui.add_sized([88.0, 30.0], egui::Button::selectable(chosen, *name)).clicked() {
                    *vertical = *is_vertical;
                    far = *is_far;
                }
            }
            ui.add_space(10.0);
            apply_to_button(ui, graph, li, *vertical);
        });
        ui.separator();
        ui.vertical(|ui| {
            tab_row(ui, tab, &AxisTab::ALL);
            ui.separator();
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                let layer = &mut graph.layers[li];
                if *tab == AxisTab::Rug {
                    // One entry per Y column (the rug refers to the first plot with that column).
                    let mut names: Vec<(ColumnId, String)> = Vec::new();
                    for (i, p) in layer.plots.iter().enumerate() {
                        if names.iter().all(|n| n.0 != p.y) {
                            let name = columns
                                .iter()
                                .find(|c| c.0 == p.y)
                                .map_or_else(|| format!("Plot {}", i + 1), |c| c.1.clone());
                            names.push((p.y, name));
                        }
                    }
                    let cartesian = layer.coords.polar().is_none() && layer.coords.ternary().is_none();
                    let axis = if *vertical { &mut layer.y_axis } else { &mut layer.x_axis };
                    let id = if *vertical { "y" } else { "x" };
                    rug_tab(ui, axis, far, cartesian, &names, id);
                    return;
                }
                let (axis, id) = if *vertical { (&mut layer.y_axis, "y") } else { (&mut layer.x_axis, "x") };
                if by_edge && far {
                    far_edge_page(ui, axis, *tab, *vertical, id);
                    return;
                }
                match *tab {
                    AxisTab::Scale => scale_tab(ui, axis, resolved, id),
                    AxisTab::TickLabels => tick_labels_tab(ui, axis, *vertical, columns, id),
                    AxisTab::Title => title_tab(ui, axis, id),
                    AxisTab::Grids => grids_tab(ui, axis, id),
                    AxisTab::LineAndTicks => line_and_ticks_tab(ui, axis, *vertical, id),
                    AxisTab::SpecialTicks => special_ticks_tab(ui, axis, resolved, id),
                    AxisTab::Breaks => breaks_tab(ui, axis, resolved, id),
                    AxisTab::Rug => {} // handled above: it picks its rug by edge
                }
            });
        });
    });
    ui.data_mut(|d| d.insert_temp(side_id, far));
}

/// The rug of the edge the list has chosen: an axis on its own edge keeps its first rug there, an axis
/// drawn on the far edge (Opposite side) the other way round.
pub fn rug_mut(axis: &mut Axis, far: bool) -> &mut Rug {
    if far != (axis.side == AxisSide::Opposite) { &mut axis.rug_opposite } else { &mut axis.rug }
}

/// The Rug tab (graphing spec 18.10): one short line per plotted point in a strip beside the chosen edge.
fn rug_tab(ui: &mut egui::Ui, axis: &mut Axis, far: bool, cartesian: bool, plots: &[(ColumnId, String)], id: &str) {
    if !cartesian {
        ui.weak("Rugs are offered for 2D Cartesian layers only.");
        return;
    }
    let id = format!("{id}_{}", if far { "far" } else { "near" });
    let rug = rug_mut(axis, far);
    ui.checkbox(&mut rug.show, "Show");
    ui.add_enabled_ui(rug.show, |ui| {
        grid(ui, &format!("{id}_rug_head"), |ui| {
            ui.label("Data");
            // A column no plot has any more draws as `<All>` too.
            let shown = rug
                .plot
                .and_then(|c| plots.iter().find(|p| p.0 == c))
                .map_or_else(|| "<All>".to_string(), |p| p.1.clone());
            egui::ComboBox::from_id_salt(format!("{id}_rug_data")).selected_text(shown).show_ui(ui, |ui| {
                ui.selectable_value(&mut rug.plot, None, "<All>");
                for (col, name) in plots {
                    ui.selectable_value(&mut rug.plot, Some(*col), name);
                }
            });
            ui.end_row();
            ui.label("Color");
            ui.horizontal(|ui| {
                let mut auto = rug.color.is_none();
                if ui.checkbox(&mut auto, "Auto").changed() {
                    rug.color = if auto { None } else { Some(Color::BLACK) };
                }
                if let Some(c) = &mut rug.color {
                    color_button(ui, c);
                }
            });
            ui.end_row();
            ui.label("Thickness");
            ui.add(DragValue::new(&mut rug.thickness_pt).range(0.0..=20.0).speed(0.05).suffix(" pt"));
            ui.end_row();
            ui.label("Transparency (%)");
            ui.add(DragValue::new(&mut rug.transparency_pct).range(0.0..=100.0).speed(0.5));
            ui.end_row();
        });
        group_box(ui, "Layout", |ui| {
            grid(ui, &format!("{id}_rug_layout"), |ui| {
                ui.label("Size (%)");
                ui.add(DragValue::new(&mut rug.size_pct).range(0.0..=100.0).speed(0.1).max_decimals(2));
                ui.end_row();
                ui.label("Overlap");
                ui.checkbox(&mut rug.overlap, "");
                ui.end_row();
                ui.label("Gap Between Rugs (%)");
                ui.add_enabled(!rug.overlap, DragValue::new(&mut rug.gap_pct).range(0.0..=100.0).speed(0.5));
                ui.end_row();
                ui.label("Margin (%)");
                ui.add(DragValue::new(&mut rug.margin_pct).range(0.0..=49.0).speed(0.5));
                ui.end_row();
            });
            ui.checkbox(&mut rug.perpendicular_border, "Show Perpendicular Border");
            ui.checkbox(&mut rug.parallel_border, "Show Parallel Border");
        });
    });
}

/// The far edge (top of X, right of Y): Sheaf keeps its line and its ticks with the near axis.
fn far_edge_page(ui: &mut egui::Ui, axis: &mut Axis, tab: AxisTab, vertical: bool, id: &str) {
    let edge = if vertical { "right" } else { "top" };
    if tab == AxisTab::LineAndTicks {
        ui.checkbox(&mut axis.show_opposite, format!("Show Line on the {edge} edge"));
        ui.checkbox(&mut axis.opposite_ticks, format!("Show Ticks on the {edge} edge"))
            .on_hover_text("Uses the tick style and length of the near axis");
        let _ = id;
    } else {
        ui.weak(format!("Labels, title and grid lines of the {edge} edge follow the near axis; Sheaf has no separate settings for them."));
    }
}

/// What "Apply To..." copies from one axis to another: the look, not the scale, limits or title.
fn copy_axis_look(dst: &mut Axis, src: &Axis) {
    let keep = Axis {
        scale: dst.scale,
        min: dst.min,
        max: dst.max,
        auto_range: dst.auto_range,
        rescale_margin_pct: dst.rescale_margin_pct,
        major_step: dst.major_step,
        minor_ticks: dst.minor_ticks,
        title: dst.title.clone(),
        side: dst.side,
        reverse: dst.reverse,
        first_tick: dst.first_tick,
        axis_breaks: dst.axis_breaks.clone(),
        rug: dst.rug.clone(),
        rug_opposite: dst.rug_opposite.clone(),
        special_ticks: dst.special_ticks.clone(),
        tick_label_source: dst.tick_label_source,
        ..src.clone()
    };
    *dst = keep;
}

/// The button "Apply To...": copies the look of the shown axis to the other axis of the layer, to the same
/// axis of every layer, or to every axis of the graph.
fn apply_to_button(ui: &mut egui::Ui, graph: &mut Graph, li: usize, vertical: bool) {
    ui.menu_button("Apply To...", |ui| {
        let src = if vertical { graph.layers[li].y_axis.clone() } else { graph.layers[li].x_axis.clone() };
        let mut done = false;
        if ui.button("Other axis of this layer").clicked() {
            let l = &mut graph.layers[li];
            copy_axis_look(if vertical { &mut l.x_axis } else { &mut l.y_axis }, &src);
            done = true;
        }
        if ui.button("Same axis in all layers").clicked() {
            for l in &mut graph.layers {
                copy_axis_look(if vertical { &mut l.y_axis } else { &mut l.x_axis }, &src);
            }
            done = true;
        }
        if ui.button("Both axes in all layers").clicked() {
            for l in &mut graph.layers {
                copy_axis_look(&mut l.x_axis, &src);
                copy_axis_look(&mut l.y_axis, &src);
            }
            done = true;
        }
        if done {
            ui.close();
        }
    });
}

fn special_ticks_tab(ui: &mut egui::Ui, axis: &mut Axis, resolved: Option<&ResolvedAxis>, id: &str) {
    ui.label("Extra ticks at chosen values, drawn as major ticks. An empty label shows the value.");
    let mut remove = None;
    egui::Grid::new(format!("{id}_special")).num_columns(3).spacing([10.0, 6.0]).show(ui, |ui| {
        ui.strong("Value");
        ui.strong("Label");
        ui.label("");
        ui.end_row();
        for (i, t) in axis.special_ticks.iter_mut().enumerate() {
            ui.add(DragValue::new(&mut t.value).speed(0.1).max_decimals(8));
            ui.add(egui::TextEdit::singleline(&mut t.label).desired_width(140.0));
            if ui.button("Remove").clicked() {
                remove = Some(i);
            }
            ui.end_row();
        }
    });
    if let Some(i) = remove {
        axis.special_ticks.remove(i);
    }
    if ui.button("Add Tick").clicked() {
        let mid = resolved.map_or(0.0, |r| r.from_unit(0.5));
        axis.special_ticks.push(SpecialTick { value: mid, label: String::new() });
    }
}

/// Origin's Breaks tab (graphing spec 3.6): Enable, the gap shared by the breaks (Origin's Break Half
/// Length; Sheaf states the whole gap in percent of the axis length), the Number of Breaks (None, 1 to 7)
/// and a row per break with Break From, Break To, Auto Position and Position. A new row takes the middle
/// half of what lies after the last break, as Origin filled the rows one at a time. Sheaf's "Step after"
/// stands for the tick step of Origin's Details... page.
fn breaks_tab(ui: &mut egui::Ui, axis: &mut Axis, resolved: Option<&ResolvedAxis>, id: &str) {
    let (lo, hi) = resolved.map_or((0.0, 1.0), |r| (r.min, r.max));
    let span = (hi - lo).abs();
    let speed = (span / 200.0).max(1e-12);
    let mut on = !axis.axis_breaks.is_empty();
    if ui.checkbox(&mut on, "Enable").changed() {
        if on {
            axis.axis_breaks = vec![AxisBreak::next_after(&[], lo, hi)];
        } else {
            axis.axis_breaks.clear();
        }
    }
    ui.add_enabled_ui(on, |ui| {
        grid(ui, &format!("{id}_breaks_head"), |ui| {
            ui.label("Gap (% of axis)");
            let mut gap = axis.axis_breaks.first().map_or(BREAK_GAP_PCT, |b| b.gap_pct);
            if ui.add(DragValue::new(&mut gap).range(0.0..=30.0).speed(0.05).max_decimals(3)).changed() {
                axis.axis_breaks.iter_mut().for_each(|b| b.gap_pct = gap);
            }
            ui.end_row();
            ui.label("Number of Breaks");
            let n = axis.axis_breaks.len();
            let name = |k: usize| if k == 0 { "None".to_string() } else { k.to_string() };
            let mut want = n;
            egui::ComboBox::from_id_salt(format!("{id}_break_count")).selected_text(name(n)).show_ui(ui, |ui| {
                for k in 0..=MAX_AXIS_BREAKS {
                    ui.selectable_value(&mut want, k, name(k));
                }
            });
            if want != n {
                axis.axis_breaks.truncate(want);
                while axis.axis_breaks.len() < want {
                    let next = AxisBreak::next_after(&axis.axis_breaks, lo, hi);
                    axis.axis_breaks.push(next);
                }
            }
            ui.end_row();
        });
        let n = axis.axis_breaks.len();
        let mut remove = None;
        if n > 0 {
            egui::Grid::new(format!("{id}_breaks_table")).striped(true).show(ui, |ui| {
                for head in ["Break From", "Break To", "Auto Position", "Position (%)", "Step after", ""] {
                    ui.strong(head);
                }
                ui.end_row();
                for (k, b) in axis.axis_breaks.iter_mut().enumerate() {
                    ui.add(DragValue::new(&mut b.from).speed(speed).max_decimals(8));
                    ui.add(DragValue::new(&mut b.to).speed(speed).max_decimals(8));
                    ui.checkbox(&mut b.auto_position, "");
                    if b.auto_position {
                        // The place the break gets, shown greyed as in Origin.
                        let mut auto = 100.0 * (k + 1) as f64 / (n + 1) as f64;
                        ui.add_enabled(false, DragValue::new(&mut auto).max_decimals(0));
                    } else {
                        ui.add(DragValue::new(&mut b.position_pct).range(2.0..=98.0).speed(0.5));
                    }
                    optional_number(ui, &mut b.step_after, resolved.map(|r| r.step), speed);
                    if ui.small_button("Delete").clicked() {
                        remove = Some(k);
                    }
                    ui.end_row();
                    if b.to < b.from {
                        std::mem::swap(&mut b.from, &mut b.to);
                    }
                }
            });
        }
        if let Some(k) = remove {
            axis.axis_breaks.remove(k);
        }
    });
}

fn scale_tab(ui: &mut egui::Ui, axis: &mut Axis, resolved: Option<&ResolvedAxis>, id: &str) {
    let span = resolved.map(|r| (r.max - r.min).abs()).unwrap_or(1.0);
    let speed = (span / 200.0).max(1e-12);
    grid(ui, &format!("{id}_scale"), |ui| {
        ui.label("From");
        optional_number(ui, &mut axis.min, resolved.map(|r| r.min), speed);
        ui.end_row();
        ui.label("To");
        optional_number(ui, &mut axis.max, resolved.map(|r| r.max), speed);
        ui.end_row();
        ui.label("Type");
        egui::ComboBox::from_id_salt(format!("{id}_type")).selected_text(axis.scale.label()).show_ui(ui, |ui| {
            for kind in ScaleKind::ALL {
                ui.selectable_value(&mut axis.scale, kind, kind.label());
            }
        });
        ui.end_row();
        ui.label("Rescale");
        egui::ComboBox::from_id_salt(format!("{id}_auto"))
            .selected_text(match axis.auto_range {
                AutoRange::Nice => "Round out to ticks",
                AutoRange::Tight => "Exact data range",
                AutoRange::SymmetricZero => "Symmetric about zero",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut axis.auto_range, AutoRange::Nice, "Round out to ticks");
                ui.selectable_value(&mut axis.auto_range, AutoRange::Tight, "Exact data range");
                ui.selectable_value(&mut axis.auto_range, AutoRange::SymmetricZero, "Symmetric about zero");
            });
        ui.end_row();
        ui.label("Rescale Margin(%)");
        // Used by "Round out to ticks" (Origin's Normal rescale); the other two modes ignore it.
        ui.add_enabled(
            axis.auto_range == AutoRange::Nice,
            DragValue::new(&mut axis.rescale_margin_pct).range(0.0..=100.0).speed(0.5).max_decimals(2),
        );
        ui.end_row();
        ui.label("Reverse");
        ui.checkbox(&mut axis.reverse, "");
        ui.end_row();
    });
    group_box(ui, "Major Ticks", |ui| {
        grid(ui, &format!("{id}_major"), |ui| {
            ui.label("Type");
            ui.label("By Increment");
            ui.end_row();
            ui.label("Value");
            optional_number(ui, &mut axis.major_step, resolved.map(|r| r.step), speed);
            ui.end_row();
            ui.label("First Tick");
            ui.add_enabled_ui(axis.scale == ScaleKind::Linear, |ui| {
                optional_number(
                    ui,
                    &mut axis.first_tick,
                    resolved.map(|r| r.major.first().copied().unwrap_or(r.min)),
                    speed,
                );
            });
            ui.end_row();
        });
    });
    group_box(ui, "Minor Ticks", |ui| {
        grid(ui, &format!("{id}_minor"), |ui| {
            ui.label("Type");
            ui.label("By Counts");
            ui.end_row();
            ui.label("Count");
            ui.add(DragValue::new(&mut axis.minor_ticks).range(0..=9));
            ui.end_row();
        });
    });
}

fn tick_labels_tab(ui: &mut egui::Ui, axis: &mut Axis, vertical: bool, columns: &[(ColumnId, String)], id: &str) {
    ui.checkbox(&mut axis.show_tick_labels, "Show");
    // Format, Table (where the labels come from) and Minor Tick Labels, as in Origin.
    let sub_id = egui::Id::new("axis_dialog_tick_label_page");
    let mut sub: u8 = ui.data(|d| d.get_temp(sub_id)).unwrap_or(0);
    ui.horizontal(|ui| {
        for (k, name) in ["Format", "Table", "Minor Tick Labels"].into_iter().enumerate() {
            if ui.add(egui::Button::selectable(sub == k as u8, name)).clicked() {
                sub = k as u8;
            }
        }
    });
    ui.data_mut(|d| d.insert_temp(sub_id, sub));
    if sub == 2 {
        ui.add_enabled_ui(axis.show_tick_labels, |ui| {
            ui.checkbox(&mut axis.minor_tick_labels, "Show Minor Tick Labels")
                .on_hover_text("Writes a label at every minor tick, in the format of the major labels");
        });
        return;
    }
    ui.add_enabled_ui(axis.show_tick_labels, |ui| {
        group_box(ui, if sub == 1 { "Table" } else { "Display" }, |ui| {
            grid(ui, &format!("{id}_labels"), |ui| {
                if sub == 1 {
                    ui.label("Type");
                    ui.horizontal(|ui| {
                        let name = match axis.tick_label_source {
                            TickLabelSource::Auto => "Automatic",
                            TickLabelSource::Numbers => "Numeric",
                            TickLabelSource::Column { .. } => "Text from dataset",
                        };
                        egui::ComboBox::from_id_salt(format!("{id}_source")).selected_text(name).show_ui(ui, |ui| {
                            // The automatic mode only exists for X: a Y axis always shows numbers.
                            if !vertical {
                                ui.selectable_value(&mut axis.tick_label_source, TickLabelSource::Auto, "Automatic");
                            }
                            ui.selectable_value(&mut axis.tick_label_source, TickLabelSource::Numbers, "Numeric");
                            let is_column = matches!(axis.tick_label_source, TickLabelSource::Column { .. });
                            let first = columns.first().map(|(c, _)| *c);
                            if ui.selectable_label(is_column, "Text from dataset").clicked()
                                && !is_column
                                && let Some(column) = first
                            {
                                axis.tick_label_source = TickLabelSource::Column { column };
                            }
                        });
                        if let TickLabelSource::Column { column } = &mut axis.tick_label_source {
                            let shown =
                                columns.iter().find(|(c, _)| c == column).map(|(_, n)| n.as_str()).unwrap_or("?");
                            egui::ComboBox::from_id_salt(format!("{id}_source_col")).selected_text(shown).show_ui(
                                ui,
                                |ui| {
                                    for (c, n) in columns {
                                        ui.selectable_value(column, *c, n);
                                    }
                                },
                            );
                        }
                    });
                    ui.end_row();
                }
                if sub == 0 {
                    ui.label("Display");
                    ui.horizontal(|ui| {
                        let mut kind = match axis.tick_format {
                            TickFormat::Auto => 0,
                            TickFormat::Decimal(_) => 1,
                            TickFormat::Scientific(_) => 2,
                            // Set automatically for a date column; the menu shows it as Automatic.
                            TickFormat::DateTime(_) => 0,
                        };
                        let digits = match axis.tick_format {
                            TickFormat::Decimal(d) | TickFormat::Scientific(d) => d,
                            TickFormat::Auto | TickFormat::DateTime(_) => 2,
                        };
                        let before = kind;
                        egui::ComboBox::from_id_salt(format!("{id}_fmt"))
                            .selected_text(["Automatic", "Decimal", "Scientific"][kind])
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut kind, 0, "Automatic");
                                ui.selectable_value(&mut kind, 1, "Decimal");
                                ui.selectable_value(&mut kind, 2, "Scientific");
                            });
                        let mut d = digits;
                        if kind != 0 {
                            ui.add(DragValue::new(&mut d).range(0..=10).suffix(" digits"));
                        }
                        if kind != before || d != digits {
                            axis.tick_format = match kind {
                                1 => TickFormat::Decimal(d),
                                2 => TickFormat::Scientific(d),
                                _ => TickFormat::Auto,
                            };
                        }
                    });
                    ui.end_row();
                    ui.label("Divide by Factor");
                    ui.add(DragValue::new(&mut axis.tick_label_divisor).speed(0.1).max_decimals(9)).on_hover_text(
                        "Labels show the value divided by this factor, for example 0.001 to show A as mA",
                    );
                    if axis.tick_label_divisor == 0.0 {
                        axis.tick_label_divisor = 1.0;
                    }
                    ui.end_row();
                    ui.label("Rotate (deg.)");
                    ui.add_enabled(
                        !vertical,
                        DragValue::new(&mut axis.tick_label_angle_deg).range(-90.0..=90.0).speed(1.0),
                    )
                    .on_hover_text("Turns the labels of a horizontal axis counter-clockwise");
                    ui.end_row();
                    ui.label("Hide Last Label");
                    ui.checkbox(&mut axis.hide_end_label, "")
                        .on_hover_text("Leaves out the label at the far end, for panels that touch");
                    ui.end_row();
                    ui.label("Prefix");
                    ui.add(egui::TextEdit::singleline(&mut axis.tick_prefix).desired_width(120.0));
                    ui.end_row();
                    ui.label("Suffix");
                    ui.add(egui::TextEdit::singleline(&mut axis.tick_suffix).desired_width(120.0));
                    ui.end_row();
                    ui.label("Formula");
                    ui.add(
                        egui::TextEdit::singleline(&mut axis.tick_label_formula).desired_width(120.0).hint_text("x"),
                    )
                    .on_hover_text("An expression of x, the tick value, written as the label, for example x*100");
                    ui.end_row();
                }
            });
        });
        if sub == 0 {
            group_box(ui, "Font", |ui| {
                font_ui(ui, &mut axis.tick_font);
            });
        }
    });
}

fn title_tab(ui: &mut egui::Ui, axis: &mut Axis, id: &str) {
    ui.checkbox(&mut axis.show_title, "Show");
    ui.add_enabled_ui(axis.show_title, |ui| {
        grid(ui, &format!("{id}_title"), |ui| {
            ui.label("Text");
            ui.horizontal(|ui| {
                let mut auto = axis.title.is_none();
                if ui
                    .checkbox(&mut auto, "From column")
                    .on_hover_text("Long name and units of the plotted column")
                    .changed()
                {
                    axis.title = if auto { None } else { Some(String::new()) };
                }
                if let Some(t) = &mut axis.title {
                    ui.add(egui::TextEdit::singleline(t).desired_width(220.0));
                }
            });
            ui.end_row();
            ui.label("Color");
            ui.horizontal(|ui| {
                ui.checkbox(&mut axis.title_color_follows_plot, "Follow plot");
                if !axis.title_color_follows_plot {
                    color_button(ui, &mut axis.title_font.color);
                }
            });
            ui.end_row();
            ui.label("Rotate (deg.)");
            ui.add_enabled(false, DragValue::new(&mut 0.0));
            ui.end_row();
        });
        ui.weak("Markup: \\+(superscript)  \\-(subscript)  \\g(greek)  \\b(bold)  \\i(italic)");
        group_box(ui, "Font", |ui| {
            font_ui(ui, &mut axis.title_font);
        });
    });
}

fn grids_tab(ui: &mut egui::Ui, axis: &mut Axis, id: &str) {
    grid_group(
        ui,
        "Major Grid Lines",
        &format!("{id}_major_grid"),
        &mut axis.major_grid,
        Color::rgb(128, 128, 128),
        0.5,
        Dash::Solid,
    );
    grid_group(
        ui,
        "Minor Grid Lines",
        &format!("{id}_minor_grid"),
        &mut axis.minor_grid,
        Color::rgb(200, 200, 200),
        0.3,
        Dash::Dash,
    );
    // Origin 2016's Additional Lines: Opposite (the axis line on the far edge, the same setting as
    // Line and Ticks) and a line at a value of the other axis ("Y =" on the X axis page).
    group_box(ui, "Additional Lines", |ui| {
        ui.checkbox(&mut axis.show_opposite, "Opposite");
        ui.horizontal(|ui| {
            let mut on = axis.additional_line.is_some();
            let label = if id == "x" { "Y =" } else { "X =" };
            if ui.checkbox(&mut on, label).changed() {
                axis.additional_line = on.then_some(0.0);
            }
            let mut v = axis.additional_line.unwrap_or(0.0);
            if ui.add_enabled(on, DragValue::new(&mut v).speed(0.1)).changed() && on {
                axis.additional_line = Some(v);
            }
        });
    });
}

/// One group of the Grids tab: Show, Color, Style and Thickness; the boxes are grey until Show is ticked.
fn grid_group(
    ui: &mut egui::Ui,
    title: &str,
    id: &str,
    line: &mut Option<LineStyle>,
    color: Color,
    width: f64,
    dash: Dash,
) {
    group_box(ui, title, |ui| {
        let mut on = line.is_some();
        if ui.checkbox(&mut on, "Show").changed() {
            *line = on.then_some(LineStyle { color, width_pt: width, dash, connect: Connect::Straight, sort_x: false });
        }
        ui.add_enabled_ui(line.is_some(), |ui| {
            let mut fallback = LineStyle { color, width_pt: width, dash, connect: Connect::Straight, sort_x: false };
            let l = line.as_mut().unwrap_or(&mut fallback);
            grid(ui, id, |ui| {
                ui.label("Color");
                color_button(ui, &mut l.color);
                ui.end_row();
                ui.label("Style");
                dash_ui(ui, &format!("{id}_dash"), &mut l.dash);
                ui.end_row();
                ui.label("Thickness");
                ui.add(DragValue::new(&mut l.width_pt).range(0.1..=5.0).speed(0.05).suffix(" pt"));
                ui.end_row();
            });
        });
    });
}

fn line_and_ticks_tab(ui: &mut egui::Ui, axis: &mut Axis, vertical: bool, id: &str) {
    let mut shown = axis.side != AxisSide::Hidden;
    if ui.checkbox(&mut shown, "Show Line and Ticks").changed() {
        axis.side = if shown { AxisSide::Normal } else { AxisSide::Hidden };
    }
    ui.add_enabled_ui(shown, |ui| {
        group_box(ui, "Line", |ui| {
            grid(ui, &format!("{id}_line"), |ui| {
                ui.label("Color");
                ui.horizontal(|ui| {
                    ui.checkbox(&mut axis.color_follows_plot, "Follow plot");
                    if !axis.color_follows_plot {
                        color_button(ui, &mut axis.color);
                    }
                });
                ui.end_row();
                ui.label("Thickness");
                ui.add(DragValue::new(&mut axis.line_width_pt).range(0.0..=10.0).speed(0.05).suffix(" pt"));
                ui.end_row();
                ui.label("Axis Position");
                let (normal, opposite) = if vertical { ("Left", "Right") } else { ("Bottom", "Top") };
                egui::ComboBox::from_id_salt(format!("{id}_side"))
                    .selected_text(if axis.side == AxisSide::Opposite { opposite } else { normal })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut axis.side, AxisSide::Normal, normal);
                        ui.selectable_value(&mut axis.side, AxisSide::Opposite, opposite);
                    });
                ui.end_row();
                ui.label("Opposite side");
                ui.horizontal(|ui| {
                    ui.checkbox(&mut axis.show_opposite, "Line")
                        .on_hover_text("With both axes: a closed box around the plot");
                    ui.checkbox(&mut axis.opposite_ticks, "Ticks");
                });
                ui.end_row();
            });
        });
        group_box(ui, "All Ticks", |ui| {
            grid(ui, &format!("{id}_all_ticks"), |ui| {
                ui.label("Color");
                ui.horizontal(|ui| {
                    let mut same = axis.tick_color.is_none();
                    if ui.checkbox(&mut same, "Same as line").changed() {
                        axis.tick_color = if same { None } else { Some(axis.color) };
                    }
                    if let Some(c) = axis.tick_color.as_mut() {
                        color_button(ui, c);
                    }
                });
                ui.end_row();
                ui.label("Thickness");
                ui.horizontal(|ui| {
                    let mut same = axis.tick_width_pt.is_none();
                    if ui.checkbox(&mut same, "Same as line").changed() {
                        axis.tick_width_pt = if same { None } else { Some(axis.line_width_pt) };
                    }
                    if let Some(w) = axis.tick_width_pt.as_mut() {
                        ui.add(DragValue::new(w).range(0.0..=10.0).speed(0.05).suffix(" pt"));
                    }
                });
                ui.end_row();
            });
        });
        group_box(ui, "Major Ticks", |ui| {
            grid(ui, &format!("{id}_major_ticks"), |ui| {
                ui.label("Style");
                tick_direction_ui(ui, &format!("{id}_major_dir"), &mut axis.major_tick);
                ui.end_row();
                ui.label("Length");
                ui.add(DragValue::new(&mut axis.major_tick_len_pt).range(0.0..=30.0).speed(0.1).suffix(" pt"));
                ui.end_row();
            });
        });
        group_box(ui, "Minor Ticks", |ui| {
            grid(ui, &format!("{id}_minor_ticks"), |ui| {
                ui.label("Style");
                tick_direction_ui(ui, &format!("{id}_minor_dir"), &mut axis.minor_tick);
                ui.end_row();
                ui.label("Length");
                ui.add(DragValue::new(&mut axis.minor_tick_len_pt).range(0.0..=30.0).speed(0.1).suffix(" pt"));
                ui.end_row();
            });
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use sheaf_core::template::Template;

    #[test]
    fn apply_to_copies_the_look_but_not_scale_limits_or_title() {
        let g = Template::Line.new_graph(sheaf_core::WindowId(1), "G");
        let mut src = g.layers[0].x_axis.clone();
        src.line_width_pt = 2.5;
        src.tick_color = Some(Color::rgb(255, 0, 0));
        src.scale = ScaleKind::Ln;
        src.title = Some("src".into());
        let mut dst = g.layers[0].y_axis.clone();
        dst.min = Some(3.0);
        dst.title = Some("dst".into());
        copy_axis_look(&mut dst, &src);
        assert_eq!((dst.line_width_pt, dst.tick_color), (2.5, Some(Color::rgb(255, 0, 0))));
        assert_eq!((dst.scale, dst.min, dst.title.as_deref()), (ScaleKind::Linear, Some(3.0), Some("dst")));
    }
}
