// Fails if any npm package in the lock file has a licence outside the ALTAIS
// allowlist (the same list as deny.toml for Rust). Runtime packages are also
// listed, because they ship inside the app.
//
// Usage: node scripts/check-npm-licenses.mjs [path/to/package-lock.json]
// (default: package-lock.json in the current folder)

import { readFileSync } from "node:fs";

const ALLOWED = new Set([
  "0BSD",
  "Apache-2.0",
  "Apache-2.0 WITH LLVM-exception",
  "BSD-2-Clause",
  "BSD-3-Clause",
  "BSL-1.0",
  "CC0-1.0",
  "ISC",
  "MIT",
  "MPL-2.0",
  "Unicode-3.0",
  "Zlib",
]);

/** Accepts a single SPDX id or an "OR" expression where at least one option is allowed. */
function isAllowed(expression) {
  const options = expression.replace(/^\(|\)$/g, "").split(/\s+OR\s+/);
  return options.some((option) => ALLOWED.has(option.trim()));
}

const lockPath = process.argv[2] ?? "package-lock.json";
const lock = JSON.parse(readFileSync(lockPath, "utf8"));
const problems = [];
const runtime = [];

for (const [path, info] of Object.entries(lock.packages ?? {})) {
  if (!path) continue; // the project itself
  const name = path.replace(/^.*node_modules\//, "");
  const license = info.license ?? "(none)";
  if (!isAllowed(license)) problems.push(`${name}@${info.version}: ${license}`);
  if (!info.dev) runtime.push(`${name}@${info.version} (${license})`);
}

console.log(`Runtime packages (${runtime.length}):`);
for (const line of runtime.sort()) console.log(`  ${line}`);

if (problems.length) {
  console.error(`\nPackages with licences outside the allowlist (${problems.length}):`);
  for (const line of problems) console.error(`  ${line}`);
  process.exit(1);
}
console.log(`\nAll ${Object.keys(lock.packages).length - 1} packages use allowed licences.`);
