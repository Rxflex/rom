use rom_runtime::{RomRuntime, RuntimeConfig};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    io::{self, Read, Write},
    process::ExitCode,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum BridgeCommand {
    Eval,
    EvalAsync,
    SurfaceSnapshot,
    FingerprintProbe,
    FingerprintJsHarness,
    FingerprintJsVersion,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct BridgeRequest {
    command: Option<BridgeCommand>,
    config: RuntimeConfig,
    script: Option<String>,
}

impl Default for BridgeRequest {
    fn default() -> Self {
        Self {
            command: None,
            config: RuntimeConfig::default(),
            script: None,
        }
    }
}

#[derive(Debug, Serialize)]
struct BridgeResponse {
    ok: bool,
    result: Option<Value>,
    error: Option<String>,
}

fn main() -> ExitCode {
    match run() {
        Ok(response) => {
            write_response(&response);
            ExitCode::SUCCESS
        }
        Err(error) => {
            write_response(&BridgeResponse {
                ok: false,
                result: None,
                error: Some(error),
            });
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<BridgeResponse, String> {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|error| error.to_string())?;

    let request: BridgeRequest = serde_json::from_str(&input).map_err(|error| error.to_string())?;
    let command = request
        .command
        .ok_or_else(|| "Missing bridge command.".to_owned())?;
    let runtime = RomRuntime::new(request.config).map_err(|error| error.to_string())?;

    let result = match command {
        BridgeCommand::Eval => Value::String(
            runtime
                .eval_as_string(&required_script(request.script)?)
                .map_err(|error| error.to_string())?,
        ),
        BridgeCommand::EvalAsync => Value::String(
            runtime
                .eval_async_as_string(&required_script(request.script)?)
                .map_err(|error| error.to_string())?,
        ),
        BridgeCommand::SurfaceSnapshot => serde_json::to_value(
            runtime
                .surface_snapshot()
                .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?,
        BridgeCommand::FingerprintProbe => serde_json::to_value(
            runtime
                .fingerprint_probe()
                .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?,
        BridgeCommand::FingerprintJsHarness => serde_json::to_value(
            runtime
                .run_fingerprintjs_harness()
                .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?,
        BridgeCommand::FingerprintJsVersion => Value::String(runtime.fingerprintjs_version().into()),
    };

    Ok(BridgeResponse {
        ok: true,
        result: Some(result),
        error: None,
    })
}

fn required_script(script: Option<String>) -> Result<String, String> {
    script.ok_or_else(|| "Missing script for bridge command.".to_owned())
}

fn write_response(response: &BridgeResponse) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    serde_json::to_writer(&mut handle, response).expect("serialize bridge response");
    handle.write_all(b"\n").expect("write trailing newline");
}
