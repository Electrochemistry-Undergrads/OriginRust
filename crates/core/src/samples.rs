//! File > Open Sample Projects and Help > Open Sample Projects > Sample Projects (T133): small projects
//! Sheaf builds on the spot, each a workbook of made-up data (from formulas and a fixed pseudo-random
//! sequence, so every opening is the same) and a graph that shows one kind of plot. Origin ships sample
//! `.opju` files; Sheaf has its own samples (`docs/spec/gui-look.md`, "Sample Projects").

use crate::column::{ColumnData, Designation};
use crate::ids::ColumnId;
use crate::project::{Project, ProjectError};
use crate::template::Template;

/// The samples: name and one-line description, in the order they are listed.
pub const SAMPLES: [(&str, &str); 6] = [
    ("Line and Symbol", "Two damped oscillations, line + symbol plot"),
    ("Peaks", "A noisy spectrum with three Gaussian peaks, line plot (try Analysis > Peaks and Baseline)"),
    ("Groups", "Three groups of measurements, box chart (try Statistics > Descriptive Statistics)"),
    ("Stock Prices", "Daily open, high, low, close and volume, OHLC-Volume chart"),
    ("Wind", "Wind directions and speeds, wind rose"),
    ("Process Control", "Thirty measurements of a process, X bar R chart in subgroups of 5"),
];

/// A fixed pseudo-random sequence in [0, 1) (a linear congruential generator), so a sample is the same
/// every time.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Roughly normal noise: the sum of four uniforms, centred, scaled to `sd`.
    fn noise(&mut self, sd: f64) -> f64 {
        let s: f64 = (0..4).map(|_| self.next()).sum();
        (s - 2.0) * sd * 3f64.sqrt()
    }
}

/// Adds a workbook named `long` with the columns `(short name, designation, values)`.
fn book(p: &mut Project, long: &str, cols: Vec<(&str, Designation, Vec<f64>)>) -> Result<Vec<ColumnId>, ProjectError> {
    let b = p.add_workbook(long);
    let mut ids = Vec::new();
    for (name, d, v) in cols {
        let id = p.add_column(b, 0, d, ColumnData::numeric(v))?;
        if let Some(c) = p.column_mut(id) {
            c.long_name = name.to_string();
        }
        ids.push(id);
    }
    Ok(ids)
}

/// The sample project of this name, `None` for an unknown name.
pub fn sample_project(name: &str) -> Option<Project> {
    build(name).ok().flatten()
}

fn build(name: &str) -> Result<Option<Project>, ProjectError> {
    use Designation::{X, Y};
    let mut p = Project::new();
    let mut rng = Rng(0x5eed_0f_5a3f);
    match name {
        "Line and Symbol" => {
            let t: Vec<f64> = (0..60).map(|i| f64::from(i) * 0.25).collect();
            let a = t.iter().map(|t| (-t / 6.0).exp() * t.cos()).collect();
            let b = t.iter().map(|t| (-t / 9.0).exp() * (t * 0.7).sin()).collect();
            let ids = book(&mut p, "Oscillations", vec![("Time", X, t), ("A", Y, a), ("B", Y, b)])?;
            p.plot_columns(Template::LineSymbol, &ids[1..])?;
        }
        "Peaks" => {
            let x: Vec<f64> = (0..400).map(|i| 10.0 + f64::from(i) * 0.1).collect();
            let peak = |x: f64, c: f64, w: f64, h: f64| h * (-0.5 * ((x - c) / w).powi(2)).exp();
            let y = x
                .iter()
                .map(|&x| {
                    2.0 + 0.02 * x + peak(x, 18.0, 0.8, 30.0) + peak(x, 26.5, 1.2, 18.0) + peak(x, 41.0, 0.6, 24.0)
                })
                .map(|y| y + rng.noise(0.6))
                .collect();
            let ids = book(&mut p, "Spectrum", vec![("Position", X, x), ("Intensity", Y, y)])?;
            p.plot_columns(Template::Line, &ids[1..])?;
        }
        "Groups" => {
            let mut cols = Vec::new();
            for (name, mean, sd) in [("Control", 10.0, 1.2), ("Dose A", 12.5, 1.6), ("Dose B", 14.0, 2.1)] {
                cols.push((name, Y, (0..25).map(|_| mean + rng.noise(sd)).collect()));
            }
            let ids = book(&mut p, "Measurements", cols)?;
            p.plot_columns(Template::BoxChart, &ids)?;
        }
        "Stock Prices" => {
            let mut close = 100.0;
            let (mut o, mut h, mut l, mut c, mut v) = (vec![], vec![], vec![], vec![], vec![]);
            for _ in 0..40 {
                let open = close + rng.noise(0.8);
                close = open + rng.noise(2.0);
                o.push(open);
                c.push(close);
                h.push(open.max(close) + rng.next() * 1.5);
                l.push(open.min(close) - rng.next() * 1.5);
                v.push((1000.0 + rng.noise(300.0)).max(100.0).round());
            }
            let day = (1..=40).map(f64::from).collect();
            let cols =
                vec![("Day", X, day), ("Open", Y, o), ("High", Y, h), ("Low", Y, l), ("Close", Y, c), ("Volume", Y, v)];
            let ids = book(&mut p, "Prices", cols)?;
            p.plot_columns(Template::OhlcVolume, &ids[1..])?;
        }
        "Wind" => {
            let mut dir = Vec::new();
            let mut speed = Vec::new();
            for _ in 0..300 {
                let d = if rng.next() < 0.7 { 225.0 + rng.noise(35.0) } else { rng.next() * 360.0 };
                dir.push(d.rem_euclid(360.0));
                speed.push((6.0 + rng.noise(2.5)).abs());
            }
            let ids = book(&mut p, "Wind", vec![("Direction", X, dir), ("Speed", Y, speed)])?;
            p.plot_columns(Template::WindRoseRaw, &ids[1..])?;
        }
        "Process Control" => {
            let v = (0..30).map(|i| 50.0 + if i >= 20 { 0.8 } else { 0.0 } + rng.noise(0.5)).collect();
            let ids = book(&mut p, "Process", vec![("Diameter", Y, v)])?;
            p.plot_qc_xbar_r(ids[0], 5)?;
        }
        _ => return Ok(None),
    }
    Ok(Some(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sample_builds_a_workbook_and_a_graph_the_same_way_each_time() {
        for (name, _) in SAMPLES {
            let p = sample_project(name).unwrap_or_else(|| panic!("{name}"));
            assert!(p.workbooks().count() >= 1, "{name}");
            assert!(p.graphs().count() == 1, "{name}");
            assert_eq!(p, sample_project(name).unwrap(), "{name} is the same every time");
        }
        assert!(sample_project("No such sample").is_none());
    }
}
