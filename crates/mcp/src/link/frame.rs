//! One JSON message per line, UTF-8, with a cap on the length of a line. Stdin and the link use the
//! same reader (docs/spec/mcp.md section 11.3).

use std::io::{self, BufRead, ErrorKind, Write};
use std::time::Instant;

/// Longest request line accepted: from an assistant on stdin and from a proxy on the link.
pub const REQUEST_CAP: usize = 8 * 1024 * 1024;

/// Longest first line (the hello) accepted from a connection that has not been let in yet.
pub const HELLO_CAP: usize = 64 * 1024;

/// Longest reply line sent over the link: a PNG in base64 plus text.
pub const REPLY_CAP: usize = 64 * 1024 * 1024;

/// What [`read_line`] found.
#[derive(Debug, PartialEq, Eq)]
pub enum Line {
    /// End of input with nothing left to read.
    Eof,
    /// One line without its line break. It may be empty. Bytes that are not UTF-8 become U+FFFD.
    Text(String),
    /// A line longer than the cap; the rest of it was read and thrown away.
    TooLong,
}

fn is_timeout(kind: ErrorKind) -> bool {
    matches!(kind, ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted)
}

/// Reads one line of at most `cap` bytes without keeping more than that in memory.
///
/// A read that times out (a socket with a read timeout) calls `on_timeout`; the bytes read so far are
/// kept and reading goes on while it returns true, otherwise the call fails with `TimedOut`. A stream
/// with no timeout never calls it.
pub fn read_line<R: BufRead + ?Sized>(
    input: &mut R,
    cap: usize,
    on_timeout: &mut dyn FnMut() -> bool,
) -> io::Result<Line> {
    read_line_by(input, cap, None, on_timeout)
}

/// Like [`read_line`], and the whole line must also arrive before `deadline`: the clock is checked after
/// every chunk that was read, so a peer that sends a byte now and then (and never times out a read) is
/// cut off too. The call then fails with `TimedOut`.
pub fn read_line_by<R: BufRead + ?Sized>(
    input: &mut R,
    cap: usize,
    deadline: Option<Instant>,
    on_timeout: &mut dyn FnMut() -> bool,
) -> io::Result<Line> {
    let mut buf: Vec<u8> = Vec::new();
    let mut too_long = false;
    let mut any = false;
    loop {
        let chunk = match input.fill_buf() {
            Ok(c) => c,
            Err(e) if is_timeout(e.kind()) => {
                if on_timeout() {
                    continue;
                }
                return Err(io::Error::from(ErrorKind::TimedOut));
            }
            Err(e) => return Err(e),
        };
        if chunk.is_empty() {
            break;
        }
        if deadline.is_some_and(|d| Instant::now() >= d) {
            return Err(io::Error::from(ErrorKind::TimedOut));
        }
        any = true;
        let (take, done) = match chunk.iter().position(|b| *b == b'\n') {
            Some(i) => (i + 1, true),
            None => (chunk.len(), false),
        };
        let payload = if done { take - 1 } else { take };
        if !too_long {
            if buf.len() + payload <= cap {
                buf.extend_from_slice(&chunk[..payload]);
            } else {
                too_long = true;
                buf = Vec::new();
            }
        }
        input.consume(take);
        if done {
            break;
        }
    }
    if too_long {
        return Ok(Line::TooLong);
    }
    if !any {
        return Ok(Line::Eof);
    }
    if buf.last() == Some(&b'\r') {
        buf.pop();
    }
    Ok(Line::Text(String::from_utf8_lossy(&buf).into_owned()))
}

/// Writes `line` and a line break, then flushes.
pub fn write_line<W: Write + ?Sized>(out: &mut W, line: &str) -> io::Result<()> {
    out.write_all(line.as_bytes())?;
    out.write_all(b"\n")?;
    out.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn read(bytes: &[u8], cap: usize) -> Vec<Line> {
        let mut input = Cursor::new(bytes.to_vec());
        let mut out = Vec::new();
        loop {
            let line = read_line(&mut input, cap, &mut || true).unwrap();
            let end = line == Line::Eof;
            out.push(line);
            if end {
                return out;
            }
        }
    }

    #[test]
    fn lines_are_split_and_the_line_break_is_dropped() {
        assert_eq!(
            read(b"a\r\nb\n\nc", 100),
            [
                Line::Text("a".into()),
                Line::Text("b".into()),
                Line::Text(String::new()),
                Line::Text("c".into()),
                Line::Eof
            ]
        );
        assert_eq!(read(b"", 10), [Line::Eof]);
    }

    #[test]
    fn a_line_over_the_cap_is_thrown_away_and_the_next_one_still_reads() {
        assert_eq!(read(b"0123456789\nok\n", 5), [Line::TooLong, Line::Text("ok".into()), Line::Eof]);
        // A line of exactly the cap passes, also at the end of input.
        assert_eq!(read(b"01234", 5), [Line::Text("01234".into()), Line::Eof]);
        assert_eq!(read(b"012345", 5), [Line::TooLong, Line::Eof]);
        assert_eq!(read(b"01234\n", 5), [Line::Text("01234".into()), Line::Eof]);
    }

    #[test]
    fn a_line_that_arrives_too_late_fails_even_when_every_read_succeeds() {
        let mut input = Cursor::new(b"slow\n".to_vec());
        let past = Instant::now();
        let e = read_line_by(&mut input, 100, Some(past), &mut || true).unwrap_err();
        assert_eq!(e.kind(), ErrorKind::TimedOut);
        let mut input = Cursor::new(b"fast\n".to_vec());
        let later = Instant::now() + std::time::Duration::from_secs(60);
        assert_eq!(read_line_by(&mut input, 100, Some(later), &mut || true).unwrap(), Line::Text("fast".into()));
    }

    #[test]
    fn bytes_that_are_not_utf8_are_replaced() {
        assert_eq!(read(b"a\xffb\n", 10), [Line::Text("a\u{fffd}b".into()), Line::Eof]);
    }
}
