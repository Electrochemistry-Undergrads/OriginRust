//! `sheaf-mcp`: the MCP server. Protocol messages go to stdout, one per line; logs go to stderr.

use sheaf_mcp::Server;
use sheaf_mcp::roots::Roots;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;

/// Longest request line accepted.
const LINE_CAP: usize = 8 * 1024 * 1024;

const HELP: &str = "sheaf-mcp: Model Context Protocol server for Sheaf, over stdin and stdout.

Usage: sheaf-mcp [--root DIR]...

Options:
  --root DIR   A folder the server may read and write (repeatable). Without one,
               tools that read or write files refuse.
  --version    Print the version.
  --help       Print this text.

Add it to Claude Code with:  claude mcp add sheaf -- sheaf-mcp --root ~/Data";

fn main() -> ExitCode {
    let mut roots: Vec<PathBuf> = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--help" | "-h" => {
                println!("{HELP}");
                return ExitCode::SUCCESS;
            }
            "--version" | "-V" => {
                println!("sheaf-mcp {}", env!("CARGO_PKG_VERSION"));
                return ExitCode::SUCCESS;
            }
            "--root" => match args.next() {
                Some(d) => roots.push(PathBuf::from(d)),
                None => {
                    eprintln!("sheaf-mcp: --root needs a folder");
                    return ExitCode::from(2);
                }
            },
            other => match other.strip_prefix("--root=") {
                Some(d) => roots.push(PathBuf::from(d)),
                None => {
                    eprintln!("sheaf-mcp: unknown argument '{other}' (see --help)");
                    return ExitCode::from(2);
                }
            },
        }
    }
    let roots = match Roots::new(&roots) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("sheaf-mcp: {e}");
            return ExitCode::from(2);
        }
    };
    if roots.is_empty() {
        eprintln!("sheaf-mcp: no --root given; tools that read or write files will refuse");
    }
    let mut server = Server::new(roots);
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let mut buf = Vec::new();
    loop {
        buf.clear();
        // Read a line, but never keep more than the cap.
        let mut too_long = false;
        loop {
            let chunk = match input.fill_buf() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("sheaf-mcp: reading stdin failed: {e}");
                    return ExitCode::SUCCESS;
                }
            };
            if chunk.is_empty() {
                break;
            }
            let (take, done) = match chunk.iter().position(|b| *b == b'\n') {
                Some(i) => (i + 1, true),
                None => (chunk.len(), false),
            };
            if buf.len() + take <= LINE_CAP + 1 {
                buf.extend_from_slice(&chunk[..take]);
            } else {
                too_long = true;
            }
            input.consume(take);
            if done {
                break;
            }
        }
        if buf.is_empty() && !too_long {
            return ExitCode::SUCCESS;
        }
        let reply = if too_long {
            Some(
                serde_json::json!({"jsonrpc": "2.0", "id": null,
                    "error": {"code": -32600, "message": "invalid request: the line is longer than 8 MB"}})
                .to_string(),
            )
        } else {
            let line = String::from_utf8_lossy(&buf);
            let line = line.trim();
            if line.is_empty() { None } else { server.handle_line(line) }
        };
        if let Some(r) = reply
            && (writeln!(out, "{r}").and_then(|()| out.flush())).is_err()
        {
            return ExitCode::SUCCESS;
        }
    }
}
