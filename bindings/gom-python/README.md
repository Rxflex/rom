# `rom-runtime`

Python bindings for the ROM browser-like runtime.

This package exposes a thin Python API on top of ROM:

- `eval()`
- `eval_async()`
- `eval_json()`
- `surface_snapshot()`
- `fingerprint_probe()`
- `run_fingerprintjs_harness()`
- `fingerprintjs_version()`

It prefers a native `PyO3` extension when available and falls back to the ROM CLI bridge otherwise.

## Install

```bash
pip install rom-runtime
```

## Usage

```python
from rom import RomRuntime, has_native_binding

runtime = RomRuntime(
    {
        "href": "https://example.test/",
        "cors_enabled": False,
        "proxy_url": None,
    }
)
href = runtime.eval_async("(async () => location.href)()")
snapshot = runtime.surface_snapshot()

print("native:", has_native_binding())
print(href)
print(snapshot["fetch"])
```

Config keys use the Rust runtime field names, so use snake_case such as `cors_enabled` and `proxy_url`.
`cors_enabled` is `False` by default.

## Optional native build from source

```bash
python -m pip install maturin
python -m maturin build --manifest-path bindings/gom-python/Cargo.toml --release
```

Tagged GitHub releases build and publish wheels for Linux, Windows, and macOS, plus an sdist for source installs.

## Common methods

- `eval()`
- `eval_async()`
- `eval_json()`
- `surface_snapshot()`
- `fingerprint_probe()`
- `run_fingerprintjs_harness()`
- `fingerprintjs_version()`

## Environment

- `ROM_FORCE_CLI_BRIDGE=1`: disable the native path and force CLI fallback
- `ROM_BRIDGE_BIN`: explicit path to the `rom_bridge` executable
- `ROM_BRIDGE_CWD`: working directory used by the CLI fallback

## More docs

- Root guide: [../../README.md](../../README.md)
- LLM guide: [../../LLMS.md](../../LLMS.md)
