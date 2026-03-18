use crate::{RomRuntime, RuntimeConfig};

#[test]
fn supports_resize_and_intersection_observers() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const target = document.createElement("div");
                document.body.appendChild(target);

                const resize = await new Promise((resolve) => {
                    const snapshots = [];
                    const observer = new ResizeObserver((entries) => {
                        const batch = entries.map((entry) => ({
                                width: entry.contentRect.width,
                                height: entry.contentRect.height,
                                inlineSize: entry.contentBoxSize[0].inlineSize,
                                blockSize: entry.contentBoxSize[0].blockSize,
                                target: entry.target.nodeName,
                            }));
                        snapshots.push(...batch);
                        if (snapshots.length >= 2) {
                            observer.disconnect();
                            resolve(snapshots);
                        }
                    });

                    observer.observe(target);
                    target.textContent = "observer text";
                });

                const intersection = await new Promise((resolve) => {
                    const observer = new IntersectionObserver((entries, currentObserver) => {
                        currentObserver.disconnect();
                        resolve(
                            entries.map((entry) => ({
                                target: entry.target.nodeName,
                                isIntersecting: entry.isIntersecting,
                                intersectionRatio: entry.intersectionRatio,
                                width: entry.boundingClientRect.width,
                                height: entry.boundingClientRect.height,
                                rootBounds: entry.rootBounds,
                            })),
                        );
                    }, { threshold: [0, 0.5, 1] });

                    observer.observe(target);
                });

                return {
                    resize,
                    intersection,
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(
        value["resize"],
        serde_json::json!([
            {
                "width": 1,
                "height": 16,
                "inlineSize": 1,
                "blockSize": 16,
                "target": "DIV"
            },
            {
                "width": 104,
                "height": 16,
                "inlineSize": 104,
                "blockSize": 16,
                "target": "DIV"
            }
        ])
    );
    assert_eq!(
        value["intersection"],
        serde_json::json!([
            {
                "target": "DIV",
                "isIntersecting": true,
                "intersectionRatio": 1,
                "width": 104,
                "height": 16,
                "rootBounds": null
            }
        ])
    );
}
