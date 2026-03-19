use pyo3::prelude::*;

#[pyfunction]
fn execute_bridge(request_json: String) -> String {
    rom_runtime::execute_bridge_request_json(&request_json)
}

#[pymodule]
fn _native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(execute_bridge, module)?)?;
    Ok(())
}
