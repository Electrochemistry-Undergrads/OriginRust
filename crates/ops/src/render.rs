//! Image export: size, margins and the bytes or file of a rendered graph.

use anyhow::{Context, Result, bail};
use sheaf_core::{Project, WindowId};
use std::path::Path;

/// The margin options of an export.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct MarginSettings {
    /// auto, border, tight, page or tight-in-page.
    pub margin: String,
    /// Border width in pixels.
    pub border_width: Option<f64>,
}

impl Default for MarginSettings {
    fn default() -> Self {
        Self { margin: "auto".to_string(), border_width: None }
    }
}

/// How a figure is rendered.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ExportSettings {
    pub dpi: f64,
    /// A length with its unit, or a percentage.
    pub image_width: Option<String>,
    pub image_height: Option<String>,
    pub margin: MarginSettings,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self { dpi: 300.0, image_width: None, image_height: None, margin: MarginSettings::default() }
    }
}

impl ExportSettings {
    /// Checks the settings and gives the size and layout the renderer takes.
    pub fn resolve(&self) -> Result<(sheaf_plot::ExportSize, sheaf_plot::ExportLayout)> {
        let size = export_size(self.image_width.as_deref(), self.image_height.as_deref(), self.dpi)?;
        Ok((size, export_layout(&self.margin)?))
    }
}

/// Renders a graph to the bytes of a figure; `ext` is the format (png, svg, pdf, ...).
pub fn render_graph(project: &Project, graph: WindowId, settings: &ExportSettings, ext: &str) -> Result<Vec<u8>> {
    crate::pick::require_positive("--dpi", Some(settings.dpi))?;
    let (size, layout) = settings.resolve()?;
    sheaf_plot::export_bytes_with(project, graph, ext, settings.dpi, size, layout)
        .with_context(|| format!("could not render the graph as {ext}"))
}

/// Renders a graph into a figure file; the format follows the extension.
pub fn write_graph(project: &Project, graph: WindowId, path: &Path, settings: &ExportSettings) -> Result<()> {
    let (size, layout) = settings.resolve()?;
    sheaf_plot::export_file_with(project, graph, path, settings.dpi, size, layout)
        .with_context(|| format!("could not write '{}'", path.display()))
}

/// Renders several graphs into one multi-page PDF file, one page per graph in the order given
/// (Origin's Export Graphs `multipage`); the size and margin settings apply to every page.
pub fn write_graphs_pdf(project: &Project, graphs: &[WindowId], path: &Path, settings: &ExportSettings) -> Result<()> {
    let (size, layout) = settings.resolve()?;
    sheaf_plot::export_multipage_pdf_file(project, graphs, path, size, layout)
        .with_context(|| format!("could not write '{}'", path.display()))
}

/// The export size from `--image-width` and `--image-height`; a value in `%` is a page ratio.
pub fn export_size(width: Option<&str>, height: Option<&str>, dpi: f64) -> Result<sheaf_plot::ExportSize> {
    let percent = |text: Option<&str>, what: &str| -> Result<Option<f64>> {
        let Some(t) = text.map(str::trim).filter(|t| t.ends_with('%')) else { return Ok(None) };
        let v: f64 = t
            .trim_end_matches('%')
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("--image-{what} '{t}' is not a percentage such as 50%"))?;
        if !(v.is_finite() && v > 0.0) {
            bail!("--image-{what} '{t}': a percentage must be positive");
        }
        Ok(Some(v))
    };
    let (pw, ph) = (percent(width, "width")?, percent(height, "height")?);
    if pw.is_some() || ph.is_some() {
        if width.is_some_and(|t| !t.trim().ends_with('%')) || height.is_some_and(|t| !t.trim().ends_with('%')) {
            bail!("--image-width and --image-height must both be percentages or both be lengths");
        }
        return Ok(sheaf_plot::ExportSize::from_unit(pw, ph, sheaf_plot::SizeUnit::PageRatio, dpi));
    }
    let inches = |text: Option<&str>, what: &str| -> Result<Option<f64>> {
        text.map(|t| parse_length_inches(t, dpi).with_context(|| format!("--image-{what} '{t}'"))).transpose()
    };
    Ok(sheaf_plot::ExportSize { width_in: inches(width, "width")?, height_in: inches(height, "height")?, ratio: None })
}

/// The margin mode and border width from `--margin` and `--border-width`.
pub fn export_layout(m: &MarginSettings) -> Result<sheaf_plot::ExportLayout> {
    let margin = sheaf_plot::Margin::parse(&m.margin).ok_or_else(|| {
        anyhow::anyhow!("--margin '{}' is not a mode; use auto, border, tight, page or tight-in-page", m.margin)
    })?;
    if let Some(w) = m.border_width
        && !(w.is_finite() && w >= 0.0)
    {
        bail!("--border-width must be a number of pixels, 0 or more (got {w})");
    }
    Ok(sheaf_plot::ExportLayout { margin, border_px: m.border_width })
}

/// A length with its unit (`in`, `cm`, `mm`, `pt` or `px`, the last at `dpi`) in inches.
pub fn parse_length_inches(text: &str, dpi: f64) -> Result<f64> {
    let t = text.trim().to_ascii_lowercase();
    let split = t.find(|c: char| c.is_ascii_alphabetic()).unwrap_or(t.len());
    let (number, unit) = t.split_at(split);
    let value: f64 =
        number.trim().parse().map_err(|_| anyhow::anyhow!("'{text}' is not a length such as 3.5in or 8.5cm"))?;
    let per_inch = match unit.trim() {
        "in" | "inch" | "inches" => 1.0,
        "cm" => 2.54,
        "mm" => 25.4,
        "pt" => 72.0,
        "px" if dpi > 0.0 => dpi,
        "" => bail!("'{text}' needs a unit: in, cm, mm, pt or px"),
        other => bail!("'{other}' is not a unit; use in, cm, mm, pt or px"),
    };
    if !(value.is_finite() && value > 0.0) {
        bail!("a length must be positive, not '{text}'");
    }
    Ok(value / per_inch)
}

#[cfg(test)]
mod length_tests {
    use super::parse_length_inches;

    #[test]
    fn lengths_need_a_unit_and_convert_to_inches() {
        let close = |a: f64, b: f64| (a - b).abs() < 1e-12;
        assert!(close(parse_length_inches("3.5in", 300.0).unwrap(), 3.5));
        assert!(close(parse_length_inches("8.5 cm", 300.0).unwrap(), 8.5 / 2.54));
        assert!(close(parse_length_inches("85mm", 300.0).unwrap(), 85.0 / 25.4));
        assert!(close(parse_length_inches("252pt", 300.0).unwrap(), 3.5));
        assert!(close(parse_length_inches("1050PX", 300.0).unwrap(), 3.5));
        for bad in ["3.5", "3.5furlong", "-2in", "0cm", "in", "abc"] {
            assert!(parse_length_inches(bad, 300.0).is_err(), "{bad}");
        }
    }
}
