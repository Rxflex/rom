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
  cors_enabled: false,
  proxy_url: process.env.ROM_PROXY_URL ?? null,
});

const href = await runtime.evalAsync("(async () => location.href)()");
const snapshot = await runtime.surfaceSnapshot();

console.log("native:", hasNativeBinding());
console.log(href);
console.log(snapshot.fetch);
```

Config keys use the Rust runtime field names, so use snake_case such as `cors_enabled` and `proxy_url`.
`cors_enabled` is `false` by default.

## Optional native build

```bash
npm run build:native
```

`npm pack` and `npm publish` now run the native release build automatically via `prepack`, and the produced `rom_node_native.node` is included in the published tarball for the platform that performed the publish.

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
