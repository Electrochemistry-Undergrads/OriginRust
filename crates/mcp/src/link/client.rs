//! The proxy's side: finding the app and saying hello (docs/spec/mcp.md section 11.2).

use super::discovery;
use super::frame::{self, Line, REPLY_CAP};
use super::proto::{self, Hello};
use super::transport::Stream;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// How a client connects.
#[derive(Clone, Debug)]
pub struct ClientOptions {
    /// The name of the client the app shows to its owner.
    pub client: String,
    /// The folders the assistant may use, absolute and canonical.
    pub roots: Vec<String>,
    /// How long to wait for `mcp.json` when the app holds the lock but has not written it yet.
    pub info_wait: Duration,
    /// How long the app has to answer the hello.
    pub hello_timeout: Duration,
}

impl ClientOptions {
    pub fn new(client: impl Into<String>, roots: Vec<String>) -> ClientOptions {
        ClientOptions {
            client: client.into(),
            roots,
            info_wait: Duration::from_secs(2),
            hello_timeout: Duration::from_secs(10),
        }
    }
}

/// A connection that has said hello.
pub struct Link {
    pub reader: BufReader<Stream>,
    pub writer: Stream,
    /// The version of the app.
    pub app: String,
    /// The run folder the app was found in.
    pub dir: PathBuf,
}

/// Looks in each of `candidates`, in order, and connects to the first app that answers. Every folder is
/// looked at, because the assistant may have been started with another environment than the app. When
/// nothing answers the error names the folders and the reason for each. An app that answers but refuses
/// (busy, a bad root) ends the search with its message.
pub fn connect(candidates: &[PathBuf], options: &ClientOptions) -> Result<Link, String> {
    let mut looked: Vec<(PathBuf, String)> = Vec::new();
    for dir in candidates {
        let info = match discovery::probe(dir, options.info_wait) {
            Ok(i) => i,
            Err(why) => {
                looked.push((dir.clone(), why));
                continue;
            }
        };
        let stream = match Stream::connect(&info.endpoint) {
            Ok(s) => s,
            Err(e) => {
                looked.push((dir.clone(), format!("could not connect: {e}")));
                continue;
            }
        };
        match hello(stream, &info.endpoint, options, dir) {
            Ok(link) => return Ok(link),
            Err(Refusal::Gone(why)) => looked.push((dir.clone(), why)),
            Err(Refusal::Said(message)) => return Err(message),
        }
    }
    Err(discovery::no_listener(&looked))
}

enum Refusal {
    /// The endpoint did not answer like the app: try the next folder.
    Gone(String),
    /// The app answered with an error.
    Said(String),
}

fn hello(
    stream: Stream,
    endpoint: &super::transport::Endpoint,
    options: &ClientOptions,
    dir: &Path,
) -> Result<Link, Refusal> {
    let mut writer = stream.try_clone().map_err(|e| Refusal::Gone(format!("could not connect: {e}")))?;
    let _ = writer.set_write_timeout(Some(Duration::from_secs(10)));
    let mut reader = BufReader::new(stream);
    let hello = Hello {
        client: options.client.clone(),
        roots: options.roots.clone(),
        token: endpoint.token().map(str::to_string),
    };
    frame::write_line(&mut writer, &hello.to_line()).map_err(|e| Refusal::Gone(format!("could not say hello: {e}")))?;
    let _ = reader.get_ref().set_read_timeout(Some(Duration::from_millis(250)));
    let deadline = Instant::now() + options.hello_timeout;
    let line = frame::read_line(&mut reader, REPLY_CAP, &mut || Instant::now() < deadline);
    // From here on a reader thread blocks without a limit.
    let _ = reader.get_ref().set_read_timeout(None);
    match line {
        Ok(Line::Text(l)) => match proto::parse_hello_reply(&l) {
            Ok(app) => Ok(Link { reader, writer, app, dir: dir.to_path_buf() }),
            Err(proto::HelloFail::Refused(message)) => Err(Refusal::Said(message)),
            Err(proto::HelloFail::NotSheaf(message)) => Err(Refusal::Gone(message)),
        },
        Ok(_) => Err(Refusal::Gone("closed the connection without an answer".to_string())),
        Err(e) => Err(Refusal::Gone(format!("did not answer the hello: {e}"))),
    }
}
