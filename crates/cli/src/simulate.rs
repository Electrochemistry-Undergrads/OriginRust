//! `sheaf simulate`: the values of a built-in fit function at X values from a range or a column.

use crate::args::SimulateArgs;
use crate::commands::{is_project, load_project, read_data_file, read_settings, save_project, write_sheet};
use anyhow::{Context, Result, bail};
use sheaf_core::Project;
use sheaf_ops::cli_simulate::simulate_spec;
use sheaf_ops::simulate::{list_functions, run};

fn print_list() {
    let functions = list_functions();
    let width = functions.iter().map(|f| f.name.len()).max().unwrap_or(4);
    let mut category = String::new();
    for f in &functions {
        if f.category != category {
            category = f.category.clone();
            println!("\n{category}");
        }
        let params: Vec<String> =
            f.parameters.iter().map(|(n, v)| format!("{n}={}", crate::analyze::short_number(*v))).collect();
        println!("  {:<width$}  {}", f.name, params.join(", "));
    }
    println!("\nThe values after each parameter are the ones a simulation starts from.");
}

pub fn simulate(a: SimulateArgs) -> Result<()> {
    if a.list {
        print_list();
        return Ok(());
    }
    if a.input.is_none() && (a.x_of.is_some() || a.book.is_some() || a.sheet.is_some()) {
        bail!("--x-of, --book and --sheet need an input: a data file or a project");
    }
    let spec = simulate_spec(&a);
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
    // The curve itself goes to standard output when nothing else says where it goes.
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
    if let Some(path) = &a.plot {
        crate::commands::require_positive("--dpi", Some(a.dpi))?;
        let graph = result.report.new_graph.expect("a plot was asked for");
        sheaf_plot::export_file(&project, graph, path, a.dpi)
            .with_context(|| format!("could not write '{}'", path.display()))?;
        say!("wrote {}", path.display());
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
