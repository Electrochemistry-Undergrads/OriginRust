# T25 follow-up: protocol limits, CHI binary import and filesystem regressions

**Status:** done and validated on Windows, 2026-11-14, branch
`codex/mcp-protocol-coverage`. This supplement covers the protocol-test leftovers in
[T25](T25-mcp-server.md); desktop attachment and additional analysis tools remain separate work.
**Size:** S.

## Goal

Exercise the real `sheaf-mcp` binary over stdin/stdout for binary CHI imports, preservation of
an original `.opj`, the request-line limit, and Windows directory junctions. Tests generate
their own synthetic inputs and use private temporary directories.

## Origin facts

None needed for the MCP protocol. Binary CHI fixtures follow
[`chi-binary-format.md`](../spec/chi-binary-format.md), without including instrument files or
the owner's data. The backup rule is the existing rule in ADR 0008.

## Changes

- Exact 8 MiB requests are accepted with LF or EOF termination. A request one byte over the
  cap is rejected with JSON-RPC error `-32600`, including at EOF. An oversized line is drained
  before processing the next request. Request sizes count bytes before LF.
- Synthetic CV binary import detects content under a `.dat` extension, splits two scans,
  exposes X/current/mA columns and formulas, and retains values after save/reopen.
- Two-channel i-t import preserves interleaved channel order, reconstructs time from zero,
  derives mA and trapezoidal charge, and honors inversion and derived-column options.
- CP binary import preserves time and potential, units, and the disable-segment-splitting option.
- Opening or importing an `.opj` and saving over it preserves the original bytes in `.opj.bak`.
  Subsequent saves, including from another server process, retain the first backup.
- Windows junctions pointing outside the allowed root reject project open/save, file import,
  and table/graph export. Rejected calls leave the outside files and the current data intact.
  Junctions resolving inside the root still permit import and save. Tests remove their junctions
  before cleaning temporary directories and do not require symlink privileges.

## Where

- `crates/mcp/tests/protocol_regressions.rs`: integration tests using the real server binary.
- `crates/mcp/src/main.rs`: request-line length enforcement at EOF.

## Validation

All checks pass on Windows with Rust 1.99.1:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`: 1,910 passed, zero failed, two ignored, including all eight new
  protocol regressions and the Windows junction test.

The EOF regression was observed failing against the original server, which accepted the
oversized request, before applying the request-length correction. No dependency changes.

## Don't

Do not include the owner's instrument data, invoke a printer or Origin, or change other sessions'
analysis, Excel, autosave or desktop files to complete these checks.
