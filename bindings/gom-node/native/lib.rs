use napi::Result;
use napi_derive::napi;

#[napi]
pub fn execute_bridge(request_json: String) -> Result<String> {
    Ok(rom_runtime::execute_bridge_request_json(&request_json))
}
