//! Spawns the `jb-mcp` binary and speaks newline-delimited JSON-RPC to it,
//! as an MCP client would over stdio.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

struct Client {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

impl Client {
    fn spawn() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_jb-mcp"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("spawn jb-mcp");
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Self { child, stdin, stdout, next_id: 1 }
    }

    fn send(&mut self, msg: Value) {
        writeln!(self.stdin, "{msg}").unwrap();
        self.stdin.flush().unwrap();
    }

    /// Sends a request and waits for the response with the same id.
    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        loop {
            let mut line = String::new();
            assert!(self.stdout.read_line(&mut line).unwrap() > 0, "server closed stdout");
            let msg: Value = serde_json::from_str(&line).unwrap_or_else(|e| panic!("non-JSON line {e}: {line}"));
            if msg["id"] == id {
                assert!(msg.get("error").is_none(), "JSON-RPC error: {msg}");
                return msg["result"].clone();
            }
        }
    }

    fn call(&mut self, tool: &str, args: Value) -> Value {
        self.request("tools/call", json!({ "name": tool, "arguments": args }))
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn initialize_list_execute_undo() {
    let dir = tempfile::tempdir().unwrap();
    let mut c = Client::spawn();

    let init = c.request(
        "initialize",
        json!({ "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "test", "version": "0" } }),
    );
    assert_eq!(init["serverInfo"]["name"], "jb-mcp");
    assert!(init["capabilities"]["tools"].is_object());
    assert!(init["capabilities"]["resources"].is_object());
    c.send(json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));

    let tools = c.request("tools/list", json!({}));
    let names: Vec<&str> = tools["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
    for expected in [
        "open_project",
        "save_project",
        "get_project",
        "list_media",
        "import_media",
        "execute_command",
        "undo",
        "redo",
        "evaluate_frame",
        "find_clips",
        "render",
    ] {
        assert!(names.contains(&expected), "missing tool {expected}");
    }
    let exec = tools["tools"].as_array().unwrap().iter().find(|t| t["name"] == "execute_command").unwrap();
    let schema = &exec["inputSchema"];
    assert!(schema["properties"]["command"]["oneOf"].as_array().unwrap().len() > 10);
    assert!(schema["$defs"]["Clip"].is_object(), "Command schema definitions are embedded");

    // Without an open project, tools return actionable errors as results.
    let r = c.call("get_project", json!({}));
    assert_eq!(r["isError"], true);
    assert_eq!(r["structuredContent"]["error"]["code"], "no_project_open");

    let path = dir.path().join("p.jbp");
    let r = c.call("new_project", json!({ "path": path, "width": 640, "height": 360, "fps": "25" }));
    assert_ne!(r["isError"], true, "{r}");

    let r = c.call("execute_command", json!({ "command": { "op": "add_track", "kind": "video" } }));
    assert_ne!(r["isError"], true, "{r}");
    let track = r["structuredContent"]["created"]["tracks"][0].as_str().unwrap().to_string();
    assert_eq!(r["structuredContent"]["session"]["can_undo"], true);

    let r = c.call(
        "execute_command",
        json!({ "command": { "op": "add_clip", "track_id": track, "clip": {
            "name": "title card", "source": { "type": "color", "color": [0, 0, 1, 1] }, "start": 0, "end": 30_000_000 } } }),
    );
    assert_ne!(r["isError"], true, "{r}");
    let found = c.call("find_clips", json!({ "name": "title", "at": "00:00:02" }));
    assert_eq!(found["structuredContent"]["count"], 1);

    let r = c.call("undo", json!({}));
    assert_eq!(r["structuredContent"]["removed"]["clips"].as_array().unwrap().len(), 1);
    let r = c.call("undo", json!({}));
    assert_eq!(r["structuredContent"]["removed"]["tracks"][0], track.as_str());
    assert_eq!(r["structuredContent"]["session"]["can_undo"], false);
    let r = c.call("undo", json!({}));
    assert_eq!(r["isError"], true);
    assert_eq!(r["structuredContent"]["error"]["code"], "nothing_to_undo");

    // Invalid commands explain what is accepted.
    let r = c.call("execute_command", json!({ "command": { "op": "explode" } }));
    assert_eq!(r["isError"], true);
    let err = &r["structuredContent"]["error"];
    assert_eq!(err["code"], "parse_error");
    assert!(err["message"].as_str().unwrap().contains("add_clip"));

    let res = c.request("resources/read", json!({ "uri": "jb://schema/command" }));
    let text = res["contents"][0]["text"].as_str().unwrap();
    assert!(serde_json::from_str::<Value>(text).unwrap()["oneOf"].is_array());
}
