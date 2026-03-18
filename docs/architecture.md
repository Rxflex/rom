# ROM Architecture Plan

## Core Decision

Use Rust as the engine implementation language and keep the repository focused on the native runtime first.

Reasoning:

1. Rust gives the cleanest path to a small native core without shipping Chromium.
2. Wrappers for Node.js and Python can be added later without polluting the core repository now.
3. Web API emulation can be built incrementally around a single host-state model.

## Target Shape

The system is split into three layers:

1. `rom-core`
   Raw embedded JavaScript runtime lifecycle, script execution, error handling.
2. `rom-webapi`
   Browser compatibility layer: globals, DOM tree, observer families, timers, storage, media, canvas/audio shims.
3. `rom-runtime`
   High-level environment assembly, configuration, validation, snapshots, future compatibility presets.

## Delivery Phases

### Phase 0

Ship a bootable browser-like shell with:

- `window`, `self`, `document`, `navigator`, `location`
- minimal DOM tree and events
- storage, history, screen, performance, crypto
- observer constructor stubs
- smoke tests proving browser-script entry works

### Phase 1

Turn the shell into a spec-driven DOM substrate:

- `Node`, `Element`, `Text`, `DocumentFragment`
- `querySelector(All)` and attribute/class behavior
- `MutationObserver`
- event propagation model
- timers and microtask scheduling

### Phase 2

Add web platform primitives needed by real-world scripts:

- `URL`, `fetch`, `Headers`, `Request`, `Response`
- `Blob`, `FormData`, `File`, `TextEncoder`, `TextDecoder`
- `crypto.subtle` surface
- `AbortController`

Status:

- `URL`, `URLSearchParams`, `fetch`, `Headers`, `Request`, `Response`, `AbortController`, `Blob`, `File`, and `FormData` are present
- `URLPattern` and `DOMParser` are present in initial but usable form
- `Request` and `Response` now expose `text()`, `json()`, `arrayBuffer()`, `blob()`, `formData()`, and `bodyUsed`
- `FormData` bodies serialize as multipart requests, and `blob:` object URLs are fetchable inside ROM
- `crypto.getRandomValues()`, `crypto.randomUUID()`, `crypto.subtle.digest()`, HMAC flows, `AES-CTR` / `AES-CBC` / `AES-GCM` with browser-like parameter validation, `AES-KW` wrapping flows, `PBKDF2` derivation, `HKDF` derivation, and secret-key wrap/unwrap flows are present
- `document.cookie` plus request/response cookie roundtrips are present in the initial networking model
- CORS enforcement is present for cross-origin `fetch`, including preflight and credential gating
- `structuredClone()`, `MessageEvent`, `MessagePort`, `MessageChannel`, and a first `Worker` model are present
- `ReadableStream`-based `Request.body` / `Response.body` and `fetch` redirect modes are present
- `EventSource` is present in an initial reconnecting SSE form with `open`, custom/message events, `lastEventId`, `retry`, reconnect, and `close()`
- `WebSocket` is present in an initial host-backed `ws:` / `wss:` form with text/binary messages, `Blob` and `ArrayBuffer` payloads, `binaryType = "arraybuffer"`, and close events
- `BroadcastChannel` and `FileReader` are present in an initial but usable form
- `MutationObserver` is present in an initial usable form for `childList`, `attributes`, `characterData`, `subtree`, and `oldValue`
- `ResizeObserver` and `IntersectionObserver` are present in initial usable form
- DOM event propagation is present in an initial usable form with capture/bubble phases, `once`, propagation stopping, and `composedPath()`
- `navigator.permissions`, `navigator.mediaDevices`, `navigator.userAgentData`, `navigator.plugins`, and `navigator.mimeTypes` are present in compatibility-oriented form
- viewport and media-query globals are present in compatibility-oriented form: `innerWidth/Height`, `outerWidth/Height`, `devicePixelRatio`, `visualViewport`, `screen.orientation`, `matchMedia`
- deeper networking and worker semantics still need work: fuller stream semantics, longer-lived `WebSocket` behavior, richer URL/file semantics, `no-cors`/opaque details, worker isolation fidelity, richer permission/device behavior, and broader `SubtleCrypto` coverage beyond the current digest/HMAC/AES-CTR/AES-CBC/AES-GCM/AES-KW/PBKDF2/HKDF slice

### Phase 3

Target fingerprint and anti-bot compatibility harnesses:

- canvas and audio behavior alignment
- plugin, mimeType, webdriver, permissions, media devices
- CSSOM and layout-related hooks that scripts probe
- deterministic presets and snapshot-based regression tests

## Acceptance Strategy

Acceptance should be based on scripted harnesses:

1. internal smoke tests
2. compatibility suites against browser-facing libraries
3. `fingerprintjs` confidence and component parity
