// Rust check/test. Uses cargo by default.
// blaze (https://github.com/indexable-inc/blaze) is opt-in for now: pass --blaze
// or set SOLOFORGE_USE_BLAZE=1. If blaze can't build here, this falls back to cargo.
// Usage: node scripts/rust.mjs check|test [--blaze]
import { spawnSync } from "node:child_process";

const cmd = process.argv[2] ?? "check";
const useBlaze = process.argv.includes("--blaze") || process.env.SOLOFORGE_USE_BLAZE === "1";

function run(bin, args) {
  const r = spawnSync(bin, args, { stdio: ["inherit", "inherit", "pipe"], encoding: "utf8" });
  if (r.stderr) process.stderr.write(r.stderr);
  return r;
}

if (useBlaze && process.platform !== "win32") {
  const r = run("blaze", [cmd]);
  if (!r.error && r.status === 0) process.exit(0);
  const why = r.error ? "blaze is not installed" : /toolchain|no dependency graph|refused/i.test(r.stderr ?? "") ? "blaze can't build here" : null;
  if (!why) process.exit(r.status ?? 1); // a real failure from blaze
  console.warn(`\n[soloforge] ${why}; using cargo instead.\n`);
}
const cargoArgs = cmd === "test" ? ["test", "--workspace"] : ["check", "--workspace", "--all-targets"];
const r = run("cargo", cargoArgs);
process.exit(r.status ?? 1);
