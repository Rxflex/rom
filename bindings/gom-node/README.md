# `@rxflex/rom`

Node.js bindings for the ROM browser-like runtime.

This package exposes a small JavaScript API on top of ROM:

- `eval()`
- `evalAsync()`
- `evalJson()`
- `surfaceSnapshot()`
- `fingerprintProbe()`
- `runFingerprintJsHarness()`
- `fingerprintJsVersion()`

It prefers a native `napi-rs` bridge when available and falls back to the ROM CLI bridge otherwise.

## Install

```bash
npm install @rxflex/rom
```

## Usage

```js
import { RomRuntime, hasNativeBinding } from "@rxflex/rom";

const runtime = new RomRuntime({
  href: "https://example.test/",
  referrer: "https://referrer.example/",
  cors_enabled: false,
  proxy_url: process.env.ROM_PROXY_URL ?? null,
});

const href = await runtime.evalAsync("(async () => location.href)()");
const snapshot = await runtime.surfaceSnapshot();

console.log("native:", hasNativeBinding());
console.log(href);
console.log(snapshot.fetch);

await runtime.evalAsync("(async () => { globalThis.__romValue = 42; return 'ok'; })()");
console.log(await runtime.evalAsync("(async () => String(globalThis.__romValue))()"));
```

Config keys use the Rust runtime field names, so use snake_case such as `cors_enabled` and `proxy_url`.
`cors_enabled` is `false` by default.
When the native addon is loaded, one `RomRuntime` instance keeps JS globals alive across multiple `eval()` and `evalAsync()` calls.

## Optional native build

```bash
npm run build:native
```

Local `npm pack` and `npm publish` still build the native addon for the current platform via `prepack`.
Tagged GitHub releases assemble multi-platform prebuilds and publish a single npm package that includes:

- `linux-x64-gnu`
- `win32-x64-msvc`
- `darwin-x64`
- `darwin-arm64`

At runtime the loader picks the matching binary from `prebuilds/<platform>/rom_node_native.node`.

## Common methods

- `eval(script)`
- `evalAsync(script)`
- `evalJson(script, { async })`
- `surfaceSnapshot()`
- `fingerprintProbe()`
- `runFingerprintJsHarness()`
- `fingerprintJsVersion()`

## Environment

- `ROM_NATIVE_NODE_BINDING`: explicit path to a compiled `.node` addon
- `ROM_FORCE_CLI_BRIDGE=1`: disable the native path and force CLI fallback
- `ROM_BRIDGE_BIN`: explicit path to the `rom_bridge` executable
- `ROM_BRIDGE_CWD`: working directory used by the CLI fallback
- `ROM_PROXY_URL`: convenience env var you can forward into `proxy_url`

## More docs

- Root guide: [../../README.md](../../README.md)
- LLM guide: [../../LLMS.md](../../LLMS.md)
