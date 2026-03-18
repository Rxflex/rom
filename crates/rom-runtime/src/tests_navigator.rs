use crate::{RomRuntime, RuntimeConfig};

#[test]
fn supports_navigator_permissions_media_and_plugin_surfaces() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const permission = await navigator.permissions.query({ name: "camera" });
                const notification = await navigator.permissions.query({ name: "notifications" });
                const devices = await navigator.mediaDevices.enumerateDevices();
                const stream = await navigator.mediaDevices.getUserMedia({ audio: true, video: true });
                const audioTrack = stream.getAudioTracks()[0];
                const videoTrack = stream.getVideoTracks()[0];
                const mimeType = navigator.mimeTypes.namedItem("application/pdf");
                const plugin = navigator.plugins.namedItem("PDF Viewer");

                return {
                    permissionState: permission.state,
                    permissionName: permission.name,
                    permissionIsStatus: permission instanceof PermissionStatus,
                    notificationState: notification.state,
                    devices: devices.map((device) => ({
                        kind: device.kind,
                        label: device.label,
                        isInput: device instanceof InputDeviceInfo,
                    })),
                    supportedConstraints: navigator.mediaDevices.getSupportedConstraints(),
                    streamIsMediaStream: stream instanceof MediaStream,
                    trackKinds: stream.getTracks().map((track) => track.kind).join(","),
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

    assert_eq!(value["permissionState"], "granted");
    assert_eq!(value["permissionName"], "camera");
    assert_eq!(value["permissionIsStatus"], true);
    assert_eq!(value["notificationState"], "default");
    assert_eq!(
        value["devices"],
        serde_json::json!([
            { "kind": "audioinput", "label": "Default Audio Input", "isInput": true },
            { "kind": "videoinput", "label": "Default Video Input", "isInput": true },
            { "kind": "audiooutput", "label": "Default Audio Output", "isInput": false }
        ])
    );
    assert_eq!(value["supportedConstraints"]["audio"], true);
    assert_eq!(value["supportedConstraints"]["video"], true);
    assert_eq!(value["streamIsMediaStream"], true);
    assert_eq!(value["trackKinds"], "audio,video");
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
