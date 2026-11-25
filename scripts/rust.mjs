// Rust check/test through blaze (https://github.com/indexable-inc/blaze),
// falling back to cargo when blaze isn't installed or can't build here
// (Windows, Intel Macs, or a blaze release without its toolchain).
// Usage: node scripts/rust.mjs check|test [--cargo]
import { spawnSync } from "node:child_process";

const cmd = process.argv[2] ?? "check";
const forceCargo = process.argv.includes("--cargo") || process.env.SOLOFORGE_NO_BLAZE === "1";

function run(bin, args) {
  const r = spawnSync(bin, args, { stdio: ["inherit", "inherit", "pipe"], encoding: "utf8" });
  if (r.stderr) process.stderr.write(r.stderr);
  return r;
}

if (!forceCargo && process.platform !== "win32") {
  const r = run("blaze", [cmd]);
  if (!r.error && r.status === 0) process.exit(0);
  const why = r.error ? "blaze is not installed" : /toolchain|no dependency graph|refused/i.test(r.stderr ?? "") ? "blaze can't build here" : null;
  if (!why) process.exit(r.status ?? 1); // a real failure from blaze
  console.warn(`\n[soloforge] ${why}; using cargo instead.\n`);
}
const cargoArgs = cmd === "test" ? ["test", "--workspace"] : ["check", "--workspace", "--all-targets"];
const r = run("cargo", cargoArgs);
process.exit(r.status ?? 1);
