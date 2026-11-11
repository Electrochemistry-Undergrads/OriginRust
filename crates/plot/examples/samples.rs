//! Renders two sample figures (a CV overlay and a line plot with a legend
//! and a gap) so that the output can be looked at.
//!
//! Usage: `cargo run -p sheaf-plot --example samples -- <output folder>`

use sheaf_core::template::Template;
use sheaf_core::{ColumnData, Designation, Project};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out: PathBuf = std::env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    std::fs::create_dir_all(&out)?;
    let mut p = Project::new();

    // Three synthetic CV scans: a triangular potential sweep with a redox pair.
    let book = p.add_workbook("CV scans");
    let n = 400;
    let sweep: Vec<f64> = (0..n)
        .map(|i| {
            let t = i as f64 / (n - 1) as f64;
            if t < 0.5 { 0.2 + 0.8 * t * 2.0 } else { 1.0 - 0.8 * (t - 0.5) * 2.0 }
        })
        .collect();
    let x = p.add_column(book, 0, Designation::X, ColumnData::numeric(sweep.clone()))?;
    {
        let c = p.column_mut(x).ok_or("column")?;
        c.long_name = "Potential".into();
        c.units = "V vs. Ag/AgCl".into();
    }
    let cv = p.add_graph(Template::CyclicVoltammetry);
    for (k, amp) in [1.0, 1.3, 1.7].into_iter().enumerate() {
        let current: Vec<f64> = sweep
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let forward = i < n / 2;
                let peak = if forward { 0.62 } else { 0.5 };
                let sign = if forward { 1.0 } else { -1.0 };
                amp * (sign * 8.0 * (-((v - peak) / 0.07).powi(2)).exp() + 0.4 * (v - 0.6))
            })
            .collect();
        let y = p.add_column(book, 0, Designation::Y, ColumnData::numeric(current))?;
        let c = p.column_mut(y).ok_or("column")?;
        c.long_name = "Current".into();
        c.units = "mA".into();
        c.comments = format!("Scan {}", k + 1);
        p.add_plot(cv, 0, y)?;
    }
    sheaf_plot::export_file(&p, cv, &out.join("sample_cv.png"), 200.0)?;
    sheaf_plot::export_file(&p, cv, &out.join("sample_cv.svg"), 200.0)?;
    sheaf_plot::export_file(&p, cv, &out.join("sample_cv.pdf"), 200.0)?;

    // Two curves with a legend and a gap in the second.
    let book2 = p.add_workbook("Decay");
    let xs: Vec<f64> = (0..60).map(|i| i as f64 * 0.5).collect();
    let tx = p.add_column(book2, 0, Designation::X, ColumnData::numeric(xs.clone()))?;
    {
        let c = p.column_mut(tx).ok_or("column")?;
        c.long_name = "Time".into();
        c.units = "s".into();
    }
    let line = p.add_graph(Template::Line);
    for (k, tau) in [6.0, 14.0].into_iter().enumerate() {
        let mut ys: Vec<f64> = xs.iter().map(|t| 120.0 * (-t / tau).exp() + 5.0 * k as f64).collect();
        if k == 1 {
            for v in &mut ys[25..32] {
                *v = f64::NAN;
            }
        }
        let y = p.add_column(book2, 0, Designation::Y, ColumnData::numeric(ys))?;
        let c = p.column_mut(y).ok_or("column")?;
        c.long_name = "Signal".into();
        c.units = "\\g(m)A".into();
        c.comments = if k == 0 { "\\g(t) = 6 s".into() } else { "\\g(t) = 14 s".into() };
        p.add_plot(line, 0, y)?;
    }
    sheaf_plot::export_file(&p, line, &out.join("sample_line.png"), 200.0)?;
    println!("wrote samples to {}", out.display());
    Ok(())
}
