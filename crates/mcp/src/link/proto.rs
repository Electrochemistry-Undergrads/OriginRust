//! The messages of the link between `sheaf-mcp --attach` and the desktop app (docs/spec/mcp.md sections
//! 11.3 and 11.4): the hello, the requests and the replies.

use serde_json::{Value, json};

/// The version of the link protocol.
pub const VERSION: u64 = 1;

/// Most connections the app keeps open, those that have not finished their hello included.
pub const MAX_CONNECTIONS: usize = 8;

/// Most folders a client may name in its hello (repeats are dropped first).
pub const MAX_ROOTS: usize = 16;

/// Protocol failure codes of a link reply.
pub const CODE_PARAMS: i64 = -32602;
pub const CODE_CANCELLED: i64 = -32800;
pub const CODE_OTHER: i64 = -32000;

pub const REPLACED: &str = "the project in Sheaf was replaced";
pub const CLOSED: &str = "the owner turned the link off";
pub const BUSY_APP: &str = "Sheaf is busy with other assistants; try again";
pub const OWNER_BUSY: &str = "the owner is busy or has a dialog open in Sheaf; the call did not run; try again";
pub const STILL_RUNNING: &str = "still running in Sheaf; check list_contents before retrying";
pub const LOST: &str = "connection to Sheaf lost; check the project before retrying";
pub const REPLACED_FOR_ASSISTANT: &str =
    "the project in Sheaf was replaced; look at it again with list_contents before changing anything";
pub const CLOSED_FOR_ASSISTANT: &str = "the owner turned the link off";
pub const NOT_ALLOWED: &str = "not allowed";
pub const ONE_CALL: &str = "a call is already in flight on this connection";

/// What a client says first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hello {
    pub client: String,
    /// Absolute folders the client may use.
    pub roots: Vec<String>,
    /// The token of a TCP endpoint; `None` on a Unix socket.
    pub token: Option<String>,
}

impl Hello {
    pub fn to_line(&self) -> String {
        let mut hello = json!({"v": VERSION, "client": self.client, "roots": self.roots});
        if let Some(t) = &self.token {
            hello["token"] = Value::String(t.clone());
        }
        json!({"hello": hello}).to_string()
    }

    /// Reads the hello line. `Err` is the reason, and it is `None` when the line is no hello at all, in
    /// which case the connection is closed without a reply.
    pub fn parse(line: &str) -> Result<Hello, Option<String>> {
        let value: Value = serde_json::from_str(line).map_err(|_| None)?;
        let hello = value.get("hello").filter(|h| h.is_object()).ok_or(None)?;
        if hello.get("v").and_then(Value::as_u64) != Some(VERSION) {
            return Err(Some(format!("unsupported link version (this Sheaf speaks version {VERSION})")));
        }
        let client = hello.get("client").and_then(Value::as_str).unwrap_or_default().to_string();
        let roots = match hello.get("roots") {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::Array(a)) => {
                let mut out: Vec<String> = Vec::new();
                for r in a {
                    match r.as_str() {
                        Some(s) if out.iter().any(|o| o == s) => {}
                        Some(_) if out.len() >= MAX_ROOTS => {
                            return Err(Some(format!("too many roots (at most {MAX_ROOTS})")));
                        }
                        Some(s) => out.push(s.to_string()),
                        None => return Err(Some("the roots must be a list of paths".to_string())),
                    }
                }
                out
            }
            Some(_) => return Err(Some("the roots must be a list of paths".to_string())),
        };
        let token = hello.get("token").and_then(Value::as_str).map(str::to_string);
        Ok(Hello { client, roots, token })
    }
}

/// The reply to a good hello.
pub fn hello_reply(app: &str) -> String {
    json!({"hello": {"v": VERSION, "app": app}}).to_string()
}

/// Why a hello reply is no welcome.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HelloFail {
    /// The app said no, with this message.
    Refused(String),
    /// The endpoint did not answer like the app.
    NotSheaf(String),
}

/// The app's version from a hello reply.
pub fn parse_hello_reply(line: &str) -> Result<String, HelloFail> {
    let other = || HelloFail::NotSheaf("the endpoint did not answer like Sheaf".to_string());
    let value: Value = serde_json::from_str(line).map_err(|_| other())?;
    if let Some(e) = value.get("error") {
        let message = e.get("message").and_then(Value::as_str).unwrap_or("Sheaf refused the connection");
        return Err(HelloFail::Refused(message.to_string()));
    }
    match value.get("hello") {
        Some(h) if h.get("v").and_then(Value::as_u64) == Some(VERSION) => {
            Ok(h.get("app").and_then(Value::as_str).unwrap_or_default().to_string())
        }
        Some(_) => Err(HelloFail::NotSheaf("Sheaf speaks another version of the link; update sheaf-mcp".to_string())),
        None => Err(other()),
    }
}

/// One request of a client after the hello.
#[derive(Clone, Debug, PartialEq)]
pub enum Request {
    Call { id: u64, name: String, args: Value, structured: bool },
    Cancel { id: u64, target: u64 },
    ResourcesList { id: u64 },
    ResourceRead { id: u64, uri: String },
}

impl Request {
    pub fn id(&self) -> u64 {
        match self {
            Request::Call { id, .. }
            | Request::Cancel { id, .. }
            | Request::ResourcesList { id }
            | Request::ResourceRead { id, .. } => *id,
        }
    }

    pub fn to_line(&self) -> String {
        match self {
            Request::Call { id, name, args, structured } => {
                json!({"id": id, "op": "call", "name": name, "args": args, "structured": structured})
            }
            Request::Cancel { id, target } => json!({"id": id, "op": "cancel", "target": target}),
            Request::ResourcesList { id } => json!({"id": id, "op": "resources_list"}),
            Request::ResourceRead { id, uri } => json!({"id": id, "op": "resource_read", "uri": uri}),
        }
        .to_string()
    }

    /// Reads a request line. `Err` holds the id, when there was one, and the reason.
    pub fn parse(line: &str) -> Result<Request, (Option<u64>, String)> {
        let value: Value = serde_json::from_str(line).map_err(|e| (None, format!("parse error: {e}")))?;
        let id = value.get("id").and_then(Value::as_u64);
        let bad = |why: &str| (id, why.to_string());
        let Some(id) = id else { return Err(bad("the request needs a number \"id\"")) };
        match value.get("op").and_then(Value::as_str) {
            Some("call") => {
                let name = value.get("name").and_then(Value::as_str).ok_or_else(|| bad("a call needs a \"name\""))?;
                let args = match value.get("args") {
                    None | Some(Value::Null) => json!({}),
                    Some(a) if a.is_object() => a.clone(),
                    Some(_) => return Err(bad("\"args\" must be an object")),
                };
                let structured = value.get("structured").and_then(Value::as_bool).unwrap_or(false);
                Ok(Request::Call { id, name: name.to_string(), args, structured })
            }
            Some("cancel") => {
                let target =
                    value.get("target").and_then(Value::as_u64).ok_or_else(|| bad("a cancel needs a \"target\""))?;
                Ok(Request::Cancel { id, target })
            }
            Some("resources_list") => Ok(Request::ResourcesList { id }),
            Some("resource_read") => {
                let uri = value.get("uri").and_then(Value::as_str).ok_or_else(|| bad("a read needs a \"uri\""))?;
                Ok(Request::ResourceRead { id, uri: uri.to_string() })
            }
            Some(other) => Err(bad(&format!("unknown operation: {other}"))),
            None => Err(bad("the request needs an \"op\"")),
        }
    }
}

/// `{"id":N,"result":...}`
pub fn result_line(id: u64, result: &Value) -> String {
    json!({"id": id, "result": result}).to_string()
}

/// `{"id":N,"error":{"code":C,"message":"..."}}`; a line with no request behind it has no id.
pub fn error_line(id: Option<u64>, code: i64, message: &str) -> String {
    match id {
        Some(id) => json!({"id": id, "error": {"code": code, "message": message}}),
        None => json!({"error": {"code": code, "message": message}}),
    }
    .to_string()
}

/// What an app line to the client says when it answers no request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoticeKind {
    /// The owner replaced the project.
    Replaced,
    /// The owner turned the link off.
    Closed,
}

impl NoticeKind {
    pub fn name(self) -> &'static str {
        match self {
            NoticeKind::Replaced => "replaced",
            NoticeKind::Closed => "closed",
        }
    }

    pub fn message(self) -> &'static str {
        match self {
            NoticeKind::Replaced => REPLACED,
            NoticeKind::Closed => CLOSED,
        }
    }

    pub fn parse(name: &str) -> Option<NoticeKind> {
        match name {
            "replaced" => Some(NoticeKind::Replaced),
            "closed" => Some(NoticeKind::Closed),
            _ => None,
        }
    }

    /// What the assistant is told when the notice reaches a call.
    pub fn for_assistant(self) -> &'static str {
        match self {
            NoticeKind::Replaced => REPLACED_FOR_ASSISTANT,
            NoticeKind::Closed => CLOSED_FOR_ASSISTANT,
        }
    }
}

pub fn notice_line(kind: NoticeKind) -> String {
    json!({"notice": {"kind": kind.name(), "message": kind.message()}}).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hello_survives_a_round_trip_and_a_bad_one_says_why() {
        let h = Hello { client: "claude 1".into(), roots: vec!["/a".into(), "/b".into()], token: Some("t".into()) };
        assert_eq!(Hello::parse(&h.to_line()), Ok(h));
        assert_eq!(Hello::parse("GET / HTTP/1.1"), Err(None));
        assert_eq!(Hello::parse(r#"{"id":1,"op":"call"}"#), Err(None));
        assert!(Hello::parse(r#"{"hello":{"v":7}}"#).unwrap_err().unwrap().contains("version"));
        assert!(Hello::parse(r#"{"hello":{"v":1,"roots":"/a"}}"#).unwrap_err().is_some());
        assert!(Hello::parse(r#"{"hello":{"v":1,"roots":[1]}}"#).unwrap_err().is_some());
        let h = Hello::parse(r#"{"hello":{"v":1}}"#).unwrap();
        assert!(h.roots.is_empty() && h.client.is_empty() && h.token.is_none());
    }

    #[test]
    fn requests_round_trip_and_bad_ones_name_the_problem() {
        let calls = [
            Request::Call { id: 1, name: "plot".into(), args: json!({"y": ["B"]}), structured: true },
            Request::Cancel { id: 2, target: 1 },
            Request::ResourcesList { id: 3 },
            Request::ResourceRead { id: 4, uri: "sheaf://project".into() },
        ];
        for r in calls {
            assert_eq!(Request::parse(&r.to_line()), Ok(r.clone()));
            assert_eq!(r.id(), Request::parse(&r.to_line()).unwrap().id());
        }
        for (line, id) in [
            (r#"{"op":"call","name":"x"}"#, None),
            (r#"{"id":1}"#, Some(1)),
            (r#"{"id":1,"op":"call"}"#, Some(1)),
            (r#"{"id":1,"op":"call","name":"x","args":[1]}"#, Some(1)),
            (r#"{"id":1,"op":"cancel"}"#, Some(1)),
            (r#"{"id":1,"op":"resource_read"}"#, Some(1)),
            (r#"{"id":1,"op":"dance"}"#, Some(1)),
            ("nonsense", None),
        ] {
            assert_eq!(Request::parse(line).unwrap_err().0, id, "{line}");
        }
    }

    #[test]
    fn a_hello_reply_is_read_back_and_an_error_line_gives_its_message() {
        assert_eq!(parse_hello_reply(&hello_reply("1.2")), Ok("1.2".to_string()));
        assert_eq!(
            parse_hello_reply(&error_line(None, CODE_OTHER, "not allowed")),
            Err(HelloFail::Refused("not allowed".to_string()))
        );
        assert!(matches!(parse_hello_reply("<html>"), Err(HelloFail::NotSheaf(_))));
        assert!(matches!(parse_hello_reply(r#"{"hello":{"v":9}}"#), Err(HelloFail::NotSheaf(_))));
    }
}
