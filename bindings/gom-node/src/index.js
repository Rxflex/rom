import { execFile } from "node:child_process";
import { loadNativeBridge } from "./native.js";
import { fileURLToPath } from "node:url";
import path from "node:path";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const repoRoot = path.resolve(__dirname, "..", "..", "..");
const nativeBridge = loadNativeBridge();

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

export class RomRuntime {
  constructor(config = {}) {
    this.config = config;
  }

  async #run(command, payload = {}) {
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
