use crate::{RomRuntime, RuntimeConfig};

#[test]
fn imports_local_and_session_storage_from_config() {
    let runtime = RomRuntime::new(RuntimeConfig {
        href: "https://www.temu.com/".to_owned(),
        local_storage: Some("{\"VerifyAuthToken\":\"seeded-local\"}".to_owned()),
        session_storage: Some("{\"session-key\":\"seeded-session\"}".to_owned()),
        ..RuntimeConfig::default()
    })
    .unwrap();

    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => JSON.stringify({
                local: localStorage.getItem("VerifyAuthToken"),
                session: sessionStorage.getItem("session-key"),
                localLength: localStorage.length,
                sessionLength: sessionStorage.length,
            }))()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(value["local"], "seeded-local");
    assert_eq!(value["session"], "seeded-session");
    assert_eq!(value["localLength"], 1);
    assert_eq!(value["sessionLength"], 1);
}
