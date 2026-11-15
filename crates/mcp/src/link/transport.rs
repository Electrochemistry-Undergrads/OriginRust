//! The two ways to reach the app: a Unix domain socket in the run folder (macOS and Linux), and a
//! loopback TCP port with a random token (Windows). The TCP code is built and tested everywhere; Unix
//! uses it in tests only (docs/spec/mcp.md section 11.2).

use serde_json::{Value, json};
use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};

/// File name of the Unix socket in the run folder.
pub const SOCKET_NAME: &str = "mcp.sock";

/// Longest socket path the app binds. `sockaddr_un` has about 104 bytes on macOS and 108 on Linux.
pub const MAX_SOCKET_PATH: usize = 100;

/// Which transport an endpoint uses.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    #[cfg(unix)]
    Unix,
    Tcp,
}

impl Kind {
    /// The transport of this platform.
    pub fn native() -> Kind {
        #[cfg(unix)]
        {
            Kind::Unix
        }
        #[cfg(not(unix))]
        {
            Kind::Tcp
        }
    }
}

/// Where the app listens, as `mcp.json` says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Endpoint {
    Unix(PathBuf),
    Tcp { addr: SocketAddr, token: String },
}

impl Endpoint {
    pub fn to_json(&self) -> Value {
        match self {
            Endpoint::Unix(path) => json!({"kind": "unix", "path": path.to_string_lossy()}),
            Endpoint::Tcp { addr, token } => json!({"kind": "tcp", "addr": addr.to_string(), "token": token}),
        }
    }

    pub fn from_json(v: &Value) -> Result<Endpoint, String> {
        match v.get("kind").and_then(Value::as_str) {
            Some("unix") => {
                let path = v.get("path").and_then(Value::as_str).ok_or("mcp.json has no socket path")?;
                Ok(Endpoint::Unix(PathBuf::from(path)))
            }
            Some("tcp") => {
                let addr: SocketAddr = v
                    .get("addr")
                    .and_then(Value::as_str)
                    .and_then(|a| a.parse().ok())
                    .ok_or("mcp.json has no address")?;
                let token = v.get("token").and_then(Value::as_str).ok_or("mcp.json has no token")?;
                Ok(Endpoint::Tcp { addr, token: token.to_string() })
            }
            _ => Err("mcp.json names an endpoint this sheaf-mcp does not know".to_string()),
        }
    }

    /// The token to send in the hello, for TCP.
    pub fn token(&self) -> Option<&str> {
        match self {
            Endpoint::Tcp { token, .. } => Some(token),
            Endpoint::Unix(_) => None,
        }
    }
}

/// A connection, whichever transport it came from.
#[derive(Debug)]
pub enum Stream {
    #[cfg(unix)]
    Unix(UnixStream),
    Tcp(TcpStream),
}

impl Stream {
    /// Connects to `endpoint`, with a limit on how long the TCP connect may take.
    pub fn connect(endpoint: &Endpoint) -> io::Result<Stream> {
        match endpoint {
            #[cfg(unix)]
            Endpoint::Unix(path) => UnixStream::connect(path).map(Stream::Unix),
            #[cfg(not(unix))]
            Endpoint::Unix(_) => Err(io::Error::new(io::ErrorKind::Unsupported, "no Unix sockets on this system")),
            Endpoint::Tcp { addr, .. } => {
                if !addr.ip().is_loopback() {
                    return Err(io::Error::new(io::ErrorKind::InvalidInput, "the address is not a loopback address"));
                }
                let s = TcpStream::connect_timeout(addr, Duration::from_secs(5))?;
                let _ = s.set_nodelay(true);
                Ok(Stream::Tcp(s))
            }
        }
    }

    pub fn try_clone(&self) -> io::Result<Stream> {
        match self {
            #[cfg(unix)]
            Stream::Unix(s) => s.try_clone().map(Stream::Unix),
            Stream::Tcp(s) => s.try_clone().map(Stream::Tcp),
        }
    }

    pub fn set_read_timeout(&self, d: Option<Duration>) -> io::Result<()> {
        match self {
            #[cfg(unix)]
            Stream::Unix(s) => s.set_read_timeout(d),
            Stream::Tcp(s) => s.set_read_timeout(d),
        }
    }

    pub fn set_write_timeout(&self, d: Option<Duration>) -> io::Result<()> {
        match self {
            #[cfg(unix)]
            Stream::Unix(s) => s.set_write_timeout(d),
            Stream::Tcp(s) => s.set_write_timeout(d),
        }
    }

    /// Ends both directions; a thread blocked on this connection wakes up with an error or end of input.
    pub fn shutdown(&self) {
        let _ = match self {
            #[cfg(unix)]
            Stream::Unix(s) => s.shutdown(Shutdown::Both),
            Stream::Tcp(s) => s.shutdown(Shutdown::Both),
        };
    }
}

impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            #[cfg(unix)]
            Stream::Unix(s) => s.read(buf),
            Stream::Tcp(s) => s.read(buf),
        }
    }
}

impl Write for Stream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            #[cfg(unix)]
            Stream::Unix(s) => s.write(buf),
            Stream::Tcp(s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            #[cfg(unix)]
            Stream::Unix(s) => s.flush(),
            Stream::Tcp(s) => s.flush(),
        }
    }
}

/// The listening side.
#[derive(Debug)]
pub enum Listener {
    #[cfg(unix)]
    Unix(UnixListener),
    Tcp(TcpListener),
}

impl Listener {
    /// Binds a new endpoint of `kind`. A Unix socket is made in `dir` with mode 0600 (the folder is 0700
    /// already); TCP takes a free port on 127.0.0.1 and a new random token. Nothing else is bound.
    pub fn bind(kind: Kind, dir: &Path) -> Result<(Listener, Endpoint), String> {
        match kind {
            #[cfg(unix)]
            Kind::Unix => {
                use std::os::unix::fs::PermissionsExt;
                let path = dir.join(SOCKET_NAME);
                if path.as_os_str().len() > MAX_SOCKET_PATH {
                    return Err(format!(
                        "the socket path is too long ({} bytes, at most {MAX_SOCKET_PATH}); set SHEAF_MCP_RUN_DIR to a shorter folder: {}",
                        path.as_os_str().len(),
                        path.display()
                    ));
                }
                let listener =
                    UnixListener::bind(&path).map_err(|e| format!("could not bind '{}': {e}", path.display()))?;
                if let Err(e) = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)) {
                    drop(listener);
                    let _ = std::fs::remove_file(&path);
                    return Err(format!("could not protect '{}': {e}", path.display()));
                }
                Ok((Listener::Unix(listener), Endpoint::Unix(path)))
            }
            Kind::Tcp => {
                let listener =
                    TcpListener::bind(("127.0.0.1", 0)).map_err(|e| format!("could not listen on 127.0.0.1: {e}"))?;
                let addr = listener.local_addr().map_err(|e| format!("could not read the port: {e}"))?;
                let token = random_token()?;
                Ok((Listener::Tcp(listener), Endpoint::Tcp { addr, token }))
            }
        }
    }

    /// Waits for a connection.
    pub fn accept(&self) -> io::Result<Stream> {
        match self {
            #[cfg(unix)]
            Listener::Unix(l) => l.accept().map(|(s, _)| Stream::Unix(s)),
            Listener::Tcp(l) => l.accept().map(|(s, _)| {
                let _ = s.set_nodelay(true);
                Stream::Tcp(s)
            }),
        }
    }
}

/// 256 random bits as 64 hex digits.
pub fn random_token() -> Result<String, String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|e| format!("no random numbers for the token: {e}"))?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// Compares two byte strings with no early exit: the time does not tell how many leading bytes match.
/// A difference in length is a difference, and is folded in the same way.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    let mut diff = (a.len() ^ b.len()) as u64;
    for i in 0..a.len().max(b.len()) {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        diff |= u64::from(x ^ y);
    }
    std::hint::black_box(diff) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_token_comparison_tells_equal_from_different_strings_of_any_length() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(constant_time_eq(b"", b""));
        for other in [&b"abd"[..], b"abcd", b"ab", b"", b"xbc"] {
            assert!(!constant_time_eq(b"abc", other), "{other:?}");
        }
        assert!(!constant_time_eq(b"", b"a"));
    }

    #[test]
    fn tokens_are_64_hex_digits_and_differ() {
        let (a, b) = (random_token().unwrap(), random_token().unwrap());
        assert_eq!(a.len(), 64);
        assert!(a.bytes().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
    }

    #[test]
    fn an_endpoint_survives_mcp_json() {
        let tcp = Endpoint::Tcp { addr: "127.0.0.1:4242".parse().unwrap(), token: "ab".repeat(32) };
        assert_eq!(Endpoint::from_json(&tcp.to_json()), Ok(tcp.clone()));
        assert_eq!(tcp.token().map(str::len), Some(64));
        let unix = Endpoint::Unix(PathBuf::from("/run/sheaf/mcp.sock"));
        assert_eq!(Endpoint::from_json(&unix.to_json()), Ok(unix.clone()));
        assert_eq!(unix.token(), None);
        assert!(Endpoint::from_json(&json!({"kind": "pipe"})).is_err());
        assert!(Endpoint::from_json(&json!({"kind": "tcp", "addr": "nope", "token": "x"})).is_err());
    }

    #[test]
    fn a_tcp_endpoint_binds_the_loopback_address_only() {
        let (listener, endpoint) = Listener::bind(Kind::Tcp, Path::new(".")).unwrap();
        let Endpoint::Tcp { addr, .. } = &endpoint else { panic!("{endpoint:?}") };
        assert!(addr.ip().is_loopback() && addr.port() != 0);
        let stream = Stream::connect(&endpoint).unwrap();
        let accepted = listener.accept().unwrap();
        drop((stream, accepted));
        // An address that is not loopback is never connected to, so a token is never sent there.
        let far = Endpoint::Tcp { addr: "192.0.2.1:9".parse().unwrap(), token: "t".into() };
        assert!(Stream::connect(&far).is_err());
    }
}
