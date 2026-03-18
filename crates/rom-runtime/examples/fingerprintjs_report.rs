use rom_runtime::{RomRuntime, RuntimeConfig};

fn main() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).expect("runtime");
    let report = runtime
        .run_fingerprintjs_harness()
        .expect("fingerprintjs harness");

    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("serialize report")
    );
}
