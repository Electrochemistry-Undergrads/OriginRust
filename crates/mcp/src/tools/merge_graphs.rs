use super::{Res, ToolError, ToolOut, args};
use crate::exec::Ctx;
use anyhow::Context;
use serde::Deserialize;
use serde_json::{Value, json};
use sheaf_core::merge::{MergeOptions, MergeTextScale};
use sheaf_ops::pick::pick_graph;

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct MergeArgs {
    graphs: Option<Vec<String>>,
    all_graphs: bool,
    rows: Option<usize>,
    cols: Option<usize>,
    gap: Option<[f64; 2]>,
    margins: Option<[f64; 4]>,
    page_size: Option<[f64; 2]>,
    portrait: bool,
    keep_aspect: bool,
    group_sources: Option<bool>,
    rearrange: Option<bool>,
    add_extra_layers: bool,
    link: bool,
    share_x: bool,
    hide_inner: bool,
    text_scale: Option<TextScale>,
    remove_sources: bool,
}

#[derive(Deserialize)]
#[serde(untagged, expecting = "\"auto\", \"keep\", or a numeric text scale factor")]
enum TextScale {
    Named(TextScaleName),
    Fixed(f64),
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum TextScaleName {
    Auto,
    Keep,
}

pub(super) fn run(ctx: &mut Ctx, raw: &Value) -> Res {
    let a: MergeArgs = args(raw)?;
    let project = &mut *ctx.project;
    let sources = match (a.graphs, a.all_graphs) {
        (Some(names), false) => names.iter().map(|n| pick_graph(project, Some(n))).collect::<Result<Vec<_>, _>>()?,
        (None, true) => project.graphs().map(|g| g.id).collect(),
        _ => return Err(ToolError::new("give either graphs (at least two names) or all_graphs: true")),
    };
    let names: Vec<_> = sources.iter().filter_map(|&id| project.graph(id)).map(|g| g.name.clone()).collect();
    let mut options = MergeOptions {
        sources,
        rows: a.rows,
        cols: a.cols,
        portrait: a.portrait,
        keep_aspect: a.keep_aspect,
        add_extra_layers: a.add_extra_layers,
        link: a.link,
        share_x: a.share_x,
        hide_inner: a.hide_inner,
        keep_sources: !a.remove_sources,
        ..MergeOptions::default()
    };
    if let Some(rearrange) = a.rearrange {
        options.rearrange = rearrange;
    }
    if let Some(group) = a.group_sources {
        options.group_sources = group;
    }
    if let Some([horizontal, vertical]) = a.gap {
        (options.h_gap, options.v_gap) = (horizontal, vertical);
    }
    if let Some([left, right, top, bottom]) = a.margins {
        (options.margin_left, options.margin_right, options.margin_top, options.margin_bottom) =
            (left, right, top, bottom);
    }
    options.page_in = a.page_size.map(|[w, h]| if a.portrait { (w.min(h), w.max(h)) } else { (w, h) });
    if let Some(scale) = a.text_scale {
        options.text_scale = match scale {
            TextScale::Named(TextScaleName::Auto) => MergeTextScale::Auto,
            TextScale::Named(TextScaleName::Keep) => MergeTextScale::Keep,
            TextScale::Fixed(factor) => MergeTextScale::Fixed(factor),
        };
    }
    let id = project.merge_graphs(&options).context("cannot merge the graphs")?;
    let graph = project.graph(id).expect("just merged");
    Ok(ToolOut::json(json!({
        "graph": graph.name, "layers": graph.layers.len(), "sources": names, "removed_sources": a.remove_sources,
    })))
}
