//! XYZ Surface Area (Analysis > Mathematics > XYZ Surface Area): the area of the surface through the points of
//! three columns X, Y and Z. The numbers come from `sheaf_calc::xyz_surface_area`; Origin 2016 returns the
//! number only, so the sheet (area, the points, triangles and merged and skipped rows) is Sheaf's own layout
//! (`docs/spec/analysis-tools.md`, section 18).

use super::statistics::Output;
use super::*;

/// The columns XYZ Surface Area starts with: the first numeric Z column with the nearest numeric Y column before it
/// and the nearest X column before that (the layout of the surface fits, `docs/spec/fitting.md` section 14); a sheet
/// with no Z column gives its first three numeric columns. `None` when the sheet has no such three columns.
pub fn default_columns(sheet: &Worksheet) -> Option<[ColumnId; 3]> {
    let numeric = |c: &&Column| c.data.is_numeric();
    if let Some(z) = sheet.columns.iter().position(|c| c.designation == Designation::Z && c.data.is_numeric()) {
        let before = &sheet.columns[..z];
        let y = before.iter().rposition(|c| c.designation == Designation::Y && c.data.is_numeric());
        let x = y.and_then(|y| {
            sheet.columns[..y].iter().rposition(|c| c.designation == Designation::X && c.data.is_numeric())
        });
        if let (Some(x), Some(y)) = (x, y) {
            return Some([sheet.columns[x].id, sheet.columns[y].id, sheet.columns[z].id]);
        }
    }
    let mut first = sheet.columns.iter().filter(numeric);
    Some([first.next()?.id, first.next()?.id, first.next()?.id])
}

/// One column's values, rows outside the range set to missing: name, units, values.
fn read(sheet: &Worksheet, id: ColumnId, rows: RowRange) -> Result<(String, String, Vec<f64>), AnalysisError> {
    let c = sheet.column(id).ok_or(AnalysisError::NoSuchColumn(id))?;
    let data = sheet.values_of(c).ok_or_else(|| AnalysisError::NotNumeric(c.short_name.clone()))?;
    if data.is_empty() {
        return Err(AnalysisError::Empty(c.short_name.clone()));
    }
    let data = match rows {
        RowRange::All => data.into_owned(),
        RowRange::Rows(..) => {
            let (first, last) = resolve_rows(rows, &c.short_name, &[], data.len())?;
            (0..data.len()).map(|i| if (first..=last).contains(&i) { data[i] } else { f64::NAN }).collect()
        }
        RowRange::XBetween(..) => {
            return Err(AnalysisError::BadOption(
                "XYZ Surface Area has no X column to limit; choose rows with a row range".into(),
            ));
        }
    };
    Ok((c.display_name().to_string(), c.units.clone(), data))
}

pub(super) fn xyz_area(sheet: &Worksheet, columns: &[ColumnId], rows: RowRange) -> Result<Output, AnalysisError> {
    let [xi, yi, zi] = columns else {
        return Err(AnalysisError::BadOption("XYZ Surface Area needs three columns: X, Y and Z".into()));
    };
    if xi == yi || xi == zi || yi == zi {
        return Err(AnalysisError::BadOption("choose three different columns for X, Y and Z".into()));
    }
    let (xn, xu, x) = read(sheet, *xi, rows)?;
    let (yn, yu, y) = read(sheet, *yi, rows)?;
    let (zn, zu, z) = read(sheet, *zi, rows)?;
    let n = x.len().max(y.len()).max(z.len());
    let fill = |mut v: Vec<f64>| {
        v.resize(n, f64::NAN);
        v
    };
    let (x, y, z) = (fill(x), fill(y), fill(z));
    let r = calc::xyz_surface_area(&x, &y, &z).map_err(|e| AnalysisError::BadOption(e.to_string()))?;
    let mut table = ResultTable::new(&["Area", "Projected area", "Points", "Triangles", "Merged rows", "Skipped rows"]);
    // The area has a unit only when all three columns share it (a length).
    let area_unit = if xu == yu && yu == zu && !xu.is_empty() { format!("{xu}^2") } else { String::new() };
    table.push_row(
        format!("{zn}({xn}, {yn})"),
        vec![r.area, r.projected_area, r.points as f64, r.triangles as f64, r.merged as f64, r.skipped as f64],
        vec![area_unit, unit_product(&xu, &yu), String::new(), String::new(), String::new(), String::new()],
    );
    let mut notes = vec![format!(
        "Surface area {} from {} points and {} triangles (Delaunay triangulation of X and Y)",
        r.area, r.points, r.triangles
    )];
    if r.merged > 0 {
        notes.push(format!("{} rows with the same X and Y were merged, their Z averaged", r.merged));
    }
    if r.skipped > 0 {
        notes.push(format!("{} rows with a missing X, Y or Z were left out", r.skipped));
    }
    Ok(Output { table: Some(table), on_sheet: Vec::new(), notes, plots: Vec::new(), comparison_pairs: Vec::new() })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> (Project, [ColumnId; 3]) {
        let mut p = Project::new();
        let book = p.add_workbook("Book1");
        let n = ColumnData::numeric;
        let x = p.add_column(book, 0, Designation::X, n(vec![0.0, 1.0, 0.0, 1.0, f64::NAN])).unwrap();
        let y = p.add_column(book, 0, Designation::Y, n(vec![0.0, 0.0, 1.0, 1.0, 1.0])).unwrap();
        let z = p.add_column(book, 0, Designation::Z, n(vec![0.0, 0.0, 0.0, 0.0, 0.0])).unwrap();
        (p, [x, y, z])
    }

    fn request(c: &[ColumnId]) -> Request {
        Request { columns: c.to_vec(), rows: RowRange::All, tool: Tool::XyzArea }
    }

    #[test]
    fn a_flat_square_has_area_one_in_a_results_sheet() {
        let (mut p, c) = project();
        let book = p.workbooks().next().unwrap().id;
        let r = p.analyze(&request(&c)).unwrap();
        let sheet = &p.workbook(book).unwrap().sheets[r.new_sheet.unwrap()];
        assert!(sheet.name.starts_with("XYZArea"), "{}", sheet.name);
        let col = |name: &str| sheet.columns.iter().find(|c| c.long_name == name).unwrap().numeric().unwrap().to_vec();
        assert_eq!(col("Area"), [1.0]);
        assert_eq!(col("Points"), [4.0]);
        assert_eq!(col("Skipped rows"), [1.0]);
        assert!(r.message.contains("XYZ surface area"), "{}", r.message);
    }

    #[test]
    fn a_row_range_limits_the_points_and_bad_input_is_refused() {
        let (mut p, c) = project();
        let before = p.clone();
        let mut req = request(&c);
        req.rows = RowRange::Rows(1, 3);
        assert!(p.analyze(&req).unwrap().notes.iter().any(|n| n.contains("3 points")));
        let mut p = before.clone();
        req.rows = RowRange::Rows(1, 2);
        assert!(matches!(p.analyze(&req), Err(AnalysisError::BadOption(m)) if m.contains("at least three")));
        assert!(matches!(p.analyze(&request(&c[..2])), Err(AnalysisError::BadOption(_))));
        assert!(matches!(p.analyze(&request(&[c[0], c[0], c[2]])), Err(AnalysisError::BadOption(_))));
        assert_eq!(p, before);
    }

    #[test]
    fn the_default_columns_are_the_z_column_with_its_y_and_x() {
        let (p, c) = project();
        let sheet = &p.workbooks().next().unwrap().sheets[0];
        assert_eq!(default_columns(sheet), Some(c));
        // With no Z column the first three numeric columns.
        let mut q = Project::new();
        let b = q.add_workbook("B");
        let ids: Vec<ColumnId> =
            (0..4).map(|_| q.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0])).unwrap()).collect();
        assert_eq!(default_columns(&q.workbook(b).unwrap().sheets[0]), Some([ids[0], ids[1], ids[2]]));
        let mut r = Project::new();
        let b = r.add_workbook("B");
        r.add_column(b, 0, Designation::Y, ColumnData::numeric(vec![1.0])).unwrap();
        assert_eq!(default_columns(&r.workbook(b).unwrap().sheets[0]), None);
    }

    #[test]
    fn running_it_is_one_undo_step() {
        let (p, c) = project();
        let mut doc = crate::Document::new(p);
        doc.edit(Tool::XyzArea.name(), |p| p.analyze(&request(&c))).unwrap();
        assert_eq!(doc.undo_label(), Some("XYZ surface area"));
        doc.undo();
        assert_eq!(doc.project().workbooks().next().unwrap().sheets.len(), 1);
    }
}
