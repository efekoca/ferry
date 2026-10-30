import { spawn, spawnSync } from "node:child_process";
import { appendFileSync, cpSync, mkdirSync, mkdtempSync, readFileSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const args = new Set(process.argv.slice(2));
const script = join(root, "e2e", args.has("--smoke") ? "native-smoke.js" : "native-full.js");
const timeoutMs = 30 * 60 * 1000;

const work = mkdtempSync(join(tmpdir(), "ferry-e2e-"));
const demo = join(work, "demo");
const fixtures = join(work, "fixtures");
cpSync(join(root, "demo"), demo, { recursive: true });

const write = (path, content) => {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, content);
};

const unicode = join(fixtures, "Proje Ğüş klasörü");
write(join(unicode, "composer.json"), JSON.stringify({ require: { php: ">=7.4" } }, null, 2));
write(join(unicode, "src/a.php"), '<?php\nfunction greet(string $name = null) {\n    return "Merhaba ${name}";\n}\n');

const odd = join(fixtures, "odd");
write(join(odd, "composer.json"), JSON.stringify({ require: { php: ">=7.4" } }, null, 2));
write(
  join(odd, "composer.lock"),
  JSON.stringify(
    {
      packages: [{ name: "acme/x", version: "1.0.0", require: { php: ">=5.3" } }],
      "packages-dev": [{ name: "acme/dev", version: "2.0.0", require: { php: "^7.1" } }],
    },
    null,
    2,
  ),
);
write(join(odd, "src/a.php"), "<?php\nclass Order {\n    public function total(Money $discount = null) {}\n}\n");
write(join(odd, "big.php"), `<?php\n${" ".repeat(2 * 1024 * 1024 + 16)}\n`);
write(join(odd, "vendor/acme/x/legacy.php"), "<?php\nmysql_query('SELECT 1');\n");
symlinkSync(".", join(odd, "loop"), "dir");

mkdirSync(join(fixtures, "empty"), { recursive: true });
write(join(fixtures, "empty/README.md"), "No PHP here.\n");

async function devServerUp() {
  try {
    return (await fetch("http://localhost:1420")).ok;
  } catch {
    return false;
  }
}

let dev = null;
if (!(await devServerUp())) {
  dev = spawn("pnpm", ["dev"], { cwd: root, stdio: "ignore" });
  for (let i = 0; i < 60 && !(await devServerUp()); i++) await new Promise((r) => setTimeout(r, 500));
}

if (!args.has("--no-build")) {
  const build = spawnSync("cargo", ["build", "-p", "ferry"], { cwd: root, stdio: "inherit" });
  if (build.status !== 0) process.exit(build.status ?? 1);
}

const exe = join(root, "target", "debug", process.platform === "win32" ? "ferry.exe" : "ferry");
const app = spawn(exe, [], {
  cwd: root,
  env: { ...process.env, FERRY_E2E: script, FERRY_E2E_ROOT: JSON.stringify({ demo, fixtures }) },
  stdio: ["ignore", "pipe", "inherit"],
});

const results = { pass: [], fail: [], errors: [] };
let buffer = "";
const finish = (code) => {
  app.kill();
  dev?.kill();
  console.log(`\nPASS ${results.pass.length}: ${results.pass.join(", ")}`);
  if (results.fail.length) console.log(`FAIL ${results.fail.length}:\n  ${results.fail.join("\n  ")}`);
  if (results.errors.length) console.log(`JS errors:\n  ${results.errors.join("\n  ")}`);
  console.log(`Fixtures: ${work}`);
  process.exit(code);
};
const timer = setTimeout(() => {
  console.log("Timed out");
  finish(1);
}, timeoutMs);

app.stdout.on("data", (chunk) => {
  buffer += chunk;
  let nl;
  while ((nl = buffer.indexOf("\n")) >= 0) {
    const line = buffer.slice(0, nl);
    buffer = buffer.slice(nl + 1);
    if (!line.startsWith("[e2e] ")) continue;
    let msg = line.slice(6);
    try {
      msg = JSON.parse(msg);
    } catch {}
    console.log(msg);
    if (msg.startsWith("MODIFY ")) appendFileSync(msg.slice(7), "\n// edited outside Ferry\n");
    else if (msg.startsWith("DUMP ")) console.log(readFileSync(msg.slice(5), "utf8"));
    else if (msg.startsWith("PASS ")) results.pass.push(msg.slice(5));
    else if (msg.startsWith("FAIL ")) results.fail.push(msg.slice(5));
    else if (/^\s*(JSERROR|REJECTION|CONSOLE)/.test(msg)) results.errors.push(msg.trim());
    else if (msg === "DONE" || msg.startsWith("STEP done")) {
      clearTimeout(timer);
      finish(results.fail.length || results.errors.length ? 1 : 0);
    }
  }
});
app.on("exit", (code) => {
  clearTimeout(timer);
  console.log(`App exited (${code})`);
  finish(results.fail.length ? 1 : code ?? 1);
});
