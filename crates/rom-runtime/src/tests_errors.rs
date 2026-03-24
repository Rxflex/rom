use crate::{RomRuntime, RuntimeConfig};

#[test]
fn surfaces_js_exception_details_from_eval_async() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();

    let error = runtime
        .eval_async_as_string("(async () => { throw new Error('temu boom'); })()")
        .unwrap_err()
        .to_string();

    assert!(error.contains("temu boom"));
    assert!(error.contains("Error: temu boom"));
}
