from rom import RomRuntime, has_native_binding


def main() -> None:
    runtime = RomRuntime(
        {
            "href": "https://example.test/",
            "cookie_store": "seed=1; path=/",
            "local_storage": {"VerifyAuthToken": "seeded-storage"},
        }
    )
    href = runtime.eval_async("(async () => location.href)()")
    runtime.eval_async("(async () => { globalThis.__romSmokeValue = 42; return 'ok'; })()")
    persisted = runtime.eval_async("(async () => String(globalThis.__romSmokeValue))()")
    cookie = runtime.eval_async("(async () => document.cookie)()")
    storage = runtime.eval_async("(async () => localStorage.getItem('VerifyAuthToken'))()")
    runtime.set_content(
        '<div id="app"><input id="name" /><button id="go">Go</button><span id="out"></span></div>'
        '<script>document.querySelector("#go").addEventListener("click",()=>{document.querySelector("#out").textContent=document.querySelector("#name").value;});</script>'
    )
    runtime.fill("#name", "ROM")
    runtime.click("#go")
    page_text = runtime.text_content("#out")
    snapshot = runtime.surface_snapshot()

    if not has_native_binding():
        raise RuntimeError("Expected Python native binding to be loaded.")

    if href != "https://example.test/":
        raise RuntimeError(f"Unexpected href: {href}")

    if snapshot["globals"]["window"] is not True:
        raise RuntimeError("Surface snapshot did not expose window.")

    if persisted != "42":
        raise RuntimeError(f"Expected persisted global state, got: {persisted}")

    if cookie != "seed=1":
        raise RuntimeError(f"Expected seeded cookie, got: {cookie}")

    if storage != "seeded-storage":
        raise RuntimeError(f"Expected seeded localStorage, got: {storage}")

    if page_text != "ROM":
        raise RuntimeError(f"Expected page helper flow to update text, got: {page_text}")

    print(
        {
            "native": True,
            "href": href,
            "persisted": persisted,
            "cookie": cookie,
            "storage": storage,
            "page_text": page_text,
        }
    )


if __name__ == "__main__":
    main()
