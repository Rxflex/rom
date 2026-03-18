# ROM

ROM is an early-stage browser emulation runtime built around a lightweight embedded JavaScript engine instead of Chromium.

Current direction:

- `ROM` is the engine and runtime layer: Rust + embedded runtime + browser-like host objects.
- JS and Python wrappers will be added later on top of the Rust core.
- Compatibility is driven by browser-facing scripts, with `fingerprintjs` planned as a hard acceptance harness.

Current repository layout:

- `crates/rom-core`: raw embedded JavaScript engine wrapper.
- `crates/rom-webapi`: browser API bootstrap and compatibility shims.
- `crates/rom-runtime`: high-level runtime that composes engine + Web API layer.

Compatibility work already has two layers:

- internal structured probes via `RomRuntime::surface_snapshot()` and `RomRuntime::fingerprint_probe()`
- a vendored `FingerprintJS` harness via `RomRuntime::run_fingerprintjs_harness()`

The browser surface now also includes an initial web data stack:

- `fetch`
- `Headers`, `Request`, `Response`
- `AbortController`, `AbortSignal`
- `Blob`, `File`, `FormData`
- `URL`, `URLSearchParams`
- `URLPattern`
- `DOMParser`
- body helpers: `text()`, `json()`, `arrayBuffer()`, `blob()`, `formData()`, `bodyUsed`
- multipart `FormData` request serialization
- `blob:` object URLs via `URL.createObjectURL()` and `URL.revokeObjectURL()`
- `crypto.getRandomValues()`, `crypto.randomUUID()`
- `crypto.subtle.digest()` for `SHA-1`, `SHA-256`, `SHA-384`, `SHA-512`
- HMAC `generateKey()`, `importKey()`, `exportKey()`, `sign()`, `verify()`
- `AES-GCM` `generateKey()`, `importKey()`, `exportKey()`, `encrypt()`, `decrypt()`
- `PBKDF2` `importKey()`, `deriveBits()`, `deriveKey()`
- `SubtleCrypto.wrapKey()` and `SubtleCrypto.unwrapKey()` via the current secret-key flow
- `document.cookie` with path/domain/secure handling
- fetch cookie roundtrip via `Cookie` / `Set-Cookie` for `same-origin` and `include` credentials modes
- CORS response gating for cross-origin `fetch`
- CORS preflight for unsafe methods/headers with `Access-Control-*` validation
- `structuredClone()`
- `MessageEvent`, `MessagePort`, `MessageChannel`
- `Worker` with `Blob` URL scripts, `postMessage()`, and `importScripts()`
- `ReadableStream`-based `Request.body` / `Response.body`
- redirect policy for `fetch`: `follow`, `error`, `manual`
- `BroadcastChannel`
- `FileReader` with `readAsText()`, `readAsArrayBuffer()`, `readAsDataURL()`

There is also an optional real-browser reference runner:

- `npm install`
- `npx playwright install chromium`
- `npm run fingerprintjs:browser-reference`
