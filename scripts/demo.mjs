import { cpSync, existsSync, readdirSync, rmSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const source = join(root, "demo");
const target = join(root, ".demo");

if (existsSync(target)) rmSync(target, { recursive: true, force: true });

const projects = readdirSync(source, { withFileTypes: true }).filter((d) => d.isDirectory());
for (const project of projects) {
  cpSync(join(source, project.name), join(target, project.name), { recursive: true });
}

console.log("Demo projects are ready:\n");
for (const project of projects) console.log(`  ${join(target, project.name)}`);
