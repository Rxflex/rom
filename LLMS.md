# ROM LLM Guide

ROM is an experimental browser-like runtime in Rust. It is useful when an agent or tool needs a deterministic, scriptable Web API surface without launching Chromium.

## When To Reach For ROM

- You need `fetch`, DOM, workers, cookies, messaging, WebSocket, or WebCrypto behavior inside a lightweight runtime.
- You want compatibility probes such as `surface_snapshot()` or `fingerprint_probe()`.
- You need released bindings for Node.js or Python instead of wiring the Rust crates directly.

## What ROM Is Not

- Not a full browser engine.
- Not a layout engine.
- Not a complete implementation of the entire Web Platform.
- Not a drop-in Chromium replacement.

## Install

### Node.js

```bash
npm install @rxflex/rom
```

### Python

```bash
pip install rom-runtime
```

## Runtime Defaults You Should Know

- `cors_enabled` defaults to `false`.
- `proxy_url` is optional and can point at an HTTP proxy for HTTPS CONNECT flows.
- ROM can use native bindings when present and fall back to the CLI bridge otherwise.

## Node Example

```js
import { RomRuntime } from "@rxflex/rom";

const runtime = new RomRuntime({
  href: "https://example.test/",
  cors_enabled: false,
  proxy_url: process.env.ROM_PROXY_URL ?? null,
});

const result = await runtime.evalAsync(`
  (async () => {
    const response = await fetch("https://example.test/data");
    return JSON.stringify({
      href: location.href,
      status: response.status,
      body: await response.text(),
    });
  })()
`);

console.log(result);
```

## Python Example

```python
from rom import RomRuntime

runtime = RomRuntime(
    {
        "href": "https://example.test/",
        "cors_enabled": False,
        "proxy_url": None,
    }
)

result = runtime.eval_async(
    """
    (async () => {
      const response = await fetch("https://example.test/data");
      return JSON.stringify({
        href: location.href,
        status: response.status,
        body: await response.text(),
      });
    })()
    """
)

print(result)
```

## Useful Calls For Agents

- `surface_snapshot()` for quick compatibility inventory.
- `fingerprint_probe()` for stable browser-surface comparisons.
- `run_fingerprintjs_harness()` for vendored FingerprintJS acceptance checks.
- `eval()` and `eval_async()` for one-off JS execution.
- `eval_json()` when the script returns JSON text.

## Good Prompt Shape

When asking an agent to use ROM, include:

- the target URL
- whether CORS should stay browser-like or be disabled
- whether a proxy is required
- whether you want a raw JS result, a snapshot, or a FingerprintJS harness report

## Limits To Remember

- DOM, layout, and media behavior are compatibility-oriented, not browser-complete.
- Native package availability depends on the release assets for the target platform.
- If native bindings are unavailable, the package falls back to the CLI bridge.

## Pointers

- General project guide: [README.md](./README.md)
- Node package guide: [bindings/gom-node/README.md](./bindings/gom-node/README.md)
- Python package guide: [bindings/gom-python/README.md](./bindings/gom-python/README.md)
