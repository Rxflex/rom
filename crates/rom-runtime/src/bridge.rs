use crate::{RomRuntime, RuntimeConfig};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BridgeCommand {
    Eval,
    EvalAsync,
    SurfaceSnapshot,
    FingerprintProbe,
    FingerprintJsHarness,
    FingerprintJsVersion,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct BridgeRequest {
    pub command: Option<BridgeCommand>,
    pub config: RuntimeConfig,
    pub script: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeResponse {
    pub ok: bool,
    pub result: Option<Value>,
    pub error: Option<String>,
}

impl BridgeResponse {
    fn success(result: Value) -> Self {
        Self {
            ok: true,
            result: Some(result),
            error: None,
        }
    }

    fn failure(error: String) -> Self {
        Self {
            ok: false,
            result: None,
            error: Some(error),
        }
    }
}

pub fn execute_bridge_request(request: BridgeRequest) -> BridgeResponse {
    match try_execute_bridge_request(request) {
        Ok(result) => BridgeResponse::success(result),
        Err(error) => BridgeResponse::failure(error),
    }
}

pub fn parse_bridge_request(input: &str) -> BridgeResponse {
    match serde_json::from_str::<BridgeRequest>(input) {
        Ok(request) => execute_bridge_request(request),
        Err(error) => BridgeResponse::failure(error.to_string()),
    }
}

pub fn execute_bridge_request_json(input: &str) -> String {
    let response = parse_bridge_request(input);
    serde_json::to_string(&response).expect("serialize bridge response")
}

fn try_execute_bridge_request(request: BridgeRequest) -> Result<Value, String> {
    let command = request
        .command
        .ok_or_else(|| "Missing bridge command.".to_owned())?;
    let runtime = RomRuntime::new(request.config).map_err(|error| error.to_string())?;

    match command {
        BridgeCommand::Eval => Ok(Value::String(
            runtime
                .eval_as_string(&required_script(request.script)?)
                .map_err(|error| error.to_string())?,
        )),
        BridgeCommand::EvalAsync => Ok(Value::String(
            runtime
                .eval_async_as_string(&required_script(request.script)?)
                .map_err(|error| error.to_string())?,
        )),
        BridgeCommand::SurfaceSnapshot => serde_json::to_value(
            runtime
                .surface_snapshot()
                .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string()),
        BridgeCommand::FingerprintProbe => serde_json::to_value(
            runtime
                .fingerprint_probe()
                .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string()),
        BridgeCommand::FingerprintJsHarness => serde_json::to_value(
            runtime
                .run_fingerprintjs_harness()
                .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string()),
        BridgeCommand::FingerprintJsVersion => {
            Ok(Value::String(runtime.fingerprintjs_version().to_owned()))
        }
    }
}

fn required_script(script: Option<String>) -> Result<String, String> {
    script.ok_or_else(|| "Missing script for bridge command.".to_owned())
}
