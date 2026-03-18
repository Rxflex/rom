# Fingerprint Harness Plan

`FingerprintJS` is the target acceptance harness for ROM, but it should be introduced in stages.

## Stage 1

Keep an internal structured probe in Rust that records the browser surface ROM currently exposes:

- navigator values
- canvas support
- audio support
- observer constructors
- storage and media hooks

This stage is implemented through `RomRuntime::surface_snapshot()` and `RomRuntime::fingerprint_probe()`.

## Stage 2

Run a pinned `FingerprintJS` bundle inside ROM and capture:

- successful execution or thrown exception
- collected component keys
- confidence score
- deltas versus a reference browser run

Current status:

- implemented with a vendored `@fingerprintjs/fingerprintjs` `5.1.0` UMD bundle
- exposed through `RomRuntime::run_fingerprintjs_harness()`
- covered by a runtime test that asserts the bundle loads and returns a real result

## Stage 3

Turn the fixture into a regression gate:

- stable fixture input
- stored expected snapshot
- failing CI when a compatibility regression drops fields or breaks execution

## Browser Reference

An optional Playwright runner is available to collect the same harness report in real Chromium:

1. `npm install`
2. `npx playwright install chromium`
3. `npm run fingerprintjs:browser-reference`

Default output:

- `fixtures/fingerprintjs/browser-chromium-harness.json`

This snapshot is intended to be compared with:

- `fixtures/fingerprintjs/rom-default-harness.json`
