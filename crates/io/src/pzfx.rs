//! GraphPad Prism XML files (`.pzfx`) as workbooks (T447; Origin 2026b's GraphPad Prism connector app, base slice;
//! not measured in Origin, which has no such reader in 2016).
//!
//! A `.pzfx` file is XML: a `GraphPadPrismFile` root with one `Table` (or `HugeTable`) per data table. A table has a
//! `Title`, an optional `RowTitlesColumn`, an optional `XColumn` and one `YColumn` per data set; each column holds one
//! `Subcolumn` per replicate (or per role: mean, SD, N ... as the table's `YFormat` says), and each subcolumn one `d`
//! element per row (empty for a missing value, `Excluded="1"` for a value Prism leaves out of its analyses).
//!
//! Each table becomes a sheet named after its title. Row titles become a Label column, the X column an X column, and
//! every subcolumn of a data set a column whose long name is the data set's title, whose comments name the replicate
//! or role, and whose user parameters `Group` and `Role` keep them for analyses; SD, SEM and %CV subcolumns are Y
//! error columns. Excluded values stay in the sheet and are masked. Prism's analyses, graphs and layouts are not read.

use crate::error::IoError;
use sheaf_core::ColumnId;
use sheaf_core::column::{Column, ColumnData, Designation, default_short_name};
use sheaf_core::worksheet::ImportSource;
use sheaf_core::{WindowId, Workbook, Worksheet};

/// An XML element: its name, attributes and children (elements and text).
#[derive(Debug, Clone, PartialEq, Default)]
struct Element {
    name: String,
    attrs: Vec<(String, String)>,
    children: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq)]
enum Node {
    Element(Element),
    Text(String),
}

impl Element {
    fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }

    fn elements(&self) -> impl Iterator<Item = &Element> {
        self.children.iter().filter_map(|n| if let Node::Element(e) = n { Some(e) } else { None })
    }

    fn child(&self, name: &str) -> Option<&Element> {
        self.elements().find(|e| e.name == name)
    }

    /// The text of the element and its descendants.
    fn text(&self) -> String {
        let mut out = String::new();
        for n in &self.children {
            match n {
                Node::Text(t) => out.push_str(t),
                Node::Element(e) => out.push_str(&e.text()),
            }
        }
        out
    }
}

/// Replaces the five named entities and numeric character references.
fn unescape(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let Some(end) = rest.find(';') else { break };
        let entity = &rest[1..end];
        let c = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ if entity.starts_with("#x") || entity.starts_with("#X") => {
                u32::from_str_radix(&entity[2..], 16).ok().and_then(char::from_u32)
            }
            _ if entity.starts_with('#') => entity[1..].parse::<u32>().ok().and_then(char::from_u32),
            _ => None,
        };
        match c {
            Some(c) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// A small XML reader for Prism's files: elements, attributes, text, comments, processing instructions, CDATA and a
/// DOCTYPE; no namespaces or external entities.
fn parse_xml(text: &str) -> Result<Element, String> {
    let mut stack: Vec<Element> = vec![Element { name: String::new(), ..Default::default() }];
    let mut i = 0;
    let b = text.as_bytes();
    while i < b.len() {
        if b[i] != b'<' {
            let end = text[i..].find('<').map_or(b.len(), |j| i + j);
            let t = &text[i..end];
            if !t.trim().is_empty() {
                stack.last_mut().expect("the document").children.push(Node::Text(unescape(t)));
            }
            i = end;
            continue;
        }
        let rest = &text[i..];
        if rest.starts_with("<!--") {
            i += rest.find("-->").ok_or("a comment is not closed")? + 3;
        } else if rest.starts_with("<![CDATA[") {
            let end = rest.find("]]>").ok_or("a CDATA section is not closed")?;
            stack.last_mut().expect("the document").children.push(Node::Text(rest[9..end].to_string()));
            i += end + 3;
        } else if rest.starts_with("<?") {
            i += rest.find("?>").ok_or("a processing instruction is not closed")? + 2;
        } else if rest.starts_with("<!") {
            i += rest.find('>').ok_or("a declaration is not closed")? + 1;
        } else if let Some(close) = rest.strip_prefix("</") {
            let end = close.find('>').ok_or("an end tag is not closed")?;
            let name = close[..end].trim();
            let done = stack.pop().ok_or("an end tag without a start")?;
            if done.name != name || stack.is_empty() {
                return Err(format!("the end tag </{name}> does not match <{}>", done.name));
            }
            stack.last_mut().expect("a parent").children.push(Node::Element(done));
            i += 2 + end + 1;
        } else {
            // A start tag: name, attributes, then > or />; quoted values may hold '>'.
            let mut j = 1;
            let mut quote: Option<u8> = None;
            while i + j < b.len() {
                let c = b[i + j];
                match quote {
                    Some(q) if c == q => quote = None,
                    Some(_) => {}
                    None if c == b'"' || c == b'\'' => quote = Some(c),
                    None if c == b'>' => break,
                    None => {}
                }
                j += 1;
            }
            if i + j >= b.len() {
                return Err("a start tag is not closed".into());
            }
            let inner = &text[i + 1..i + j];
            let (inner, empty) = match inner.strip_suffix('/') {
                Some(s) => (s, true),
                None => (inner, false),
            };
            let name_end = inner.find(|c: char| c.is_whitespace()).unwrap_or(inner.len());
            let mut el = Element { name: inner[..name_end].to_string(), ..Default::default() };
            let mut attrs = inner[name_end..].trim_start();
            while !attrs.is_empty() {
                let eq = attrs.find('=').ok_or("an attribute without a value")?;
                let key = attrs[..eq].trim().to_string();
                let after = attrs[eq + 1..].trim_start();
                let q = after.chars().next().ok_or("an attribute without a value")?;
                if q != '"' && q != '\'' {
                    return Err("an attribute value without quotes".into());
                }
                let close = after[1..].find(q).ok_or("an attribute value is not closed")?;
                el.attrs.push((key, unescape(&after[1..1 + close])));
                attrs = after[1 + close + 1..].trim_start();
            }
            if empty {
                stack.last_mut().expect("a parent").children.push(Node::Element(el));
            } else {
                stack.push(el);
            }
            i += j + 1;
        }
    }
    if stack.len() != 1 {
        return Err(format!("<{}> is not closed", stack.last().map_or("", |e| e.name.as_str())));
    }
    let doc = stack.pop().expect("the document");
    doc.elements().next().cloned().ok_or_else(|| "the file has no root element".to_string())
}

/// Whether `text` is a Prism XML file.
pub fn looks_like_pzfx(text: &str) -> bool {
    let head: String = text.chars().take(4000).collect();
    head.contains("<GraphPadPrismFile")
}

/// What a `.pzfx` file gave.
#[derive(Debug, Clone)]
pub struct PzfxImport {
    pub workbook: Workbook,
    /// Table type and title of each sheet, for the import's summary.
    pub tables: Vec<(String, String)>,
    pub warnings: Vec<String>,
}

/// The name of subcolumn `i` (0-based) of `count` under a table's `YFormat`, and whether it is an error column.
fn role(y_format: &str, i: usize, count: usize) -> (String, bool) {
    let named: &[&str] = match y_format.to_ascii_lowercase().as_str() {
        "sdn" => &["Mean", "SD", "N"],
        "sen" => &["Mean", "SEM", "N"],
        "cvn" => &["Mean", "%CV", "N"],
        "sd" => &["Mean", "SD"],
        "se" => &["Mean", "SEM"],
        "cv" => &["Mean", "%CV"],
        "low-high" => &["Mean", "Plus error", "Minus error"],
        "upper-lower-limits" => &["Mean", "Upper limit", "Lower limit"],
        _ => &[],
    };
    match named.get(i) {
        Some(n) if named.len() == count => (n.to_string(), matches!(*n, "SD" | "SEM" | "%CV" | "Plus error")),
        _ if count == 1 => ("Value".to_string(), false),
        _ => (format!("Replicate {}", i + 1), false),
    }
}

/// The values of a `Subcolumn`: numbers (NaN when empty or not a number), the texts and the excluded rows.
fn subcolumn(e: &Element) -> (Vec<f64>, Vec<String>, Vec<usize>) {
    let (mut values, mut texts, mut excluded) = (Vec::new(), Vec::new(), Vec::new());
    for (r, d) in e.elements().filter(|d| d.name == "d").enumerate() {
        let t = d.text().trim().to_string();
        values.push(t.parse::<f64>().unwrap_or(f64::NAN));
        if d.attr("Excluded").is_some_and(|v| v == "1") {
            excluded.push(r);
        }
        texts.push(t);
    }
    (values, texts, excluded)
}

/// Reads the text of a `.pzfx` file; `file_name` titles the workbook.
pub fn read_pzfx(text: &str, file_name: &str) -> Result<PzfxImport, IoError> {
    let fail = |detail: String| IoError::NoData { source_name: file_name.to_string(), detail };
    let root = parse_xml(text).map_err(|e| fail(format!("the Prism XML could not be read: {e}")))?;
    if root.name != "GraphPadPrismFile" {
        return Err(fail(format!("the root element is <{}>, not <GraphPadPrismFile>", root.name)));
    }
    let mut sheets: Vec<Worksheet> = Vec::new();
    let mut tables = Vec::new();
    let mut warnings = Vec::new();
    for (ti, table) in root.elements().filter(|e| e.name == "Table" || e.name == "HugeTable").enumerate() {
        let title = table.child("Title").map(|t| t.text().trim().to_string()).filter(|t| !t.is_empty());
        let title = title.unwrap_or_else(|| format!("Data {}", ti + 1));
        let mut name = title.clone();
        let mut k = 2;
        while sheets.iter().any(|s| s.name == name) {
            name = format!("{title} ({k})");
            k += 1;
        }
        let mut sheet = Worksheet::new(name.clone());
        sheet.long_name.clone_from(&title);
        let y_format = table.attr("YFormat").unwrap_or("replicates");
        let push = |sheet: &mut Worksheet,
                    designation: Designation,
                    data: ColumnData,
                    long: &str,
                    comment: String,
                    params: Vec<(String, String)>| {
            let index = sheet.columns.len();
            let mut col = Column::new(ColumnId(index as u64 + 1), default_short_name(index), designation, data);
            col.long_name = long.to_string();
            col.comments = comment;
            for (k, v) in params {
                col.params.insert(k, v);
            }
            sheet.columns.push(col);
            index
        };
        if let Some(rt) = table.child("RowTitlesColumn")
            && let Some(sub) = rt.child("Subcolumn")
        {
            let (_, texts, _) = subcolumn(sub);
            if texts.iter().any(|t| !t.is_empty()) {
                push(&mut sheet, Designation::Label, ColumnData::text(texts), "Row title", String::new(), Vec::new());
            }
        }
        for xname in ["XColumn", "XAdvancedColumn"] {
            if let Some(x) = table.child(xname) {
                let xt = x.child("Title").map(|t| t.text().trim().to_string()).unwrap_or_default();
                let subs: Vec<&Element> = x.elements().filter(|e| e.name == "Subcolumn").collect();
                for (i, sub) in subs.iter().enumerate() {
                    let (values, texts, excluded) = subcolumn(sub);
                    let numeric = texts.iter().all(|t| t.is_empty() || t.parse::<f64>().is_ok());
                    let data = if numeric { ColumnData::numeric(values) } else { ColumnData::text(texts) };
                    let designation = if i == 0 { Designation::X } else { Designation::XError };
                    let long = if xt.is_empty() { "X".to_string() } else { xt.clone() };
                    let at = push(&mut sheet, designation, data, &long, String::new(), Vec::new());
                    sheet.columns[at].mask.extend(excluded);
                }
                break;
            }
        }
        for (yi, y) in table.elements().filter(|e| e.name == "YColumn").enumerate() {
            let group = y.child("Title").map(|t| t.text().trim().to_string()).filter(|t| !t.is_empty());
            let group = group.unwrap_or_else(|| format!("Data set {}", yi + 1));
            let subs: Vec<&Element> = y.elements().filter(|e| e.name == "Subcolumn").collect();
            for (i, sub) in subs.iter().enumerate() {
                let (values, texts, excluded) = subcolumn(sub);
                let bad = texts.iter().filter(|t| !t.is_empty() && t.parse::<f64>().is_err()).count();
                if bad > 0 {
                    warnings.push(format!("{name}, {group}: {bad} cell(s) are not numbers and were left empty"));
                }
                let (r, error) = role(y_format, i, subs.len());
                let designation = if error { Designation::YError } else { Designation::Y };
                let params = vec![("Group".to_string(), group.clone()), ("Role".to_string(), r.clone())];
                let at = push(&mut sheet, designation, ColumnData::numeric(values), &group, r, params);
                sheet.columns[at].mask.extend(excluded);
            }
        }
        if sheet.columns.is_empty() {
            warnings.push(format!("table \"{title}\" has no columns and was left out"));
            continue;
        }
        tables.push((table.attr("TableType").unwrap_or("XY").to_string(), name));
        sheets.push(sheet);
    }
    if sheets.is_empty() {
        return Err(fail("the file has no data tables".into()));
    }
    let mut workbook = Workbook::new(WindowId(0), "Book1");
    workbook.long_name = file_name.to_string();
    workbook.sheets = sheets;
    workbook.source = Some(ImportSource { path: file_name.to_string(), importer: "prism".to_string() });
    Ok(PzfxImport { workbook, tables, warnings })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<GraphPadPrismFile PrismXMLVersion="5.00">
<!-- a comment -->
<Created><OriginalVersion CreatedByProgram="GraphPad Prism" CreatedByVersion="9.0"/></Created>
<TableSequence><Ref ID="Table0" Selected="1"/><Ref ID="Table1"/></TableSequence>
<Table ID="Table0" XFormat="numbers" YFormat="replicates" Replicates="2" TableType="XY">
<Title>Dose &amp; response</Title>
<RowTitlesColumn Width="39"><Subcolumn><d>a</d><d>b</d><d>c</d></Subcolumn></RowTitlesColumn>
<XColumn Width="81" Subcolumns="1" Decimals="0"><Title>Dose</Title><Subcolumn><d>1</d><d>10</d><d>100</d></Subcolumn></XColumn>
<YColumn Width="211" Decimals="1" Subcolumns="2"><Title>Control</Title>
<Subcolumn><d>3.5</d><d Excluded="1">40</d><d>5</d></Subcolumn>
<Subcolumn><d>3.1</d><d>4.2</d><d/></Subcolumn>
</YColumn>
<YColumn Subcolumns="2"><Title>Treated</Title><Subcolumn><d>1</d><d>2</d><d>3</d></Subcolumn><Subcolumn><d>1.5</d><d>2.5</d><d>3.5</d></Subcolumn></YColumn>
</Table>
<Table ID="Table1" YFormat="SDN" TableType="OneWay">
<Title>Summary</Title>
<YColumn Subcolumns="3"><Title>WT</Title><Subcolumn><d>10</d></Subcolumn><Subcolumn><d>2</d></Subcolumn><Subcolumn><d>6</d></Subcolumn></YColumn>
</Table>
</GraphPadPrismFile>"#;

    #[test]
    fn tables_become_sheets_with_groups_replicates_and_masks() {
        assert!(looks_like_pzfx(SAMPLE));
        let r = read_pzfx(SAMPLE, "assay.pzfx").unwrap();
        let b = &r.workbook;
        assert_eq!(b.sheets.len(), 2);
        let s = &b.sheets[0];
        assert_eq!(s.name, "Dose & response");
        let names: Vec<(&str, &str)> = s.columns.iter().map(|c| (c.long_name.as_str(), c.comments.as_str())).collect();
        assert_eq!(
            names,
            [
                ("Row title", ""),
                ("Dose", ""),
                ("Control", "Replicate 1"),
                ("Control", "Replicate 2"),
                ("Treated", "Replicate 1"),
                ("Treated", "Replicate 2")
            ]
        );
        assert_eq!(s.columns[0].designation, Designation::Label);
        assert_eq!(s.columns[1].designation, Designation::X);
        assert_eq!(s.columns[1].numeric().unwrap(), [1.0, 10.0, 100.0]);
        let ctrl = &s.columns[2];
        assert_eq!(ctrl.numeric().unwrap(), [3.5, 40.0, 5.0]);
        assert!(ctrl.mask.contains(&1), "the excluded value is masked");
        assert!(s.columns[3].numeric().unwrap()[2].is_nan(), "an empty cell is missing");
        assert_eq!(ctrl.params.get("Group").map(String::as_str), Some("Control"));
        let sum = &b.sheets[1];
        let roles: Vec<(&str, Designation)> =
            sum.columns.iter().map(|c| (c.comments.as_str(), c.designation)).collect();
        assert_eq!(roles, [("Mean", Designation::Y), ("SD", Designation::YError), ("N", Designation::Y)]);
        assert_eq!(
            r.tables,
            [("XY".to_string(), "Dose & response".to_string()), ("OneWay".to_string(), "Summary".to_string())]
        );
        assert_eq!(b.source.as_ref().unwrap().importer, "prism");
    }

    #[test]
    fn broken_or_foreign_xml_is_refused() {
        assert!(read_pzfx("<GraphPadPrismFile><Table>", "x.pzfx").is_err());
        assert!(read_pzfx("<Other/>", "x.pzfx").is_err());
        assert!(read_pzfx("<GraphPadPrismFile></GraphPadPrismFile>", "x.pzfx").is_err(), "no tables");
        assert_eq!(unescape("a &lt;b&gt; &#65;&#x42; &unknown; &"), "a <b> AB &unknown; &");
        assert!(!looks_like_pzfx("x,y\n1,2\n"));
    }
}
