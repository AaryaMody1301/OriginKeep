import { execFileSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, writeFileSync } from "node:fs";
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

const outputDirectory = join("src-tauri", "binaries");
const destination = join(
  outputDirectory,
  `originkeep-native-host-${targetTriple}${extension}`,
);
mkdirSync(outputDirectory, { recursive: true });

// Tauri's compile-time context validates configured bundle assets. A generated,
// ignored placeholder breaks the build-order cycle before Cargo produces the
// real native host, and is always replaced after a successful build.
if (!existsSync(destination)) writeFileSync(destination, "");

const cargoArgs = [
  "build",
  "--locked",
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
copyFileSync(source, destination);
console.log(`Prepared OriginKeep native host sidecar: ${destination}`);
