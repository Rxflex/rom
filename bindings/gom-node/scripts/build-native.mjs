import { copyFileSync, existsSync, mkdirSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { detectNativePrebuildId } from "../src/platform.js";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const packageRoot = path.resolve(__dirname, "..");
const args = process.argv.slice(2);
const profile = args.includes("--release") ? "release" : "debug";
const targetDir = process.env.CARGO_TARGET_DIR || path.join(packageRoot, "target");
const artifactDir = path.join(targetDir, profile);
const outputPath =
  readOptionValue(args, "--output") ||
  defaultOutputPath();

const cargoArgs = ["build", "--manifest-path", "Cargo.toml"];
if (profile === "release") {
  cargoArgs.push("--release");
}

const build = spawnSync("cargo", cargoArgs, {
  cwd: packageRoot,
  stdio: "inherit",
  env: process.env,
});

if (build.status !== 0) {
  process.exit(build.status ?? 1);
}

const artifactName =
  process.platform === "win32"
    ? "rom_node_native.dll"
    : process.platform === "darwin"
      ? "librom_node_native.dylib"
      : "librom_node_native.so";
const artifactPath = path.join(artifactDir, artifactName);

if (!existsSync(artifactPath)) {
  console.error(`Native addon artifact not found: ${artifactPath}`);
  process.exit(1);
}

mkdirSync(path.dirname(outputPath), { recursive: true });
copyFileSync(artifactPath, outputPath);
console.log(`Built ${outputPath}`);

function readOptionValue(argv, optionName) {
  const index = argv.indexOf(optionName);
  if (index === -1) {
    return null;
  }

  const value = argv[index + 1];
  if (!value || value.startsWith("--")) {
    console.error(`Missing value for ${optionName}`);
    process.exit(1);
  }

  return path.resolve(packageRoot, value);
}

function defaultOutputPath() {
  const prebuildId = detectNativePrebuildId();
  if (prebuildId === null) {
    return path.join(packageRoot, "rom_node_native.node");
  }

  return path.join(packageRoot, "prebuilds", prebuildId, "rom_node_native.node");
}
