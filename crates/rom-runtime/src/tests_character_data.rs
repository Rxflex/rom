use crate::{RomRuntime, RuntimeConfig};

#[test]
fn supports_character_data_length_and_substring() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r##"
            (async () => {
                const text = document.createTextNode("hello world");
                const comment = document.createComment("alpha-beta");

                let errorName = null;
                try {
                    text.substringData(99, 1);
                } catch (error) {
                    errorName = error.name;
                }

                return JSON.stringify({
                    textLength: text.length,
                    textSlice: text.substringData(6, 5),
                    textTail: text.substringData(3, 99),
                    commentLength: comment.length,
                    commentSlice: comment.substringData(0, 5),
                    commentMiddle: comment.substringData(6, 4),
                    errorName,
                });
            })()
            "##,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["textLength"], 11);
    assert_eq!(value["textSlice"], "world");
    assert_eq!(value["textTail"], "lo world");
    assert_eq!(value["commentLength"], 10);
    assert_eq!(value["commentSlice"], "alpha");
    assert_eq!(value["commentMiddle"], "beta");
    assert_eq!(value["errorName"], "IndexSizeError");
}
