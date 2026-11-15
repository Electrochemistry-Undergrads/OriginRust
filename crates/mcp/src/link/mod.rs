//! What the desktop app and the `--attach` proxy share: the link between them (docs/spec/mcp.md
//! section 11) and the sanitizer for text that came over the wire.
//!
//! - [`frame`]: one JSON message per line, with a cap.
//! - [`proto`]: the hello, the requests, the replies and the notices.
//! - [`transport`]: the Unix socket and the loopback TCP port.
//! - [`discovery`]: the run folder, the lock and `mcp.json`.
//! - [`host`]: the app's side. [`client`]: the proxy's side.

pub mod client;
pub mod discovery;
pub mod frame;
pub mod host;
pub mod proto;
pub mod text;
pub mod transport;

pub use text::sanitize;
