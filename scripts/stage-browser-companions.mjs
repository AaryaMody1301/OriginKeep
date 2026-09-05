import { cpSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const root = process.argv[2] || "artifacts/browser-companions";
const source = "extension";
const shared = ["service-worker.js", "context-capture.js", "icon.png"];

rmSync(root, { recursive: true, force: true });
mkdirSync(root, { recursive: true });

function stage(name, manifestPath) {
  const destination = join(root, name);
  mkdirSync(destination, { recursive: true });
  for (const file of shared) cpSync(join(source, file), join(destination, file));
  const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
  writeFileSync(join(destination, "manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`);
  return destination;
}

stage("chromium", join(source, "manifest.json"));
stage("firefox", join(source, "manifest.firefox.json"));
console.log(`Staged Chromium and Firefox companions under ${root}`);
