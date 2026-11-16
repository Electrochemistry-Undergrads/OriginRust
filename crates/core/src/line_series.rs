//! Line Series (T82), Origin's Plot > Line + Symbol > Line Series: every worksheet row of the selected Y columns
//! becomes one line whose values stand at X = 1, 2, ..., k (k columns) and are joined; the rows are not joined to
//! each other. Origin makes one plot with a k-point segment line connection and keeps the rearranged data out of
//! sight; Sheaf puts it on a new sheet `LineSeries` of the workbook (positions, values, column names).

use crate::column::{Column, ColumnData, Designation};
use crate::graph::{Connect, SymbolShape, TickLabelSource};
use crate::project::ProjectError;
use crate::template::Template;
use crate::{Color, ColumnId, Project, WindowId, Worksheet};

impl Project {
    /// Makes the Line Series graph of the given Y columns (at least two, all of one sheet): one undoable change of
    /// the project when called through the document's edit. The helper sheet is added to the source workbook.
    pub fn plot_line_series(&mut self, ys: &[ColumnId]) -> Result<WindowId, ProjectError> {
        if ys.len() < 2 {
            return Err(ProjectError::LineSeriesNeedsTwoColumns(ys.len()));
        }
        let k = ys.len();
        let (book, sheet) = self.locate_sheet(ys[0]).ok_or(ProjectError::NoSuchColumn(ys[0]))?;
        let mut cols: Vec<(String, Vec<f64>)> = Vec::with_capacity(k);
        {
            let ws = &self.workbook(book).expect("located").sheets[sheet];
            for &y in ys {
                let c = ws.column(y).ok_or(ProjectError::LineSeriesOneSheet)?;
                cols.push((c.display_name().to_string(), c.numeric().map(<[f64]>::to_vec).unwrap_or_default()));
            }
        }
        let rows = cols.iter().map(|(_, v)| v.len()).max().unwrap_or(0);
        let (mut xs, mut vals) = (Vec::new(), Vec::new());
        for r in 0..rows {
            let row: Vec<f64> =
                cols.iter().map(|(_, v)| v.get(r).copied().filter(|x| !x.is_nan()).unwrap_or(f64::NAN)).collect();
            if row.iter().all(|x| x.is_nan()) {
                continue;
            }
            for (i, v) in row.into_iter().enumerate() {
                xs.push((i + 1) as f64);
                vals.push(v);
            }
        }
        let names: Vec<String> = cols.into_iter().map(|(n, _)| n).collect();

        let mut ws = Worksheet::new(crate::analysis::free_sheet_name(
            &self.workbook(book).expect("located").sheets,
            "LineSeries",
        ));
        let mut add = |this: &mut Project, d: Designation, data: ColumnData, name: &str| {
            let id = this.new_column_id();
            let short = ws.next_short_name();
            ws.columns.push(Column::new(id, short, d, data).with_labels(name, "", ""));
            id
        };
        let x = add(self, Designation::X, ColumnData::numeric(xs), "Position");
        let y = add(self, Designation::Y, ColumnData::numeric(vals), "Value");
        let label = add(self, Designation::Label, ColumnData::text(names), "Series");
        self.workbook_mut(book).expect("located").sheets.push(ws);

        let graph = self.add_graph(Template::LineSymbol);
        self.add_plot_xy(graph, 0, Some(x), y, None)?;
        let g = self.graph_mut(graph).expect("just added");
        let l = &mut g.layers[0];
        let p = &mut l.plots[0];
        p.line.connect = match k {
            2 => Connect::Segment2,
            3 => Connect::Segment3,
            _ => Connect::Segments(u16::try_from(k).unwrap_or(u16::MAX)),
        };
        p.auto_color = false;
        p.line.color = Color::BLACK;
        p.symbol.color = Color::BLACK;
        p.symbol.shape = SymbolShape::Square;
        p.symbol.filled = true;
        p.symbol.size_pt = 8.0;
        l.x_axis.tick_label_source = TickLabelSource::Column { column: label };
        l.x_axis.title = Some("Series".to_string());
        l.y_axis.title = Some("Values".to_string());
        Ok(graph)
    }

    /// The workbook and the sheet index that hold a column.
    fn locate_sheet(&self, id: ColumnId) -> Option<(WindowId, usize)> {
        self.workbooks().find_map(|b| b.sheets.iter().position(|s| s.column(id).is_some()).map(|i| (b.id, i)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn book() -> (Project, WindowId, Vec<ColumnId>) {
        let mut p = Project::new();
        let b = p.add_workbook("demo");
        let nan = f64::NAN;
        let data = [
            ("B", vec![1.0, 2.0, nan, 4.0, 5.0]),
            ("C", vec![10.0, nan, nan, 40.0, 50.0]),
            ("D", vec![100.0, 200.0, nan, 400.0, 500.0]),
        ];
        let ys = data
            .into_iter()
            .map(|(n, v)| {
                let id = p.add_column(b, 0, Designation::Y, ColumnData::numeric(v)).unwrap();
                p.workbook_mut(b).unwrap().sheets[0].column_mut(id).unwrap().long_name = n.to_string();
                id
            })
            .collect();
        (p, b, ys)
    }

    #[test]
    fn the_helper_sheet_holds_positions_values_and_names() {
        let (mut p, b, ys) = book();
        let g = p.plot_line_series(&ys).unwrap();
        let wb = p.workbook(b).unwrap();
        assert_eq!(wb.sheets.len(), 2);
        let s = &wb.sheets[1];
        assert_eq!(s.name, "LineSeries1");
        let xs = s.columns[0].numeric().unwrap();
        // the all-missing row 3 is skipped: 4 rows of 3 values
        assert_eq!(xs, [1.0, 2.0, 3.0].repeat(4).as_slice());
        let v = s.columns[1].numeric().unwrap();
        assert_eq!(v.len(), 12);
        assert_eq!(&v[..2], &[1.0, 10.0]);
        assert_eq!(v[2], 100.0);
        assert_eq!(v[3], 2.0);
        assert!(v[4].is_nan(), "a missing value inside a row stays missing");
        assert_eq!(v[5], 200.0);
        assert_eq!(&v[6..9], &[4.0, 40.0, 400.0]);
        assert_eq!(s.columns[2].text().unwrap(), ["B", "C", "D"]);
        let gr = p.graph(g).unwrap();
        let l = &gr.layers[0];
        assert_eq!(l.plots.len(), 1);
        assert_eq!(l.plots[0].line.connect, Connect::Segment3);
        assert_eq!(l.x_axis.title.as_deref(), Some("Series"));
        assert_eq!(l.y_axis.title.as_deref(), Some("Values"));
        assert_eq!(l.x_axis.tick_label_source, TickLabelSource::Column { column: s.columns[2].id });
    }
}
