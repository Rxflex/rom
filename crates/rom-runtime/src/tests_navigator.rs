use crate::{RomRuntime, RuntimeConfig};

#[test]
fn supports_navigator_permissions_media_and_plugin_surfaces() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const permission = await navigator.permissions.query({ name: "camera" });
                const microphoneBefore = await navigator.permissions.query({ name: "microphone" });
                const notification = await navigator.permissions.query({ name: "notifications" });
                const userAgentData = navigator.userAgentData;
                const highEntropyValues = await userAgentData.getHighEntropyValues([
                    "architecture",
                    "bitness",
                    "platformVersion",
                    "uaFullVersion",
                    "fullVersionList",
                    "wow64",
                ]);
                const devicesBefore = await navigator.mediaDevices.enumerateDevices();
                const deviceEvents = [];
                navigator.mediaDevices.addEventListener("devicechange", (event) => {
                    deviceEvents.push(`listener:${event.type}`);
                });
                navigator.mediaDevices.ondevicechange = (event) => {
                    deviceEvents.push(`handler:${event.type}`);
                };
                const stream = await navigator.mediaDevices.getUserMedia({ audio: true, video: true });
                await new Promise((resolve) => setTimeout(resolve, 0));
                const cameraAfter = await navigator.permissions.query({ name: "camera" });
                const microphoneAfter = await navigator.permissions.query({ name: "microphone" });
                const devicesAfter = await navigator.mediaDevices.enumerateDevices();
                const audioTrack = stream.getAudioTracks()[0];
                const videoTrack = stream.getVideoTracks()[0];
                const mimeType = navigator.mimeTypes.namedItem("application/pdf");
                const plugin = navigator.plugins.namedItem("PDF Viewer");

                return {
                    permissionState: permission.state,
                    permissionName: permission.name,
                    permissionIsStatus: permission instanceof PermissionStatus,
                    microphoneBefore: microphoneBefore.state,
                    notificationState: notification.state,
                    uaDataIsPresent: userAgentData instanceof NavigatorUAData,
                    vendor: navigator.vendor,
                    uaDataBrands: userAgentData.brands,
                    uaDataPlatform: userAgentData.platform,
                    uaDataMobile: userAgentData.mobile,
                    highEntropyValues,
                    devicesBefore: devicesBefore.map((device) => ({
                        kind: device.kind,
                        label: device.label,
                        isInput: device instanceof InputDeviceInfo,
                    })),
                    devicesAfter: devicesAfter.map((device) => ({
                        kind: device.kind,
                        label: device.label,
                        isInput: device instanceof InputDeviceInfo,
                    })),
                    deviceEvents,
                    supportedConstraints: navigator.mediaDevices.getSupportedConstraints(),
                    streamIsMediaStream: stream instanceof MediaStream,
                    trackKinds: stream.getTracks().map((track) => track.kind).join(","),
                    cameraAfter: cameraAfter.state,
                    microphoneAfter: microphoneAfter.state,
                    audioTrackState: audioTrack.readyState,
                    audioTrackSettings: audioTrack.getSettings(),
                    videoTrackLabel: videoTrack.label,
                    pluginLength: navigator.plugins.length,
                    pluginName: plugin.name,
                    mimeTypeType: mimeType.type,
                    mimeTypePluginName: mimeType.enabledPlugin.name,
                    pluginMimeTypeType: plugin.item(0).type,
                    pdfViewerEnabled: navigator.pdfViewerEnabled,
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(value["permissionState"], "prompt");
    assert_eq!(value["permissionName"], "camera");
    assert_eq!(value["permissionIsStatus"], true);
    assert_eq!(value["microphoneBefore"], "prompt");
    assert_eq!(value["notificationState"], "default");
    assert_eq!(value["uaDataIsPresent"], true);
    assert_eq!(value["vendor"], "Google Inc.");
    assert_eq!(
        value["uaDataBrands"],
        serde_json::json!([
            { "brand": "Chromium", "version": "137" },
            { "brand": "Google Chrome", "version": "137" },
            { "brand": "Not=A?Brand", "version": "24" }
        ])
    );
    assert_eq!(value["uaDataPlatform"], "Windows");
    assert_eq!(value["uaDataMobile"], false);
    assert_eq!(value["highEntropyValues"]["architecture"], "x86");
    assert_eq!(value["highEntropyValues"]["bitness"], "64");
    assert_eq!(value["highEntropyValues"]["platformVersion"], "15.0.0");
    assert_eq!(value["highEntropyValues"]["uaFullVersion"], "137.0.0.0");
    assert_eq!(value["highEntropyValues"]["wow64"], false);
    assert_eq!(
        value["highEntropyValues"]["fullVersionList"],
        serde_json::json!([
            { "brand": "Chromium", "version": "137" },
            { "brand": "Google Chrome", "version": "137" },
            { "brand": "Not=A?Brand", "version": "24" }
        ])
    );
    assert_eq!(
        value["devicesBefore"],
        serde_json::json!([
            { "kind": "audioinput", "label": "", "isInput": true },
            { "kind": "videoinput", "label": "", "isInput": true },
            { "kind": "audiooutput", "label": "", "isInput": false }
        ])
    );
    assert_eq!(
        value["devicesAfter"],
        serde_json::json!([
            { "kind": "audioinput", "label": "Default Audio Input", "isInput": true },
            { "kind": "videoinput", "label": "Default Video Input", "isInput": true },
            { "kind": "audiooutput", "label": "Default Audio Output", "isInput": false }
        ])
    );
    assert_eq!(
        value["deviceEvents"],
        serde_json::json!(["handler:devicechange", "listener:devicechange"])
    );
    assert_eq!(value["supportedConstraints"]["audio"], true);
    assert_eq!(value["supportedConstraints"]["video"], true);
    assert_eq!(value["streamIsMediaStream"], true);
    assert_eq!(value["trackKinds"], "audio,video");
    assert_eq!(value["cameraAfter"], "granted");
    assert_eq!(value["microphoneAfter"], "granted");
    assert_eq!(value["audioTrackState"], "live");
    assert_eq!(value["audioTrackSettings"]["deviceId"], "audio-default");
    assert_eq!(value["videoTrackLabel"], "Default Video Input");
    assert_eq!(value["pluginLength"], 1);
    assert_eq!(value["pluginName"], "PDF Viewer");
    assert_eq!(value["mimeTypeType"], "application/pdf");
    assert_eq!(value["mimeTypePluginName"], "PDF Viewer");
    assert_eq!(value["pluginMimeTypeType"], "application/pdf");
    assert_eq!(value["pdfViewerEnabled"], true);
}
