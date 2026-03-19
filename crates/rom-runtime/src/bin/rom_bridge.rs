use rom_runtime::parse_bridge_request;
use std::{
    io::{self, Read, Write},
    process::ExitCode,
};

fn main() -> ExitCode {
    let mut input = String::new();
    if let Err(error) = io::stdin().read_to_string(&mut input) {
        write_text(
            &serde_json::to_string(&serde_json::json!({
                "ok": false,
                "result": null,
                "error": error.to_string(),
            }))
            .expect("serialize stdin error"),
        );
        return ExitCode::from(1);
    }

    let response = parse_bridge_request(&input);
    let serialized = serde_json::to_string(&response).expect("serialize bridge response");
    let exit_code = if response.ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    };
    write_text(&serialized);
    exit_code
}

fn write_text(text: &str) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    handle
        .write_all(text.as_bytes())
        .expect("write bridge response");
    handle.write_all(b"\n").expect("write trailing newline");
}
