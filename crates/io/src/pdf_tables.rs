//! File > Import > PDF Tables (T463; Origin 2021's Import PDF Tables app, Sheaf's own reader): the text of a PDF read
//! into worksheets, one sheet per page that holds text, laid out as a table.
//!
//! The reader is a small one of Sheaf's own (no PDF library): it finds the file's objects by their `N G obj` headers
//! (and inside compressed object streams), inflates `FlateDecode` streams, walks the page tree and runs the text
//! operators of each page's content (`BT`, `Tf`, `Td`, `TD`, `Tm`, `T*`, `TL`, `Tc`, `Tw`, `Tj`, `TJ`, `'`, `"`, with
//! `q`, `Q` and `cm`). Strings are decoded through the font's `ToUnicode` map when it has one, else its simple
//! encoding (WinAnsi, Standard or MacRoman with `Differences`). Glyph advances come from the font's `Widths` (half
//! an em when it has none).
//!
//! The table: text runs whose baselines are within a third of the font size are one line, lines top to bottom are
//! rows; runs of a line closer than 0.7 em join into one cell; a column is a band of the page across which cells
//! overlap, so left-, right- and centre-aligned columns all line up. A column whose filled cells are all numbers is
//! numeric. Scanned pages (pictures of text), encrypted files, rotated text and cells that span columns are not
//! read as tables.

use crate::IoError;
use sheaf_core::column::{Column, ColumnData, Designation, default_short_name};
use sheaf_core::ids::{ColumnId, WindowId};
use sheaf_core::worksheet::{ImportSource, Workbook, Worksheet};
use std::collections::HashMap;
use std::io::Read;

/// True for the bytes of a PDF file.
pub fn looks_like_pdf(bytes: &[u8]) -> bool {
    bytes.len() > 8
        && bytes[..1024.min(bytes.len())].windows(5).any(|w| w == b"%PDF-")
        && find(bytes, b" obj", 0)
            .or_else(|| {
                find(
                    bytes, b"
obj", 0,
                )
            })
            .is_some()
}

/// What the PDF import read.
pub struct PdfImport {
    pub workbook: Workbook,
    /// Pages read, pages with text.
    pub pages: (usize, usize),
    pub warnings: Vec<String>,
}

// ----- objects -----------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
enum Obj {
    Null,
    Bool(bool),
    Num(f64),
    Str(Vec<u8>),
    Name(String),
    Array(Vec<Obj>),
    Dict(Dict),
    Ref(u32),
    Stream(Dict, Vec<u8>),
    /// A content stream operator.
    Op(String),
}

type Dict = HashMap<String, Obj>;

impl Obj {
    fn num(&self) -> Option<f64> {
        if let Obj::Num(n) = self { Some(*n) } else { None }
    }
    fn name(&self) -> Option<&str> {
        if let Obj::Name(n) = self { Some(n) } else { None }
    }
    fn dict(&self) -> Option<&Dict> {
        match self {
            Obj::Dict(d) | Obj::Stream(d, _) => Some(d),
            _ => None,
        }
    }
}

fn is_ws(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\r' | b'\n' | b'\x0c' | b'\0')
}

fn is_delim(c: u8) -> bool {
    matches!(c, b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%')
}

struct Lexer<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Lexer<'a> {
    fn new(b: &'a [u8], i: usize) -> Self {
        Lexer { b, i }
    }

    fn skip(&mut self) {
        while self.i < self.b.len() {
            let c = self.b[self.i];
            if is_ws(c) {
                self.i += 1;
            } else if c == b'%' {
                while self.i < self.b.len() && !matches!(self.b[self.i], b'\r' | b'\n') {
                    self.i += 1;
                }
            } else {
                break;
            }
        }
    }

    fn word(&mut self) -> &'a [u8] {
        let s = self.i;
        while self.i < self.b.len() && !is_ws(self.b[self.i]) && !is_delim(self.b[self.i]) {
            self.i += 1;
        }
        &self.b[s..self.i]
    }

    /// The next object; numbers followed by `G R` are references. `None` at the end or before `]` / `>>`.
    fn obj(&mut self) -> Option<Obj> {
        self.skip();
        let c = *self.b.get(self.i)?;
        match c {
            b'/' => {
                self.i += 1;
                let w = self.word();
                Some(Obj::Name(decode_name(w)))
            }
            b'(' => Some(Obj::Str(self.literal())),
            b'<' if self.b.get(self.i + 1) == Some(&b'<') => {
                self.i += 2;
                let mut d = Dict::new();
                loop {
                    self.skip();
                    if self.b.get(self.i..self.i + 2) == Some(b">>") {
                        self.i += 2;
                        break;
                    }
                    let key = match self.obj()? {
                        Obj::Name(n) => n,
                        _ => return None,
                    };
                    let v = self.obj().unwrap_or(Obj::Null);
                    d.insert(key, v);
                }
                Some(Obj::Dict(d))
            }
            b'<' => {
                self.i += 1;
                let s = self.i;
                while self.i < self.b.len() && self.b[self.i] != b'>' {
                    self.i += 1;
                }
                let hex: Vec<u8> = self.b[s..self.i].iter().copied().filter(|c| c.is_ascii_hexdigit()).collect();
                self.i += 1;
                Some(Obj::Str(hex_bytes(&hex)))
            }
            b'[' => {
                self.i += 1;
                let mut v = Vec::new();
                loop {
                    self.skip();
                    if self.b.get(self.i) == Some(&b']') {
                        self.i += 1;
                        break;
                    }
                    match self.obj() {
                        Some(o) => v.push(o),
                        None => break,
                    }
                }
                Some(Obj::Array(v))
            }
            b']' | b'>' | b')' | b'{' | b'}' => {
                // Stray delimiters: step over them.
                self.i += 1;
                Some(Obj::Op(String::new()))
            }
            _ => {
                let w = self.word();
                if w.is_empty() {
                    self.i += 1;
                    return Some(Obj::Op(String::new()));
                }
                let s = String::from_utf8_lossy(w).into_owned();
                if let Ok(n) = s.parse::<f64>() {
                    // `N G R` is a reference.
                    let save = self.i;
                    self.skip();
                    let g = self.word();
                    if !g.is_empty() && g.iter().all(u8::is_ascii_digit) {
                        self.skip();
                        if self.b.get(self.i) == Some(&b'R')
                            && self.b.get(self.i + 1).is_none_or(|c| is_ws(*c) || is_delim(*c))
                        {
                            self.i += 1;
                            return Some(Obj::Ref(n as u32));
                        }
                    }
                    self.i = save;
                    return Some(Obj::Num(n));
                }
                Some(match s.as_str() {
                    "true" => Obj::Bool(true),
                    "false" => Obj::Bool(false),
                    "null" => Obj::Null,
                    _ => Obj::Op(s),
                })
            }
        }
    }

    fn literal(&mut self) -> Vec<u8> {
        self.i += 1;
        let mut out = Vec::new();
        let mut depth = 1;
        while self.i < self.b.len() {
            let c = self.b[self.i];
            self.i += 1;
            match c {
                b'\\' => {
                    let Some(&n) = self.b.get(self.i) else { break };
                    self.i += 1;
                    match n {
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'b' => out.push(8),
                        b'f' => out.push(12),
                        b'\r' => {
                            if self.b.get(self.i) == Some(&b'\n') {
                                self.i += 1;
                            }
                        }
                        b'\n' => {}
                        b'0'..=b'7' => {
                            let mut v = u32::from(n - b'0');
                            for _ in 0..2 {
                                match self.b.get(self.i) {
                                    Some(&d @ b'0'..=b'7') => {
                                        v = v * 8 + u32::from(d - b'0');
                                        self.i += 1;
                                    }
                                    _ => break,
                                }
                            }
                            out.push(v as u8);
                        }
                        other => out.push(other),
                    }
                }
                b'(' => {
                    depth += 1;
                    out.push(c);
                }
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    out.push(c);
                }
                _ => out.push(c),
            }
        }
        out
    }
}

fn decode_name(w: &[u8]) -> String {
    let mut out = Vec::new();
    let mut i = 0;
    while i < w.len() {
        if w[i] == b'#'
            && i + 2 < w.len() + 1
            && w.get(i + 1..i + 3).is_some_and(|h| h.iter().all(u8::is_ascii_hexdigit))
        {
            out.extend(hex_bytes(&w[i + 1..i + 3]));
            i += 3;
        } else {
            out.push(w[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_bytes(hex: &[u8]) -> Vec<u8> {
    let val = |c: u8| (c as char).to_digit(16).unwrap_or(0) as u8;
    hex.chunks(2).map(|p| (val(p[0]) << 4) | p.get(1).map_or(0, |c| val(*c))).collect()
}

/// The file's objects by number (the last definition wins), with object streams opened.
struct Doc {
    objects: HashMap<u32, Obj>,
    warnings: Vec<String>,
}

impl Doc {
    fn parse(b: &[u8]) -> Doc {
        let mut objects = HashMap::new();
        let mut i = 0;
        while let Some(p) = find(b, b"obj", i) {
            i = p + 3;
            // `N G obj`: two integers before, the keyword standing alone.
            if b.get(p + 3).is_some_and(|c| !is_ws(*c) && !is_delim(*c)) || (p > 0 && !is_ws(b[p - 1])) {
                continue;
            }
            let Some((num, _)) = header(b, p) else { continue };
            let mut lx = Lexer::new(b, p + 3);
            let Some(o) = lx.obj() else { continue };
            let o = match o {
                Obj::Dict(d) => {
                    lx.skip();
                    if b.get(lx.i..lx.i + 6) == Some(b"stream") {
                        let mut s = lx.i + 6;
                        if b.get(s) == Some(&b'\r') {
                            s += 1;
                        }
                        if b.get(s) == Some(&b'\n') {
                            s += 1;
                        }
                        let len = d.get("Length").and_then(Obj::num).map(|n| n as usize);
                        let end = match len {
                            Some(l)
                                if s + l <= b.len() && find(b, b"endstream", s + l).is_some_and(|e| e < s + l + 20) =>
                            {
                                s + l
                            }
                            _ => {
                                let e = find(b, b"endstream", s).unwrap_or(b.len());
                                let mut e2 = e;
                                while e2 > s && matches!(b[e2 - 1], b'\r' | b'\n') {
                                    e2 -= 1;
                                }
                                e2
                            }
                        };
                        i = end;
                        Obj::Stream(d, b[s..end].to_vec())
                    } else {
                        Obj::Dict(d)
                    }
                }
                other => other,
            };
            objects.insert(num, o);
        }
        let mut doc = Doc { objects, warnings: Vec::new() };
        doc.open_object_streams();
        doc
    }

    fn open_object_streams(&mut self) {
        let streams: Vec<(Dict, Vec<u8>)> = self
            .objects
            .values()
            .filter_map(|o| match o {
                Obj::Stream(d, data) if d.get("Type").and_then(Obj::name) == Some("ObjStm") => {
                    Some((d.clone(), data.clone()))
                }
                _ => None,
            })
            .collect();
        for (d, data) in streams {
            let Some(raw) = self.decode(&d, &data) else { continue };
            let n = d.get("N").and_then(Obj::num).unwrap_or(0.0) as usize;
            let first = d.get("First").and_then(Obj::num).unwrap_or(0.0) as usize;
            let mut lx = Lexer::new(&raw, 0);
            let mut heads = Vec::new();
            for _ in 0..n {
                match (lx.obj(), lx.obj()) {
                    (Some(Obj::Num(num)), Some(Obj::Num(off))) => heads.push((num as u32, off as usize)),
                    _ => break,
                }
            }
            for (num, off) in heads {
                let mut lx = Lexer::new(&raw, first + off);
                if let Some(o) = lx.obj() {
                    self.objects.entry(num).or_insert(o);
                }
            }
        }
    }

    fn get<'s>(&'s self, o: &'s Obj) -> &'s Obj {
        let mut o = o;
        for _ in 0..16 {
            match o {
                Obj::Ref(n) => o = self.objects.get(n).unwrap_or(&Obj::Null),
                _ => return o,
            }
        }
        &Obj::Null
    }

    fn key<'s>(&'s self, d: &'s Dict, k: &str) -> &'s Obj {
        d.get(k).map_or(&Obj::Null, |o| self.get(o))
    }

    /// A stream's data after its filters, or `None` for a filter Sheaf does not read.
    fn decode(&self, d: &Dict, data: &[u8]) -> Option<Vec<u8>> {
        let filters: Vec<String> = match self.key(d, "Filter") {
            Obj::Name(n) => vec![n.clone()],
            Obj::Array(v) => v.iter().filter_map(|o| self.get(o).name().map(str::to_string)).collect(),
            _ => Vec::new(),
        };
        let mut out = data.to_vec();
        for f in filters {
            out = match f.as_str() {
                "FlateDecode" | "Fl" => {
                    let mut z = flate2::read::ZlibDecoder::new(&out[..]);
                    let mut v = Vec::new();
                    if z.read_to_end(&mut v).is_err() && v.is_empty() {
                        return None;
                    }
                    v
                }
                "ASCIIHexDecode" | "AHx" => {
                    let hex: Vec<u8> =
                        out.iter().copied().take_while(|c| *c != b'>').filter(u8::is_ascii_hexdigit).collect();
                    hex_bytes(&hex)
                }
                _ => return None,
            };
        }
        Some(out)
    }
}

fn find(b: &[u8], pat: &[u8], from: usize) -> Option<usize> {
    if from >= b.len() {
        return None;
    }
    b[from..].windows(pat.len()).position(|w| w == pat).map(|p| p + from)
}

/// The object number of the `N G obj` header ending at `obj_at`.
fn header(b: &[u8], obj_at: usize) -> Option<(u32, u16)> {
    let mut j = obj_at;
    let back_word = |j: &mut usize| -> Option<String> {
        while *j > 0 && is_ws(b[*j - 1]) {
            *j -= 1;
        }
        let e = *j;
        while *j > 0 && b[*j - 1].is_ascii_digit() {
            *j -= 1;
        }
        (e > *j).then(|| String::from_utf8_lossy(&b[*j..e]).into_owned())
    };
    let g = back_word(&mut j)?;
    let n = back_word(&mut j)?;
    Some((n.parse().ok()?, g.parse().ok()?))
}

// ----- fonts -------------------------------------------------------------------------------------

struct Font {
    /// Code to text through the font's ToUnicode map.
    to_unicode: HashMap<u32, String>,
    /// Bytes per code (2 for composite fonts).
    code_len: usize,
    /// A simple font's code to character.
    encoding: Vec<Option<char>>,
    first_char: u32,
    widths: Vec<f64>,
}

impl Font {
    fn read(doc: &Doc, d: &Dict) -> Font {
        let subtype = doc.key(d, "Subtype").name().unwrap_or("").to_string();
        let composite = subtype == "Type0";
        let mut to_unicode = HashMap::new();
        let mut code_len = if composite { 2 } else { 1 };
        if let Obj::Stream(sd, data) = doc.key(d, "ToUnicode")
            && let Some(raw) = doc.decode(sd, data)
        {
            let (map, len) = parse_cmap(&raw);
            to_unicode = map;
            if let Some(l) = len {
                code_len = l;
            }
        }
        let mut encoding: Vec<Option<char>> = (0..256u32).map(|c| win_ansi(c as u8)).collect();
        match doc.key(d, "Encoding") {
            Obj::Name(n) => apply_base(&mut encoding, n),
            Obj::Dict(e) => {
                if let Some(n) = doc.key(e, "BaseEncoding").name() {
                    apply_base(&mut encoding, n);
                }
                if let Obj::Array(diffs) = doc.key(e, "Differences") {
                    let mut code = 0usize;
                    for o in diffs {
                        match doc.get(o) {
                            Obj::Num(n) => code = *n as usize,
                            Obj::Name(g) => {
                                if code < 256 {
                                    encoding[code] = glyph_char(g);
                                }
                                code += 1;
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
        let first_char = doc.key(d, "FirstChar").num().unwrap_or(0.0) as u32;
        let widths = match doc.key(d, "Widths") {
            Obj::Array(v) => v.iter().map(|o| doc.get(o).num().unwrap_or(500.0)).collect(),
            _ => Vec::new(),
        };
        Font { to_unicode, code_len, encoding, first_char, widths }
    }

    /// The codes of a string, each with its text.
    fn codes(&self, s: &[u8]) -> Vec<(u32, String)> {
        s.chunks(self.code_len)
            .map(|c| {
                let code = c.iter().fold(0u32, |a, b| (a << 8) | u32::from(*b));
                let text = match self.to_unicode.get(&code) {
                    Some(t) => t.clone(),
                    None if self.code_len == 1 => self.encoding[code as usize].map(String::from).unwrap_or_default(),
                    None => String::new(),
                };
                (code, text)
            })
            .collect()
    }

    /// The advance of a code in thousandths of the font size.
    fn width(&self, code: u32) -> f64 {
        code.checked_sub(self.first_char).and_then(|i| self.widths.get(i as usize)).copied().unwrap_or(500.0)
    }
}

/// A ToUnicode CMap: code to text, and the code length from its codespace range.
fn parse_cmap(raw: &[u8]) -> (HashMap<u32, String>, Option<usize>) {
    let mut lx = Lexer::new(raw, 0);
    let mut toks = Vec::new();
    while let Some(o) = lx.obj() {
        toks.push(o);
        if lx.i >= raw.len() {
            break;
        }
    }
    let code = |s: &[u8]| s.iter().fold(0u32, |a, b| (a << 8) | u32::from(*b));
    let utf16 = |s: &[u8]| {
        let units: Vec<u16> =
            s.chunks(2).map(|p| (u16::from(p[0]) << 8) | u16::from(*p.get(1).unwrap_or(&0))).collect();
        String::from_utf16_lossy(&units)
    };
    let mut map = HashMap::new();
    let mut len = None;
    let mut i = 0;
    while i < toks.len() {
        match &toks[i] {
            Obj::Op(op) if op == "begincodespacerange" => {
                if let Some(Obj::Str(lo)) = toks.get(i + 1) {
                    len = Some(lo.len().max(1));
                }
            }
            Obj::Op(op) if op == "beginbfchar" => {
                let mut k = i + 1;
                while let (Some(Obj::Str(src)), Some(Obj::Str(dst))) = (toks.get(k), toks.get(k + 1)) {
                    map.insert(code(src), utf16(dst));
                    k += 2;
                }
                i = k;
                continue;
            }
            Obj::Op(op) if op == "beginbfrange" => {
                let mut k = i + 1;
                while let (Some(Obj::Str(lo)), Some(Obj::Str(hi))) = (toks.get(k), toks.get(k + 1)) {
                    let (lo, hi) = (code(lo), code(hi));
                    match toks.get(k + 2) {
                        Some(Obj::Str(dst)) => {
                            let base: Vec<u16> = dst
                                .chunks(2)
                                .map(|p| (u16::from(p[0]) << 8) | u16::from(*p.get(1).unwrap_or(&0)))
                                .collect();
                            for (n, c) in (lo..=hi.min(lo + 65535)).enumerate() {
                                let mut units = base.clone();
                                if let Some(last) = units.last_mut() {
                                    *last = last.wrapping_add(n as u16);
                                }
                                map.insert(c, String::from_utf16_lossy(&units));
                            }
                        }
                        Some(Obj::Array(v)) => {
                            for (n, o) in v.iter().enumerate() {
                                if let Obj::Str(dst) = o {
                                    map.insert(lo + n as u32, utf16(dst));
                                }
                            }
                        }
                        _ => {}
                    }
                    k += 3;
                }
                i = k;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    (map, len)
}

/// Windows-1252 (WinAnsiEncoding) as Unicode.
fn win_ansi(b: u8) -> Option<char> {
    const HIGH: [u16; 32] = [
        0x20AC, 0, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, 0x02C6, 0x2030, 0x0160, 0x2039, 0x0152, 0, 0x017D,
        0, 0, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014, 0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0,
        0x017E, 0x0178,
    ];
    match b {
        0x20..=0x7E | 0xA0..=0xFF => Some(b as char),
        0x80..=0x9F => char::from_u32(u32::from(HIGH[(b - 0x80) as usize])).filter(|c| *c != '\0'),
        _ => None,
    }
}

fn apply_base(enc: &mut [Option<char>], name: &str) {
    if name == "StandardEncoding" {
        // The differences from WinAnsi that matter for text: quotes.
        enc[0x27] = Some('\u{2019}');
        enc[0x60] = Some('\u{2018}');
    }
}

/// The character of a glyph name.
fn glyph_char(g: &str) -> Option<char> {
    if g.len() == 1 {
        return g.chars().next();
    }
    if let Some(hex) = g.strip_prefix("uni").filter(|h| h.len() == 4) {
        return u32::from_str_radix(hex, 16).ok().and_then(char::from_u32);
    }
    const NAMES: [(&str, char); 40] = [
        ("space", ' '),
        ("exclam", '!'),
        ("quotedbl", '"'),
        ("numbersign", '#'),
        ("dollar", '$'),
        ("percent", '%'),
        ("ampersand", '&'),
        ("quoteright", '\u{2019}'),
        ("parenleft", '('),
        ("parenright", ')'),
        ("asterisk", '*'),
        ("plus", '+'),
        ("comma", ','),
        ("hyphen", '-'),
        ("period", '.'),
        ("slash", '/'),
        ("zero", '0'),
        ("one", '1'),
        ("two", '2'),
        ("three", '3'),
        ("four", '4'),
        ("five", '5'),
        ("six", '6'),
        ("seven", '7'),
        ("eight", '8'),
        ("nine", '9'),
        ("colon", ':'),
        ("semicolon", ';'),
        ("less", '<'),
        ("equal", '='),
        ("greater", '>'),
        ("question", '?'),
        ("at", '@'),
        ("bracketleft", '['),
        ("bracketright", ']'),
        ("underscore", '_'),
        ("minus", '\u{2212}'),
        ("degree", '°'),
        ("plusminus", '±'),
        ("mu", 'µ'),
    ];
    NAMES.iter().find(|(n, _)| *n == g).map(|(_, c)| *c)
}

// ----- content -----------------------------------------------------------------------------------

/// A run of text drawn at one place: its baseline start and end (page units) and the font size there.
#[derive(Clone, Debug)]
struct Run {
    x0: f64,
    x1: f64,
    y: f64,
    size: f64,
    text: String,
}

type Matrix = [f64; 6];

fn mul(a: &Matrix, b: &Matrix) -> Matrix {
    [
        a[0] * b[0] + a[1] * b[2],
        a[0] * b[1] + a[1] * b[3],
        a[2] * b[0] + a[3] * b[2],
        a[2] * b[1] + a[3] * b[3],
        a[4] * b[0] + a[5] * b[2] + b[4],
        a[4] * b[1] + a[5] * b[3] + b[5],
    ]
}

const IDENTITY: Matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

/// The text state that showing a string reads.
struct TextState<'f> {
    font: Option<&'f Font>,
    size: f64,
    tc: f64,
    tw: f64,
}

/// Shows string `s` (with TJ adjustments before code indices) at `tm`, advancing it; adds its run to `out`.
fn show(st: &TextState, ctm: &Matrix, tm: &mut Matrix, s: &[u8], adjust: &[(usize, f64)], out: &mut Vec<Run>) {
    let Some(f) = st.font else { return };
    let start = mul(tm, ctm);
    let scale = (start[2] * start[2] + start[3] * start[3]).sqrt().max(1e-9);
    let mut text = String::new();
    let mut tx = 0.0;
    for (k, (code, t)) in f.codes(s).into_iter().enumerate() {
        for (at, a) in adjust {
            if *at == k {
                tx -= a / 1000.0 * st.size;
            }
        }
        text.push_str(&t);
        tx += f.width(code) / 1000.0 * st.size + st.tc + if t == " " && f.code_len == 1 { st.tw } else { 0.0 };
    }
    *tm = mul(&[1.0, 0.0, 0.0, 1.0, tx, 0.0], tm);
    let end = mul(tm, ctm);
    if !text.trim().is_empty() {
        out.push(Run { x0: start[4], x1: end[4], y: start[5], size: st.size * scale, text });
    }
}

fn runs_of(content: &[u8], fonts: &HashMap<String, Font>) -> Vec<Run> {
    let mut out = Vec::new();
    let mut lx = Lexer::new(content, 0);
    let mut ops: Vec<Obj> = Vec::new();
    let mut ctm = IDENTITY;
    let mut stack: Vec<Matrix> = Vec::new();
    let (mut tm, mut tlm) = (IDENTITY, IDENTITY);
    let (mut font, mut size, mut leading, mut tc, mut tw) = (None::<String>, 0.0, 0.0, 0.0, 0.0);
    let num = |ops: &[Obj], k: usize| ops.get(k).and_then(Obj::num).unwrap_or(0.0);
    while let Some(o) = lx.obj() {
        let Obj::Op(op) = o else {
            ops.push(o);
            continue;
        };
        let n = ops.len();
        match op.as_str() {
            "q" => stack.push(ctm),
            "Q" => ctm = stack.pop().unwrap_or(IDENTITY),
            "cm" if n >= 6 => {
                let m = [
                    num(&ops, n - 6),
                    num(&ops, n - 5),
                    num(&ops, n - 4),
                    num(&ops, n - 3),
                    num(&ops, n - 2),
                    num(&ops, n - 1),
                ];
                ctm = mul(&m, &ctm);
            }
            "BT" => {
                tm = IDENTITY;
                tlm = IDENTITY;
            }
            "Tf" if n >= 2 => {
                font = ops[n - 2].name().map(str::to_string);
                size = num(&ops, n - 1);
            }
            "TL" if n >= 1 => leading = num(&ops, n - 1),
            "Tc" if n >= 1 => tc = num(&ops, n - 1),
            "Tw" if n >= 1 => tw = num(&ops, n - 1),
            "Td" | "TD" if n >= 2 => {
                let (x, y) = (num(&ops, n - 2), num(&ops, n - 1));
                if op == "TD" {
                    leading = -y;
                }
                tlm = mul(&[1.0, 0.0, 0.0, 1.0, x, y], &tlm);
                tm = tlm;
            }
            "Tm" if n >= 6 => {
                tlm = [
                    num(&ops, n - 6),
                    num(&ops, n - 5),
                    num(&ops, n - 4),
                    num(&ops, n - 3),
                    num(&ops, n - 2),
                    num(&ops, n - 1),
                ];
                tm = tlm;
            }
            "T*" => {
                tlm = mul(&[1.0, 0.0, 0.0, 1.0, 0.0, -leading], &tlm);
                tm = tlm;
            }
            "Tj" | "'" | "\"" => {
                if op != "Tj" {
                    if op == "\"" && n >= 3 {
                        tw = num(&ops, n - 3);
                        tc = num(&ops, n - 2);
                    }
                    tlm = mul(&[1.0, 0.0, 0.0, 1.0, 0.0, -leading], &tlm);
                    tm = tlm;
                }
                if let Some(Obj::Str(s)) = ops.last() {
                    let st = TextState { font: font.as_ref().and_then(|f| fonts.get(f)), size, tc, tw };
                    show(&st, &ctm, &mut tm, s, &[], &mut out);
                }
            }
            "TJ" => {
                if let Some(Obj::Array(parts)) = ops.last() {
                    // One run per TJ: the strings joined, the kerning numbers as adjustments before the next code.
                    let mut bytes = Vec::new();
                    let mut adjust = Vec::new();
                    let code_len = font.as_ref().and_then(|f| fonts.get(f)).map_or(1, |f| f.code_len);
                    for p in parts {
                        match p {
                            Obj::Str(s) => bytes.extend_from_slice(s),
                            Obj::Num(a) => {
                                let at = bytes.len() / code_len;
                                adjust.push((at, *a));
                            }
                            _ => {}
                        }
                    }
                    let st = TextState { font: font.as_ref().and_then(|f| fonts.get(f)), size, tc, tw };
                    show(&st, &ctm, &mut tm, &bytes, &adjust, &mut out);
                }
            }
            _ => {}
        }
        ops.clear();
    }
    out
}

// ----- pages and tables --------------------------------------------------------------------------

/// The pages in order, each with its inherited resources.
fn pages(doc: &Doc) -> Vec<(Dict, Dict)> {
    let catalog =
        doc.objects.values().find(|o| o.dict().and_then(|d| d.get("Type")).and_then(Obj::name) == Some("Catalog"));
    let mut out = Vec::new();
    fn walk(doc: &Doc, node: &Obj, res: &Dict, out: &mut Vec<(Dict, Dict)>, depth: usize) {
        if depth > 64 {
            return;
        }
        let Some(d) = doc.get(node).dict() else { return };
        let res = match doc.key(d, "Resources") {
            Obj::Dict(r) => r.clone(),
            _ => res.clone(),
        };
        match d.get("Type").and_then(Obj::name) {
            Some("Pages") => {
                if let Obj::Array(kids) = doc.key(d, "Kids") {
                    for k in kids {
                        walk(doc, k, &res, out, depth + 1);
                    }
                }
            }
            _ => out.push((d.clone(), res)),
        }
    }
    if let Some(root) = catalog.and_then(Obj::dict).and_then(|c| c.get("Pages")) {
        walk(doc, root, &Dict::new(), &mut out, 0);
    }
    if out.is_empty() {
        let mut nums: Vec<&u32> = doc.objects.keys().collect();
        nums.sort();
        for n in nums {
            if let Some(d) = doc.objects[n].dict()
                && d.get("Type").and_then(Obj::name) == Some("Page")
            {
                let res = doc.key(d, "Resources").dict().cloned().unwrap_or_default();
                out.push((d.clone(), res));
            }
        }
    }
    out
}

/// The runs of a page laid out as a table: rows of cells, each cell in its column.
fn table(mut runs: Vec<Run>) -> Vec<Vec<String>> {
    if runs.is_empty() {
        return Vec::new();
    }
    // Lines, top to bottom.
    runs.sort_by(|a, b| b.y.total_cmp(&a.y).then(a.x0.total_cmp(&b.x0)));
    let mut lines: Vec<Vec<Run>> = Vec::new();
    for r in runs {
        match lines.last_mut() {
            Some(line) if (line[0].y - r.y).abs() <= 0.35 * r.size.max(line[0].size) => line.push(r),
            _ => lines.push(vec![r]),
        }
    }
    // Cells: runs of a line closer than 0.7 em join.
    let cells: Vec<Vec<Run>> = lines
        .into_iter()
        .map(|mut line| {
            line.sort_by(|a, b| a.x0.total_cmp(&b.x0));
            let mut cells: Vec<Run> = Vec::new();
            for r in line {
                match cells.last_mut() {
                    Some(c) if r.x0 - c.x1 < 0.7 * r.size.max(c.size) => {
                        let gap = r.x0 - c.x1;
                        if gap > 0.15 * r.size && !c.text.ends_with(' ') && !r.text.starts_with(' ') {
                            c.text.push(' ');
                        }
                        c.text.push_str(&r.text);
                        c.x1 = c.x1.max(r.x1);
                    }
                    _ => cells.push(r),
                }
            }
            cells
        })
        .collect();
    // Columns: bands across which cells overlap.
    let mut bands: Vec<(f64, f64)> = cells.iter().flatten().map(|c| (c.x0, c.x1.max(c.x0 + 1e-6))).collect();
    bands.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut merged: Vec<(f64, f64)> = Vec::new();
    for (a, b) in bands {
        match merged.last_mut() {
            Some(m) if a <= m.1 => m.1 = m.1.max(b),
            _ => merged.push((a, b)),
        }
    }
    cells
        .into_iter()
        .map(|line| {
            let mut row = vec![String::new(); merged.len()];
            for c in line {
                let mid = (c.x0 + c.x1) / 2.0;
                let k = merged.iter().position(|(a, b)| mid >= *a && mid <= *b).unwrap_or(0);
                if row[k].is_empty() {
                    row[k] = c.text.trim().to_string();
                } else {
                    row[k] = format!("{} {}", row[k], c.text.trim());
                }
            }
            row
        })
        .collect()
}

/// A column of cells: numbers when every filled cell is one (a Unicode minus read as `-`), else text.
fn column_data(cells: Vec<String>) -> ColumnData {
    let parse = |s: &str| s.trim().replace('\u{2212}', "-").parse::<f64>().ok();
    if cells.iter().any(|s| !s.trim().is_empty()) && cells.iter().all(|s| s.trim().is_empty() || parse(s).is_some()) {
        ColumnData::numeric(cells.iter().map(|s| parse(s).unwrap_or(f64::NAN)).collect())
    } else {
        ColumnData::text(cells)
    }
}

/// Reads the tables of a PDF file's pages. `file_name` names the workbook.
pub fn read_pdf(bytes: &[u8], file_name: &str) -> Result<PdfImport, IoError> {
    let fail = |detail: String| IoError::NoData { source_name: file_name.to_string(), detail };
    if find(bytes, b"/Encrypt", 0).is_some() {
        return Err(fail("the PDF is encrypted; Sheaf reads only unencrypted PDFs".into()));
    }
    let doc = Doc::parse(bytes);
    let mut warnings = doc.warnings.clone();
    let pages = pages(&doc);
    let mut sheets = Vec::new();
    let mut unreadable = 0;
    for (k, (page, res)) in pages.iter().enumerate() {
        // Fonts of the page.
        let mut fonts = HashMap::new();
        if let Obj::Dict(fd) = doc.key(res, "Font") {
            for (name, f) in fd {
                if let Some(d) = doc.get(f).dict() {
                    fonts.insert(name.clone(), Font::read(&doc, d));
                }
            }
        }
        // Contents: one stream or several in order.
        let parts: Vec<&Obj> = match doc.key(page, "Contents") {
            Obj::Array(v) => v.iter().map(|o| doc.get(o)).collect(),
            o => vec![o],
        };
        let mut content = Vec::new();
        for p in parts {
            if let Obj::Stream(d, data) = p {
                match doc.decode(d, data) {
                    Some(raw) => {
                        content.extend(raw);
                        content.push(b'\n');
                    }
                    None => unreadable += 1,
                }
            }
        }
        let rows = table(runs_of(&content, &fonts));
        if rows.is_empty() {
            continue;
        }
        let mut sheet = Worksheet::new(format!("Page{}", k + 1));
        let ncols = rows.iter().map(Vec::len).max().unwrap_or(0);
        for c in 0..ncols {
            let cells: Vec<String> = rows.iter().map(|r| r.get(c).cloned().unwrap_or_default()).collect();
            let col = Column::new(ColumnId(c as u64 + 1), default_short_name(c), Designation::Y, column_data(cells));
            sheet.columns.push(col);
        }
        if let Some(first) = sheet.columns.first_mut() {
            first.designation = Designation::X;
        }
        sheets.push(sheet);
    }
    if unreadable > 0 {
        warnings.push(format!("{unreadable} content stream(s) use a compression Sheaf does not read and were skipped"));
    }
    if sheets.is_empty() {
        return Err(fail(format!(
            "no text was found on its {} page(s) (a scanned PDF holds pictures of text, which Sheaf does not read)",
            pages.len()
        )));
    }
    let with_text = sheets.len();
    let mut workbook = Workbook::new(WindowId(0), "Book1");
    workbook.long_name = file_name.to_string();
    workbook.sheets = sheets;
    workbook.source = Some(ImportSource { path: file_name.to_string(), importer: "pdf".to_string() });
    Ok(PdfImport { workbook, pages: (pages.len(), with_text), warnings })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdf_writer::{Content, Filter, Finish, Name, Pdf, Rect, Ref, Str};

    /// A one-page PDF drawing `rows` of `(x, text)` cells with Helvetica, `compress` its content.
    fn make_pdf(rows: &[(f32, &[(f32, &str)])], compress: bool) -> Vec<u8> {
        let mut pdf = Pdf::new();
        let (catalog, tree, page, font, content) = (Ref::new(1), Ref::new(2), Ref::new(3), Ref::new(4), Ref::new(5));
        pdf.catalog(catalog).pages(tree);
        pdf.pages(tree).kids([page]).count(1);
        let mut p = pdf.page(page);
        p.media_box(Rect::new(0.0, 0.0, 595.0, 842.0)).parent(tree).contents(content);
        p.resources().fonts().pair(Name(b"F1"), font);
        p.finish();
        pdf.type1_font(font).base_font(Name(b"Helvetica")).encoding_predefined(Name(b"WinAnsiEncoding"));
        let mut c = Content::new();
        for (y, cells) in rows {
            for (x, t) in *cells {
                c.begin_text();
                c.set_font(Name(b"F1"), 10.0);
                c.next_line(*x, *y);
                c.show(Str(t.as_bytes()));
                c.end_text();
            }
        }
        let data = c.finish();
        if compress {
            use std::io::Write;
            let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
            e.write_all(&data).unwrap();
            let z = e.finish().unwrap();
            pdf.stream(content, &z).filter(Filter::FlateDecode);
        } else {
            pdf.stream(content, &data);
        }
        pdf.finish()
    }

    #[test]
    fn a_table_of_three_columns_is_read_with_its_numbers() {
        let rows: Vec<(f32, &[(f32, &str)])> = vec![
            (700.0, &[(72.0, "Sample"), (200.0, "Mass"), (300.0, "Rate")]),
            (685.0, &[(72.0, "A 1"), (200.0, "1.25"), (300.0, "-3")]),
            (670.0, &[(72.0, "B 2"), (205.0, "10.5"), (300.0, "4e2")]),
        ];
        for compress in [false, true] {
            let bytes = make_pdf(&rows, compress);
            assert!(looks_like_pdf(&bytes));
            let r = read_pdf(&bytes, "t.pdf").unwrap();
            assert_eq!(r.pages, (1, 1));
            let s = &r.workbook.sheets[0];
            assert_eq!(s.name, "Page1");
            assert_eq!(s.columns.len(), 3, "compress {compress}");
            let text = |c: &Column| {
                (0..3).map(|i| sheaf_core::indexed::level_text(c, i).unwrap_or_default()).collect::<Vec<_>>()
            };
            assert_eq!(text(&s.columns[0]), ["Sample", "A 1", "B 2"]);
            // A header over numbers keeps the column text; the numbers below are its cells.
            assert_eq!(text(&s.columns[1]), ["Mass", "1.25", "10.5"]);
        }
    }

    #[test]
    fn a_numeric_column_and_cmaps_and_bad_input() {
        assert!(matches!(column_data(vec!["1".into(), "".into(), "\u{2212}2.5".into()]), ColumnData::Numeric(_)));
        assert!(matches!(column_data(vec!["1".into(), "x".into()]), ColumnData::Text(_) | ColumnData::Mixed { .. }));
        let cmap = b"/CIDInit begincmap 1 begincodespacerange <0000> <FFFF> endcodespacerange \
            2 beginbfchar <0003> <0020> <0024> <0041> endbfchar 1 beginbfrange <0030> <0032> <0061> endbfrange endcmap";
        let (map, len) = parse_cmap(cmap);
        assert_eq!(len, Some(2));
        assert_eq!(map.get(&0x24).map(String::as_str), Some("A"));
        assert_eq!(map.get(&0x32).map(String::as_str), Some("c"));
        assert_eq!(glyph_char("parenleft"), Some('('));
        assert_eq!(glyph_char("uni00B5"), Some('µ'));
        assert!(read_pdf(b"%PDF-1.4\n%%EOF", "e.pdf").is_err(), "no pages, no text");
    }
}
