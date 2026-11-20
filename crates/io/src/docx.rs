//! Send Graphs to Word (T242): a Word document (`.docx`, Office Open XML) with one graph per page, the
//! graph as a PNG picture as large as fits the text area of a Letter page with one-inch margins (keeping
//! its proportions, centred), and the graph's name in a paragraph under it. Origin hands its graphs to a
//! running Word; Sheaf writes the file, which Word (and LibreOffice Writer) opens. The package holds the
//! document, its relationships and the pictures, and no more.

use crate::pptx::png_size;
use std::io::{Cursor, Write};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

/// The text area of a US Letter page with one-inch margins, in EMU (914400 an inch): 6.5 x 9 inches,
/// less half an inch kept for the caption.
pub const TEXT_EMU: (i64, i64) = (5_943_600, 7_772_400);

/// One graph: a PNG picture and its name (the caption).
#[derive(Clone, Debug, PartialEq)]
pub struct DocxPicture {
    pub png: Vec<u8>,
    pub name: String,
}

/// The size in EMU of a picture of `w` x `h` pixels: as large as fits [`TEXT_EMU`], proportions kept.
pub fn fit(w: u32, h: u32) -> (i64, i64) {
    let k = (TEXT_EMU.0 as f64 / f64::from(w)).min(TEXT_EMU.1 as f64 / f64::from(h));
    ((f64::from(w) * k).round() as i64, (f64::from(h) * k).round() as i64)
}

/// The bytes of a `.docx` with the pictures in order, one per page. An error names a picture that is
/// not a PNG, or says that there is none.
pub fn docx_bytes(pictures: &[DocxPicture]) -> Result<Vec<u8>, String> {
    if pictures.is_empty() {
        return Err("there is no graph to send".to_string());
    }
    let sizes = pictures
        .iter()
        .map(|p| png_size(&p.png).ok_or_else(|| format!("the picture of '{}' is not a PNG", p.name)))
        .collect::<Result<Vec<_>, _>>()?;
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let mut put = |name: &str, bytes: &[u8], deflate: bool| -> Result<(), String> {
        zip.start_file(name, if deflate { deflated } else { stored }).map_err(|e| e.to_string())?;
        zip.write_all(bytes).map_err(|e| e.to_string())
    };
    put("[Content_Types].xml", CONTENT_TYPES.as_bytes(), true)?;
    put("_rels/.rels", ROOT_RELS.as_bytes(), true)?;
    put("docProps/core.xml", CORE_PROPS.as_bytes(), true)?;
    put("docProps/app.xml", APP_PROPS.as_bytes(), true)?;
    put("word/document.xml", document(pictures, &sizes).as_bytes(), true)?;
    put("word/_rels/document.xml.rels", document_rels(pictures.len()).as_bytes(), true)?;
    for (i, p) in pictures.iter().enumerate() {
        // PNG data is already compressed.
        put(&format!("word/media/image{}.png", i + 1), &p.png, false)?;
    }
    let cursor = zip.finish().map_err(|e| e.to_string())?;
    Ok(cursor.into_inner())
}

/// Text for XML.
fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c if (c as u32) < 0x20 && !matches!(c, '\t' | '\n' | '\r') => {}
            c => out.push(c),
        }
    }
    out
}

const XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#;

const CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Default Extension="png" ContentType="image/png"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/><Override PartName="/docProps/app.xml" ContentType="application/vnd.openxmlformats-officedocument.extended-properties+xml"/></Types>"#;

const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml"/></Relationships>"#;

const CORE_PROPS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>Graphs</dc:title><dc:creator>Sheaf</dc:creator></cp:coreProperties>"#;

const APP_PROPS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"><Application>Sheaf</Application></Properties>"#;

fn document(pictures: &[DocxPicture], sizes: &[(u32, u32)]) -> String {
    let mut body = String::new();
    for (i, (p, &(w, h))) in pictures.iter().zip(sizes).enumerate() {
        let k = i + 1;
        let (cx, cy) = fit(w, h);
        let name = escape(&p.name);
        // The picture, centred, in a paragraph of its own; the caption under it; a page break before
        // every graph but the first.
        let brk = if i > 0 { r#"<w:r><w:br w:type="page"/></w:r>"# } else { "" };
        body.push_str(&format!(
            r#"<w:p><w:pPr><w:jc w:val="center"/></w:pPr>{brk}<w:r><w:drawing><wp:inline distT="0" distB="0" distL="0" distR="0"><wp:extent cx="{cx}" cy="{cy}"/><wp:docPr id="{k}" name="{name}" descr="{name}"/><wp:cNvGraphicFramePr><a:graphicFrameLocks noChangeAspect="1"/></wp:cNvGraphicFramePr><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:pic><pic:nvPicPr><pic:cNvPr id="{k}" name="image{k}.png"/><pic:cNvPicPr/></pic:nvPicPr><pic:blipFill><a:blip r:embed="rId{k}"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill><pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{cx}" cy="{cy}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p><w:p><w:pPr><w:jc w:val="center"/></w:pPr><w:r><w:t xml:space="preserve">{name}</w:t></w:r></w:p>"#
        ));
    }
    format!(
        r#"{XML}<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"><w:body>{body}<w:sectPr><w:pgSz w:w="12240" w:h="15840"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="720" w:footer="720" w:gutter="0"/></w:sectPr></w:body></w:document>"#
    )
}

fn document_rels(n: usize) -> String {
    let mut s = format!(r#"{XML}<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#);
    for k in 1..=n {
        s.push_str(&format!(
            r#"<Relationship Id="rId{k}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image{k}.png"/>"#
        ));
    }
    s.push_str("</Relationships>");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    /// The 24 header bytes `png_size` reads.
    fn png(w: u32, h: u32) -> Vec<u8> {
        let mut v = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13];
        v.extend_from_slice(b"IHDR");
        v.extend_from_slice(&w.to_be_bytes());
        v.extend_from_slice(&h.to_be_bytes());
        v
    }

    #[test]
    fn a_picture_fills_the_text_width_or_height_keeping_its_proportions() {
        assert_eq!(fit(2000, 1000), (TEXT_EMU.0, TEXT_EMU.0 / 2));
        let (cx, cy) = fit(500, 1000);
        assert_eq!((cx, cy), (TEXT_EMU.1 / 2, TEXT_EMU.1));
    }

    #[test]
    fn one_picture_per_page_with_its_name_under_it() {
        let pics = vec![
            DocxPicture { png: png(800, 600), name: "Graph1".into() },
            DocxPicture { png: png(600, 800), name: "B & <C>".into() },
        ];
        let mut zip = zip::ZipArchive::new(Cursor::new(docx_bytes(&pics).unwrap())).unwrap();
        let mut text = |name: &str| {
            let mut s = String::new();
            zip.by_name(name).unwrap().read_to_string(&mut s).unwrap();
            s
        };
        let doc = text("word/document.xml");
        assert_eq!(doc.matches("<wp:inline ").count(), 2);
        assert_eq!(doc.matches(r#"<w:br w:type="page"/>"#).count(), 1, "a break between the two");
        assert!(doc.contains(r#"r:embed="rId2""#) && doc.contains("B &amp; &lt;C&gt;"));
        assert!(text("word/_rels/document.xml.rels").contains(r#"Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image2.png""#));
        assert!(zip.by_name("word/media/image1.png").is_ok());
    }

    #[test]
    fn nothing_to_send_or_not_a_png_is_an_error() {
        assert!(docx_bytes(&[]).is_err());
        let bad = [DocxPicture { png: vec![1, 2, 3], name: "G".into() }];
        assert_eq!(docx_bytes(&bad).unwrap_err(), "the picture of 'G' is not a PNG");
    }
}
