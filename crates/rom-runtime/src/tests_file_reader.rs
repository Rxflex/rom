use crate::{RomRuntime, RuntimeConfig};

#[test]
fn supports_file_reader_variants_and_events() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const blob = new Blob(["hello"], { type: "text/plain" });

                const textResult = await new Promise((resolve) => {
                    const reader = new FileReader();
                    const events = [];

                    reader.onloadstart = () => events.push("loadstart");
                    reader.onprogress = () => events.push("progress");
                    reader.onload = () => events.push("load");
                    reader.onloadend = () => resolve({
                        readyState: reader.readyState,
                        result: reader.result,
                        events,
                    });

                    reader.readAsText(blob);
                });

                const arrayBufferResult = await new Promise((resolve) => {
                    const reader = new FileReader();
                    reader.onloadend = () => resolve(Array.from(new Uint8Array(reader.result)));
                    reader.readAsArrayBuffer(blob);
                });

                const dataUrlResult = await new Promise((resolve) => {
                    const reader = new FileReader();
                    reader.onloadend = () => resolve(reader.result);
                    reader.readAsDataURL(blob);
                });

                const abortResult = await new Promise((resolve) => {
                    const reader = new FileReader();
                    const events = [];
                    reader.onabort = () => events.push("abort");
                    reader.onloadend = () => resolve({
                        readyState: reader.readyState,
                        result: reader.result,
                        events,
                    });
                    reader.readAsText(blob);
                    reader.abort();
                });

                return {
                    empty: FileReader.EMPTY,
                    loading: FileReader.LOADING,
                    done: FileReader.DONE,
                    textResult,
                    arrayBufferResult,
                    dataUrlResult,
                    abortResult,
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["empty"], 0);
    assert_eq!(value["loading"], 1);
    assert_eq!(value["done"], 2);
    assert_eq!(value["textResult"]["readyState"], 2);
    assert_eq!(value["textResult"]["result"], "hello");
    assert_eq!(
        value["textResult"]["events"],
        serde_json::json!(["loadstart", "progress", "load"])
    );
    assert_eq!(
        value["arrayBufferResult"],
        serde_json::json!([104, 101, 108, 108, 111])
    );
    assert_eq!(value["dataUrlResult"], "data:text/plain;base64,aGVsbG8=");
    assert_eq!(value["abortResult"]["readyState"], 2);
    assert_eq!(value["abortResult"]["result"], serde_json::Value::Null);
    assert_eq!(value["abortResult"]["events"], serde_json::json!(["abort"]));
}
