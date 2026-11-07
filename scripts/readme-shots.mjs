// Takes the README screenshots from the mock UI (pnpm dev on port 1420) with headless Chrome,
// frames each one in a window on a gradient and writes assets/readme/<name>-<lang>.png.
// Usage: node scripts/readme-shots.mjs [out-folder]; then convert with cwebp.
import { spawn } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const outDir = resolve(process.argv[2] ?? join(root, "assets", "readme"));
mkdirSync(outDir, { recursive: true });
const APP = "http://localhost:1420/";
const VIEW = { width: 1280, height: 820 };
const FRAME = { width: 1432, height: 948 };
const CHROME = process.env.CHROME ?? "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
const icon = `data:image/png;base64,${readFileSync(join(root, "src-tauri", "icons", "64x64.png")).toString("base64")}`;

const port = 9300 + Math.floor(Math.random() * 500);
const chrome = spawn(CHROME, [
  "--headless=new",
  `--remote-debugging-port=${port}`,
  `--user-data-dir=${mkdtempSync(join(tmpdir(), "ferry-shot-"))}`,
  "--hide-scrollbars",
  "--no-first-run",
  "--force-color-profile=srgb",
  "about:blank",
]);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

let ws;
for (let i = 0; i < 50 && !ws; i++) {
  try {
    const page = (await (await fetch(`http://127.0.0.1:${port}/json/list`)).json()).find((t) => t.type === "page");
    if (page) ws = new WebSocket(page.webSocketDebuggerUrl);
  } catch {
    await sleep(200);
  }
}
await new Promise((r) => ws.addEventListener("open", r));
let seq = 0;
const pending = new Map();
ws.addEventListener("message", (e) => {
  const msg = JSON.parse(e.data);
  if (msg.id && pending.has(msg.id)) {
    pending.get(msg.id)(msg);
    pending.delete(msg.id);
  }
});
const send = (method, params = {}) =>
  new Promise((ok, fail) => {
    const n = ++seq;
    pending.set(n, (m) => (m.error ? fail(new Error(`${method}: ${JSON.stringify(m.error)}`)) : ok(m.result)));
    ws.send(JSON.stringify({ id: n, method, params }));
  });
const evaluate = async (expression) => {
  const r = await send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
  if (r.exceptionDetails) throw new Error(`${r.exceptionDetails.text} in ${expression.slice(0, 80)}`);
  return r.result.value;
};
const click = async (text, selector = "button, [role=option], [role=radio], a, li") => {
  const ok = await evaluate(`(() => {
    const want = ${JSON.stringify(text)};
    const el = [...document.querySelectorAll(${JSON.stringify(selector)})].find((e) => !e.disabled && (e.textContent.trim().includes(want) || e.getAttribute("aria-label") === want));
    if (!el) return false;
    el.click();
    return true;
  })()`);
  if (!ok) throw new Error(`nothing to click: "${text}"`);
  await sleep(600);
};
const capture = async () => {
  await sleep(700);
  return (await send("Page.captureScreenshot", { format: "png" })).data;
};

const frame = (shot) => `<!doctype html><html><head><meta charset="utf-8"><style>
  * { box-sizing: border-box; margin: 0; }
  body { width: ${FRAME.width}px; height: ${FRAME.height}px; overflow: hidden; font: 13px -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    background: radial-gradient(70% 80% at 0% 0%, #1c3270 0%, transparent 70%), radial-gradient(70% 80% at 100% 100%, #3b2a86 0%, transparent 70%), #0a0c1a; }
  .window { position: absolute; left: ${(FRAME.width - VIEW.width) / 2}px; top: ${(FRAME.height - VIEW.height - 30) / 2}px; width: ${VIEW.width}px;
    border-radius: 10px; overflow: hidden; border: 1px solid rgba(255,255,255,.09); box-shadow: 0 30px 80px rgba(0,0,0,.55), 0 4px 16px rgba(0,0,0,.35); background: #11101e; }
  .bar { height: 30px; display: flex; align-items: center; gap: 8px; padding: 0 12px; color: #c9c8d8; font-size: 12px; background: #0f0e1b; border-bottom: 1px solid rgba(255,255,255,.06); }
  .bar img { width: 14px; height: 14px; }
  .controls { margin-left: auto; display: flex; gap: 22px; align-items: center; color: #9b9ab0; }
  .controls svg { width: 11px; height: 11px; stroke: currentColor; stroke-width: 1.2; fill: none; }
  .shot { display: block; width: ${VIEW.width}px; height: ${VIEW.height}px; }
</style></head><body>
  <div class="window">
    <div class="bar"><img src="${icon}" alt=""><span>Ferry</span>
      <span class="controls"><svg viewBox="0 0 11 11"><path d="M1 5.5h9"/></svg><svg viewBox="0 0 11 11"><rect x="1" y="1" width="9" height="9"/></svg><svg viewBox="0 0 11 11"><path d="M1 1l9 9M10 1l-9 9"/></svg></span>
    </div>
    <img class="shot" src="data:image/png;base64,${shot}" alt="">
  </div>
</body></html>`;

const save = async (name, lang, shot) => {
  const file = join(tmpdir(), `ferry-frame-${name}-${lang}.html`);
  writeFileSync(file, frame(shot));
  await send("Emulation.setDeviceMetricsOverride", { ...FRAME, deviceScaleFactor: 2, mobile: false });
  await send("Page.navigate", { url: `file://${file}` });
  await sleep(900);
  const { data } = await send("Page.captureScreenshot", { format: "png", clip: { x: 0, y: 0, ...FRAME, scale: 1 } });
  writeFileSync(join(outDir, `${name}-${lang}.png`), Buffer.from(data, "base64"));
  console.log("saved", `${name}-${lang}.png`);
};

const open = async (lang) => {
  await send("Emulation.setDeviceMetricsOverride", { ...VIEW, deviceScaleFactor: 2, mobile: false });
  await send("Emulation.setUserAgentOverride", { userAgent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/130 Safari/537.36", platform: "Win32" });
  await send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-color-scheme", value: "dark" }] });
  await send("Page.navigate", { url: APP });
  await sleep(1200);
  await evaluate(`localStorage.clear();
    localStorage.setItem("ferry.lang", JSON.stringify(${JSON.stringify(lang)}));
    localStorage.setItem("ferry.theme", JSON.stringify("dark"));
    localStorage.setItem("ferry.target", JSON.stringify("8.4")); true`);
  await send("Page.reload");
  await sleep(1800);
};

await send("Page.enable");
for (const lang of ["en", "tr"]) {
  const t =
    lang === "tr"
      ? { pick: "Proje klasörü seçin", scan: "Taramayı başlat", findings: "Bulgular", fixes: "Düzeltmeler", calc: "Önizlemeyi hazırla", deps: "Bağımlılıklar", history: "Geçmiş", applyAll: "Tümünü uygula", apply: "Uygula" }
      : { pick: "Choose project folder", scan: "Start scan", findings: "Findings", fixes: "Fixes", calc: "Prepare preview", deps: "Dependencies", history: "History", applyAll: "Apply all", apply: "Apply" };
  await open(lang);
  await click(t.pick);
  await sleep(800);
  await click(t.scan);
  await sleep(3500);
  const overview = await capture();

  await click(t.fixes);
  await click(t.calc);
  await sleep(3000);
  await click("includes/class-cart.php", ".change-item__body");
  await sleep(500);
  const fixes = await capture();

  await click(t.findings);
  await evaluate(`(() => {
    const f = [...document.querySelectorAll("[role=option]")].find((o) => o.textContent.includes("each()"));
    f.click();
    const list = f.closest(".finding-list") ?? f.closest(".pane");
    list.scrollTop = f.offsetTop - list.offsetTop - 140;
    return true;
  })()`);
  const findings = await capture();

  await click(t.deps);
  await sleep(800);
  const dependencies = await capture();

  await click(t.fixes);
  await evaluate(`(() => { const all = document.querySelector('.change-list__header input'); if (all && !all.checked) all.click(); return true; })()`);
  await sleep(400);
  await click(t.applyAll);
  await click(t.apply, "[role=dialog] button");
  await sleep(3000);
  await click(t.history);
  await sleep(1200);
  const history = await capture();

  await save("hero", lang, overview);
  await save("findings", lang, findings);
  await save("fixes", lang, fixes);
  await save("dependencies", lang, dependencies);
  await save("history", lang, history);
}

ws.close();
chrome.kill();
process.exit(0);
