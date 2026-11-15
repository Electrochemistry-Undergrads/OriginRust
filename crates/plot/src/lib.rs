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
mod emf;
mod eps;
mod fill;
mod fonts;
pub mod hit;
mod layout;
mod legend;
mod margin;
pub mod markup;
mod multipage;
mod pie;
mod plots;
mod polar;
mod psd;
mod raster;
pub mod scene;
mod shapes;
mod sheet;
mod stack;
mod svg;
mod symbol_gaps;
mod ternary;
mod text;
pub mod ticks;
mod ticks_scales;
mod vector;
mod violin;

#[cfg(test)]
mod detail_tests;
#[cfg(test)]
mod export_tests;
#[cfg(test)]
mod fill_inclusive_tests;
#[cfg(test)]
mod fill_sideline_tests;
#[cfg(test)]
mod label_frame_tests;
#[cfg(test)]
mod label_whiteout_tests;
#[cfg(test)]
mod legend_tests;
#[cfg(test)]
mod lollipop_tests;
#[cfg(test)]
mod margin_tests;
#[cfg(test)]
mod multipage_tests;
#[cfg(test)]
mod offset_multiplier_tests;
#[cfg(test)]
mod skip_tests;
#[cfg(test)]
mod symbol_gap_tests;
#[cfg(test)]
mod symbol_line_order_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod violin_tests;

pub use eps::EpsOutput;
pub use fonts::{FALLBACK_FAMILIES, FixedMetrics, SystemFonts, TextMeasurer};
pub use margin::{DEFAULT_BORDER_PT, ExportLayout, Margin, drawn_bounds};
pub use raster::{ColorDepth, RasterOptions, TiffCompression};
pub use sheet::{SheetPages, render_sheet_emf_pages, render_sheet_pdf, render_sheet_raster, sheet_pages};

use sheaf_core::graph::ScaleKind;
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
    /// Encoding the TIFF failed.
    #[error("could not make the TIFF: {0}")]
    Tiff(String),
    /// Encoding a JPEG, BMP or GIF failed.
    #[error("could not make the image: {0}")]
    Image(String),
    /// The export size is not usable (not positive, or far from the page size).
    #[error("{0}")]
    Size(String),
    /// Reading or writing a file failed.
    #[error("cannot write '{path}': {source}")]
    Io {
        /// The file that was being written.
        path: PathBuf,
        /// The operating system's error.
        #[source]
        source: std::io::Error,
    },
    /// The file name has an unsupported figure extension.
    #[error(
        "cannot export to '.{0}' files; use .svg, .png, .tif, .jpg, .bmp, .gif, .tga, .pcx, .psd, .pdf, .eps or .emf"
    )]
    UnknownFormat(String),
}

/// The unit of an export size in Origin's export settings (spec graphing.md 9.3).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum SizeUnit {
    #[default]
    Inch,
    Cm,
    /// Pixels of the raster export: inches x DPI.
    Pixel,
    /// Percent of the page size (100 is the size the page has).
    PageRatio,
}

impl SizeUnit {
    /// A length of `v` in this unit as inches. `dpi` is the pixel resolution; `area_in` is the
    /// width (or height) of the exported area in inches that 100 % stands for.
    pub fn to_inches(self, v: f64, dpi: f64, area_in: f64) -> f64 {
        match self {
            SizeUnit::Inch => v,
            SizeUnit::Cm => v / 2.54,
            SizeUnit::Pixel => v / dpi,
            SizeUnit::PageRatio => v / 100.0 * area_in,
        }
    }

    /// The inverse of [`SizeUnit::to_inches`].
    pub fn from_inches(self, inches: f64, dpi: f64, area_in: f64) -> f64 {
        match self {
            SizeUnit::Inch => inches,
            SizeUnit::Cm => inches * 2.54,
            SizeUnit::Pixel => inches * dpi,
            SizeUnit::PageRatio => inches / area_in * 100.0,
        }
    }

    /// Parses `in`, `cm`, `px` or `%` (also the words inch, pixel, percent, ratio).
    pub fn parse(s: &str) -> Option<SizeUnit> {
        match s.trim().to_ascii_lowercase().as_str() {
            "in" | "inch" | "inches" => Some(SizeUnit::Inch),
            "cm" => Some(SizeUnit::Cm),
            "px" | "pixel" | "pixels" => Some(SizeUnit::Pixel),
            "%" | "percent" | "ratio" | "page-ratio" => Some(SizeUnit::PageRatio),
            _ => None,
        }
    }
}

/// The size of an exported figure, that of the exported area (the page, or less with a margin
/// mode other than page). With neither width nor height the figure has the size of that area.
/// With one of them the whole figure is scaled to it, text and lines included, as Origin's export
/// size does with "keep aspect ratio"; with both, it is scaled to fit inside both.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct ExportSize {
    /// Width in inches.
    pub width_in: Option<f64>,
    /// Height in inches.
    pub height_in: Option<f64>,
    /// Scale factor against the area at its own size (1.0 is 100 %); wins over width and height.
    pub ratio: Option<f64>,
}

impl ExportSize {
    /// A size given in `unit`. `dpi` turns pixels into inches. A page ratio is a percentage and
    /// rescales by width or by height alike, the aspect ratio being kept; width wins.
    pub fn from_unit(width: Option<f64>, height: Option<f64>, unit: SizeUnit, dpi: f64) -> ExportSize {
        let conv = |v: f64| match unit {
            SizeUnit::Inch => v,
            SizeUnit::Cm => v / 2.54,
            SizeUnit::Pixel => v / dpi,
            SizeUnit::PageRatio => v,
        };
        if unit == SizeUnit::PageRatio {
            return ExportSize { ratio: width.or(height).map(|p| p / 100.0), ..ExportSize::default() };
        }
        ExportSize { width_in: width.map(conv), height_in: height.map(conv), ratio: None }
    }

    /// The factor by which an area of `page_w` x `page_h` points is scaled.
    pub fn scale(&self, page_w: f64, page_h: f64) -> Result<f64, PlotError> {
        let mut scale: Option<f64> = self.ratio;
        if let Some(r) = self.ratio
            && !(r.is_finite() && r > 0.0)
        {
            return Err(PlotError::Size(format!("the page ratio must be a positive percentage, not {} %", r * 100.0)));
        }
        let (width, height) = if self.ratio.is_some() { (None, None) } else { (self.width_in, self.height_in) };
        for (want, have, what) in [(width, page_w, "width"), (height, page_h, "height")] {
            let Some(inches) = want else { continue };
            if !(inches.is_finite() && inches > 0.0) {
                return Err(PlotError::Size(format!("the export {what} must be a positive length, not {inches} in")));
            }
            let s = inches * 72.0 / have;
            scale = Some(scale.map_or(s, |t: f64| t.min(s)));
        }
        let s = scale.unwrap_or(1.0);
        if !(0.01..=100.0).contains(&s) {
            return Err(PlotError::Size(format!(
                "an export size of {:.3} x {:.3} in is too far from the page size ({:.2} x {:.2} in)",
                page_w * s / 72.0,
                page_h * s / 72.0,
                page_w / 72.0,
                page_h / 72.0
            )));
        }
        Ok(s)
    }
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
    /// The circle of a polar layer. Then `x` is the angle axis (degrees) and `y` the radius axis,
    /// and the data are placed on the circle (graphing spec section 14.18).
    pub polar: Option<polar::PolarMap>,
    /// The triangle of a ternary layer. Then the data are the normalised X and Y fractions of the
    /// rows (graphing spec section 14.20).
    pub ternary: Option<ternary::TernaryMap>,
}

impl LayerGeometry {
    /// The fraction of the circle's radius at which radius value `r` stands in a polar layer. With the
    /// centre at zero (Origin's default for polar graphs) it is `|r|` over the upper limit, so the lower
    /// limit leaves a hole round the centre; otherwise the lower limit is the centre.
    pub(crate) fn radius_unit(&self, r: f64) -> f64 {
        match &self.polar {
            Some(map) if map.zero_centred && self.y.scale == ScaleKind::Linear && self.y.max > 0.0 => {
                r.abs() / self.y.max
            }
            _ => self.y.to_unit(r),
        }
    }

    /// Page position (points) of a data point. Values that cannot be placed
    /// (non-positive on a log axis) give NaN.
    pub fn data_to_page(&self, x: f64, y: f64) -> (f64, f64) {
        if let Some(map) = &self.polar {
            // A negative radius of a polar graph is drawn on the opposite side of the centre.
            let (theta, r) = if map.zero_centred && y < 0.0 { (x + map.circle / 2.0, -y) } else { (x, y) };
            return map.point(theta, self.radius_unit(r));
        }
        if let Some(map) = &self.ternary {
            return map.point(x, y);
        }
        let f = self.frame;
        (f.x + self.x.to_unit(x) * f.w, f.y + (1.0 - self.y.to_unit(y)) * f.h)
    }

    /// Data value under a page position (points). Inverse of [`LayerGeometry::data_to_page`].
    pub fn page_to_data(&self, px: f64, py: f64) -> (f64, f64) {
        if let Some(map) = &self.polar {
            let (theta, u) = map.angle_and_unit(px, py);
            let r = if map.zero_centred && self.y.scale == ScaleKind::Linear {
                u * self.y.max
            } else {
                self.y.from_unit(u)
            };
            return (theta, r);
        }
        if let Some(map) = &self.ternary {
            return map.fractions(px, py);
        }
        let f = self.frame;
        (self.x.from_unit((px - f.x) / f.w), self.y.from_unit(1.0 - (py - f.y) / f.h))
    }

    /// Like [`LayerGeometry::data_to_page`], but `None` when the point is not
    /// finite or cannot be placed. Used to break lines at gaps.
    pub(crate) fn checked_data_to_page(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        let p = self.data_to_page(x, y);
        if let Some(map) = &self.polar {
            // A radius outside the axis range is not drawn (it would leave the circle or cross the
            // centre; with the centre at zero the hole below the lower limit stays empty too).
            let inside = if map.zero_centred && self.y.scale == ScaleKind::Linear {
                (self.y.min - 1e-9..=self.y.max + 1e-9).contains(&y.abs())
            } else {
                (-1e-9..=1.0 + 1e-9).contains(&self.y.to_unit(y))
            };
            if !inside {
                return None;
            }
        }
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
/// to another one has its frame placed relative to its parent's (the same frame for the right Y
/// axis of a double-Y graph, its own place for a panel of a stack) and, when its X scale is
/// linked, its parent's X axis.
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
    Ok(stack_places(project, graph, layer)?.into_iter().map(|p| (p.ox, p.oy)).collect())
}

/// Display multipliers, offsets and stack membership, in plot order. Reader tools and overlays
/// use these placements to follow the drawing while retaining the original worksheet values.
pub fn stack_places(
    project: &Project,
    graph: WindowId,
    layer: usize,
) -> Result<Vec<sheaf_core::stack::Place>, PlotError> {
    let g = project.graph(graph).ok_or(PlotError::NoSuchGraph(graph))?;
    let layer = g.layers.get(layer).ok_or(PlotError::NoSuchLayer(layer))?;
    Ok(stack::places(project, layer).0)
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

/// The text of the legend of layer `layer`: its own text when it has one, otherwise the automatic
/// text with one `\l(n) %(n)` line per plot the legend lists (see [`sheaf_core::graph::Legend::text`]).
pub fn legend_text(project: &Project, graph: WindowId, layer: usize) -> Result<String, PlotError> {
    let g = project.graph(graph).ok_or(PlotError::NoSuchGraph(graph))?;
    let l = g.layers.get(layer).ok_or(PlotError::NoSuchLayer(layer))?;
    Ok(l.legend.text.clone().unwrap_or(legend_automatic_text(project, graph, layer)?))
}

/// The automatic text of the legend of layer `layer`, which "Refresh legend" goes back to.
pub fn legend_automatic_text(project: &Project, graph: WindowId, layer: usize) -> Result<String, PlotError> {
    let g = project.graph(graph).ok_or(PlotError::NoSuchGraph(graph))?;
    if layer >= g.layers.len() {
        return Err(PlotError::NoSuchLayer(layer));
    }
    let counts: Vec<usize> = g.layers.iter().map(|l| data::collect(project, l).plots.len()).collect();
    Ok(sheaf_core::graph::Legend::automatic_text(&legend::ids(g, layer, &|j| counts[j])))
}

/// The scene of a graph, with the pickable objects that [`hit::hit_test`] looks at, measured
/// with the installed fonts. Draw it with [`render_scene_raster`].
pub fn graph_scene(project: &Project, graph: WindowId) -> Result<scene::Scene, PlotError> {
    layout::build_scene(project, graph, SystemFonts::shared())
}

/// Renders a scene from [`graph_scene`] to pixels at `pixels_per_point` (1 = 72 dpi). The
/// picture is the same as [`render_raster`] of the graph the scene was made from.
pub fn render_scene_raster(scene: &scene::Scene, pixels_per_point: f64) -> Result<Raster, PlotError> {
    raster::rasterise(&svg::write_svg(scene), pixels_per_point, SystemFonts::shared())
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

/// Renders the graph on the project's paper using the saved Page Setup, as a vector PDF.
/// The paper is treated as fully printable, as in Print Preview.
pub fn render_print_pdf(project: &Project, graph: WindowId) -> Result<Vec<u8>, PlotError> {
    let svg = print_svg(project, graph)?;
    raster::to_pdf(&svg, SystemFonts::shared())
}

/// Renders several graphs into one PDF, one page per graph in the order given (Origin's
/// Export Graphs `multipage`, graphing.md 9.7). Each page is what [`export_bytes_with`] writes
/// as a one-page PDF of that graph with the same size and margin settings: the page is the
/// exported area of the graph, vector, with text kept as text. A graph may appear twice.
pub fn render_multipage_pdf(
    project: &Project,
    graphs: &[WindowId],
    size: ExportSize,
    layout: ExportLayout,
) -> Result<Vec<u8>, PlotError> {
    multipage::svgs_to_pdf(graphs.iter().map(|&g| export_svg(project, g, size, layout)), SystemFonts::shared())
}

/// [`render_multipage_pdf`] written to `path`.
pub fn export_multipage_pdf_file(
    project: &Project,
    graphs: &[WindowId],
    path: &Path,
    size: ExportSize,
    layout: ExportLayout,
) -> Result<(), PlotError> {
    let bytes = render_multipage_pdf(project, graphs, size, layout)?;
    std::fs::write(path, bytes).map_err(|source| PlotError::Io { path: path.to_path_buf(), source })
}

/// Several graphs on the project's paper, one sheet per graph, each placed by the saved Page
/// Setup as [`render_print_pdf`] places one graph.
pub fn render_print_multipage_pdf(project: &Project, graphs: &[WindowId]) -> Result<Vec<u8>, PlotError> {
    multipage::svgs_to_pdf(graphs.iter().map(|&g| print_svg(project, g)), SystemFonts::shared())
}

fn print_svg(project: &Project, graph: WindowId) -> Result<String, PlotError> {
    let g = project.graph(graph).ok_or(PlotError::NoSuchGraph(graph))?;
    let (w, h) = (g.page.width_pt, g.page.height_pt);
    let setup = project.page_setup;
    let margins = setup.margins;
    if ![w, h].iter().all(|x| x.is_finite() && *x > 0.0)
        || ![margins.left, margins.right, margins.top, margins.bottom].iter().all(|x| x.is_finite() && *x >= 0.0)
    {
        return Err(PlotError::Size("invalid print page size or margins".into()));
    }
    let place = setup.place(w, h).ok_or_else(|| PlotError::Size("print margins leave no space on the paper".into()))?;
    let (paper_w, paper_h) = setup.paper_pt(w, h);
    if ![paper_w, paper_h, place.x, place.y, place.w, place.h].iter().all(|x| x.is_finite())
        || place.w <= 0.0
        || place.h <= 0.0
    {
        return Err(PlotError::Size("invalid print placement".into()));
    }
    let svg = render_svg(project, graph)?;
    // write_svg always emits one root element; its contents use graph-page point coordinates.
    let start = svg.find('>').ok_or_else(|| PlotError::Svg("missing SVG root".into()))? + 1;
    let end = svg.rfind("</svg>").ok_or_else(|| PlotError::Svg("missing SVG closing tag".into()))?;
    let scale = place.w / w;
    if !scale.is_finite() || scale <= 0.0 {
        return Err(PlotError::Size("invalid print scale".into()));
    }
    let paper_svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{paper_w}pt" height="{paper_h}pt" viewBox="0 0 {paper_w} {paper_h}"><defs><clipPath id="print-page-clip"><rect x="0" y="0" width="{w}" height="{h}"/></clipPath></defs><g transform="translate({} {}) scale({scale})"><g clip-path="url(#print-page-clip)">{}</g></g></svg>"#,
        place.x,
        place.y,
        &svg[start..end]
    );
    Ok(paper_svg)
}

/// Renders a graph as Encapsulated PostScript (level 2): vector paths, text in the standard
/// Helvetica, Times or Courier fonts with ISO Latin-1 encoding, clipping per layer. PostScript
/// has no transparency, so translucent colours are blended onto the page background.
pub fn render_eps(project: &Project, graph: WindowId) -> Result<Vec<u8>, PlotError> {
    Ok(render_eps_with_warnings(project, graph)?.bytes)
}

/// [`render_eps`] together with the warnings about characters that had to be replaced.
pub fn render_eps_with_warnings(project: &Project, graph: WindowId) -> Result<EpsOutput, PlotError> {
    let scene = layout::build_scene(project, graph, SystemFonts::shared())?;
    Ok(eps::write_eps(&scene, 1.0))
}

/// Renders a graph as a Windows Enhanced Metafile (vector; text stays text in the graph's
/// font). The format has no transparency, so translucent colours are blended onto the page
/// background.
pub fn render_emf(project: &Project, graph: WindowId) -> Result<Vec<u8>, PlotError> {
    let fonts = SystemFonts::shared();
    let scene = layout::build_scene(project, graph, fonts)?;
    Ok(emf::write_emf(&scene, 1.0, fonts))
}

/// Renders a graph as an LZW-compressed RGB TIFF at `dpi` dots per inch; the resolution is
/// recorded in the file.
pub fn render_tiff(project: &Project, graph: WindowId, dpi: f64) -> Result<Vec<u8>, PlotError> {
    export_bytes(project, graph, "tif", dpi, ExportSize::default())
}

/// Writes a graph to `path`; the format follows the extension (`.svg`,
/// `.png`, `.tif`/`.tiff`, `.pdf`, `.eps` or `.emf`, any case). `dpi` applies to PNG and TIFF only.
pub fn export_file(project: &Project, graph: WindowId, path: &Path, dpi: f64) -> Result<(), PlotError> {
    export_file_sized(project, graph, path, dpi, ExportSize::default())
}

/// [`export_file`] with an export size other than the page size (see [`ExportSize`]).
pub fn export_file_sized(
    project: &Project,
    graph: WindowId,
    path: &Path,
    dpi: f64,
    size: ExportSize,
) -> Result<(), PlotError> {
    export_file_with(project, graph, path, dpi, size, ExportLayout::default())
}

/// [`export_file_sized`] with a margin mode (see [`ExportLayout`]).
pub fn export_file_with(
    project: &Project,
    graph: WindowId,
    path: &Path,
    dpi: f64,
    size: ExportSize,
    layout: ExportLayout,
) -> Result<(), PlotError> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    let bytes = export_bytes_with(project, graph, &ext, dpi, size, layout)?;
    std::fs::write(path, bytes).map_err(|source| PlotError::Io { path: path.to_path_buf(), source })
}

/// The bytes of an exported figure in the format named by `ext` (`svg`, `png`, `tif`, `tiff`,
/// `pdf`, `eps`, `emf`).
pub fn export_bytes(
    project: &Project,
    graph: WindowId,
    ext: &str,
    dpi: f64,
    size: ExportSize,
) -> Result<Vec<u8>, PlotError> {
    export_bytes_with(project, graph, ext, dpi, size, ExportLayout::default())
}

/// The picture an export to a raster format would have, as straight RGBA pixels (what the
/// clipboard copy of a page needs).
pub fn export_raster(
    project: &Project,
    graph: WindowId,
    dpi: f64,
    size: ExportSize,
    layout: ExportLayout,
) -> Result<Raster, PlotError> {
    let fonts = SystemFonts::shared();
    let scene = layout::build_scene(project, graph, fonts)?;
    let view = layout.view(&scene, fonts, Some(dpi));
    let scale = size.scale(view.w, view.h)?;
    raster::rasterise(&svg::write_svg_view(&scene, view, scale), dpi / 72.0, fonts)
}

/// The area of the page an export with `layout` shows, in page points (the page itself with
/// margin mode page). `raster_dpi` is the resolution of a raster export; it gives the border
/// width its unit.
pub fn export_area(
    project: &Project,
    graph: WindowId,
    layout: ExportLayout,
    raster_dpi: Option<f64>,
) -> Result<Rect, PlotError> {
    let scene = layout::build_scene(project, graph, SystemFonts::shared())?;
    Ok(layout.view(&scene, SystemFonts::shared(), raster_dpi))
}

/// [`export_bytes`] with a margin mode: the figure shows the area chosen by `layout` and `size`
/// is the size of that area.
pub fn export_bytes_with(
    project: &Project,
    graph: WindowId,
    ext: &str,
    dpi: f64,
    size: ExportSize,
    layout: ExportLayout,
) -> Result<Vec<u8>, PlotError> {
    export_bytes_opts(project, graph, ext, dpi, size, layout, &RasterOptions::default())
}

/// [`export_file_with`] with raster options (colour depth, TIFF compression, JPEG quality).
pub fn export_file_opts(
    project: &Project,
    graph: WindowId,
    path: &Path,
    dpi: f64,
    size: ExportSize,
    layout: ExportLayout,
    opts: &RasterOptions,
) -> Result<(), PlotError> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    let bytes = export_bytes_opts(project, graph, &ext, dpi, size, layout, opts)?;
    std::fs::write(path, bytes).map_err(|source| PlotError::Io { path: path.to_path_buf(), source })
}

/// [`export_bytes_with`] with raster options.
pub fn export_bytes_opts(
    project: &Project,
    graph: WindowId,
    ext: &str,
    dpi: f64,
    size: ExportSize,
    layout: ExportLayout,
    opts: &RasterOptions,
) -> Result<Vec<u8>, PlotError> {
    let fonts = SystemFonts::shared();
    let scene = layout::build_scene(project, graph, fonts)?;
    let raster_dpi = raster::is_raster_ext(ext).then_some(dpi);
    let view = layout.view(&scene, fonts, raster_dpi);
    let scale = size.scale(view.w, view.h)?;
    if matches!(ext, "eps" | "emf") {
        return Ok(if ext == "eps" {
            eps::write_eps_view(&scene, scale, view).bytes
        } else {
            emf::write_emf_view(&scene, scale, fonts, view)
        });
    }
    let svg = svg::write_svg_view(&scene, view, scale);
    match ext {
        "svg" => Ok(svg.into_bytes()),
        e if raster::is_raster_ext(e) => {
            let max_side = (e == "psd").then_some(psd::MAX_DIMENSION);
            let image = raster::rasterise_with_limit(&svg, dpi / 72.0, fonts, max_side)?;
            raster::encode_raster(&image, e, dpi, opts)
        }
        "pdf" => raster::to_pdf(&svg, fonts),
        _ => Err(PlotError::UnknownFormat(ext.to_string())),
    }
}

/// The SVG a vector export of `graph` draws (what `.svg` and `.pdf` export write).
fn export_svg(project: &Project, graph: WindowId, size: ExportSize, layout: ExportLayout) -> Result<String, PlotError> {
    let fonts = SystemFonts::shared();
    let scene = layout::build_scene(project, graph, fonts)?;
    let view = layout.view(&scene, fonts, None);
    let scale = size.scale(view.w, view.h)?;
    Ok(svg::write_svg_view(&scene, view, scale))
}
