import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync } from "node:fs";
import { join } from "node:path";

const release = process.argv.includes("--release");
const profile = release ? "release" : "debug";
const extension = process.platform === "win32" ? ".exe" : "";
const targetTriple = execFileSync("rustc", ["--print", "host-tuple"], {
  encoding: "utf8",
}).trim();

if (!targetTriple) {
  throw new Error("rustc did not report a host target triple");
}

const cargoArgs = [
  "build",
  "--manifest-path",
  "src-tauri/Cargo.toml",
  "--bin",
  "originkeep-native-host",
];
if (release) cargoArgs.push("--release");

execFileSync("cargo", cargoArgs, { stdio: "inherit" });

const source = join(
  "src-tauri",
  "target",
  profile,
  `originkeep-native-host${extension}`,
);
const outputDirectory = join("src-tauri", "binaries");
const destination = join(
  outputDirectory,
  `originkeep-native-host-${targetTriple}${extension}`,
);

mkdirSync(outputDirectory, { recursive: true });
copyFileSync(source, destination);
console.log(`Prepared OriginKeep native host sidecar: ${destination}`);
