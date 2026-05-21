use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

fn binary_path() -> std::path::PathBuf {
    let mut p = std::env::current_exe().expect("test exe path");
    p.pop();
    if p.ends_with("deps") {
        p.pop();
    }
    p.push(if cfg!(windows) {
        "substack-mcp.exe"
    } else {
        "substack-mcp"
    });
    p
}

struct Server {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    stdout: BufReader<std::process::ChildStdout>,
}

impl Server {
    fn spawn() -> Self {
        let path = binary_path();
        assert!(path.exists(), "binary not built: {}", path.display());
        let mut child = Command::new(&path)
            .env("SUBSTACK_SID", "test-dummy-sid")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn substack-mcp");
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            stdin,
            stdout,
        }
    }

    fn send(&mut self, msg: &Value) {
        let line = serde_json::to_string(msg).unwrap();
        self.stdin.write_all(line.as_bytes()).unwrap();
        self.stdin.write_all(b"\n").unwrap();
        self.stdin.flush().unwrap();
    }

    fn send_raw(&mut self, raw: &str) {
        self.stdin.write_all(raw.as_bytes()).unwrap();
        self.stdin.write_all(b"\n").unwrap();
        self.stdin.flush().unwrap();
    }

    fn recv(&mut self) -> Value {
        let mut line = String::new();
        self.stdout.read_line(&mut line).unwrap();
        serde_json::from_str(line.trim()).expect("valid JSON response")
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let start = std::time::Instant::now();
        loop {
            match self.child.try_wait() {
                Ok(Some(_)) | Err(_) => break,
                Ok(None) if start.elapsed() > Duration::from_secs(2) => break,
                Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            }
        }
    }
}

#[test]
fn test_initialize() {
    let mut s = Server::spawn();
    s.send(&json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {"protocolVersion": "2024-11-05"}
    }));
    let resp = s.recv();
    assert_eq!(resp["jsonrpc"], "2.0");
    assert_eq!(resp["id"], 1);
    assert_eq!(
        resp["result"]["serverInfo"]["name"],
        "unofficial-substack-mcp"
    );
    assert!(resp["result"]["capabilities"]["tools"].is_object());
}

#[test]
fn test_tools_list() {
    let mut s = Server::spawn();
    s.send(&json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}));
    let resp = s.recv();
    let tools = resp["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 14);
    for tool in tools {
        assert!(tool.get("name").is_some());
        assert!(tool.get("inputSchema").is_some());
    }
}

#[test]
fn test_unknown_method() {
    let mut s = Server::spawn();
    s.send(&json!({"jsonrpc": "2.0", "id": 3, "method": "no/such/method"}));
    let resp = s.recv();
    assert_eq!(resp["error"]["code"], -32601);
}

#[test]
fn test_invalid_json() {
    let mut s = Server::spawn();
    s.send_raw("{this is not valid json");
    let resp = s.recv();
    assert_eq!(resp["error"]["code"], -32700);
}

#[test]
fn test_notification_no_response() {
    let mut s = Server::spawn();
    s.send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
    s.send(&json!({"jsonrpc": "2.0", "id": 99, "method": "tools/list"}));
    let resp = s.recv();
    assert_eq!(resp["id"], 99);
}
