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

const runtime = new RomRuntime({ href: "https://example.test/" });
const href = await runtime.evalAsync("(async () => location.href)()");

console.log("native:", hasNativeBinding());
console.log(href);
```

## Optional native build

```bash
npm run build:native
```

## Environment

- `ROM_NATIVE_NODE_BINDING`: explicit path to a compiled `.node` addon
- `ROM_FORCE_CLI_BRIDGE=1`: disable the native path and force CLI fallback
- `ROM_BRIDGE_BIN`: explicit path to the `rom_bridge` executable
- `ROM_BRIDGE_CWD`: working directory used by the CLI fallback
