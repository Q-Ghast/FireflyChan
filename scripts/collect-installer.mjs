import { createHash } from "node:crypto";
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { basename, join } from "node:path";

const root = process.cwd();
const packageJson = JSON.parse(
  readFileSync(join(root, "package.json"), "utf8"),
);
const version = packageJson.version;
const source = join(
  root,
  "src-tauri",
  "target",
  "release",
  "bundle",
  "nsis",
  `FireflyChan_${version}_x64-setup.exe`,
);

if (!existsSync(source)) {
  console.error(`Installer not found: ${source}`);
  process.exit(1);
}

const outputDir = join(root, "installers");
mkdirSync(outputDir, { recursive: true });

const fileName = basename(source);
const destination = join(outputDir, fileName);
copyFileSync(source, destination);

const hash = createHash("sha256")
  .update(readFileSync(destination))
  .digest("hex")
  .toUpperCase();
const checksumPath = join(outputDir, "SHA256.txt");
const existing = existsSync(checksumPath)
  ? readFileSync(checksumPath, "utf8")
  : "";
const lines = existing
  .split(/\r?\n/)
  .filter((line) => line.trim() && !line.endsWith(`  ${fileName}`));
lines.push(`${hash}  ${fileName}`);
writeFileSync(checksumPath, `${lines.join("\n")}\n`, "utf8");

console.log(`Installer copied to ${destination}`);
console.log(`SHA-256: ${hash}`);
