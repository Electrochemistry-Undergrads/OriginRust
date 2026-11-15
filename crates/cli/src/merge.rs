//! `sheaf merge-graphs`: Graph > Merge Graph Windows without a window (graphing spec 15.11).

use crate::commands::{load_project, save_project};
use anyhow::{Context, Result, bail};
use sheaf_core::WindowId;
use sheaf_core::merge::{MergeOptions, MergeTextScale};
use sheaf_ops::args::MergeGraphsArgs;
use sheaf_ops::render::ExportSettings;

/// The text size choice of `--text-scale`: `auto`, `keep` or a factor.
pub(crate) fn text_scale(text: &str) -> Result<MergeTextScale> {
    match text.trim().to_ascii_lowercase().as_str() {
        "auto" => Ok(MergeTextScale::Auto),
        "keep" => Ok(MergeTextScale::Keep),
        t => match t.parse::<f64>() {
            Ok(f) if f.is_finite() => Ok(MergeTextScale::Fixed(f)),
            _ => bail!("--text-scale takes auto, keep or a factor such as 0.5, not '{text}'"),
        },
    }
}

pub(crate) fn merge_graphs(a: MergeGraphsArgs) -> Result<()> {
    if a.output.is_none() && a.save.is_none() {
        bail!("say where the merged graph goes: --output for a figure, --save for the project, or both");
    }
    if !(a.dpi.is_finite() && a.dpi > 0.0) {
        bail!("--dpi must be a positive number");
    }
    let mut project = load_project(&a.project)?;
    let sources: Vec<WindowId> = if a.all_graphs {
        project.graphs().map(|g| g.id).collect()
    } else {
        a.graph.iter().map(|n| sheaf_ops::pick::pick_graph(&project, Some(n))).collect::<Result<_>>()?
    };
    let mut opts = MergeOptions {
        sources,
        rows: a.rows,
        cols: a.cols,
        portrait: a.portrait,
        keep_aspect: a.keep_aspect,
        link: a.link,
        share_x: a.share_x,
        hide_inner: a.hide_inner,
        keep_sources: !a.remove_sources,
        text_scale: text_scale(&a.text_scale)?,
        ..MergeOptions::default()
    };
    match a.gap[..] {
        [] => {}
        [h, v] => (opts.h_gap, opts.v_gap) = (h, v),
        _ => bail!("--gap takes two numbers, the horizontal and the vertical gap (for example 5,0)"),
    }
    match a.margins[..] {
        [] => {}
        [l, r, t, b] => (opts.margin_left, opts.margin_right, opts.margin_top, opts.margin_bottom) = (l, r, t, b),
        _ => bail!("--margins takes four numbers: left, right, top and bottom (for example 15,10,10,15)"),
    }
    match a.page_size[..] {
        [] => {}
        [w, h] => opts.page_in = Some((w, h)),
        _ => bail!("--page-size takes two numbers in inches, the width and the height (for example 8,10)"),
    }
    let id = project.merge_graphs(&opts).context("cannot merge the graphs")?;
    let g = project.graph(id).expect("just made");
    println!("merged {} graphs into {} ({} layers)", opts.sources.len(), g.name, g.layers.len());
    if let Some(out) = &a.output {
        let settings = ExportSettings { dpi: a.dpi, ..ExportSettings::default() };
        sheaf_ops::render::write_graph(&project, id, out, &settings)?;
        println!("wrote {}", out.display());
    }
    if let Some(path) = &a.save {
        save_project(&project, path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_scale_takes_auto_keep_or_a_factor() {
        assert_eq!(text_scale("auto").unwrap(), MergeTextScale::Auto);
        assert_eq!(text_scale(" Keep ").unwrap(), MergeTextScale::Keep);
        assert_eq!(text_scale("0.5").unwrap(), MergeTextScale::Fixed(0.5));
        assert!(text_scale("half").is_err());
        assert!(text_scale("inf").is_err());
    }
}
