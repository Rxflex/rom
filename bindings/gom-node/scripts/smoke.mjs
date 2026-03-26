import { RomRuntime, hasNativeBinding } from "../src/index.js";

async function main() {
  const runtime = new RomRuntime({
    href: "https://example.test/",
    cookie_store: "seed=1; path=/",
    local_storage: { VerifyAuthToken: "seeded-storage" },
  });
  const href = await runtime.evalAsync("(async () => location.href)()");
  await runtime.evalAsync("(async () => { globalThis.__romSmokeValue = 42; return 'ok'; })()");
  const persisted = await runtime.evalAsync("(async () => String(globalThis.__romSmokeValue))()");
  const cookie = await runtime.evalAsync("(async () => document.cookie)()");
  const storage = await runtime.evalAsync("(async () => localStorage.getItem('VerifyAuthToken'))()");
  await runtime.setContent(
    '<div id="app"><input id="name" /><button id="go">Go</button><span id="out"></span></div>' +
      '<script>document.querySelector("#go").addEventListener("click",()=>{document.querySelector("#out").textContent=document.querySelector("#name").value;});</script>',
  );
  await runtime.fill("#name", "ROM");
  await runtime.click("#go");
  const pageText = await runtime.textContent("#out");
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

  if (cookie !== "seed=1") {
    throw new Error(`Expected seeded cookie, got: ${cookie}`);
  }

  if (storage !== "seeded-storage") {
    throw new Error(`Expected seeded localStorage, got: ${storage}`);
  }

  if (pageText !== "ROM") {
    throw new Error(`Expected page helper flow to update text, got: ${pageText}`);
  }

  console.log(JSON.stringify({ native: true, href, persisted, cookie, storage, pageText }));
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
