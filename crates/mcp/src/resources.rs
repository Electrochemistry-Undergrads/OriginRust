//! Resources: the project tree, one column as CSV, one graph as PNG.

use crate::b64;
use crate::server::{INVALID_PARAMS, RESOURCE_NOT_FOUND, RpcError};
use crate::tools::{CELL_CAP, cell_value, contents};
use serde_json::{Value, json};
use sheaf_core::Project;
use sheaf_ops::render::ExportSettings;

const PROJECT_URI: &str = "sheaf://project";

fn encode(name: &str) -> String {
    let mut out = String::new();
    for b in name.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = text.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

pub fn templates() -> Vec<Value> {
    vec![
        json!({
            "uriTemplate": "sheaf://column/{book}/{sheet}/{column}", "name": "column",
            "description": "One column with its X column as CSV, at most 10000 cells; a longer column is cut with a last comment line giving the total, and read_columns pages through it. Names are percent-encoded.",
            "mimeType": "text/csv"
        }),
        json!({
            "uriTemplate": "sheaf://graph/{name}.png", "name": "graph",
            "description": "A graph rendered as a PNG at 150 dpi.",
            "mimeType": "image/png"
        }),
    ]
}

pub fn list(p: &Project) -> Vec<Value> {
    let mut out = vec![json!({
        "uri": PROJECT_URI, "name": "project", "description": "Folders, workbooks, sheets, columns and graphs.",
        "mimeType": "application/json"
    })];
    for g in p.graphs() {
        out.push(json!({
            "uri": format!("sheaf://graph/{}.png", encode(&g.name)), "name": g.name,
            "description": "The graph as a PNG.", "mimeType": "image/png"
        }));
    }
    if let Some(b) = p.workbooks().next() {
        let b = p.workbooks().find(|w| w.active_sheet < w.sheets.len()).unwrap_or(b);
        let ws = b.sheet();
        for c in &ws.columns {
            out.push(json!({
                "uri": format!("sheaf://column/{}/{}/{}", encode(&b.name), encode(&ws.name), encode(&c.short_name)),
                "name": format!("{}:{}:{}", b.name, ws.name, c.short_name),
                "description": c.long_name, "mimeType": "text/csv"
            }));
        }
    }
    out
}

fn not_found(uri: &str) -> RpcError {
    RpcError::new(RESOURCE_NOT_FOUND, format!("resource not found: {uri}"))
}

fn csv_field(text: &str) -> String {
    if text.contains([',', '"', '\n']) { format!("\"{}\"", text.replace('"', "\"\"")) } else { text.to_string() }
}

fn csv_value(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::String(t) => csv_field(t),
        other => other.to_string(),
    }
}

pub fn read(p: &Project, uri: &str) -> Result<Value, RpcError> {
    if uri == PROJECT_URI {
        let text = serde_json::to_string_pretty(&contents(p, None)).unwrap_or_default();
        return Ok(json!({"contents": [{"uri": uri, "mimeType": "application/json", "text": text}]}));
    }
    if let Some(rest) = uri.strip_prefix("sheaf://graph/") {
        let name = rest.strip_suffix(".png").and_then(decode).ok_or_else(|| not_found(uri))?;
        let graph = p.graphs().find(|g| g.name.eq_ignore_ascii_case(&name)).ok_or_else(|| not_found(uri))?;
        let settings = ExportSettings { dpi: 150.0, ..ExportSettings::default() };
        let png = sheaf_ops::render::render_graph(p, graph.id, &settings, "png")
            .map_err(|e| RpcError::new(INVALID_PARAMS, crate::tools::clean_message(&format!("{e:#}"))))?;
        return Ok(json!({"contents": [{"uri": uri, "mimeType": "image/png", "blob": b64::encode(&png)}]}));
    }
    if let Some(rest) = uri.strip_prefix("sheaf://column/") {
        let parts: Vec<Option<String>> = rest.split('/').map(decode).collect();
        let [Some(book), Some(sheet), Some(column)] = parts.as_slice() else { return Err(not_found(uri)) };
        let wb = p.workbooks().find(|b| b.name.eq_ignore_ascii_case(book)).ok_or_else(|| not_found(uri))?;
        let ws = wb.sheet_by_name(sheet).ok_or_else(|| not_found(uri))?;
        let index =
            ws.columns.iter().position(|c| c.short_name.eq_ignore_ascii_case(column)).ok_or_else(|| not_found(uri))?;
        let y = &ws.columns[index];
        let x = ws.x_for(index).filter(|x| x.id != y.id);
        let head = |c: &sheaf_core::column::Column| {
            let mut h = if c.long_name.is_empty() { c.short_name.clone() } else { c.long_name.clone() };
            if !c.units.is_empty() {
                h.push_str(&format!(" ({})", c.units));
            }
            csv_field(&h)
        };
        let mut cols = Vec::new();
        cols.extend(x);
        cols.push(y);
        let mut text = cols.iter().map(|c| head(c)).collect::<Vec<_>>().join(",");
        text.push('\n');
        let rows = cols.iter().map(|c| c.len()).max().unwrap_or(0).min(CELL_CAP / cols.len());
        for r in 0..rows {
            let line: Vec<String> = cols.iter().map(|c| csv_value(&cell_value(c, r))).collect();
            text.push_str(&line.join(","));
            text.push('\n');
        }
        let total = cols.iter().map(|c| c.len()).max().unwrap_or(0);
        if total > rows {
            text.push_str(&format!(
                "# truncated: {rows} of {total} rows; use read_columns with from and limit for the rest
"
            ));
        }
        return Ok(json!({"contents": [{"uri": uri, "mimeType": "text/csv", "text": text}]}));
    }
    Err(not_found(uri))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sheaf_core::column::{ColumnData, Designation};

    #[test]
    fn names_survive_encoding() {
        for n in ["Book1", "a b/c", "Ünï%x", "S:1"] {
            assert_eq!(decode(&encode(n)).unwrap(), n);
        }
        assert!(decode("%zz").is_none());
    }

    #[test]
    fn project_and_column_resources_read() {
        let mut project = Project::new();
        let book = project.add_workbook("");
        let x = project.add_column(book, 0, Designation::X, ColumnData::numeric(vec![1.0, 2.0])).unwrap();
        let y = project.add_column(book, 0, Designation::Y, ColumnData::numeric(vec![10.0, f64::NAN])).unwrap();
        project.column_mut(x).unwrap().short_name = "A".into();
        project.column_mut(y).unwrap().short_name = "B".into();
        project.column_mut(y).unwrap().units = "mA".into();
        let listed = list(&project);
        assert!(listed.iter().any(|r| r["uri"] == PROJECT_URI));
        assert!(listed.iter().any(|r| r["uri"] == "sheaf://column/Book1/Sheet1/B"));
        let r = read(&project, "sheaf://column/Book1/Sheet1/B").unwrap();
        assert_eq!(r["contents"][0]["text"], "A,B (mA)\n1.0,10.0\n2.0,\n");
        let p = read(&project, PROJECT_URI).unwrap();
        assert!(p["contents"][0]["text"].as_str().unwrap().contains("Book1"));
        let long: Vec<f64> = (0..20000).map(|i| i as f64).collect();
        let z = project.add_column(book, 0, Designation::Y, ColumnData::numeric(long)).unwrap();
        project.column_mut(z).unwrap().short_name = "C".into();
        let t = read(&project, "sheaf://column/Book1/Sheet1/C").unwrap();
        let text = t["contents"][0]["text"].as_str().unwrap();
        assert!(text.contains("# truncated: 5000 of 20000 rows"), "{}", &text[text.len().saturating_sub(120)..]);
        assert_eq!(read(&project, "sheaf://column/Book1/Sheet1/Q").unwrap_err().code, RESOURCE_NOT_FOUND);
        assert_eq!(read(&project, "sheaf://graph/Nope.png").unwrap_err().code, RESOURCE_NOT_FOUND);
        assert_eq!(read(&project, "file:///etc/passwd").unwrap_err().code, RESOURCE_NOT_FOUND);
    }
}
