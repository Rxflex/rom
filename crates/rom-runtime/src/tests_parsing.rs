use crate::{RomRuntime, RuntimeConfig};

#[test]
fn supports_url_pattern_matching() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const first = new URLPattern({ pathname: "/books/:id" });
                const second = new URLPattern("/users/:name", "https://api.example.com");
                const third = new URLPattern(
                    { pathname: "/Case/:value" },
                    { ignoreCase: true },
                );

                return {
                    firstTest: first.test("https://example.com/books/123"),
                    firstGroup: first.exec("https://example.com/books/123").pathname.groups.id,
                    secondHost: second.exec("https://api.example.com/users/alice").hostname.input,
                    secondName: second.exec("https://api.example.com/users/alice").pathname.groups.name,
                    thirdTest: third.test("https://example.com/case/UPPER"),
                    thirdGroup: third.exec("https://example.com/case/UPPER").pathname.groups.value,
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["firstTest"], true);
    assert_eq!(value["firstGroup"], "123");
    assert_eq!(value["secondHost"], "api.example.com");
    assert_eq!(value["secondName"], "alice");
    assert_eq!(value["thirdTest"], true);
    assert_eq!(value["thirdGroup"], "UPPER");
}

#[test]
fn supports_dom_parser_for_html_and_xml() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r##"
            (async () => {
                const parser = new DOMParser();
                const htmlDoc = parser.parseFromString(
                    `<div id="root"><span class="value">Hello</span></div>`,
                    "text/html",
                );
                const xmlDoc = parser.parseFromString(
                    `<note><to>Ada</to><from>ROM</from></note>`,
                    "application/xml",
                );
                const brokenXml = parser.parseFromString(
                    `<note><to>Ada</note>`,
                    "application/xml",
                );

                return {
                    htmlContentType: htmlDoc.contentType,
                    htmlText: htmlDoc.querySelector(".value").textContent,
                    htmlRootId: htmlDoc.querySelector("#root").id,
                    xmlContentType: xmlDoc.contentType,
                    xmlRoot: xmlDoc.documentElement.tagName,
                    xmlTo: xmlDoc.querySelector("to").textContent,
                    hasParserError: brokenXml.querySelector("parsererror") !== null,
                };
            })()
            "##,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["htmlContentType"], "text/html");
    assert_eq!(value["htmlText"], "Hello");
    assert_eq!(value["htmlRootId"], "root");
    assert_eq!(value["xmlContentType"], "application/xml");
    assert_eq!(value["xmlRoot"], "NOTE");
    assert_eq!(value["xmlTo"], "Ada");
    assert_eq!(value["hasParserError"], true);
}
