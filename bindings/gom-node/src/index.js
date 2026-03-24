import { execFile } from "node:child_process";
import { loadNativeBridge } from "./native.js";
import { fileURLToPath } from "node:url";
import path from "node:path";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const repoRoot = path.resolve(__dirname, "..", "..", "..");
const nativeBridge = loadNativeBridge();
const NativeRomRuntime = nativeBridge?.NativeRomRuntime ?? null;
const COOKIE_ATTRIBUTE_NAMES = new Set([
  "domain",
  "path",
  "expires",
  "max-age",
  "samesite",
  "secure",
  "httponly",
]);

function resolveBridgeCommand() {
  if (process.env.ROM_BRIDGE_BIN) {
    return {
      file: process.env.ROM_BRIDGE_BIN,
      args: [],
      cwd: process.env.ROM_BRIDGE_CWD || repoRoot,
    };
  }

  return {
    file: "cargo",
    args: ["run", "--quiet", "-p", "rom-runtime", "--bin", "rom_bridge"],
    cwd: process.env.ROM_BRIDGE_CWD || repoRoot,
  };
}

function parseBridgeResponse(stdout, stderr, error) {
  const trimmed = stdout.trim();

  if (!trimmed) {
    throw error ?? new Error(`ROM bridge produced no output.\n${stderr}`);
  }

  let response;
  try {
    response = JSON.parse(trimmed);
  } catch (parseError) {
    throw new Error(
      `ROM bridge returned invalid JSON: ${parseError.message}\n${stdout}\n${stderr}`,
    );
  }

  if (error || !response.ok) {
    throw new Error(response.error || error?.message || "ROM bridge command failed.");
  }

  return {
    result: response.result,
    state: response.state ?? null,
  };
}

function runNativeBridge(command, payload) {
  if (!nativeBridge || typeof nativeBridge.executeBridge !== "function") {
    return null;
  }

  const responseText = nativeBridge.executeBridge(JSON.stringify({ command, ...payload }));
  return Promise.resolve(parseBridgeResponse(responseText, "", null));
}

function runCliBridge(command, payload) {
  const bridge = resolveBridgeCommand();

  return new Promise((resolve, reject) => {
    const child = execFile(
      bridge.file,
      bridge.args,
      {
        cwd: bridge.cwd,
        env: process.env,
        maxBuffer: 10 * 1024 * 1024,
      },
      (error, stdout, stderr) => {
        try {
          resolve(parseBridgeResponse(stdout, stderr, error));
        } catch (bridgeError) {
          reject(bridgeError);
        }
      },
    );

    child.stdin.end(JSON.stringify({ command, ...payload }));
  });
}

function runBridge(command, payload) {
  return runNativeBridge(command, payload) ?? runCliBridge(command, payload);
}

function applyBridgeState(targetConfig, state) {
  if (!state || typeof state.cookie_store !== "string") {
    return targetConfig;
  }

  return {
    ...targetConfig,
    cookie_store: state.cookie_store,
  };
}

function normalizeRuntimeConfig(config = {}) {
  const normalized = { ...config };
  const cookieStore = normalizeCookieStoreInput(
    normalized.cookie_store ?? normalized.cookies ?? null,
    normalized.href,
  );

  delete normalized.cookies;
  if (cookieStore !== null) {
    normalized.cookie_store = cookieStore;
  }

  return normalized;
}

function normalizeCookieStoreInput(value, href) {
  if (value === null || value === undefined || value === "") {
    return null;
  }

  if (typeof value === "string") {
    const trimmed = value.trim();
    if (!trimmed) {
      return null;
    }

    if (looksLikeSerializedCookieStore(trimmed)) {
      return trimmed;
    }

    return serializeCookieEntries(parseCookieHeaderString(trimmed, href));
  }

  if (Array.isArray(value)) {
    return serializeCookieEntries(normalizeCookieEntries(value, href));
  }

  if (typeof value === "object") {
    return serializeCookieEntries(normalizeCookieEntries(value, href));
  }

  return null;
}

function looksLikeSerializedCookieStore(value) {
  try {
    const parsed = JSON.parse(value);
    return Array.isArray(parsed);
  } catch {
    return false;
  }
}

function normalizeCookieEntries(value, href) {
  if (Array.isArray(value)) {
    return value
      .map((entry) => normalizeCookieEntryObject(entry, href))
      .filter(Boolean);
  }

  return Object.entries(value).map(([name, entryValue]) =>
    createCookieEntry(name, entryValue, href),
  );
}

function normalizeCookieEntryObject(entry, href) {
  if (!entry || typeof entry !== "object") {
    return null;
  }

  const url = safeCookieUrl(href);
  const path = typeof entry.path === "string" && entry.path.startsWith("/") ? entry.path : "/";
  const domain =
    typeof entry.domain === "string" && entry.domain.trim()
      ? entry.domain.trim().replace(/^\./, "").toLowerCase()
      : url.hostname.toLowerCase();

  return {
    name: String(entry.name ?? ""),
    value: String(entry.value ?? ""),
    domain,
    hostOnly: entry.hostOnly ?? !("domain" in entry),
    path,
    secure: Boolean(entry.secure ?? (url.protocol === "https:")),
    httpOnly: Boolean(entry.httpOnly),
    sameSite: normalizeSameSite(entry.sameSite),
    expiresAt: normalizeExpiresAt(entry.expiresAt),
  };
}

function parseCookieHeaderString(value, href) {
  return value
    .split(";")
    .map((part) => part.trim())
    .filter(Boolean)
    .map((part) => {
      const separator = part.indexOf("=");
      if (separator <= 0) {
        return null;
      }

      const name = part.slice(0, separator).trim();
      if (!name || COOKIE_ATTRIBUTE_NAMES.has(name.toLowerCase())) {
        return null;
      }

      return createCookieEntry(name, part.slice(separator + 1), href);
    })
    .filter(Boolean);
}

function createCookieEntry(name, value, href) {
  const url = safeCookieUrl(href);
  return {
    name: String(name),
    value: String(value),
    domain: url.hostname.toLowerCase(),
    hostOnly: true,
    path: "/",
    secure: url.protocol === "https:",
    httpOnly: false,
    sameSite: "Lax",
    expiresAt: null,
  };
}

function safeCookieUrl(href) {
  try {
    return new URL(href || "https://rom.local/");
  } catch {
    return new URL("https://rom.local/");
  }
}

function normalizeSameSite(value) {
  const normalized = String(value ?? "Lax").toLowerCase();
  if (normalized === "strict") {
    return "Strict";
  }
  if (normalized === "none") {
    return "None";
  }
  return "Lax";
}

function normalizeExpiresAt(value) {
  if (value === null || value === undefined || value === "") {
    return null;
  }

  const numeric = Number(value);
  return Number.isFinite(numeric) ? numeric : null;
}

function serializeCookieEntries(entries) {
  if (!Array.isArray(entries) || entries.length === 0) {
    return null;
  }

  return JSON.stringify(entries.filter((entry) => entry && entry.name));
}

export class RomRuntime {
  #nativeRuntime = null;

  constructor(config = {}) {
    this.config = normalizeRuntimeConfig(config);
    if (typeof NativeRomRuntime === "function") {
      this.#nativeRuntime = new NativeRomRuntime(JSON.stringify(this.config));
    }
  }

  #applyCookieStore(cookieStore) {
    if (typeof cookieStore !== "string") {
      return;
    }

    this.config = {
      ...this.config,
      cookie_store: cookieStore,
    };
  }

  #syncNativeState() {
    if (!this.#nativeRuntime || typeof this.#nativeRuntime.exportCookieStore !== "function") {
      return;
    }

    this.#applyCookieStore(this.#nativeRuntime.exportCookieStore());
  }

  async #runNative(method, ...args) {
    if (!this.#nativeRuntime || typeof this.#nativeRuntime[method] !== "function") {
      return null;
    }

    const result = await Promise.resolve(this.#nativeRuntime[method](...args));
    this.#syncNativeState();
    return result;
  }

  async #run(command, payload = {}) {
    if (this.#nativeRuntime) {
      if (command === "eval") {
        return this.#runNative("eval", payload.script);
      }
      if (command === "eval-async") {
        return this.#runNative("evalAsync", payload.script);
      }
      if (command === "surface-snapshot") {
        return JSON.parse(await this.#runNative("surfaceSnapshotJson"));
      }
      if (command === "fingerprint-probe") {
        return JSON.parse(await this.#runNative("fingerprintProbeJson"));
      }
      if (command === "fingerprint-js-harness") {
        return JSON.parse(await this.#runNative("fingerprintJsHarnessJson"));
      }
      if (command === "fingerprint-js-version") {
        return this.#runNative("fingerprintJsVersion");
      }
    }

    const response = await runBridge(command, {
      config: this.config,
      ...payload,
    });
    this.config = applyBridgeState(this.config, response.state);
    return response.result;
  }

  eval(script) {
    return this.#run("eval", { script });
  }

  evalAsync(script) {
    return this.#run("eval-async", { script });
  }

  async evalJson(script, { async = true } = {}) {
    const result = async ? await this.evalAsync(script) : await this.eval(script);
    return JSON.parse(result);
  }

  surfaceSnapshot() {
    return this.#run("surface-snapshot");
  }

  fingerprintProbe() {
    return this.#run("fingerprint-probe");
  }

  runFingerprintJsHarness() {
    return this.#run("fingerprint-js-harness");
  }

  fingerprintJsVersion() {
    return this.#run("fingerprint-js-version");
  }
}

export function createRuntime(config = {}) {
  return new RomRuntime(config);
}

export function hasNativeBinding() {
  return !!nativeBridge;
}
