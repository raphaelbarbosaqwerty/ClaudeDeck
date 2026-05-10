#!/usr/bin/env node
// Bumps the project version in lockstep across the three places Tauri
// looks at it: package.json, src-tauri/Cargo.toml, src-tauri/tauri.conf.json.
// Drift between these causes the bundle to ship one version while the app
// reports another, so we keep them mechanically in sync.
//
// Usage:
//   pnpm bump 0.2.0
//   pnpm bump 0.2.0-beta.1
//
// The script does NOT commit, tag, or push — it only edits the files. A
// follow-up `git commit && git tag v$VERSION && git push origin v$VERSION`
// triggers the release workflow. Keeping that explicit prevents accidental
// releases when someone runs `pnpm bump` to test a value.

import { readFileSync, writeFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, "..");

const SEMVER =
  /^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$/;

const target = process.argv[2];
if (!target) {
  console.error("usage: pnpm bump <version>");
  console.error("  e.g. pnpm bump 0.2.0");
  process.exit(1);
}
if (!SEMVER.test(target)) {
  console.error(`error: "${target}" is not a valid SemVer version.`);
  console.error('Examples that work: "0.2.0", "1.0.0", "0.3.0-beta.1"');
  process.exit(1);
}

/** Edit package.json. JSON, so straight read-parse-write. */
function bumpPackageJson() {
  const path = resolve(ROOT, "package.json");
  const pkg = JSON.parse(readFileSync(path, "utf8"));
  const before = pkg.version;
  pkg.version = target;
  // Preserve trailing newline that pnpm/npm tooling expects.
  writeFileSync(path, JSON.stringify(pkg, null, 2) + "\n");
  return { path: "package.json", before };
}

/** Edit src-tauri/tauri.conf.json. Same shape as package.json. */
function bumpTauriConf() {
  const path = resolve(ROOT, "src-tauri/tauri.conf.json");
  const conf = JSON.parse(readFileSync(path, "utf8"));
  const before = conf.version;
  conf.version = target;
  writeFileSync(path, JSON.stringify(conf, null, 2) + "\n");
  return { path: "src-tauri/tauri.conf.json", before };
}

/** Edit src-tauri/Cargo.toml. We don't want a TOML parser dep, so we do a
 *  surgical regex on the [package] block's `version` line. The pattern
 *  anchors on `^version =` to avoid touching dependency version strings. */
function bumpCargoToml() {
  const path = resolve(ROOT, "src-tauri/Cargo.toml");
  const text = readFileSync(path, "utf8");
  const re = /(^|\n)version\s*=\s*"([^"]+)"/;
  const match = text.match(re);
  if (!match) {
    console.error(`error: could not find a 'version = "..."' line in ${path}`);
    process.exit(1);
  }
  const before = match[2];
  const replaced = text.replace(re, `${match[1]}version = "${target}"`);
  writeFileSync(path, replaced);
  return { path: "src-tauri/Cargo.toml", before };
}

const edits = [bumpPackageJson(), bumpTauriConf(), bumpCargoToml()];

console.log(`Bumped version to ${target}:\n`);
for (const e of edits) {
  console.log(`  ${e.path}: ${e.before} → ${target}`);
}
console.log(`
Next steps:
  git add package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json
  git commit -m "chore: bump version to ${target}"
  git tag v${target}
  git push origin main v${target}

The release workflow triggers on the v${target} tag and produces
.dmg / .exe / .deb / .AppImage artifacts attached to a draft GitHub
release. Review the draft, edit the changelog body, then click Publish.
`);
