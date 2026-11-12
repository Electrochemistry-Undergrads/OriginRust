//! Turns a Sheaf graph plus the project's column data into figures.
//!
//! The graph is laid out once into a scene (lines, polygons and positioned
//! text in points, see the `scene` module) and the scene is written as SVG.
//! PNG is that SVG rasterised, PDF is that SVG converted, and the screen
//! shows the same raster, so a figure looks the same everywhere. The rules
//! are in `docs/spec/figure-rendering.md`.
//!
//! The functions to start with are [`render_svg`], [`render_png`],
//! [`render_pdf`], [`export_file`] and, for an interactive view,
//! [`render_raster`] together with [`layout_geometry`].

mod data;
mod fonts;
mod layout;
pub mod markup;
mod plots;
mod raster;
pub mod scene;
mod stack;
mod svg;
mod text;
pub mod ticks;

#[cfg(test)]
mod tests;

pub use fonts::{FALLBACK_FAMILIES, FixedMetrics, SystemFonts, TextMeasurer};

use sheaf_core::{Project, WindowId};
use std::path::{Path, PathBuf};

/// Why a figure could not be produced.
#[derive(Debug, thiserror::Error)]
pub enum PlotError {
    /// The project has no graph with this id.
    #[error("there is no graph with id {0:?} in the project")]
    NoSuchGraph(WindowId),
    /// The graph has no layer with this index.
    #[error("the graph has no layer {0}")]
    NoSuchLayer(usize),
    /// The scene could not be written as SVG.
    #[error("could not build the SVG: {0}")]
    Svg(String),
    /// Rasterising or encoding the PNG failed.
    #[error("could not make the image: {0}")]
    Png(String),
    /// Converting to PDF failed.
    #[error("could not make the PDF: {0}")]
    Pdf(String),
    /// Reading or writing a file failed.
    #[error("cannot write '{path}': {source}")]
    Io {
        /// The file that was being written.
        path: PathBuf,
        /// The operating system's error.
        #[source]
        source: std::io::Error,
    },
    /// The file name has an extension that is not one of svg, png, pdf.
    #[error("cannot export to '.{0}' files; use .svg, .png or .pdf")]
    UnknownFormat(String),
}

/// A rectangle in page points; `(x, y)` is the top-left corner, y grows downwards.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// Where one layer's frame is on the page and how data maps into it.
/// The GUI uses this for read-outs, zooming and hit testing.
#[derive(Clone, Debug, PartialEq)]
pub struct LayerGeometry {
    /// The rectangle the axes are drawn on and data is mapped into, in page points.
    pub frame: Rect,
    /// The rectangle data is clipped to. It is `frame`, except in a waterfall, where `frame` is
    /// the smaller front frame and later curves are drawn moved into the rest of this rectangle.
    pub clip: Rect,
    /// The resolved X axis.
    pub x: ticks::ResolvedAxis,
    /// The resolved Y axis.
    pub y: ticks::ResolvedAxis,
}

impl LayerGeometry {
    /// Page position (points) of a data point. Values that cannot be placed
    /// (non-positive on a log axis) give NaN.
    pub fn data_to_page(&self, x: f64, y: f64) -> (f64, f64) {
        let f = self.frame;
        (f.x + self.x.to_unit(x) * f.w, f.y + (1.0 - self.y.to_unit(y)) * f.h)
    }

    /// Data value under a page position (points). Inverse of [`LayerGeometry::data_to_page`].
    pub fn page_to_data(&self, px: f64, py: f64) -> (f64, f64) {
        let f = self.frame;
        (self.x.from_unit((px - f.x) / f.w), self.y.from_unit(1.0 - (py - f.y) / f.h))
    }

    /// Like [`LayerGeometry::data_to_page`], but `None` when the point is not
    /// finite or cannot be placed. Used to break lines at gaps.
    pub(crate) fn checked_data_to_page(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        let p = self.data_to_page(x, y);
        (x.is_finite() && y.is_finite() && p.0.is_finite() && p.1.is_finite()).then_some(p)
    }
}

/// An image as straight (non-premultiplied) RGBA8, rows from the top.
#[derive(Clone, Debug, PartialEq)]
pub struct Raster {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// `width * height * 4` bytes: red, green, blue, alpha per pixel.
    pub rgba: Vec<u8>,
}

/// Frame rectangles and resolved axes of every layer of a graph, in layer order. A layer linked
/// to another one (the right Y axis of a double-Y graph) has its parent's frame and, when its X
/// scale is linked, its parent's X axis.
pub fn layout_geometry(project: &Project, graph: WindowId) -> Result<Vec<LayerGeometry>, PlotError> {
    let g = project.graph(graph).ok_or(PlotError::NoSuchGraph(graph))?;
    let pds: Vec<data::LayerData> = g.layers.iter().map(|layer| data::collect(project, layer)).collect();
    Ok(layout::graph_geometry(project, g, &pds, SystemFonts::shared()))
}

/// The offsets `(x, y)`, in axis units, at which each plot of a layer is drawn, one pair per plot
/// in plot order (graphing spec section 13). They are `(0, 0)` unless the layer stacks its curves;
/// in automatic mode they are computed from the current data. The desktop app copies them into
/// the layer when the user switches to individual offsets, so that the picture does not jump.
pub fn stack_offsets(project: &Project, graph: WindowId, layer: usize) -> Result<Vec<(f64, f64)>, PlotError> {
    let g = project.graph(graph).ok_or(PlotError::NoSuchGraph(graph))?;
    let layer = g.layers.get(layer).ok_or(PlotError::NoSuchLayer(layer))?;
    Ok(stack::places(project, layer).0.into_iter().map(|p| (p.ox, p.oy)).collect())
}

/// Renders a graph as SVG text, measuring text with the installed fonts.
pub fn render_svg(project: &Project, graph: WindowId) -> Result<String, PlotError> {
    render_svg_with(project, graph, SystemFonts::shared())
}

/// Renders a graph as SVG text with a chosen text measurer. With
/// [`FixedMetrics`] the output does not depend on installed fonts.
pub fn render_svg_with(project: &Project, graph: WindowId, measurer: &dyn TextMeasurer) -> Result<String, PlotError> {
    let scene = layout::build_scene(project, graph, measurer)?;
    Ok(svg::write_svg(&scene))
}

/// Renders a graph to pixels at `pixels_per_point` (1 = 72 dpi), for showing on screen.
pub fn render_raster(project: &Project, graph: WindowId, pixels_per_point: f64) -> Result<Raster, PlotError> {
    let svg = render_svg(project, graph)?;
    raster::rasterise(&svg, pixels_per_point, SystemFonts::shared())
}

/// Renders a graph as an encoded PNG at `dpi` dots per inch; the resolution
/// is recorded in the file.
pub fn render_png(project: &Project, graph: WindowId, dpi: f64) -> Result<Vec<u8>, PlotError> {
    let svg = render_svg(project, graph)?;
    let r = raster::rasterise(&svg, dpi / 72.0, SystemFonts::shared())?;
    raster::encode_png(&r, dpi)
}

/// Renders a graph as a one-page PDF (vector, text kept as text).
pub fn render_pdf(project: &Project, graph: WindowId) -> Result<Vec<u8>, PlotError> {
    let svg = render_svg(project, graph)?;
    raster::to_pdf(&svg, SystemFonts::shared())
}

/// Writes a graph to `path`; the format follows the extension (`.svg`,
/// `.png` or `.pdf`, any case). `dpi` applies to PNG only.
pub fn export_file(project: &Project, graph: WindowId, path: &Path, dpi: f64) -> Result<(), PlotError> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    let bytes = match ext.as_str() {
        "svg" => render_svg(project, graph)?.into_bytes(),
        "png" => render_png(project, graph, dpi)?,
        "pdf" => render_pdf(project, graph)?,
        _ => return Err(PlotError::UnknownFormat(ext)),
    };
    std::fs::write(path, bytes).map_err(|source| PlotError::Io { path: path.to_path_buf(), source })
}
