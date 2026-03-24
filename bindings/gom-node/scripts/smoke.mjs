import { RomRuntime, hasNativeBinding } from "../src/index.js";

async function main() {
  const runtime = new RomRuntime({ href: "https://example.test/" });
  const href = await runtime.evalAsync("(async () => location.href)()");
  await runtime.evalAsync("(async () => { globalThis.__romSmokeValue = 42; return 'ok'; })()");
  const persisted = await runtime.evalAsync("(async () => String(globalThis.__romSmokeValue))()");
  const snapshot = await runtime.surfaceSnapshot();

  if (!hasNativeBinding()) {
    throw new Error("Expected Node native binding to be loaded.");
  }

  if (href !== "https://example.test/") {
    throw new Error(`Unexpected href: ${href}`);
  }

  if (snapshot?.globals?.window !== true) {
    throw new Error("Surface snapshot did not expose window.");
  }

  if (persisted !== "42") {
    throw new Error(`Expected persisted global state, got: ${persisted}`);
  }

  console.log(JSON.stringify({ native: true, href, persisted }));
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
