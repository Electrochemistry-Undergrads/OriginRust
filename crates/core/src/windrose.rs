//! Plot > Specialized > Wind Rose - Binned Data and Wind Rose - Raw Data (T113; Raw Data measured against
//! Origin 2016's `plot_windrose` by T324, `docs/spec/graphing.md`, "Wind rose").
//!
//! - A wind rose is a polar layer with north (0 degrees) up and the angles running clockwise. Each
//!   direction sector holds a pile of wedges, one per speed band, from the centre outwards.
//! - **Binned Data** plots an existing frequency table: the X column holds the sector directions (degrees,
//!   or text such as N, NNE, ... spread evenly round the circle), and each selected Y column is one speed
//!   band, piled in selection order.
//! - **Raw Data** bins the rows itself: the X column holds the directions in degrees and the selected Y
//!   column the speeds. As Origin's (measured): 16 sectors centred on 0, 22.5, ... degrees; equal speed bands
//!   from 0 whose width is the 1-2-5 step nearest a fifth of the top speed, up to the first multiple above it,
//!   then an open band ">= that"; a band's wedge is the count of the rows in the sector and the band.
//! - One plot per band, so the legend lists the bands. Bands are coloured black, yellow, red, blue, green,
//!   cyan (Origin's), outlined in black.

use crate::Color;
use crate::graph::{Coords, PlotKind};
use crate::ids::{ColumnId, WindowId};
use crate::project::{Project, ProjectError};
use crate::template::Template;
use serde::{Deserialize, Serialize};

/// One speed band of a wind rose plot (in [`crate::graph::PlotDecor::wind_rose`]).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WindRoseSpec {
    /// Raw Data: the number of direction sectors.
    pub sectors: usize,
    /// Raw Data: the band's speed range `[lo, hi)` (`hi` included for the top band); `None` for Binned
    /// Data, whose Y column holds the band's frequencies.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed: Option<(f64, f64, bool)>,
}

/// The number of sectors of Raw Data.
pub const RAW_SECTORS: usize = 16;

/// The colours of the speed bands, from the slowest (Origin 2016's, measured).
pub const BAND_COLORS: [Color; 6] = [
    Color::rgb(0, 0, 0),
    Color::rgb(255, 255, 0),
    Color::rgb(255, 0, 0),
    Color::rgb(0, 0, 255),
    Color::rgb(0, 128, 0),
    Color::rgb(0, 255, 255),
];

/// The upper speed of the open top band (a finite number, so that it saves as JSON).
pub const OPEN_TOP: f64 = f64::MAX;

/// Origin's speed step (measured): the 1, 2 or 5 times a power of ten nearest a fifth of the top speed
/// (8.5 gives 2, 25.5 gives 5, 0.85 gives 0.2).
pub fn band_step(top: f64) -> f64 {
    if !(top > 0.0 && top.is_finite()) {
        return 1.0;
    }
    let raw = top / 5.0;
    let decade = 10f64.powf(raw.log10().floor());
    [1.0, 2.0, 5.0, 10.0]
        .iter()
        .map(|m| m * decade)
        .min_by(|a, b| (a / raw).ln().abs().total_cmp(&(b / raw).ln().abs()))
        .unwrap_or(decade)
}

/// Origin's speed bands for a top speed (measured): `[k s, (k + 1) s)` up to the first multiple of the
/// step above the top speed, then the open band from there.
pub fn raw_bands(top: f64) -> Vec<(f64, f64)> {
    let step = band_step(top);
    let count = (((top / step) + 1e-9).floor() as usize + 1).max(1);
    let mut bands: Vec<(f64, f64)> = (0..count).map(|b| (b as f64 * step, (b + 1) as f64 * step)).collect();
    bands.push((count as f64 * step, OPEN_TOP));
    bands
}

/// Writes a band edge without trailing zeros.
fn edge(v: f64) -> String {
    let s = format!("{:.6}", v);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// The sector a direction in degrees falls in, of `n` sectors centred on 0, 360/n, ...
pub fn sector_of(direction: f64, n: usize) -> Option<usize> {
    if !direction.is_finite() || n == 0 {
        return None;
    }
    let w = 360.0 / n as f64;
    Some(((direction.rem_euclid(360.0) / w).round() as usize) % n)
}

impl Project {
    /// Plots a wind rose from the selected Y columns `ys` and the X column of the first: Raw Data takes
    /// one speed column, Binned Data one frequency column per speed band.
    pub fn plot_wind_rose(&mut self, template: Template, ys: &[ColumnId]) -> Result<WindowId, ProjectError> {
        let raw = template == Template::WindRoseRaw;
        let first = *ys
            .first()
            .ok_or(ProjectError::WindRoseColumns("a wind rose needs a Y column with an X column of directions"))?;
        if raw && ys.len() != 1 {
            return Err(ProjectError::WindRoseColumns(
                "Wind Rose - Raw Data takes one speed column with its X column of directions",
            ));
        }
        let (book, sheet, _) = self.locate(first).ok_or(ProjectError::NoSuchColumn(first))?;
        let x = sheet.column_index(first).and_then(|i| sheet.x_for(i)).map(|c| c.id);
        let Some(x) = x else {
            return Err(ProjectError::WindRoseColumns("a wind rose needs an X column of directions"));
        };
        let name = book.title_for_graph();
        // Each band: its column, spec and legend text.
        let bands: Vec<(ColumnId, WindRoseSpec, Option<String>)> = if raw {
            let top =
                self.numeric(first).unwrap_or_default().iter().copied().filter(|v| v.is_finite()).fold(0.0, f64::max);
            raw_bands(top)
                .into_iter()
                .map(|(lo, hi)| {
                    let spec = WindRoseSpec { sectors: RAW_SECTORS, speed: Some((lo, hi, false)) };
                    let label = if hi == OPEN_TOP {
                        format!(">= {}", edge(lo))
                    } else {
                        format!("{} - {}", edge(lo), edge(hi))
                    };
                    (first, spec, Some(label))
                })
                .collect()
        } else {
            ys.iter().map(|&y| (y, WindRoseSpec { sectors: 0, speed: None }, None)).collect()
        };
        let id = self.add_graph(template);
        for (k, (column, spec, label)) in bands.into_iter().enumerate() {
            self.add_plot_xy(id, 0, Some(x), column, None)?;
            let plot = self.graph_mut(id).and_then(|g| g.layers[0].plots.last_mut()).expect("a plot was just added");
            plot.kind = PlotKind::WindRose;
            plot.auto_color = false;
            plot.symbol.color = BAND_COLORS[k % BAND_COLORS.len()];
            plot.line.color = Color::BLACK;
            plot.line.width_pt = 0.5;
            plot.label = label;
            plot.decor.wind_rose = Some(spec);
        }
        if let Some(g) = self.graph_mut(id) {
            g.long_name = name;
            debug_assert!(matches!(g.layers[0].coords, Coords::Polar(_)));
        }
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::{ColumnData, Designation};

    #[test]
    fn sectors_are_centred_on_north_and_bands_take_a_1_2_5_step() {
        assert_eq!(sector_of(0.0, 16), Some(0));
        assert_eq!(sector_of(11.0, 16), Some(0));
        assert_eq!(sector_of(12.0, 16), Some(1));
        assert_eq!(sector_of(355.0, 16), Some(0));
        assert_eq!(sector_of(-90.0, 16), Some(12));
        // Origin's steps and bands (measured).
        assert_eq!(band_step(8.5), 2.0);
        assert_eq!(band_step(25.5), 5.0);
        assert!((band_step(0.85) - 0.2).abs() < 1e-12);
        assert_eq!(raw_bands(8.5), vec![(0.0, 2.0), (2.0, 4.0), (4.0, 6.0), (6.0, 8.0), (8.0, 10.0), (10.0, OPEN_TOP)]);
        assert_eq!(raw_bands(25.5).len(), 7, "0-5 ... 25-30 and >= 30");
    }

    #[test]
    fn raw_data_makes_six_bands_and_binned_data_one_per_column() {
        let mut p = Project::new();
        let b = p.add_workbook("");
        let x = p.add_column(b, 0, Designation::X, ColumnData::numeric(vec![0.0, 90.0, 180.0, 270.0])).unwrap();
        let s = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0, 5.0, 9.0, 11.5])).unwrap();
        let f = p.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![2.0, 3.0, 4.0, 5.0])).unwrap();
        assert!(p.plot_wind_rose(Template::WindRoseRaw, &[s, f]).is_err());
        let g = p.plot_columns(Template::WindRoseRaw, &[s]).unwrap();
        let graph = p.graph(g).unwrap();
        let plots = &graph.layers[0].plots;
        assert_eq!(plots.len(), 7, "top 11.5: six bands of 2 and >= 12");
        assert!(plots.iter().all(|pl| pl.kind == PlotKind::WindRose && pl.x == Some(x) && pl.y == s));
        assert_eq!(plots[0].label.as_deref(), Some("0 - 2"));
        assert_eq!(plots[6].label.as_deref(), Some(">= 12"));
        assert_eq!(plots[0].symbol.color, Color::BLACK, "Origin's first band colour");
        let polar = graph.layers[0].coords.polar().unwrap();
        assert!(polar.clockwise && polar.zero_deg == 90.0, "north up, clockwise");
        let g = p.plot_columns(Template::WindRoseBinned, &[s, f]).unwrap();
        let plots = &p.graph(g).unwrap().layers[0].plots;
        assert_eq!(plots.len(), 2);
        assert_eq!((plots[1].y, plots[1].symbol.color), (f, BAND_COLORS[1]));
    }
}
