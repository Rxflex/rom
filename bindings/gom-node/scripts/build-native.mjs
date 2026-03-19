import { copyFileSync, existsSync, mkdirSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const packageRoot = path.resolve(__dirname, "..");
const profile = process.argv.includes("--release") ? "release" : "debug";
const targetDir = process.env.CARGO_TARGET_DIR || path.join(packageRoot, "target");
const artifactDir = path.join(targetDir, profile);
const outputPath = path.join(packageRoot, "rom_node_native.node");

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
