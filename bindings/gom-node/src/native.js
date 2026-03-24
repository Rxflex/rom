import { existsSync } from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import { detectNativePrebuildId } from "./platform.js";

const require = createRequire(import.meta.url);
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const packageRoot = path.resolve(__dirname, "..");

function candidatePaths() {
  const candidates = [];

  if (process.env.ROM_NATIVE_NODE_BINDING) {
    candidates.push(process.env.ROM_NATIVE_NODE_BINDING);
  }

  const prebuildId = detectNativePrebuildId();
  if (prebuildId !== null) {
    candidates.push(path.join(packageRoot, "prebuilds", prebuildId, "rom_node_native.node"));
  }

  candidates.push(path.join(packageRoot, "rom_node_native.node"));

  return candidates;
}

export function loadNativeBridge() {
  if (process.env.ROM_FORCE_CLI_BRIDGE === "1") {
    return null;
  }

  for (const candidate of candidatePaths()) {
    if (!existsSync(candidate)) {
      continue;
    }

    try {
      return require(candidate);
    } catch {
      continue;
    }
  }

  return null;
}
