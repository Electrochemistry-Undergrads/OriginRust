//! CLI plot arguments converted to shared operation specifications.
use crate as sheaf_ops;
use crate::args::{AxisColors, PanelLinkArg, PanelOrderArg, PlotArgs, StyleArgs};
use crate::plot::{PanelsSpec, PlotSpec, Style};

pub fn style_spec(s: &StyleArgs) -> Style {
    Style {
        x_title: s.x_title.clone(),
        y_title: s.y_title.clone(),
        x_min: s.x_min,
        x_max: s.x_max,
        y_min: s.y_min,
        y_max: s.y_max,
        x_step: s.x_step,
        y_step: s.y_step,
        y2_title: s.y2_title.clone(),
        y2_min: s.y2_min,
        y2_max: s.y2_max,
        y2_step: s.y2_step,
        axis_colors: s.axis_colors.map(|m| match m {
            AxisColors::Plot => sheaf_ops::plot::AxisColors::Plot,
            AxisColors::Fixed => sheaf_ops::plot::AxisColors::Fixed,
        }),
        width: s.width,
        height: s.height,
        line_width: s.line_width,
        colors: s.colors.clone(),
        labels: s.labels.clone(),
        no_legend: s.no_legend,
        stack: s.stack.clone(),
        offsets: s.offsets.clone(),
        waterfall: s.waterfall.clone(),
        no_y_tick_labels: s.no_y_tick_labels,
        no_y_ticks: s.no_y_ticks,
        x_labels: s.x_labels.clone(),
        x_label_angle: s.x_label_angle,
        bar_gap: s.bar_gap,
        bar_overlap: s.bar_overlap,
        label: s.label.clone(),
        draw: s.draw.clone(),
    }
}

pub fn plot_spec(a: &PlotArgs) -> PlotSpec {
    PlotSpec {
        template: a.template.clone(),
        y: a.y.clone(),
        y2: a.y2.clone(),
        book: a.book.clone(),
        sheet: a.sheet.clone(),
        graph: a.graph.clone(),
        panels: a.panels.as_ref().map(|shape| PanelsSpec {
            shape: shape.clone(),
            panel_y: a.panel_y.clone(),
            per_panel: a.per_panel.clone(),
            gap: a.panel_gap.clone(),
            margins: a.panel_margins.clone(),
            order: a.panel_order.map(|o| match o {
                PanelOrderArg::BottomUp => sheaf_ops::plot::PanelOrderSetting::BottomUp,
                PanelOrderArg::TopDown => sheaf_ops::plot::PanelOrderSetting::TopDown,
            }),
            link: a.panel_link.map(|l| match l {
                PanelLinkArg::X => sheaf_ops::plot::PanelLinkSetting::X,
                PanelLinkArg::Y => sheaf_ops::plot::PanelLinkSetting::Y,
                PanelLinkArg::Xy => sheaf_ops::plot::PanelLinkSetting::Xy,
                PanelLinkArg::None => sheaf_ops::plot::PanelLinkSetting::None,
            }),
        }),
        overlay: a.inputs.len() > 1,
        overlay_books: Vec::new(),
        style: style_spec(&a.style),
    }
}
