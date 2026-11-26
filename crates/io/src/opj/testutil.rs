//! Builders for small synthetic `.opj` byte sequences, used by the tests of this module.
//! Nothing here comes from a real project file.

/// A block with its size field and delimiters.
pub(crate) fn framed(body: &[u8]) -> Vec<u8> {
    let mut v = (body.len() as u32).to_le_bytes().to_vec();
    v.push(b'\n');
    if !body.is_empty() {
        v.extend_from_slice(body);
        v.push(b'\n');
    }
    v
}

fn put(buf: &mut [u8], at: usize, bytes: &[u8]) {
    buf[at..at + bytes.len()].copy_from_slice(bytes);
}

/// A window header block body (346 bytes) with a name and a label.
pub(crate) fn window_header(name: &str, label: &str) -> Vec<u8> {
    let mut h = vec![0u8; 346];
    put(&mut h, 0x02, name.as_bytes());
    put(&mut h, 0xC3, label.as_bytes());
    h
}

/// A layer header block body (361 bytes) with a sheet name.
pub(crate) fn layer_header(sheet: &str) -> Vec<u8> {
    let mut h = vec![0u8; 361];
    put(&mut h, 0xD0, b"Pd");
    put(&mut h, 0xD2, sheet.as_bytes());
    h
}

/// A column record block body (515 bytes).
pub(crate) fn column_record(dataset: u16, designation: u8, short_name: &str) -> Vec<u8> {
    let mut h = vec![0u8; 515];
    put(&mut h, 0x04, &dataset.to_le_bytes());
    h[0x11] = designation;
    put(&mut h, 0x12, short_name.as_bytes());
    h[0x1E] = 9;
    h
}

/// One annotation: a header, then (for a group) two blocks, an unframed member header
/// and the member's three blocks; otherwise three blocks.
pub(crate) fn annotation(group: bool) -> Vec<u8> {
    let mut v = framed(&[0x23; 123]);
    if group {
        v.extend(framed(&[7; 0x5E]));
        v.extend(framed(&123u32.to_le_bytes()));
        v.extend_from_slice(&[0x23; 123]);
        v.push(b'\n');
        v.extend(framed(&[1; 20]));
        v.extend(framed(&[2; 18]));
        v.extend(framed(&[]));
    } else {
        v.extend(framed(&[7; 64]));
        v.extend(framed(&[8; 30]));
        v.extend(framed(&[]));
    }
    v
}

/// A data-set header block body (147 bytes).
pub(crate) fn dataset_header(name: &str, type_code: u16, cell_size: u8, rows: u32) -> Vec<u8> {
    let mut h = vec![0u8; 147];
    put(&mut h, 0x16, &type_code.to_le_bytes());
    put(&mut h, 0x19, &rows.to_le_bytes());
    h[0x3D] = cell_size;
    put(&mut h, 0x58, name.as_bytes());
    h
}

/// The bytes of a 10-byte mixed cell holding a number.
pub(crate) fn number_cell(x: f64) -> Vec<u8> {
    let mut c = vec![0u8, 0];
    c.extend_from_slice(&x.to_le_bytes());
    c
}

/// The bytes of a mixed cell of `size` bytes holding text.
pub(crate) fn text_cell(s: &str, size: usize) -> Vec<u8> {
    let mut c = vec![1u8, 0];
    c.extend_from_slice(s.as_bytes());
    c.resize(size, 0);
    c
}

/// A whole project file: signature line, version header, data sets, windows.
#[derive(Default)]
pub(crate) struct FileBuilder {
    /// The Origin version written to the header; 9.3 when left at zero.
    pub version: f64,
    pub datasets: Vec<Vec<u8>>,
    pub windows: Vec<u8>,
}

impl FileBuilder {
    pub fn dataset(&mut self, name: &str, type_code: u16, cell_size: u8, data: &[u8]) -> &mut Self {
        let rows = data.len() / usize::from(cell_size.max(1));
        let mut b = framed(&dataset_header(name, type_code, cell_size, rows as u32));
        b.extend(framed(data));
        b.extend(framed(&[]));
        self.datasets.push(b);
        self
    }

    /// A data set with a mask block in Origin's layout (opj-format.md 3.2): `masked` are 0-based rows.
    pub fn dataset_masked(
        &mut self,
        name: &str,
        type_code: u16,
        cell_size: u8,
        data: &[u8],
        masked: &[usize],
    ) -> &mut Self {
        let rows = data.len() / usize::from(cell_size.max(1));
        let mut b = framed(&dataset_header(name, type_code, cell_size, rows as u32));
        b.extend(framed(data));
        let len = 40 + rows + 4;
        let mut mask = vec![0u8; len];
        mask[..4].copy_from_slice(&[0x90, 0xe5, 0x2e, 0x0f]);
        mask[4..8].copy_from_slice(&(len as u32).to_le_bytes());
        for &r in masked {
            mask[40 + r] = 1;
        }
        b.extend(framed(&mask));
        self.datasets.push(b);
        self
    }

    pub fn window(&mut self, name: &str, label: &str, sheets: &[(&str, &[(u16, u8, &str, &[u8])])]) -> &mut Self {
        self.window_with(name, label, sheets, false, 0)
    }

    /// A window whose layers have `axis_blocks` blocks in each of the three axis groups and, when
    /// `graph_layer` is set, the graph-layer bit in the layer header (spec section 10.0). Origin 8.x
    /// workbooks have axis blocks and no bit.
    pub fn window_with(
        &mut self,
        name: &str,
        label: &str,
        sheets: &[(&str, &[(u16, u8, &str, &[u8])])],
        graph_layer: bool,
        axis_blocks: usize,
    ) -> &mut Self {
        self.windows.extend(framed(&window_header(name, label)));
        for (sheet, cols) in sheets {
            let mut header = layer_header(sheet);
            header[0x02] = if graph_layer { 0x1F } else { 0x5E };
            self.windows.extend(framed(&header));
            self.windows.extend(annotation(false));
            self.windows.extend(framed(&[]));
            for (idx, des, short, note) in cols.iter() {
                self.windows.extend(framed(&column_record(*idx, *des, short)));
                self.windows.extend(framed(note));
            }
            self.windows.extend(framed(&[]));
            // the extra block, then the three axis groups, each closed by an empty block
            self.windows.extend(framed(&[]));
            for _ in 0..3 {
                for _ in 0..axis_blocks {
                    self.windows.extend(framed(&[0; 515]));
                }
                self.windows.extend(framed(&[]));
            }
        }
        self.windows.extend(framed(&[]));
        self
    }

    /// A matrix window (T471): one layer per sheet with the matrix flag byte, the size at `0x2B`
    /// (columns) and `0x52` (rows), and an annotation per `(name, text)` (the coordinates `X1` ...).
    pub fn matrix(&mut self, name: &str, label: &str, sheets: &[(&str, u16, u16, &[(&str, &str)])]) -> &mut Self {
        self.windows.extend(framed(&window_header(name, label)));
        for (sheet, cols, rows, notes) in sheets {
            let mut header = layer_header(sheet);
            header[0x02] = 0xDE;
            put(&mut header, 0x2B, &cols.to_le_bytes());
            put(&mut header, 0x52, &rows.to_le_bytes());
            self.windows.extend(framed(&header));
            for (key, text) in notes.iter() {
                let mut h = vec![0u8; 123];
                put(&mut h, 0x46, key.as_bytes());
                self.windows.extend(framed(&h));
                let mut t = text.as_bytes().to_vec();
                t.resize(20, 0);
                self.windows.extend(framed(&t));
                self.windows.extend(framed(&[]));
                self.windows.extend(framed(&[]));
            }
            self.windows.extend(framed(&[]));
            // no column records, the extra block, three empty axis groups
            self.windows.extend(framed(&[]));
            self.windows.extend(framed(&[]));
            for _ in 0..3 {
                self.windows.extend(framed(&[]));
            }
        }
        self.windows.extend(framed(&[]));
        self
    }

    pub fn build(&self) -> Vec<u8> {
        let mut v = b"CPYA 4.3168 226 W64 #\n".to_vec();
        let mut header = vec![0u8; 111];
        let version = if self.version == 0.0 { 9.3 } else { self.version };
        header[0x1B..0x23].copy_from_slice(&version.to_le_bytes());
        v.extend(framed(&header));
        v.extend(framed(&[]));
        for d in &self.datasets {
            v.extend_from_slice(d);
        }
        v.extend(framed(&[]));
        v.extend_from_slice(&self.windows);
        v.extend(framed(&[]));
        v.extend_from_slice(b"trailing sections are ignored\n");
        v
    }
}
