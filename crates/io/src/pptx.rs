//! Send Graphs to PowerPoint (T232): a PowerPoint presentation (`.pptx`, Office Open XML) with one
//! 16:9 slide per picture, the picture made as large as fits inside a margin of a quarter inch and
//! centred. Origin hands its graphs to a running PowerPoint; Sheaf writes the file, which PowerPoint
//! (and LibreOffice Impress) opens. The package holds the parts PowerPoint needs and no more: the
//! presentation, one slide master with one blank layout, a theme, the slides and their PNG pictures.

use std::io::{Cursor, Write};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

/// The slide size: 13.333 x 7.5 inches (PowerPoint's 16:9 "Widescreen"), in EMU (914400 an inch).
pub const SLIDE_EMU: (i64, i64) = (12_192_000, 6_858_000);
/// The margin kept free around a picture: a quarter inch.
pub const MARGIN_EMU: i64 = 228_600;

/// One slide: a PNG picture and the name shown for it (the graph's name).
#[derive(Clone, Debug, PartialEq)]
pub struct PptxSlide {
    pub png: Vec<u8>,
    pub name: String,
}

/// The pixel size of a PNG from its header, or `None` when the bytes are not a PNG.
pub fn png_size(png: &[u8]) -> Option<(u32, u32)> {
    const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    if png.len() < 24 || png[..8] != SIGNATURE || &png[12..16] != b"IHDR" {
        return None;
    }
    let w = u32::from_be_bytes(png[16..20].try_into().ok()?);
    let h = u32::from_be_bytes(png[20..24].try_into().ok()?);
    (w > 0 && h > 0).then_some((w, h))
}

/// Where a picture of `w` x `h` pixels goes on a slide: the offset and size in EMU, as large as fits
/// inside the margin with its proportions kept, centred.
pub fn fit(w: u32, h: u32) -> (i64, i64, i64, i64) {
    let (sw, sh) = SLIDE_EMU;
    let (room_w, room_h) = ((sw - 2 * MARGIN_EMU) as f64, (sh - 2 * MARGIN_EMU) as f64);
    let k = (room_w / f64::from(w)).min(room_h / f64::from(h));
    let (cx, cy) = ((f64::from(w) * k).round() as i64, (f64::from(h) * k).round() as i64);
    ((sw - cx) / 2, (sh - cy) / 2, cx, cy)
}

/// The bytes of a `.pptx` with one slide per entry of `slides`, in order. An error names a slide
/// whose picture is not a PNG, or says that there is no slide.
pub fn pptx_bytes(slides: &[PptxSlide]) -> Result<Vec<u8>, String> {
    if slides.is_empty() {
        return Err("there is no graph to send".to_string());
    }
    let sizes = slides
        .iter()
        .map(|s| png_size(&s.png).ok_or_else(|| format!("the picture of '{}' is not a PNG", s.name)))
        .collect::<Result<Vec<_>, _>>()?;
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let mut put = |name: &str, bytes: &[u8], deflate: bool| -> Result<(), String> {
        zip.start_file(name, if deflate { options } else { stored }).map_err(|e| e.to_string())?;
        zip.write_all(bytes).map_err(|e| e.to_string())
    };
    let n = slides.len();
    put("[Content_Types].xml", content_types(n).as_bytes(), true)?;
    put("_rels/.rels", ROOT_RELS.as_bytes(), true)?;
    put("docProps/app.xml", app_props(n).as_bytes(), true)?;
    put("docProps/core.xml", CORE_PROPS.as_bytes(), true)?;
    put("ppt/presentation.xml", presentation(n).as_bytes(), true)?;
    put("ppt/_rels/presentation.xml.rels", presentation_rels(n).as_bytes(), true)?;
    put("ppt/presProps.xml", PRES_PROPS.as_bytes(), true)?;
    put("ppt/viewProps.xml", VIEW_PROPS.as_bytes(), true)?;
    put("ppt/tableStyles.xml", TABLE_STYLES.as_bytes(), true)?;
    put("ppt/theme/theme1.xml", THEME.as_bytes(), true)?;
    put("ppt/slideMasters/slideMaster1.xml", SLIDE_MASTER.as_bytes(), true)?;
    put("ppt/slideMasters/_rels/slideMaster1.xml.rels", MASTER_RELS.as_bytes(), true)?;
    put("ppt/slideLayouts/slideLayout1.xml", SLIDE_LAYOUT.as_bytes(), true)?;
    put("ppt/slideLayouts/_rels/slideLayout1.xml.rels", LAYOUT_RELS.as_bytes(), true)?;
    for (i, (s, &(w, h))) in slides.iter().zip(&sizes).enumerate() {
        let k = i + 1;
        put(&format!("ppt/slides/slide{k}.xml"), slide(&s.name, fit(w, h)).as_bytes(), true)?;
        put(&format!("ppt/slides/_rels/slide{k}.xml.rels"), slide_rels(k).as_bytes(), true)?;
        // PNG data is already compressed.
        put(&format!("ppt/media/image{k}.png"), &s.png, false)?;
    }
    let cursor = zip.finish().map_err(|e| e.to_string())?;
    Ok(cursor.into_inner())
}

/// Text for an XML attribute.
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
const NS: &str = r#"xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main""#;
const REL_NS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
const OFFICE_REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

fn content_types(n: usize) -> String {
    let mut s = format!(
        r#"{XML}<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Default Extension="png" ContentType="image/png"/><Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/><Override PartName="/ppt/slideMasters/slideMaster1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml"/><Override PartName="/ppt/slideLayouts/slideLayout1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml"/><Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/><Override PartName="/ppt/presProps.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presProps+xml"/><Override PartName="/ppt/viewProps.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.viewProps+xml"/><Override PartName="/ppt/tableStyles.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.tableStyles+xml"/><Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/><Override PartName="/docProps/app.xml" ContentType="application/vnd.openxmlformats-officedocument.extended-properties+xml"/>"#
    );
    for k in 1..=n {
        s.push_str(&format!(
            r#"<Override PartName="/ppt/slides/slide{k}.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>"#
        ));
    }
    s.push_str("</Types>");
    s
}

const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml"/></Relationships>"#;

fn app_props(n: usize) -> String {
    format!(
        r#"{XML}<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"><Application>Sheaf</Application><Slides>{n}</Slides></Properties>"#
    )
}

const CORE_PROPS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>Graphs</dc:title><dc:creator>Sheaf</dc:creator></cp:coreProperties>"#;

// Relationship ids of presentation.xml: rId1 the master, rId2.. the slides, then the property parts
// and the theme.
fn presentation(n: usize) -> String {
    let (sw, sh) = SLIDE_EMU;
    let slides: String = (1..=n).map(|k| format!(r#"<p:sldId id="{}" r:id="rId{}"/>"#, 255 + k, k + 1)).collect();
    format!(
        r#"{XML}<p:presentation {NS} saveSubsetFonts="1"><p:sldMasterIdLst><p:sldMasterId id="2147483648" r:id="rId1"/></p:sldMasterIdLst><p:sldIdLst>{slides}</p:sldIdLst><p:sldSz cx="{sw}" cy="{sh}"/><p:notesSz cx="6858000" cy="9144000"/><p:defaultTextStyle><a:defPPr><a:defRPr lang="en-US"/></a:defPPr></p:defaultTextStyle></p:presentation>"#
    )
}

fn presentation_rels(n: usize) -> String {
    let mut s = format!(
        r#"{XML}<Relationships xmlns="{REL_NS}"><Relationship Id="rId1" Type="{OFFICE_REL}/slideMaster" Target="slideMasters/slideMaster1.xml"/>"#
    );
    for k in 1..=n {
        s.push_str(&format!(
            r#"<Relationship Id="rId{}" Type="{OFFICE_REL}/slide" Target="slides/slide{k}.xml"/>"#,
            k + 1
        ));
    }
    let next = n + 2;
    s.push_str(&format!(
        r#"<Relationship Id="rId{}" Type="{OFFICE_REL}/presProps" Target="presProps.xml"/><Relationship Id="rId{}" Type="{OFFICE_REL}/viewProps" Target="viewProps.xml"/><Relationship Id="rId{}" Type="{OFFICE_REL}/theme" Target="theme/theme1.xml"/><Relationship Id="rId{}" Type="{OFFICE_REL}/tableStyles" Target="tableStyles.xml"/></Relationships>"#,
        next,
        next + 1,
        next + 2,
        next + 3
    ));
    s
}

const PRES_PROPS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:presentationPr xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"/>"#;

const VIEW_PROPS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:viewPr xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:normalViewPr><p:restoredLeft sz="15620"/><p:restoredTop sz="94660"/></p:normalViewPr><p:gridSpacing cx="76200" cy="76200"/></p:viewPr>"#;

const TABLE_STYLES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><a:tblStyleLst xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" def="{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}"/>"#;

const EMPTY_TREE: &str = r#"<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>"#;

const SLIDE_MASTER: &str = concat!(
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:bg><p:bgRef idx="1001"><a:schemeClr val="bg1"/></p:bgRef></p:bg><p:spTree>"#,
    r#"<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>"#,
    r#"</p:spTree></p:cSld><p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/><p:sldLayoutIdLst><p:sldLayoutId id="2147483649" r:id="rId1"/></p:sldLayoutIdLst><p:txStyles><p:titleStyle><a:lvl1pPr><a:defRPr sz="4400"/></a:lvl1pPr></p:titleStyle><p:bodyStyle><a:lvl1pPr><a:defRPr sz="2800"/></a:lvl1pPr></p:bodyStyle><p:otherStyle><a:lvl1pPr><a:defRPr sz="1800"/></a:lvl1pPr></p:otherStyle></p:txStyles></p:sldMaster>"#
);

const MASTER_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="../theme/theme1.xml"/></Relationships>"#;

const SLIDE_LAYOUT: &str = concat!(
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" type="blank" preserve="1"><p:cSld name="Blank"><p:spTree>"#,
    r#"<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>"#,
    r#"</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sldLayout>"#
);

const LAYOUT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="../slideMasters/slideMaster1.xml"/></Relationships>"#;

fn slide(name: &str, (x, y, cx, cy): (i64, i64, i64, i64)) -> String {
    let name = escape(name);
    format!(
        r#"{XML}<p:sld {NS}><p:cSld><p:spTree>{EMPTY_TREE}<p:pic><p:nvPicPr><p:cNvPr id="2" name="{name}" descr="{name}"/><p:cNvPicPr><a:picLocks noChangeAspect="1"/></p:cNvPicPr><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed="rId2"/><a:stretch><a:fillRect/></a:stretch></p:blipFill><p:spPr><a:xfrm><a:off x="{x}" y="{y}"/><a:ext cx="{cx}" cy="{cy}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr></p:pic></p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>"#
    )
}

fn slide_rels(k: usize) -> String {
    format!(
        r#"{XML}<Relationships xmlns="{REL_NS}"><Relationship Id="rId1" Type="{OFFICE_REL}/slideLayout" Target="../slideLayouts/slideLayout1.xml"/><Relationship Id="rId2" Type="{OFFICE_REL}/image" Target="../media/image{k}.png"/></Relationships>"#
    )
}

/// The Office theme, cut to what a theme part must hold: colours, fonts and the format scheme with
/// the three entries each list needs.
const THEME: &str = concat!(
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Office Theme"><a:themeElements>"#,
    r#"<a:clrScheme name="Office"><a:dk1><a:sysClr val="windowText" lastClr="000000"/></a:dk1><a:lt1><a:sysClr val="window" lastClr="FFFFFF"/></a:lt1><a:dk2><a:srgbClr val="44546A"/></a:dk2><a:lt2><a:srgbClr val="E7E6E6"/></a:lt2><a:accent1><a:srgbClr val="4472C4"/></a:accent1><a:accent2><a:srgbClr val="ED7D31"/></a:accent2><a:accent3><a:srgbClr val="A5A5A5"/></a:accent3><a:accent4><a:srgbClr val="FFC000"/></a:accent4><a:accent5><a:srgbClr val="5B9BD5"/></a:accent5><a:accent6><a:srgbClr val="70AD47"/></a:accent6><a:hlink><a:srgbClr val="0563C1"/></a:hlink><a:folHlink><a:srgbClr val="954F72"/></a:folHlink></a:clrScheme>"#,
    r#"<a:fontScheme name="Office"><a:majorFont><a:latin typeface="Calibri Light"/><a:ea typeface=""/><a:cs typeface=""/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/><a:ea typeface=""/><a:cs typeface=""/></a:minorFont></a:fontScheme>"#,
    r#"<a:fmtScheme name="Office"><a:fillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:fillStyleLst>"#,
    r#"<a:lnStyleLst><a:ln w="6350"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln><a:ln w="12700"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln><a:ln w="19050"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln></a:lnStyleLst>"#,
    r#"<a:effectStyleLst><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle></a:effectStyleLst>"#,
    r#"<a:bgFillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:bgFillStyleLst></a:fmtScheme>"#,
    r#"</a:themeElements><a:objectDefaults/><a:extraClrSchemeLst/></a:theme>"#
);

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    /// A 1 x 1 PNG header with the given size (enough for `png_size`; PowerPoint is not asked to read it).
    fn png(w: u32, h: u32) -> Vec<u8> {
        let mut v = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13];
        v.extend_from_slice(b"IHDR");
        v.extend_from_slice(&w.to_be_bytes());
        v.extend_from_slice(&h.to_be_bytes());
        v.extend_from_slice(&[8, 6, 0, 0, 0]);
        v
    }

    #[test]
    fn a_picture_fits_inside_the_margin_and_is_centred_keeping_its_proportions() {
        // A wide picture fills the width between the margins.
        let (x, y, cx, cy) = fit(2000, 500);
        assert_eq!((x, cx), (MARGIN_EMU, SLIDE_EMU.0 - 2 * MARGIN_EMU));
        assert_eq!(cy, (cx as f64 / 4.0).round() as i64);
        assert_eq!(y, (SLIDE_EMU.1 - cy) / 2);
        // A tall one fills the height.
        let (x, y, cx, cy) = fit(600, 800);
        assert_eq!((y, cy), (MARGIN_EMU, SLIDE_EMU.1 - 2 * MARGIN_EMU));
        assert_eq!(x, (SLIDE_EMU.0 - cx) / 2);
    }

    #[test]
    fn the_package_holds_one_slide_and_picture_per_graph_with_escaped_names() {
        let slides = vec![
            PptxSlide { png: png(800, 600), name: "Graph1".into() },
            PptxSlide { png: png(400, 400), name: "A & <B>".into() },
        ];
        let bytes = pptx_bytes(&slides).unwrap();
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        let mut text = |name: &str| {
            let mut s = String::new();
            zip.by_name(name).unwrap().read_to_string(&mut s).unwrap();
            s
        };
        let types = text("[Content_Types].xml");
        assert!(types.contains("/ppt/slides/slide2.xml") && !types.contains("slide3.xml"));
        let pres = text("ppt/presentation.xml");
        assert_eq!(pres.matches("<p:sldId ").count(), 2);
        assert!(pres.contains(r#"<p:sldSz cx="12192000" cy="6858000"/>"#));
        let rels = text("ppt/_rels/presentation.xml.rels");
        assert!(rels.contains(r#"Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide2.xml""#));
        assert!(text("ppt/slides/slide2.xml").contains(r#"name="A &amp; &lt;B&gt;""#));
        assert!(text("ppt/slides/_rels/slide1.xml.rels").contains("../media/image1.png"));
        assert_eq!(zip.by_name("ppt/media/image2.png").unwrap().size(), slides[1].png.len() as u64);
    }

    #[test]
    fn no_slides_or_a_picture_that_is_not_a_png_is_an_error() {
        assert!(pptx_bytes(&[]).is_err());
        let bad = [PptxSlide { png: b"GIF89a".to_vec(), name: "G".into() }];
        assert_eq!(pptx_bytes(&bad).unwrap_err(), "the picture of 'G' is not a PNG");
        assert_eq!(png_size(&png(3, 4)), Some((3, 4)));
    }
}
