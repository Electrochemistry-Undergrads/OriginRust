//! File > Import > Binary 2D Array (T199, matrices spec 7f; measured against Origin 2016's `impBin2D`
//! by T294, probes in `re-workspace/scratch/t294`): a file of raw numbers, row by row,
//! read into one matrix sheet.
//!
//! As Origin: the options are the number of columns (128 by default), the byte length (2), signed,
//! unsigned or floating point (unsigned), big-endian (no), the header bytes to skip (0) and a transpose
//! after the import (no). The number of rows comes from the file's size, rounded up. When the file ends
//! inside a row, that last row is all zeros: Origin adds it so and reports the import as failed. Integers
//! are 1, 2 or 4 bytes long; floating point numbers 4 or 8, or 16, of which the first 8 bytes are read as
//! a double. Other lengths, no columns, or a header that is not inside the file are refused. A NaN float
//! is a missing value; infinite floats stay infinite.

/// How each number is stored (Origin's Signed/Unsigned option).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sign {
    Unsigned,
    Signed,
    Float,
}

impl Sign {
    pub const ALL: [Sign; 3] = [Sign::Unsigned, Sign::Signed, Sign::Float];

    /// The name in the options window.
    pub fn name(self) -> &'static str {
        match self {
            Sign::Unsigned => "Unsigned",
            Sign::Signed => "Signed",
            Sign::Float => "Floating Point",
        }
    }
}

/// The byte lengths the options window offers (Origin's list).
pub const BYTE_LENGTHS: [usize; 5] = [1, 2, 4, 8, 16];

/// How to read a binary 2D array (Origin's `impBin2D` options; [`Default`] gives Origin's defaults).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bin2dOptions {
    /// Numbers in a row.
    pub cols: usize,
    /// Bytes per number.
    pub bytes: usize,
    pub sign: Sign,
    pub big_endian: bool,
    /// Bytes to skip at the start of the file.
    pub header: usize,
    /// Rows become columns after the import.
    pub transpose: bool,
}

impl Default for Bin2dOptions {
    fn default() -> Self {
        Bin2dOptions { cols: 128, bytes: 2, sign: Sign::Unsigned, big_endian: false, header: 0, transpose: false }
    }
}

impl Bin2dOptions {
    /// A refusal when the byte length does not go with the sign (as Origin refuses it).
    pub fn check_type(&self) -> Result<(), String> {
        match (self.sign, self.bytes) {
            (Sign::Unsigned | Sign::Signed, 1 | 2 | 4) | (Sign::Float, 4 | 8 | 16) => Ok(()),
            (Sign::Float, n) => Err(format!("floating point numbers are 4, 8 or 16 bytes long, not {n}")),
            (_, n) => Err(format!("integers are 1, 2 or 4 bytes long, not {n}")),
        }
    }

    /// The number of rows a file of `len` bytes gives: its numbers after the header, rounded up to
    /// whole rows; `None` where the options cannot read it.
    pub fn rows_for(&self, len: usize) -> Option<usize> {
        let row = self.cols.checked_mul(self.bytes).filter(|&r| r > 0)?;
        let avail = len.checked_sub(self.header).filter(|&a| a > 0)?;
        Some(avail.div_ceil(row))
    }

    fn read(&self, b: &[u8]) -> f64 {
        macro_rules! num {
            ($t:ty, $n:expr) => {{
                let mut a = [0u8; $n];
                a.copy_from_slice(&b[..$n]);
                if self.big_endian { <$t>::from_be_bytes(a) as f64 } else { <$t>::from_le_bytes(a) as f64 }
            }};
        }
        match (self.sign, self.bytes) {
            (Sign::Unsigned, 1) => f64::from(b[0]),
            (Sign::Signed, 1) => f64::from(b[0] as i8),
            (Sign::Unsigned, 2) => num!(u16, 2),
            (Sign::Signed, 2) => num!(i16, 2),
            (Sign::Unsigned, 4) => num!(u32, 4),
            (Sign::Signed, 4) => num!(i32, 4),
            (Sign::Float, 4) => num!(f32, 4),
            // 8, and the first 8 of 16 (measured).
            (Sign::Float, _) => num!(f64, 8),
            _ => f64::NAN,
        }
    }
}

/// An imported array, row by row.
#[derive(Clone, Debug, PartialEq)]
pub struct Bin2d {
    pub rows: usize,
    pub cols: usize,
    pub values: Vec<f64>,
    /// The file ended inside the last row, which is all zeros (Origin reports this import as failed).
    pub partial: bool,
}

/// The array in `bytes` read with `o`; an error says why the options cannot read it.
pub fn read_bin2d(bytes: &[u8], o: &Bin2dOptions) -> Result<Bin2d, String> {
    o.check_type()?;
    if o.cols == 0 {
        return Err("the array needs at least one column".into());
    }
    if o.header >= bytes.len() {
        return Err(format!("the file has {} bytes, none after a header of {}", bytes.len(), o.header));
    }
    let rows = o.rows_for(bytes.len()).ok_or("the array is too large")?;
    let n = rows.checked_mul(o.cols).filter(|&n| n <= 1 << 28).ok_or("the array is too large")?;
    let row_bytes = o.cols * o.bytes;
    let whole = (bytes.len() - o.header) / row_bytes;
    let mut values: Vec<f64> = bytes[o.header..o.header + whole * row_bytes]
        .chunks_exact(o.bytes)
        .map(|c| o.read(c))
        .map(|v| if v.is_nan() { f64::NAN } else { v })
        .collect();
    values.resize(n, 0.0);
    let (mut rows, mut cols) = (rows, o.cols);
    if o.transpose {
        let mut t = vec![0.0; n];
        for r in 0..rows {
            for c in 0..cols {
                t[c * rows + r] = values[r * cols + c];
            }
        }
        values = t;
        (rows, cols) = (cols, rows);
    }
    Ok(Bin2d { rows, cols, values, partial: whole * o.cols < n })
}

#[cfg(test)]
// Origin's numbers as it printed them.
#[allow(clippy::excessive_precision)]
mod tests {
    use super::*;

    fn opts(cols: usize, bytes: usize, sign: Sign) -> Bin2dOptions {
        Bin2dOptions { cols, bytes, sign, ..Bin2dOptions::default() }
    }

    #[test]
    fn origins_defaults_and_rows_from_the_file_size() {
        let o = Bin2dOptions::default();
        assert_eq!(
            (o.cols, o.bytes, o.sign, o.big_endian, o.header, o.transpose),
            (128, 2, Sign::Unsigned, false, 0, false)
        );
        // 256 unsigned 16-bit numbers are two rows of 128 (measured).
        let bytes: Vec<u8> = (0u16..256).flat_map(|v| v.to_le_bytes()).collect();
        let a = read_bin2d(&bytes, &o).unwrap();
        assert_eq!((a.rows, a.cols, a.values[129], a.partial), (2, 128, 129.0, false));
    }

    #[test]
    fn each_type_as_origin_reads_it() {
        // All measured in Origin 2016.
        let le = |v: &[u16]| v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>();
        assert_eq!(
            read_bin2d(&le(&[1, 2, 3, 4, 5, 6]), &opts(3, 2, Sign::Unsigned)).unwrap().values,
            [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]
        );
        let i8s = [-1i8, -128, 127, 0, 5, -5].map(|v| v as u8);
        assert_eq!(read_bin2d(&i8s, &opts(3, 1, Sign::Signed)).unwrap().values, [-1.0, -128.0, 127.0, 0.0, 5.0, -5.0]);
        assert_eq!(
            read_bin2d(&i8s, &opts(3, 1, Sign::Unsigned)).unwrap().values,
            [255.0, 128.0, 127.0, 0.0, 5.0, 251.0]
        );
        let be: Vec<u8> = [-1i16, -32768, 32767, 0, 300, -300].iter().flat_map(|v| v.to_be_bytes()).collect();
        let o = Bin2dOptions { big_endian: true, ..opts(3, 2, Sign::Signed) };
        assert_eq!(read_bin2d(&be, &o).unwrap().values, [-1.0, -32768.0, 32767.0, 0.0, 300.0, -300.0]);
        let i32s: Vec<u8> = [-1i32, 2, -3, 4, i32::MIN, i32::MAX].iter().flat_map(|v| v.to_le_bytes()).collect();
        assert_eq!(read_bin2d(&i32s, &opts(3, 4, Sign::Signed)).unwrap().values[4], -2147483648.0);
        assert_eq!(read_bin2d(&i32s, &opts(3, 4, Sign::Unsigned)).unwrap().values[0], 4294967295.0);
        let f32s: Vec<u8> =
            [1.5f32, f32::NAN, f32::INFINITY, -2.25, 0.0, 1e30].iter().flat_map(|v| v.to_le_bytes()).collect();
        let v = read_bin2d(&f32s, &opts(3, 4, Sign::Float)).unwrap().values;
        assert!(v[1].is_nan() && v[2] == f64::INFINITY, "NaN is missing, infinity stays");
        assert_eq!(v[5], 1.0000000150474662e30);
        let f64s: Vec<u8> = [1.25f64, -1e300, 3.0].iter().flat_map(|v| v.to_le_bytes()).collect();
        assert_eq!(read_bin2d(&f64s, &opts(3, 8, Sign::Float)).unwrap().values, [1.25, -1e300, 3.0]);
        // 16-byte floats: the first 8 bytes as a double.
        let b16: Vec<u8> = (0u8..96).collect();
        let v = read_bin2d(&b16, &opts(3, 16, Sign::Float)).unwrap().values;
        assert_eq!(v[0], 7.9499288951273625e-275);
        assert_eq!(v[4], 2.3127085096212408e35);
    }

    #[test]
    fn a_header_a_partial_row_and_transpose() {
        let mut bytes = vec![0xAA, 0xBB, 0xCC, 0xDD];
        bytes.extend([10u16, 20, 30, 40, 50, 60].iter().flat_map(|v| v.to_le_bytes()));
        let o = Bin2dOptions { header: 4, ..opts(3, 2, Sign::Unsigned) };
        assert_eq!(read_bin2d(&bytes, &o).unwrap().values, [10.0, 20.0, 30.0, 40.0, 50.0, 60.0]);
        // Seven numbers in rows of three: a third row of zeros, the 7 not read (measured).
        let seven: Vec<u8> = (1u16..=7).flat_map(|v| v.to_le_bytes()).collect();
        let a = read_bin2d(&seven, &opts(3, 2, Sign::Unsigned)).unwrap();
        assert_eq!((a.rows, a.partial), (3, true));
        assert_eq!(a.values, [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 0.0, 0.0, 0.0]);
        // A header of 2 leaves 5 numbers: one row and a row of zeros (measured).
        let six: Vec<u8> = (1u16..=6).flat_map(|v| v.to_le_bytes()).collect();
        let a = read_bin2d(&six, &Bin2dOptions { header: 2, ..opts(3, 2, Sign::Unsigned) }).unwrap();
        assert_eq!(a.values, [2.0, 3.0, 4.0, 0.0, 0.0, 0.0]);
        // Origin's default 128 columns on six numbers: one row of zeros.
        let a = read_bin2d(&six, &Bin2dOptions::default()).unwrap();
        assert_eq!((a.rows, a.cols, a.partial), (1, 128, true));
        assert!(a.values.iter().all(|v| *v == 0.0));
        let t = read_bin2d(&six, &Bin2dOptions { transpose: true, ..opts(3, 2, Sign::Unsigned) }).unwrap();
        assert_eq!((t.rows, t.cols), (3, 2));
        assert_eq!(t.values, [1.0, 4.0, 2.0, 5.0, 3.0, 6.0]);
    }

    #[test]
    fn origins_refusals() {
        let six: Vec<u8> = (1u16..=6).flat_map(|v| v.to_le_bytes()).collect();
        for (bytes, sign) in [
            (8, Sign::Signed),
            (8, Sign::Unsigned),
            (16, Sign::Signed),
            (16, Sign::Unsigned),
            (2, Sign::Float),
            (1, Sign::Float),
        ] {
            assert!(read_bin2d(&[0; 96], &opts(3, bytes, sign)).is_err(), "{bytes} {sign:?}");
        }
        assert!(read_bin2d(&six, &opts(0, 2, Sign::Unsigned)).is_err(), "no columns");
        assert!(read_bin2d(&six, &Bin2dOptions { header: 100, ..opts(3, 2, Sign::Unsigned) }).is_err());
        assert!(
            read_bin2d(&six, &Bin2dOptions { header: 12, ..opts(3, 2, Sign::Unsigned) }).is_err(),
            "nothing after the header"
        );
    }
}
