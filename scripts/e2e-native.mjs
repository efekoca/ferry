import { spawn, spawnSync } from "node:child_process";
import { appendFileSync, cpSync, mkdirSync, mkdtempSync, readFileSync, symlinkSync, writeFileSync } from "node:fs";
import { homedir, tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const args = new Set(process.argv.slice(2));
const script = join(root, "e2e", args.has("--smoke") ? "native-smoke.js" : "native-full.js");
const timeoutMs = 20 * 60 * 1000;

const syntax = spawnSync(process.execPath, ["--check", script], { stdio: "inherit" });
if (syntax.status !== 0) process.exit(1);

const work = mkdtempSync(join(tmpdir(), "ferry-e2e-"));
const bundled = join(work, "e2e.js");
writeFileSync(bundled, `${readFileSync(join(root, "node_modules", "axe-core", "axe.min.js"), "utf8")}\n${readFileSync(script, "utf8")}`);
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

write(join(fixtures, "php_errors.log"), "PHP Warning:  Undefined variable $x in /other-project/src/a.php on line 2\n");

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
symlinkSync(process.platform === "win32" ? odd : ".", join(odd, "loop"), process.platform === "win32" ? "junction" : "dir");

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
  dev = spawn("pnpm", ["dev"], { cwd: root, stdio: "ignore", shell: process.platform === "win32", detached: process.platform !== "win32" });
  for (let i = 0; i < 60 && !(await devServerUp()); i++) await new Promise((r) => setTimeout(r, 500));
}

if (!args.has("--no-build")) {
  const build = spawnSync("cargo", ["build", "-p", "ferry"], { cwd: root, stdio: "inherit" });
  if (build.status !== 0) process.exit(build.status ?? 1);
}

const defaultTools = {
  darwin: join(homedir(), "Library", "Application Support", "io.github.efekoca.ferry", "tools"),
  win32: join(process.env.APPDATA ?? "", "io.github.efekoca.ferry", "tools"),
}[process.platform] ?? join(homedir(), ".local", "share", "io.github.efekoca.ferry", "tools");
const tools = process.env.FERRY_TEST_TOOLS ?? defaultTools;

const exe = join(root, "target", "debug", process.platform === "win32" ? "ferry.exe" : "ferry");
const app = spawn(exe, [], {
  cwd: root,
  env: {
    ...process.env,
    FERRY_E2E: bundled,
    FERRY_E2E_ROOT: JSON.stringify({ demo, fixtures }),
    FERRY_DATA_DIR: join(work, "data"),
    FERRY_TOOLS_DIR: tools,
  },
  stdio: ["ignore", "pipe", "inherit"],
});

const results = { pass: [], fail: [], errors: [] };
let buffer = "";
const stopDev = () => {
  if (!dev) return;
  if (process.platform === "win32") spawnSync("taskkill", ["/pid", String(dev.pid), "/T", "/F"], { stdio: "ignore" });
  else process.kill(-dev.pid, "SIGTERM");
  dev = null;
};
const finish = (code) => {
  app.kill();
  stopDev();
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
