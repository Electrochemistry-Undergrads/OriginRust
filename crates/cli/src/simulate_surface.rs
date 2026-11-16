//! `sheaf simulate-surface`: XYZ data of a surface fit function on a grid.

use crate::args::SimulateSurfaceArgs;
use crate::commands::{is_project, load_project, read_data_file, read_settings, save_project, write_sheet};
use anyhow::{Result, bail};
use sheaf_core::Project;
use sheaf_ops::cli_simulate::simulate_surface_spec;
use sheaf_ops::simulate_surface::{list_functions, run};

fn print_list() {
    let functions = list_functions();
    let width = functions.iter().map(|f| f.name.len()).max().unwrap_or(4);
    for f in &functions {
        let params: Vec<String> =
            f.parameters.iter().map(|(n, v)| format!("{n}={}", crate::analyze::short_number(*v))).collect();
        println!(
            "{:<width$}  X, Y {} to {}, noise {}%  {}",
            f.name,
            crate::analyze::short_number(f.range.0),
            crate::analyze::short_number(f.range.1),
            crate::analyze::short_number(f.noise),
            params.join(", ")
        );
    }
    println!("\nThe values are those of Origin's Simulate Surface window; 20 by 20 points.");
}

pub fn simulate_surface(a: SimulateSurfaceArgs) -> Result<()> {
    if a.list {
        print_list();
        return Ok(());
    }
    if a.input.is_none() && a.book.is_some() {
        bail!("--book needs an input: a project");
    }
    let spec = simulate_surface_spec(&a);
    let mut project = match &a.input {
        Some(input) if is_project(input) => load_project(input)?,
        Some(input) => {
            let mut p = Project::new();
            p.adopt_workbook(read_data_file(input, &read_settings(&a.read)?)?.workbook);
            p
        }
        None => Project::new(),
    };
    let result = run(&mut project, &spec)?;
    let book = project.workbook(result.book).expect("the run made or found it");
    let ws = &book.sheets[result.result_sheet];
    let quiet = a.output.is_none() && a.save.is_none();
    // The data itself goes to standard output when nothing else says where it goes.
    macro_rules! say {
        ($($t:tt)*) => { if quiet { eprintln!($($t)*) } else { println!($($t)*) } };
    }
    for note in &result.report.notes {
        say!("{note}");
    }
    say!("{}", result.summary(&project));
    if quiet {
        let text = sheaf_ops::tabular::sheet_to_bytes(ws, sheaf_ops::tabular::SheetFormat::Csv)?;
        print!("{}", String::from_utf8_lossy(&text));
    }
    if let Some(path) = &a.output {
        if sheaf_ops::tabular::is_xlsx_path(path) {
            let request = sheaf_ops::tabular::XlsxRequest::default();
            sheaf_ops::tabular::write_xlsx(&project, result.book, Some(&ws.name), &request, path)?;
            println!("wrote {}", path.display());
        } else {
            write_sheet(ws, path)?;
        }
    }
    if let Some(path) = &a.save {
        save_project(&project, path)?;
    }
    Ok(())
}
