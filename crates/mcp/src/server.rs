//! JSON-RPC 2.0 framing, the MCP methods and the state of the one project.

use crate::roots::Roots;
use crate::{resources, tools};
use serde_json::{Map, Value, json};
use sheaf_core::Project;
use sheaf_core::template::Template;
use std::collections::HashMap;
use std::path::PathBuf;

pub const NEWEST_PROTOCOL: &str = "2025-06-18";
pub const PROTOCOLS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];

pub const PARSE_ERROR: i64 = -32700;
pub const INVALID_REQUEST: i64 = -32600;
pub const METHOD_NOT_FOUND: i64 = -32601;
pub const INVALID_PARAMS: i64 = -32602;
pub const RESOURCE_NOT_FOUND: i64 = -32002;

/// A protocol error: the request itself was wrong, not the work it asked for.
#[derive(Debug, Clone, PartialEq)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
}

impl RpcError {
    pub fn new(code: i64, message: impl Into<String>) -> Self {
        RpcError { code, message: message.into() }
    }
}

pub struct Server {
    pub(crate) project: Project,
    pub(crate) roots: Roots,
    /// The file the project was opened from or last saved to.
    pub(crate) path: Option<PathBuf>,
    /// Origin projects this session read: saving over one keeps a `.bak` copy first.
    pub(crate) foreign: Vec<PathBuf>,
    /// The template each imported book suggested, by lower-case book name.
    pub(crate) suggested: HashMap<String, Template>,
    protocol: String,
}

fn error_reply(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

impl Server {
    pub fn new(roots: Roots) -> Server {
        Server {
            project: Project::new(),
            roots,
            path: None,
            foreign: Vec::new(),
            suggested: HashMap::new(),
            protocol: NEWEST_PROTOCOL.to_string(),
        }
    }

    /// Handles one line of input and returns the line to send back, if any.
    pub fn handle_line(&mut self, line: &str) -> Option<String> {
        let reply = match serde_json::from_str::<Value>(line) {
            Err(e) => Some(error_reply(Value::Null, PARSE_ERROR, &format!("parse error: {e}"))),
            Ok(Value::Array(items)) => {
                if items.is_empty() {
                    Some(error_reply(Value::Null, INVALID_REQUEST, "invalid request: an empty batch"))
                } else {
                    let replies: Vec<Value> = items.into_iter().filter_map(|m| self.handle_message(m)).collect();
                    if replies.is_empty() { None } else { Some(Value::Array(replies)) }
                }
            }
            Ok(message) => self.handle_message(message),
        };
        reply.map(|v| v.to_string())
    }

    /// Handles one JSON-RPC message. Notifications and replies from the client get no answer.
    pub fn handle_message(&mut self, message: Value) -> Option<Value> {
        let Some(obj) = message.as_object() else {
            return Some(error_reply(Value::Null, INVALID_REQUEST, "invalid request: not an object"));
        };
        let id = obj.get("id").cloned();
        if let Some(id) = &id
            && !(id.is_string() || id.is_number() || id.is_null())
        {
            return Some(error_reply(
                Value::Null,
                INVALID_REQUEST,
                "invalid request: the id must be a string or number",
            ));
        }
        let Some(method) = obj.get("method").and_then(Value::as_str) else {
            if obj.contains_key("result") || obj.contains_key("error") {
                return None;
            }
            return Some(error_reply(id.unwrap_or(Value::Null), INVALID_REQUEST, "invalid request: no method"));
        };
        if obj.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
            return Some(error_reply(
                id.unwrap_or(Value::Null),
                INVALID_REQUEST,
                "invalid request: jsonrpc must be \"2.0\"",
            ));
        }
        let params = match obj.get("params") {
            None | Some(Value::Null) => Value::Object(Map::new()),
            Some(p) if p.is_object() || p.is_array() => p.clone(),
            Some(_) => {
                return id.map(|id| error_reply(id, INVALID_PARAMS, "invalid params: must be an object"));
            }
        };
        let result = self.dispatch(method, &params);
        let id = id?;
        Some(match result {
            Ok(v) => json!({"jsonrpc": "2.0", "id": id, "result": v}),
            Err(e) => error_reply(id, e.code, &e.message),
        })
    }

    fn dispatch(&mut self, method: &str, params: &Value) -> Result<Value, RpcError> {
        match method {
            "initialize" => self.initialize(params),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({"tools": tools::list()})),
            "tools/call" => self.call_tool(params),
            "resources/list" => Ok(json!({"resources": resources::list(self)})),
            "resources/templates/list" => Ok(json!({"resourceTemplates": resources::templates()})),
            "resources/read" => {
                let uri = params
                    .get("uri")
                    .and_then(Value::as_str)
                    .ok_or_else(|| RpcError::new(INVALID_PARAMS, "invalid params: missing \"uri\""))?;
                resources::read(self, uri)
            }
            m if m.starts_with("notifications/") => Ok(json!({})),
            m => Err(RpcError::new(METHOD_NOT_FOUND, format!("method not found: {m}"))),
        }
    }

    fn initialize(&mut self, params: &Value) -> Result<Value, RpcError> {
        let asked = params
            .get("protocolVersion")
            .and_then(Value::as_str)
            .ok_or_else(|| RpcError::new(INVALID_PARAMS, "invalid params: missing \"protocolVersion\""))?;
        self.protocol = if PROTOCOLS.contains(&asked) { asked.to_string() } else { NEWEST_PROTOCOL.to_string() };
        Ok(json!({
            "protocolVersion": self.protocol,
            "capabilities": {
                "tools": {"listChanged": false},
                "resources": {"subscribe": false, "listChanged": false}
            },
            "serverInfo": {"name": "sheaf-mcp", "version": env!("CARGO_PKG_VERSION")},
            "instructions": self.instructions()
        }))
    }

    fn instructions(&self) -> String {
        let roots = if self.roots.is_empty() {
            "No folder is allowed (the server was started without --root), so tools that read or write files refuse."
                .to_string()
        } else {
            let list: Vec<String> =
                self.roots.dirs().iter().map(|d| crate::roots::plain(d.clone()).display().to_string()).collect();
            format!(
                "Files can be read and written only under: {}. Relative paths start at the first folder.",
                list.join(", ")
            )
        };
        format!(
            "Sheaf holds one project per server. Workbooks, sheets, columns and graphs are named by their names \
             (Book1, Sheet1, B, Graph1), never by numbers; rows count from 1. Start with import_files or \
             open_project, look with list_contents and read_columns, then plot, analyze, fit, export_graph \
             and save_project. {roots}"
        )
    }

    fn call_tool(&mut self, params: &Value) -> Result<Value, RpcError> {
        let name = params
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| RpcError::new(INVALID_PARAMS, "invalid params: missing the tool \"name\""))?;
        let args = match params.get("arguments") {
            None | Some(Value::Null) => Value::Object(Map::new()),
            Some(a) if a.is_object() => a.clone(),
            Some(_) => return Err(RpcError::new(INVALID_PARAMS, "invalid params: \"arguments\" must be an object")),
        };
        if !tools::exists(name) {
            return Err(RpcError::new(INVALID_PARAMS, format!("unknown tool: {name}")));
        }
        Ok(match tools::call(self, name, &args) {
            Ok(out) => out.into_result(self.protocol == NEWEST_PROTOCOL),
            Err(e) => e.into_result(self.protocol == NEWEST_PROTOCOL),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server() -> Server {
        Server::new(Roots::default())
    }

    fn reply(s: &mut Server, line: &str) -> Value {
        serde_json::from_str(&s.handle_line(line).expect("a reply")).unwrap()
    }

    fn code(v: &Value) -> i64 {
        v["error"]["code"].as_i64().unwrap_or_else(|| panic!("no error in {v}"))
    }

    #[test]
    fn malformed_json_is_a_parse_error() {
        let mut s = server();
        for bad in ["{", "not json", "{\"jsonrpc\":\"2.0\",", "[1,"] {
            let r = reply(&mut s, bad);
            assert_eq!(code(&r), PARSE_ERROR, "{bad}");
            assert_eq!(r["id"], Value::Null);
        }
    }

    #[test]
    fn unknown_methods_are_not_found() {
        let mut s = server();
        let r = reply(&mut s, r#"{"jsonrpc":"2.0","id":7,"method":"no/such"}"#);
        assert_eq!(code(&r), METHOD_NOT_FOUND);
        assert_eq!(r["id"], 7);
    }

    #[test]
    fn bad_params_are_invalid_params() {
        let mut s = server();
        let cases = [
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{}}"#,
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"no_such_tool"}}"#,
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"new_project","arguments":[1]}}"#,
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":"x"}"#,
            r#"{"jsonrpc":"2.0","id":1,"method":"resources/read","params":{}}"#,
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
        ];
        for c in cases {
            assert_eq!(code(&reply(&mut s, c)), INVALID_PARAMS, "{c}");
        }
    }

    #[test]
    fn invalid_requests_are_refused() {
        let mut s = server();
        assert_eq!(code(&reply(&mut s, "42")), INVALID_REQUEST);
        assert_eq!(code(&reply(&mut s, "[]")), INVALID_REQUEST);
        assert_eq!(code(&reply(&mut s, r#"{"jsonrpc":"2.0","id":1}"#)), INVALID_REQUEST);
        assert_eq!(code(&reply(&mut s, r#"{"jsonrpc":"1.0","id":1,"method":"ping"}"#)), INVALID_REQUEST);
        assert_eq!(code(&reply(&mut s, r#"{"jsonrpc":"2.0","id":[1],"method":"ping"}"#)), INVALID_REQUEST);
    }

    #[test]
    fn notifications_get_no_reply() {
        let mut s = server();
        assert!(s.handle_line(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#).is_none());
        assert!(s.handle_line(r#"{"jsonrpc":"2.0","method":"notifications/anything"}"#).is_none());
        // Even a request-shaped call without an id is a notification.
        assert!(s.handle_line(r#"{"jsonrpc":"2.0","method":"ping"}"#).is_none());
        assert!(s.handle_line(r#"{"jsonrpc":"2.0","method":"no/such"}"#).is_none());
        // A reply from the client is not answered.
        assert!(s.handle_line(r#"{"jsonrpc":"2.0","id":3,"result":{}}"#).is_none());
    }

    #[test]
    fn ping_and_batches() {
        let mut s = server();
        let r = reply(&mut s, r#"{"jsonrpc":"2.0","id":"a","method":"ping"}"#);
        assert_eq!(r["result"], json!({}));
        assert_eq!(r["id"], "a");
        let r = reply(
            &mut s,
            r#"[{"jsonrpc":"2.0","id":1,"method":"ping"},{"jsonrpc":"2.0","method":"notifications/initialized"},{"jsonrpc":"2.0","id":2,"method":"x"}]"#,
        );
        let list = r.as_array().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0]["id"], 1);
        assert_eq!(code(&list[1]), METHOD_NOT_FOUND);
        assert!(s.handle_line(r#"[{"jsonrpc":"2.0","method":"notifications/initialized"}]"#).is_none());
    }

    #[test]
    fn initialize_negotiates_the_version() {
        let mut s = server();
        let init = |v: &str| {
            format!(r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"protocolVersion":"{v}"}}}}"#)
        };
        for v in PROTOCOLS {
            assert_eq!(reply(&mut s, &init(v))["result"]["protocolVersion"], v);
        }
        let r = reply(&mut s, &init("1999-01-01"));
        assert_eq!(r["result"]["protocolVersion"], NEWEST_PROTOCOL);
        assert_eq!(r["result"]["serverInfo"]["name"], "sheaf-mcp");
        assert_eq!(r["result"]["capabilities"]["tools"]["listChanged"], false);
        assert!(r["result"]["instructions"].as_str().unwrap().contains("--root"));
    }

    #[test]
    fn tools_list_has_every_tool_with_a_schema() {
        let mut s = server();
        let r = reply(&mut s, r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#);
        let tools = r["result"]["tools"].as_array().unwrap();
        assert!(tools.len() >= 16);
        for t in tools {
            assert!(t["name"].is_string() && t["description"].is_string(), "{t}");
            assert_eq!(t["inputSchema"]["type"], "object", "{t}");
            assert!(t["annotations"]["readOnlyHint"].is_boolean(), "{t}");
        }
    }

    #[test]
    fn a_failing_tool_is_a_tool_error_not_a_protocol_error() {
        let mut s = server();
        let r = reply(
            &mut s,
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"set_column_values","arguments":{"column":"B"}}}"#,
        );
        assert!(r.get("error").is_none(), "{r}");
        assert_eq!(r["result"]["isError"], true);
        assert!(r["result"]["content"][0]["text"].as_str().unwrap().contains("missing field \"formula\""), "{r}");
    }

    #[test]
    fn file_tools_refuse_without_a_root() {
        let mut s = server();
        let r = reply(
            &mut s,
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"open_project","arguments":{"path":"a.opj"}}}"#,
        );
        assert_eq!(r["result"]["isError"], true);
        assert!(r["result"]["content"][0]["text"].as_str().unwrap().contains("--root"));
    }
}
