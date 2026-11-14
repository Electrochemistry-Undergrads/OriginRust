//! `sheaf-mcp`: a Model Context Protocol server over stdio. It is a thin layer over `sheaf-ops`,
//! which the command line uses too. See `docs/spec/mcp.md`.

pub mod analysis_tools;
pub mod b64;
pub mod resources;
pub mod roots;
pub mod schemas;
pub mod server;
pub mod tools;

pub use server::Server;
