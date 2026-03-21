import process from "node:process";

export function detectNativePrebuildId() {
  if (process.platform === "linux" && process.arch === "x64") {
    if (detectLinuxLibcFlavor() !== "gnu") {
      return null;
    }
    return "linux-x64-gnu";
  }

  if (process.platform === "win32" && process.arch === "x64") {
    return "win32-x64-msvc";
  }

  if (process.platform === "darwin" && process.arch === "x64") {
    return "darwin-x64";
  }

  if (process.platform === "darwin" && process.arch === "arm64") {
    return "darwin-arm64";
  }

  return null;
}

function detectLinuxLibcFlavor() {
  const report = process.report?.getReport?.();
  const glibcVersion =
    report?.header?.glibcVersionRuntime ?? report?.header?.glibcVersionCompiler ?? null;
  return glibcVersion ? "gnu" : "musl";
}
