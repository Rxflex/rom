use crate::{RomRuntime, RuntimeConfig};

#[test]
fn supports_character_data_length_and_substring() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r##"
            (async () => {
                const host = document.createElement("div");
                const text = document.createTextNode("hello world");
                const comment = document.createComment("alpha-beta");
                host.append(text, comment);
                document.body.appendChild(host);

                const records = [];
                const observer = new MutationObserver((entries) => {
                    records.push(
                        ...entries.map((entry) => ({
                            type: entry.type,
                            target: entry.target.nodeName,
                            oldValue: entry.oldValue ?? null,
                            value: entry.target.nodeValue,
                        })),
                    );
                });
                observer.observe(host, {
                    characterData: true,
                    characterDataOldValue: true,
                    subtree: true,
                });

                let errorName = null;
                try {
                    text.substringData(99, 1);
                } catch (error) {
                    errorName = error.name;
                }

                const initialTextLength = text.length;
                const initialTextSlice = text.substringData(6, 5);
                const initialTextTail = text.substringData(3, 99);
                const initialCommentLength = comment.length;
                const initialCommentSlice = comment.substringData(0, 5);
                const initialCommentMiddle = comment.substringData(6, 4);

                text.appendData("!");
                text.deleteData(5, 1);
                text.insertData(5, ",");
                text.replaceData(6, 6, "ROM");

                comment.appendData("!");
                comment.deleteData(5, 1);
                comment.insertData(5, "_");
                comment.replaceData(6, 5, "note");

                await Promise.resolve();

                return JSON.stringify({
                    initialTextLength,
                    initialTextSlice,
                    initialTextTail,
                    initialCommentLength,
                    initialCommentSlice,
                    initialCommentMiddle,
                    finalTextLength: text.length,
                    finalCommentLength: comment.length,
                    finalText: text.data,
                    finalComment: comment.data,
                    errorName,
                    records,
                });
            })()
            "##,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["initialTextLength"], 11);
    assert_eq!(value["initialTextSlice"], "world");
    assert_eq!(value["initialTextTail"], "lo world");
    assert_eq!(value["initialCommentLength"], 10);
    assert_eq!(value["initialCommentSlice"], "alpha");
    assert_eq!(value["initialCommentMiddle"], "beta");
    assert_eq!(value["finalTextLength"], 9);
    assert_eq!(value["finalCommentLength"], 10);
    assert_eq!(value["finalText"], "hello,ROM");
    assert_eq!(value["finalComment"], "alpha_note");
    assert_eq!(value["errorName"], "IndexSizeError");
    assert_eq!(
        value["records"],
        serde_json::json!([
            { "type": "characterData", "target": "#text", "oldValue": "hello world", "value": "hello,ROM" },
            { "type": "characterData", "target": "#text", "oldValue": "hello world!", "value": "hello,ROM" },
            { "type": "characterData", "target": "#text", "oldValue": "helloworld!", "value": "hello,ROM" },
            { "type": "characterData", "target": "#text", "oldValue": "hello,world!", "value": "hello,ROM" },
            { "type": "characterData", "target": "#comment", "oldValue": "alpha-beta", "value": "alpha_note" },
            { "type": "characterData", "target": "#comment", "oldValue": "alpha-beta!", "value": "alpha_note" },
            { "type": "characterData", "target": "#comment", "oldValue": "alphabeta!", "value": "alpha_note" },
            { "type": "characterData", "target": "#comment", "oldValue": "alpha_beta!", "value": "alpha_note" }
        ])
    );
}
