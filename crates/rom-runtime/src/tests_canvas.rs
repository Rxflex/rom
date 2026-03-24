use crate::{RomRuntime, RuntimeConfig};
use flate2::{Compression, write::ZlibEncoder};
use std::io::Write;

#[test]
fn supports_canvas_data_urls_text_metrics_and_entropy() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r##"
            (async () => {
                const canvas = document.createElement("canvas");
                const ctx = canvas.getContext("2d");
                ctx.font = "16px Arial";
                ctx.fillStyle = "#1a2b3c";
                ctx.fillRect(12, 8, 140, 42);
                ctx.fillText("anti-content-seed", 18, 36);

                const secondCanvas = document.createElement("canvas");
                const secondCtx = secondCanvas.getContext("2d");
                secondCtx.font = "16px Arial";
                secondCtx.fillStyle = "#1a2b3c";
                secondCtx.fillRect(12, 8, 140, 42);
                secondCtx.fillText("different-seed", 18, 36);

                const metrics = ctx.measureText("anti-content-seed");
                const imageData = ctx.getImageData(0, 0, 32, 16).data;
                let nonZeroBytes = 0;
                let distinctValues = new Set();
                for (const value of imageData) {
                    if (value !== 0) {
                        nonZeroBytes += 1;
                    }
                    distinctValues.add(value);
                }

                return JSON.stringify({
                    metricsType: Object.prototype.toString.call(metrics),
                    width: metrics.width,
                    actualBoundingBoxAscent: metrics.actualBoundingBoxAscent,
                    actualBoundingBoxDescent: metrics.actualBoundingBoxDescent,
                    ownMetricKeys: Object.keys(metrics).sort(),
                    dataUrl: canvas.toDataURL(),
                    dataUrlPrefix: canvas.toDataURL().slice(0, 22),
                    dataUrlLength: canvas.toDataURL().length,
                    secondDataUrlLength: secondCanvas.toDataURL().length,
                    differsByContent: canvas.toDataURL() !== secondCanvas.toDataURL(),
                    imageDataLength: imageData.length,
                    nonZeroBytes,
                    distinctValues: distinctValues.size,
                });
            })()
            "##,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    let data_url_length = value["dataUrlLength"].as_u64().unwrap_or(0);
    let data_url = value["dataUrl"].as_str().unwrap_or("");

    assert_eq!(value["metricsType"], "[object Object]");
    assert!(value["width"].as_f64().unwrap_or(0.0) > 0.0);
    assert!(value["actualBoundingBoxAscent"].as_f64().unwrap_or(0.0) > 0.0);
    assert!(value["actualBoundingBoxDescent"].as_f64().unwrap_or(0.0) > 0.0);
    assert_eq!(value["dataUrlPrefix"], "data:image/png;base64,");
    assert!(data_url_length > 5_000);
    assert!(value["secondDataUrlLength"].as_u64().unwrap_or(0) > 5_000);
    assert_eq!(value["differsByContent"], true);
    assert_eq!(value["imageDataLength"], 32 * 16 * 4);
    assert!(value["nonZeroBytes"].as_u64().unwrap_or(0) > 1_000);
    assert!(value["distinctValues"].as_u64().unwrap_or(0) > 64);
    let keys = value["ownMetricKeys"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert!(keys.iter().any(|entry| entry == "width"));
    assert!(keys.iter().any(|entry| entry == "actualBoundingBoxAscent"));

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(data_url.as_bytes())
        .expect("compress canvas probe");
    let compressed = encoder.finish().expect("finish compression");
    assert!(
        compressed.len() > 500,
        "canvas probe compressed too aggressively: {}",
        compressed.len()
    );
}
