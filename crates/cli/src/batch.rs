//! `sheaf batch`: File > Batch Processing... without a window (T235; workbook-model 13p). The run is
//! `Project::batch_process`, the same as the window's.

use crate::commands::{load_project, read_data_file, read_settings, save_project, write_sheet};
use anyhow::{Context, Result, bail};
use sheaf_core::{BatchSummaryRequest, Project};
use sheaf_ops::args::BatchArgs;

pub(crate) fn batch(mut a: BatchArgs) -> Result<()> {
    if a.output.is_none() && a.summary_csv.is_none() {
        bail!("say where the result goes: --output for the project, --summary-csv for the summary, or both");
    }
    if let Some(out) = &a.output {
        let out = sheaf_io::save::default_extension(out);
        if a.files.iter().any(|f| f == &out) {
            bail!("the output '{}' is also one of the inputs", out.display());
        }
        a.output = Some(out);
    }
    let template = sheaf_io::user_templates::load_book_file(&a.template)
        .with_context(|| format!("could not read the workbook template '{}'", a.template.display()))?;
    if let Some(sheet) = &a.summary
        && !template.book.sheets.iter().any(|s| s.name.eq_ignore_ascii_case(sheet))
    {
        let names: Vec<&str> = template.book.sheets.iter().map(|s| s.name.as_str()).collect();
        bail!("the template {} has no sheet '{sheet}' (its sheets: {})", template.name, names.join(", "));
    }
    let settings = read_settings(&a.read)?;
    let mut data = Vec::new();
    let mut failed = 0;
    for path in &a.files {
        match read_data_file(path, &settings) {
            Ok(imported) => {
                let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                data.push((name, imported.workbook));
            }
            Err(e) => {
                eprintln!("warning: skipped '{}': {e:#}", path.display());
                failed += 1;
            }
        }
    }
    if data.is_empty() {
        bail!("no data file could be read");
    }
    let mut project = match &a.output {
        Some(out) if a.append && out.exists() => load_project(out)?,
        _ => Project::new(),
    };
    let identifier =
        if a.identifier.trim().is_empty() { "Dataset".to_string() } else { a.identifier.trim().to_string() };
    let request = a.summary.as_ref().map(|sheet| BatchSummaryRequest {
        sheet: sheet.clone(),
        identifier,
        delete_books: a.delete_books,
    });
    let run = project.batch_process(&template, &data, request.as_ref());
    for (made, (name, _)) in run.books.iter().zip(&data) {
        let book = project.workbook(made.book).map(|b| b.name.clone());
        match book {
            Some(book) => println!("{book}: {name} ({} column(s) filled)", made.filled),
            None => println!("{name}: {} column(s) filled", made.filled),
        }
        if made.ignored > 0 {
            eprintln!("note: {name}: {} column(s) beyond the template's data columns were left out", made.ignored);
        }
        for p in &made.problems {
            eprintln!("warning: {name}: {p}");
        }
    }
    if let Some(id) = run.summary {
        let b = project.workbook(id).context("the summary book")?;
        let rows = b.sheets[0].columns.first().map_or(0, |c| c.len());
        println!("summary {}: {rows} row(s){}", b.name, if run.deleted { "; the books were deleted" } else { "" });
        if let Some(csv) = &a.summary_csv {
            write_sheet(&b.sheets[0], csv)?;
        }
    }
    if let Some(out) = &a.output {
        save_project(&project, out)?;
    }
    if failed > 0 {
        eprintln!("note: {failed} file(s) could not be read");
    }
    Ok(())
}
