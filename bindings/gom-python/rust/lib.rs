use pyo3::{exceptions::PyRuntimeError, prelude::*};
use rom_runtime::{RomRuntime, RuntimeConfig};
use serde::Serialize;

fn parse_config(config_json: String) -> PyResult<RuntimeConfig> {
    serde_json::from_str(&config_json)
        .map_err(|error| PyRuntimeError::new_err(format!("Invalid ROM config JSON: {error}")))
}

fn to_json<T>(value: &T) -> PyResult<String>
where
    T: Serialize,
{
    serde_json::to_string(value)
        .map_err(|error| PyRuntimeError::new_err(format!("Failed to serialize ROM value: {error}")))
}

fn map_runtime_error<T>(result: rom_runtime::Result<T>) -> PyResult<T> {
    result.map_err(|error| PyRuntimeError::new_err(error.to_string()))
}

#[pyfunction]
fn execute_bridge(request_json: String) -> String {
    rom_runtime::execute_bridge_request_json(&request_json)
}

#[pyclass(unsendable)]
struct NativeRomRuntime {
    runtime: RomRuntime,
}

#[pymethods]
impl NativeRomRuntime {
    #[new]
    fn new(config_json: String) -> PyResult<Self> {
        Ok(Self {
            runtime: map_runtime_error(RomRuntime::new(parse_config(config_json)?))?,
        })
    }

    fn eval(&self, script: String) -> PyResult<String> {
        map_runtime_error(self.runtime.eval_as_string(&script))
            .map(|value| value.to_owned())
    }

    fn eval_async(&self, script: String) -> PyResult<String> {
        map_runtime_error(self.runtime.eval_async_as_string(&script))
            .map(|value| value.to_owned())
    }

    fn surface_snapshot_json(&self) -> PyResult<String> {
        to_json(&map_runtime_error(self.runtime.surface_snapshot())?)
    }

    fn fingerprint_probe_json(&self) -> PyResult<String> {
        to_json(&map_runtime_error(self.runtime.fingerprint_probe())?)
    }

    fn fingerprint_js_harness_json(&self) -> PyResult<String> {
        to_json(&map_runtime_error(self.runtime.run_fingerprintjs_harness())?)
    }

    fn fingerprint_js_version(&self) -> String {
        self.runtime.fingerprintjs_version().to_owned()
    }

    fn export_cookie_store(&self) -> PyResult<String> {
        map_runtime_error(self.runtime.export_cookie_store())
            .map(|value| value.to_owned())
    }
}

#[pymodule]
fn _native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(execute_bridge, module)?)?;
    module.add_class::<NativeRomRuntime>()?;
    Ok(())
}
