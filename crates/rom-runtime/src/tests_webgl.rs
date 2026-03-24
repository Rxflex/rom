use crate::{RomRuntime, RuntimeConfig};

#[test]
fn supports_webgl_entropy_and_debug_surfaces() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r##"
            (async () => {
                const canvas = document.createElement("canvas");
                const gl = canvas.getContext("webgl");
                const sameGl = canvas.getContext("experimental-webgl");
                const gl2Canvas = document.createElement("canvas");
                const gl2 = gl2Canvas.getContext("webgl2");
                const debug = gl.getExtension("WEBGL_debug_renderer_info");
                const pixels = new Uint8Array(4 * 8 * 8);

                gl.clearColor(0.15, 0.3, 0.45, 1);
                gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
                gl.viewport(0, 0, 128, 64);
                gl.readPixels(0, 0, 8, 8, gl.RGBA, gl.UNSIGNED_BYTE, pixels);

                const secondCanvas = document.createElement("canvas");
                const secondGl = secondCanvas.getContext("webgl");
                secondGl.clearColor(0.75, 0.2, 0.1, 1);
                secondGl.clear(secondGl.COLOR_BUFFER_BIT);

                return JSON.stringify({
                    hasGl: gl instanceof WebGLRenderingContext,
                    aliasIsCached: gl === sameGl,
                    hasGl2: gl2 instanceof WebGL2RenderingContext,
                    supportedExtensions: gl.getSupportedExtensions(),
                    vendor: gl.getParameter(gl.VENDOR),
                    renderer: gl.getParameter(gl.RENDERER),
                    version: gl.getParameter(gl.VERSION),
                    shadingLanguageVersion: gl.getParameter(gl.SHADING_LANGUAGE_VERSION),
                    unmaskedVendor: gl.getParameter(debug.UNMASKED_VENDOR_WEBGL),
                    unmaskedRenderer: gl.getParameter(debug.UNMASKED_RENDERER_WEBGL),
                    viewportDims: Array.from(gl.getParameter(gl.MAX_VIEWPORT_DIMS)),
                    lineWidthRange: Array.from(gl.getParameter(gl.ALIASED_LINE_WIDTH_RANGE)),
                    pointSizeRange: Array.from(gl.getParameter(gl.ALIASED_POINT_SIZE_RANGE)),
                    maxTextureSize: gl.getParameter(gl.MAX_TEXTURE_SIZE),
                    precision: gl.getShaderPrecisionFormat(gl.FRAGMENT_SHADER, gl.HIGH_FLOAT).precision,
                    contextAttributes: gl.getContextAttributes(),
                    pixelDistinctValues: new Set(Array.from(pixels)).size,
                    pixelNonZero: Array.from(pixels).filter((value) => value !== 0).length,
                    differsByWebGlState: canvas.toDataURL() !== secondCanvas.toDataURL(),
                    dataUrlLength: canvas.toDataURL().length,
                });
            })()
            "##,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(value["hasGl"], true);
    assert_eq!(value["aliasIsCached"], true);
    assert_eq!(value["hasGl2"], true);
    assert!(
        value["supportedExtensions"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .any(|entry| entry == "WEBGL_debug_renderer_info")
    );
    assert_eq!(value["vendor"], "WebKit");
    assert_eq!(value["renderer"], "WebKit WebGL");
    assert!(
        value["version"]
            .as_str()
            .unwrap_or("")
            .starts_with("WebGL 1.0")
    );
    assert!(
        value["shadingLanguageVersion"]
            .as_str()
            .unwrap_or("")
            .contains("GLSL ES")
    );
    assert!(
        value["unmaskedVendor"]
            .as_str()
            .unwrap_or("")
            .contains("Google Inc.")
    );
    assert!(
        value["unmaskedRenderer"]
            .as_str()
            .unwrap_or("")
            .contains("ANGLE")
    );
    assert_eq!(value["viewportDims"], serde_json::json!([300, 150]));
    assert_eq!(value["lineWidthRange"], serde_json::json!([1, 8]));
    assert_eq!(value["pointSizeRange"], serde_json::json!([1, 1024]));
    assert_eq!(value["maxTextureSize"], 16384);
    assert_eq!(value["precision"], 23);
    assert_eq!(value["contextAttributes"]["antialias"], true);
    assert!(value["pixelDistinctValues"].as_u64().unwrap_or(0) > 64);
    assert!(value["pixelNonZero"].as_u64().unwrap_or(0) > 100);
    assert_eq!(value["differsByWebGlState"], true);
    assert!(value["dataUrlLength"].as_u64().unwrap_or(0) > 5_000);
}
