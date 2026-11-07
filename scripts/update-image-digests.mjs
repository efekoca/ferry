#!/usr/bin/env node
// Refreshes the Docker image digests in crates/ferry-core/src/images.rs. Needs Docker with buildx.
import { spawnSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const file = join(dirname(fileURLToPath(import.meta.url)), "../crates/ferry-core/src/images.rs");
let text = readFileSync(file, "utf8");
const entries = [...text.matchAll(/\("([^"]+)", "([^"]*)"\),/g)];
let failed = false;

for (const [line, tag, old] of entries) {
  const done = spawnSync("docker", ["buildx", "imagetools", "inspect", tag, "--format", "{{.Manifest.Digest}}"], { encoding: "utf8" });
  const digest = done.stdout.trim();
  if (done.status !== 0 || !/^sha256:[0-9a-f]{64}$/.test(digest)) {
    console.error(`${tag}: ${done.stderr.trim() || digest}`);
    failed = true;
    continue;
  }
  text = text.replace(line, `("${tag}", "${digest}"),`);
  console.log(`${tag} ${digest === old ? "unchanged" : digest}`);
}

writeFileSync(file, text);
process.exit(failed ? 1 : 0);
