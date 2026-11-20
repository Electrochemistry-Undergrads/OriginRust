//! File > Import > Binary 2D Array (T199, matrices spec 7f): a file of raw numbers, row by row,
//! read into one matrix sheet. The options are those of Origin's `impBin2D`: the number type,
//! the number of columns and rows, a header to skip, and the byte order. Origin's own defaults
//! are not measured.

/// The type of each number in the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinType {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    F32,
    F64,
}

impl BinType {
    pub const ALL: [BinType; 8] =
        [BinType::I8, BinType::U8, BinType::I16, BinType::U16, BinType::I32, BinType::U32, BinType::F32, BinType::F64];

    /// Bytes per number.
    pub fn size(self) -> usize {
        match self {
            BinType::I8 | BinType::U8 => 1,
            BinType::I16 | BinType::U16 => 2,
            BinType::I32 | BinType::U32 | BinType::F32 => 4,
            BinType::F64 => 8,
        }
    }

    /// The name in the options window.
    pub fn name(self) -> &'static str {
        match self {
            BinType::I8 => "8-bit signed integer",
            BinType::U8 => "8-bit unsigned integer (byte)",
            BinType::I16 => "16-bit signed integer",
            BinType::U16 => "16-bit unsigned integer",
            BinType::I32 => "32-bit signed integer",
            BinType::U32 => "32-bit unsigned integer",
            BinType::F32 => "32-bit float",
            BinType::F64 => "64-bit float (double)",
        }
    }

    fn read(self, b: &[u8], big_endian: bool) -> f64 {
        macro_rules! num {
            ($t:ty, $n:expr) => {{
                let mut a = [0u8; $n];
                a.copy_from_slice(&b[..$n]);
                if big_endian { <$t>::from_be_bytes(a) as f64 } else { <$t>::from_le_bytes(a) as f64 }
            }};
        }
        match self {
            BinType::I8 => b[0] as i8 as f64,
            BinType::U8 => b[0] as f64,
            BinType::I16 => num!(i16, 2),
            BinType::U16 => num!(u16, 2),
            BinType::I32 => num!(i32, 4),
            BinType::U32 => num!(u32, 4),
            BinType::F32 => num!(f32, 4),
            BinType::F64 => num!(f64, 8),
        }
    }
}

/// How to read a binary 2D array.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bin2dOptions {
    pub kind: BinType,
    pub cols: usize,
    pub rows: usize,
    /// Bytes to skip at the start of the file.
    pub header: usize,
    pub big_endian: bool,
}

impl Bin2dOptions {
    /// Options for a file of `len` bytes of `kind`: square when the numbers make a square,
    /// else one row.
    pub fn guess(len: usize, kind: BinType) -> Bin2dOptions {
        let n = len / kind.size();
        let side = (n as f64).sqrt().round() as usize;
        let (cols, rows) = if side > 0 && side * side == n { (side, side) } else { (n.max(1), 1) };
        Bin2dOptions { kind, cols, rows, header: 0, big_endian: false }
    }
}

/// The numbers of the array, row by row (`rows * cols` of them). An error says what is wrong when
/// the file is too short or the size is empty. Bytes after the array are ignored. NaN and infinite
/// floats become missing values.
pub fn read_bin2d(bytes: &[u8], o: &Bin2dOptions) -> Result<Vec<f64>, String> {
    let n = o.rows.checked_mul(o.cols).filter(|&n| n > 0).ok_or("the array needs at least one row and column")?;
    let need = n.checked_mul(o.kind.size()).and_then(|b| b.checked_add(o.header)).ok_or("the array is too large")?;
    if bytes.len() < need {
        return Err(format!(
            "the file has {} bytes; {} x {} numbers of {} bytes after a {}-byte header need {}",
            bytes.len(),
            o.rows,
            o.cols,
            o.kind.size(),
            o.header,
            need
        ));
    }
    let size = o.kind.size();
    Ok(bytes[o.header..need]
        .chunks_exact(size)
        .map(|c| o.kind.read(c, o.big_endian))
        .map(|v| if v.is_finite() { v } else { f64::NAN })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_each_type_in_both_byte_orders_after_a_header() {
        let mut bytes = vec![0xAA, 0xBB];
        for v in [1.5f32, -2.0, 3.25, 4.0, 5.0, 6.5] {
            bytes.extend(v.to_le_bytes());
        }
        let o = Bin2dOptions { kind: BinType::F32, cols: 3, rows: 2, header: 2, big_endian: false };
        assert_eq!(read_bin2d(&bytes, &o).unwrap(), [1.5, -2.0, 3.25, 4.0, 5.0, 6.5]);
        let be: Vec<u8> = [258i16, -1].iter().flat_map(|v| v.to_be_bytes()).collect();
        let o = Bin2dOptions { kind: BinType::I16, cols: 2, rows: 1, header: 0, big_endian: true };
        assert_eq!(read_bin2d(&be, &o).unwrap(), [258.0, -1.0]);
        let o = Bin2dOptions { kind: BinType::U8, cols: 2, rows: 1, header: 0, big_endian: false };
        assert_eq!(read_bin2d(&[255, 7, 9], &o).unwrap(), [255.0, 7.0], "extra bytes are ignored");
        let o = Bin2dOptions { kind: BinType::I8, cols: 1, rows: 1, header: 0, big_endian: false };
        assert_eq!(read_bin2d(&[255], &o).unwrap(), [-1.0]);
        let nan: Vec<u8> = f64::NAN.to_le_bytes().to_vec();
        let o = Bin2dOptions { kind: BinType::F64, cols: 1, rows: 1, header: 0, big_endian: false };
        assert!(read_bin2d(&nan, &o).unwrap()[0].is_nan());
    }

    #[test]
    fn a_short_file_or_an_empty_size_is_refused_and_the_guess_is_square() {
        let o = Bin2dOptions { kind: BinType::F64, cols: 2, rows: 2, header: 0, big_endian: false };
        assert!(read_bin2d(&[0; 31], &o).unwrap_err().contains("need 32"));
        let o = Bin2dOptions { rows: 0, ..o };
        assert!(read_bin2d(&[0; 32], &o).is_err());
        assert_eq!((Bin2dOptions::guess(64, BinType::F32).cols, Bin2dOptions::guess(64, BinType::F32).rows), (4, 4));
        assert_eq!((Bin2dOptions::guess(12, BinType::F32).cols, Bin2dOptions::guess(12, BinType::F32).rows), (3, 1));
    }
}
