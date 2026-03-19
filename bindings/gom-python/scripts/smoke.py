from rom import RomRuntime, has_native_binding


def main() -> None:
    runtime = RomRuntime({"href": "https://example.test/"})
    href = runtime.eval_async("(async () => location.href)()")
    snapshot = runtime.surface_snapshot()

    if not has_native_binding():
        raise RuntimeError("Expected Python native binding to be loaded.")

    if href != "https://example.test/":
        raise RuntimeError(f"Unexpected href: {href}")

    if snapshot["globals"]["window"] is not True:
        raise RuntimeError("Surface snapshot did not expose window.")

    print({"native": True, "href": href})


if __name__ == "__main__":
    main()
