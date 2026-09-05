import { cpSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const source = "extension";
const outputRoot = "browser-packages";
const chromium = join(outputRoot, "chromium");
const firefox = join(outputRoot, "firefox");

rmSync(outputRoot, { recursive: true, force: true });
mkdirSync(outputRoot, { recursive: true });

cpSync(source, chromium, {
  recursive: true,
  filter: (path) => !path.endsWith("manifest.firefox.json"),
});
cpSync(source, firefox, {
  recursive: true,
  filter: (path) => !path.endsWith("manifest.json") || path.endsWith("manifest.firefox.json"),
});

const firefoxManifest = JSON.parse(
  readFileSync(join(source, "manifest.firefox.json"), "utf8"),
);
rmSync(join(firefox, "manifest.firefox.json"), { force: true });
writeFileSync(join(firefox, "manifest.json"), `${JSON.stringify(firefoxManifest, null, 2)}\n`);

console.log(`Prepared Chromium companion: ${chromium}`);
console.log(`Prepared Firefox companion: ${firefox}`);
