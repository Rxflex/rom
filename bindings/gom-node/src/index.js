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

  return response.result;
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

export class RomRuntime {
  constructor(config = {}) {
    this.config = config;
  }

  eval(script) {
    return runBridge("eval", { config: this.config, script });
  }

  evalAsync(script) {
    return runBridge("eval-async", { config: this.config, script });
  }

  async evalJson(script, { async = true } = {}) {
    const result = async ? await this.evalAsync(script) : await this.eval(script);
    return JSON.parse(result);
  }

  surfaceSnapshot() {
    return runBridge("surface-snapshot", { config: this.config });
  }

  fingerprintProbe() {
    return runBridge("fingerprint-probe", { config: this.config });
  }

  runFingerprintJsHarness() {
    return runBridge("fingerprint-js-harness", { config: this.config });
  }

  fingerprintJsVersion() {
    return runBridge("fingerprint-js-version", { config: this.config });
  }
}

export function createRuntime(config = {}) {
  return new RomRuntime(config);
}

export function hasNativeBinding() {
  return !!nativeBridge;
}
