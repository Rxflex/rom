# `rom-runtime`

Python bindings for the ROM browser-like runtime.

This package exposes a thin Python API on top of ROM:

- `eval()`
- `eval_async()`
- `eval_json()`
- `goto()`
- `set_content()`
- `content()`
- `evaluate()`
- `wait_for_selector()`
- `wait_for_function()`
- `click()`
- `fill()`
- `text_content()`
- `inner_html()`
- `locator()`
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
        "referrer": "https://referrer.example/",
        "cors_enabled": False,
        "proxy_url": None,
    }
)
href = runtime.eval_async("(async () => location.href)()")
snapshot = runtime.surface_snapshot()

print("native:", has_native_binding())
print(href)
print(snapshot["fetch"])

runtime.eval_async("(async () => { globalThis.__rom_value = 42; return 'ok'; })()")
print(runtime.eval_async("(async () => String(globalThis.__rom_value))()"))

runtime.set_content(
    '<div id="app"><input id="name" /><button id="go">Go</button><span id="out"></span></div>'
    '<script>document.querySelector("#go").addEventListener("click",()=>{document.querySelector("#out").textContent=document.querySelector("#name").value;});</script>'
)
runtime.fill("#name", "ROM")
runtime.click("#go")
print(runtime.text_content("#out"))
```

Config keys use the Rust runtime field names, so use snake_case such as `cors_enabled` and `proxy_url`.
`cors_enabled` is `False` by default.
When the native extension is loaded, one `RomRuntime` instance keeps JS globals alive across multiple `eval()` and `eval_async()` calls.
The page-like helpers such as `goto()`, `set_content()`, `click()`, and `wait_for_selector()` require the native extension because CLI bridge mode is stateless across calls.
For cookie seeding, the wrapper accepts serialized `cookie_store`, a raw cookie header string, or a `cookies` alias with string/object/array inputs and normalizes them automatically.
For storage seeding, the wrapper accepts `local_storage` and `session_storage` as serialized JSON objects, Python dicts, or entry arrays such as `[('VerifyAuthToken', 'seeded')]`.
The default navigator surface is Chrome-like, including `navigator.userAgent`, `navigator.vendor`, and `navigator.userAgentData`.

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
- `goto(url, options=None)`
- `set_content(html, options=None)`
- `content()`
- `evaluate(js_expression, arg=None)`
- `wait_for_selector(selector, options=None)`
- `wait_for_function(js_expression, arg=None, options=None)`
- `click(selector, options=None)`
- `fill(selector, value, options=None)`
- `text_content(selector, options=None)`
- `inner_html(selector, options=None)`
- `locator(selector)`
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
