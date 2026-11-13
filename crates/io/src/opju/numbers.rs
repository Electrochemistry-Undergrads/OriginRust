//! The value coding of a numeric column of an `.opju` file (spec section 5).
//!
//! Every value is stored as the exclusive-or of its 64 bits with the bits of the value before it
//! (the first one with zero), and only the low bytes that are not zero are written. Two values share
//! one tag byte: the low nibble describes the first, the high nibble the second.

/// Origin's marker for a missing cell, `-1.23456789E-300`.
pub const MISSING: f64 = -1.234_567_89e-300;

/// Magnitudes below this are taken as missing values.
const TINY: f64 = 1e-250;

/// Reads a base-128 number, low group first, with the high bit of a byte meaning "more follows".
pub fn read_varint(bytes: &[u8], pos: &mut usize) -> Option<u64> {
    let mut value = 0u64;
    let mut shift = 0u32;
    loop {
        let b = *bytes.get(*pos)?;
        *pos += 1;
        value |= u64::from(b & 0x7f).checked_shl(shift)?;
        if b & 0x80 == 0 {
            return Some(value);
        }
        shift += 7;
        if shift > 63 {
            return None;
        }
    }
}

/// How many bytes follow for a nibble of a tag byte: none for 0 and 8, otherwise the low three bits plus one.
pub fn nibble_len(nibble: u8) -> usize {
    match nibble & 7 {
        0 => 0,
        k => usize::from(k) + 1,
    }
}

/// Decodes `count` values from `bytes[start..end]`. Returns the values and the position after the
/// last byte used, or `None` when the data ends early.
pub fn decode_values(bytes: &[u8], start: usize, end: usize, count: usize) -> Option<(Vec<f64>, usize)> {
    let end = end.min(bytes.len());
    let mut pos = start;
    let mut prev = 0u64;
    let mut out = Vec::with_capacity(count.min(1 << 20));
    while out.len() < count {
        let tag = *bytes.get(pos).filter(|_| pos < end)?;
        pos += 1;
        for nibble in [tag & 0x0f, tag >> 4] {
            if out.len() == count {
                break;
            }
            let n = nibble_len(nibble);
            if pos + n > end {
                return None;
            }
            let mut raw = [0u8; 8];
            raw[..n].copy_from_slice(&bytes[pos..pos + n]);
            pos += n;
            let x = u64::from_le_bytes(raw);
            // Eight bytes with the flag bit clear are the value itself; everything else is a change.
            let bits = if n == 8 && nibble & 8 == 0 { x } else { prev ^ x };
            prev = bits;
            let v = f64::from_bits(bits);
            // Values this small are not data: they are the missing marker, or the marker with a few
            // bits changed by the exclusive-or against a neighbour (spec section 5.3).
            out.push(if v != 0.0 && v.abs() < TINY { f64::NAN } else { v });
        }
    }
    Some((out, pos))
}

/// Reads `count` strings, each a length (base-128) followed by that many bytes, from `bytes[start..end]`.
pub fn decode_strings(bytes: &[u8], start: usize, end: usize, count: usize) -> Option<Vec<String>> {
    let end = end.min(bytes.len());
    let mut pos = start;
    let mut out = Vec::with_capacity(count.min(1 << 20));
    for _ in 0..count {
        if pos >= end {
            return None;
        }
        let len = read_varint(bytes, &mut pos)? as usize;
        if pos + len > end {
            return None;
        }
        out.push(String::from_utf8_lossy(&bytes[pos..pos + len]).into_owned());
        pos += len;
    }
    Some(out)
}
