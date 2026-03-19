use serde_json::{Value, json};
use std::{
    io::Write,
    process::{Command, Stdio},
};

fn run_bridge(payload: Value) -> (bool, Value) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rom_bridge"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    serde_json::to_writer(child.stdin.as_mut().unwrap(), &payload).unwrap();
    child.stdin.as_mut().unwrap().write_all(b"\n").unwrap();

    let output = child.wait_with_output().unwrap();
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    (output.status.success(), value)
}

#[test]
fn bridge_evaluates_async_scripts() {
    let (success, value) = run_bridge(json!({
        "command": "eval-async",
        "script": r#"
            (async () => {
                await Promise.resolve();
                return location.href;
            })()
        "#,
    }));

    assert!(success);
    assert_eq!(value["ok"], true);
    assert_eq!(value["result"], "https://rom.local/");
}

#[test]
fn bridge_returns_surface_snapshot() {
    let (success, value) = run_bridge(json!({
        "command": "surface-snapshot",
        "config": {
            "href": "https://example.test/app",
            "user_agent": "ROM Test Agent",
        }
    }));

    assert!(success);
    assert_eq!(value["ok"], true);
    assert_eq!(value["result"]["globals"]["window"], true);
    assert_eq!(value["result"]["navigator"]["user_agent"], "ROM Test Agent");
}

#[test]
fn bridge_reports_protocol_errors() {
    let (success, value) = run_bridge(json!({
        "command": "eval",
    }));

    assert!(!success);
    assert_eq!(value["ok"], false);
    assert_eq!(value["error"], "Missing script for bridge command.");
}
