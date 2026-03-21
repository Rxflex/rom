import { cpSync, existsSync, mkdirSync, readdirSync, rmSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const packageRoot = path.resolve(__dirname, "..");
const args = process.argv.slice(2);
const sourceRoot =
  readOptionValue(args, "--source") || path.join(packageRoot, ".release-inputs", "prebuilds");
const outputRoot =
  readOptionValue(args, "--output") || path.join(packageRoot, "prebuilds");

if (!existsSync(sourceRoot)) {
  console.error(`Prebuild source directory not found: ${sourceRoot}`);
  process.exit(1);
}

rmSync(outputRoot, { recursive: true, force: true });
mkdirSync(outputRoot, { recursive: true });

for (const entry of readdirSync(sourceRoot, { withFileTypes: true })) {
  if (!entry.isDirectory()) {
    continue;
  }

  const sourceDir = path.join(sourceRoot, entry.name);
  const outputDir = path.join(outputRoot, entry.name);
  cpSync(sourceDir, outputDir, { recursive: true });
}

console.log(`Staged prebuilds from ${sourceRoot} to ${outputRoot}`);

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
