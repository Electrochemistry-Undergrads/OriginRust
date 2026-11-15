//! `sheaf-mcp`: the MCP server. Protocol messages go to stdout, one per line; logs go to stderr.

use sheaf_mcp::Server;
use sheaf_mcp::link::frame::{self, Line, REQUEST_CAP};
use sheaf_mcp::proxy::{self, ProxyConfig};
use sheaf_mcp::roots::Roots;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

const HELP: &str = "sheaf-mcp: Model Context Protocol server for Sheaf, over stdin and stdout.

Usage: sheaf-mcp [--attach] [--root DIR]...

Options:
  --attach     Do not open a project of its own: drive the project open in the Sheaf
               app, which must have Tools > AI Assistants (MCP) turned on.
  --root DIR   A folder the server may read and write (repeatable). Without one,
               tools that read or write files refuse.
  --version    Print the version.
  --help       Print this text.

Add it to Claude Code with:  claude mcp add sheaf -- sheaf-mcp --root ~/Data
To drive the open app instead:  claude mcp add sheaf -- sheaf-mcp --attach --root ~/Data";

fn main() -> ExitCode {
    let mut roots: Vec<PathBuf> = Vec::new();
    let mut attach = false;
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
            "--attach" => attach = true,
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
    if attach {
        let stdout = std::io::stdout();
        let mut out = stdout.lock();
        let code = proxy::run(ProxyConfig::new(&roots), std::io::BufReader::new(std::io::stdin()), &mut out);
        return ExitCode::from(code as u8);
    }
    let mut server = Server::new(roots);
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    loop {
        let reply = match frame::read_line(&mut input, REQUEST_CAP, &mut || true) {
            Ok(Line::Eof) => return ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("sheaf-mcp: reading stdin failed: {e}");
                return ExitCode::SUCCESS;
            }
            Ok(Line::TooLong) => Some(
                serde_json::json!({"jsonrpc": "2.0", "id": null,
                    "error": {"code": -32600, "message": "invalid request: the line is longer than 8 MB"}})
                .to_string(),
            ),
            Ok(Line::Text(line)) => {
                let line = line.trim();
                if line.is_empty() { None } else { server.handle_line(line) }
            }
        };
        if let Some(r) = reply
            && (writeln!(out, "{r}").and_then(|()| out.flush())).is_err()
        {
            return ExitCode::SUCCESS;
        }
    }
}
