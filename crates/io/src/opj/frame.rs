//! Block framing of an `.opj` file (see `docs/spec/opj-format.md`, section 2).
//!
//! After the signature line the file is a run of blocks. A block is a 4-byte
//! little-endian size, a newline byte, `size` bytes of body and, when the size
//! is not zero, one more newline byte. Every length is checked against the
//! file length before anything is allocated or read.

use std::fmt;
use std::io::{self, Read, Seek, SeekFrom};

/// Why a block could not be read.
#[derive(Debug)]
pub(crate) enum FrameError {
    /// The file ends inside a block (or a size field promises more than the file holds).
    Truncated { at: u64 },
    /// A newline byte that should delimit a block is something else.
    BadDelimiter { at: u64 },
    /// The operating system failed.
    Io(io::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Truncated { at } => write!(f, "the file ends unexpectedly near byte {at}"),
            FrameError::BadDelimiter { at } => write!(f, "the block structure is damaged near byte {at}"),
            FrameError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl From<io::Error> for FrameError {
    fn from(e: io::Error) -> Self {
        if e.kind() == io::ErrorKind::UnexpectedEof { FrameError::Truncated { at: 0 } } else { FrameError::Io(e) }
    }
}

pub(crate) type FrameResult<T> = Result<T, FrameError>;

/// Where the body of a block lies, for blocks that are read later (or never).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Extent {
    pub pos: u64,
    pub len: u32,
}

/// A position-tracking reader of blocks over a seekable byte source.
pub(crate) struct Frames<R> {
    inner: R,
    pos: u64,
    len: u64,
}

impl<R: Read + Seek> Frames<R> {
    pub fn new(mut inner: R) -> io::Result<Self> {
        let len = inner.seek(SeekFrom::End(0))?;
        inner.seek(SeekFrom::Start(0))?;
        Ok(Frames { inner, pos: 0, len })
    }

    pub fn pos(&self) -> u64 {
        self.pos
    }

    pub fn remaining(&self) -> u64 {
        self.len.saturating_sub(self.pos)
    }

    fn take(&mut self, n: u64) -> FrameResult<Vec<u8>> {
        if n > self.remaining() {
            return Err(FrameError::Truncated { at: self.pos });
        }
        let mut v = vec![0u8; n as usize];
        self.inner.read_exact(&mut v).map_err(|e| match FrameError::from(e) {
            FrameError::Truncated { .. } => FrameError::Truncated { at: self.pos },
            other => other,
        })?;
        self.pos += n;
        Ok(v)
    }

    /// Reads bytes up to (not including) the next newline; at most `max` bytes.
    pub fn line(&mut self, max: usize) -> FrameResult<Vec<u8>> {
        let mut out = Vec::new();
        loop {
            let b = self.take(1)?;
            if b[0] == b'\n' {
                return Ok(out);
            }
            if out.len() >= max {
                return Err(FrameError::BadDelimiter { at: self.pos });
            }
            out.push(b[0]);
        }
    }

    /// Reads the 5-byte block head and returns the body size.
    pub fn head(&mut self) -> FrameResult<u32> {
        let h = self.take(5)?;
        if h[4] != b'\n' {
            return Err(FrameError::BadDelimiter { at: self.pos - 1 });
        }
        Ok(u32::from_le_bytes([h[0], h[1], h[2], h[3]]))
    }

    fn end_of_body(&mut self) -> FrameResult<()> {
        let b = self.take(1)?;
        if b[0] != b'\n' {
            return Err(FrameError::BadDelimiter { at: self.pos - 1 });
        }
        Ok(())
    }

    /// Reads a whole block; an empty vector for a block of size zero.
    pub fn block(&mut self) -> FrameResult<Vec<u8>> {
        let size = self.head()?;
        if size == 0 {
            return Ok(Vec::new());
        }
        let body = self.take(u64::from(size))?;
        self.end_of_body()?;
        Ok(body)
    }

    /// Steps over a block without reading its body; returns where the body is.
    pub fn skip_block(&mut self) -> FrameResult<Extent> {
        let size = self.head()?;
        let ext = Extent { pos: self.pos, len: size };
        if size > 0 {
            let end = self.pos + u64::from(size);
            if end >= self.len {
                return Err(FrameError::Truncated { at: self.pos });
            }
            self.inner.seek(SeekFrom::Start(end))?;
            self.pos = end;
            self.end_of_body()?;
        }
        Ok(ext)
    }

    /// Reads `n` raw bytes followed by a newline, with no size field in front
    /// (the member header of a grouped annotation, section 5.4 of the spec).
    pub fn raw(&mut self, n: u32) -> FrameResult<Vec<u8>> {
        let body = self.take(u64::from(n))?;
        self.end_of_body()?;
        Ok(body)
    }

    /// Reads the body of an earlier-skipped block, then returns to the current position.
    pub fn read_extent(&mut self, ext: Extent) -> FrameResult<Vec<u8>> {
        if u64::from(ext.len) > self.len.saturating_sub(ext.pos) {
            return Err(FrameError::Truncated { at: ext.pos });
        }
        let back = self.pos;
        self.inner.seek(SeekFrom::Start(ext.pos))?;
        let mut v = vec![0u8; ext.len as usize];
        let r = self.inner.read_exact(&mut v);
        self.inner.seek(SeekFrom::Start(back))?;
        r.map_err(FrameError::from)?;
        Ok(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    pub(crate) fn framed(body: &[u8]) -> Vec<u8> {
        let mut v = (body.len() as u32).to_le_bytes().to_vec();
        v.push(b'\n');
        if !body.is_empty() {
            v.extend_from_slice(body);
            v.push(b'\n');
        }
        v
    }

    #[test]
    fn reads_blocks_and_empty_blocks() {
        let mut bytes = b"CPYA 4.3168 1 W64 #\n".to_vec();
        bytes.extend(framed(b"abc"));
        bytes.extend(framed(b""));
        bytes.extend(framed(&[1, 2, 3, 4]));
        let mut f = Frames::new(Cursor::new(bytes)).unwrap();
        assert_eq!(f.line(100).unwrap(), b"CPYA 4.3168 1 W64 #");
        assert_eq!(f.block().unwrap(), b"abc");
        assert_eq!(f.block().unwrap(), b"");
        assert_eq!(f.block().unwrap(), [1, 2, 3, 4]);
        assert_eq!(f.remaining(), 0);
        assert!(matches!(f.block(), Err(FrameError::Truncated { .. })));
    }

    #[test]
    fn skip_reports_the_extent_and_read_extent_gets_it_back() {
        let mut bytes = framed(b"hello");
        bytes.extend(framed(b"world"));
        let mut f = Frames::new(Cursor::new(bytes)).unwrap();
        let e = f.skip_block().unwrap();
        assert_eq!(e, Extent { pos: 5, len: 5 });
        assert_eq!(f.block().unwrap(), b"world");
        assert_eq!(f.read_extent(e).unwrap(), b"hello");
    }

    #[test]
    fn damaged_framing_is_an_error_not_a_panic() {
        // size field promises more than the file holds
        let mut f = Frames::new(Cursor::new(vec![0xFF, 0xFF, 0xFF, 0x7F, b'\n', 1, 2])).unwrap();
        assert!(matches!(f.block(), Err(FrameError::Truncated { .. })));
        // missing newline after the size
        let mut f = Frames::new(Cursor::new(vec![2, 0, 0, 0, b'x', 1, 2, b'\n'])).unwrap();
        assert!(matches!(f.block(), Err(FrameError::BadDelimiter { .. })));
        // missing newline after the body
        let mut f = Frames::new(Cursor::new(vec![2, 0, 0, 0, b'\n', 1, 2, b'x'])).unwrap();
        assert!(matches!(f.block(), Err(FrameError::BadDelimiter { .. })));
        let mut f = Frames::new(Cursor::new(vec![2, 0, 0, 0, b'\n', 1, 2, b'x'])).unwrap();
        assert!(f.skip_block().is_err());
        // a raw run needs its newline too
        let mut f = Frames::new(Cursor::new(vec![1, 2, 3, b'\n'])).unwrap();
        assert_eq!(f.raw(3).unwrap(), [1, 2, 3]);
        let mut f = Frames::new(Cursor::new(vec![1, 2, 3])).unwrap();
        assert!(f.raw(3).is_err());
        assert!(f.raw(u32::MAX).is_err());
    }

    #[test]
    fn a_line_without_newline_or_too_long_is_an_error() {
        let mut f = Frames::new(Cursor::new(b"abcdef".to_vec())).unwrap();
        assert!(f.line(100).is_err());
        let mut f = Frames::new(Cursor::new(b"abcdef\n".to_vec())).unwrap();
        assert!(f.line(3).is_err());
    }
}
