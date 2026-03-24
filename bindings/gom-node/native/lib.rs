use napi::{Error, Result};
use napi_derive::napi;
use rom_runtime::{RomRuntime, RuntimeConfig};
use serde::Serialize;
use serde_json::Value;

fn parse_config(config_json: String) -> Result<RuntimeConfig> {
    serde_json::from_str(&config_json)
        .map_err(|error| Error::from_reason(format!("Invalid ROM config JSON: {error}")))
}

fn to_json<T>(value: &T) -> Result<String>
where
    T: Serialize,
{
    serde_json::to_string(value)
        .map_err(|error| Error::from_reason(format!("Failed to serialize ROM value: {error}")))
}

fn map_runtime_error<T>(result: rom_runtime::Result<T>) -> Result<T> {
    result.map_err(|error| Error::from_reason(error.to_string()))
}

#[napi]
pub fn execute_bridge(request_json: String) -> Result<String> {
    Ok(rom_runtime::execute_bridge_request_json(&request_json))
}

#[napi]
pub struct NativeRomRuntime {
    runtime: RomRuntime,
}

#[napi]
impl NativeRomRuntime {
    #[napi(constructor)]
    pub fn new(config_json: String) -> Result<Self> {
        Ok(Self {
            runtime: map_runtime_error(RomRuntime::new(parse_config(config_json)?))?,
        })
    }

    #[napi(js_name = "eval")]
    pub fn eval(&self, script: String) -> Result<String> {
        map_runtime_error(self.runtime.eval_as_string(&script))
            .map(|value| value.to_owned())
    }

    #[napi(js_name = "evalAsync")]
    pub fn eval_async(&self, script: String) -> Result<String> {
        map_runtime_error(self.runtime.eval_async_as_string(&script))
            .map(|value| value.to_owned())
    }

    #[napi(js_name = "surfaceSnapshotJson")]
    pub fn surface_snapshot_json(&self) -> Result<String> {
        to_json(&map_runtime_error(self.runtime.surface_snapshot())?)
    }

    #[napi(js_name = "fingerprintProbeJson")]
    pub fn fingerprint_probe_json(&self) -> Result<String> {
        to_json(&map_runtime_error(self.runtime.fingerprint_probe())?)
    }

    #[napi(js_name = "fingerprintJsHarnessJson")]
    pub fn fingerprint_js_harness_json(&self) -> Result<String> {
        to_json(&map_runtime_error(self.runtime.run_fingerprintjs_harness())?)
    }

    #[napi(js_name = "fingerprintJsVersion")]
    pub fn fingerprint_js_version(&self) -> Result<String> {
        Ok(self.runtime.fingerprintjs_version().to_owned())
    }

    #[napi(js_name = "exportCookieStore")]
    pub fn export_cookie_store(&self) -> Result<String> {
        map_runtime_error(self.runtime.export_cookie_store())
            .map(|value| value.to_owned())
    }

    #[napi(js_name = "evalJson")]
    pub fn eval_json(&self, script: String, asynchronous: bool) -> Result<String> {
        let value = if asynchronous {
            map_runtime_error(self.runtime.eval_async_as_string(&script))?
        } else {
            map_runtime_error(self.runtime.eval_as_string(&script))?
        };

        let parsed: Value = serde_json::from_str(&value)
            .map_err(|error| Error::from_reason(format!("ROM eval did not return JSON: {error}")))?;
        to_json(&parsed)
    }
}
