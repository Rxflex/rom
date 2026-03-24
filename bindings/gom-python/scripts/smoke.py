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

    print({"native": True, "href": href, "persisted": persisted, "cookie": cookie, "storage": storage})


if __name__ == "__main__":
    main()
