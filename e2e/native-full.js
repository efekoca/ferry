(async () => {
  const invoke = window.__TAURI_INTERNALS__.invoke;
  let lastLabel = "start";
  const emit = (m) => {
    lastLabel = String(m).slice(0, 80);
    console.warn("E2E " + lastLabel);
    return invoke("plugin:event|emit", { event: "e2e://log", payload: String(m) });
  };
  setInterval(() => console.warn("E2E alive after: " + lastLabel), 10000);
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  const until = async (fn, what, ms = 30000) => {
    const start = Date.now();
    while (Date.now() - start < ms) {
      const v = fn();
      if (v) return v;
      await sleep(100);
    }
    throw new Error(`timeout: ${what}`);
  };
  const settles = (fn, ms = 5000) => until(fn, "", ms).then(() => true, () => false);
  const $ = (s, scope = document) => scope.querySelector(s);
  const $$ = (s, scope = document) => [...scope.querySelectorAll(s)];
  const button = (text, scope = document) =>
    $$("button", scope).find((b) => !b.disabled && b.textContent.trim().includes(text));
  const click = async (text, scope, ms) => (await until(() => button(text, scope || document), `"${text}" button`, ms)).click();
  const radio = (text) => $$("[role=radio]").find((r) => r.textContent.trim() === text);
  const nav = (text) => click(text, $('nav[aria-label="Bölümler"]') || $('nav[aria-label="Sections"]'));
  const text = () => document.body.innerText;
  const dialog = () => until(() => $('[role="dialog"]'), "dialog");
  const key = (k, mod = true) => window.dispatchEvent(new KeyboardEvent("keydown", { key: k, metaKey: mod, bubbles: true }));
  const check = (cond, msg) => {
    if (!cond) throw new Error(msg);
  };

  const seen = [];
  const errorToasts = new Set();
  let scenarioSkipped = () => {};
  new MutationObserver(() => {
    for (const el of $$(".toast__title")) {
      if (!el.dataset.e2e) {
        el.dataset.e2e = "1";
        if (el.closest(".toast--error")) errorToasts.add(seen.length);
        seen.push(el.textContent);
        emit(`  toast: ${el.textContent} | ${el.parentElement.querySelector(".toast__desc")?.textContent ?? ""}`);
      }
    }
  }).observe(document.body, { childList: true, subtree: true });
  window.addEventListener("error", (e) => emit(`  JSERROR ${e.message}`));
  let leaving = false;
  window.addEventListener("pagehide", () => (leaving = true));
  window.addEventListener("unhandledrejection", (e) => {
    if (!leaving) emit(`  REJECTION ${e.reason?.message ?? e.reason}`);
  });
  const origError = console.error;
  console.error = (...a) => {
    emit(`  CONSOLE ${a.map(String).join(" ").slice(0, 300)}`);
    origError(...a);
  };
  const noErrorToast = (since, expected) =>
    check(![...errorToasts].some((at) => at >= since && !expected?.test(seen[at])), `error toast: ${seen.slice(since)}`);

  const P = JSON.parse(window.__FERRY_E2E_ROOT__);
  const FX = P.fixtures;
  const projects = {
    kuzey: `${P.demo}/kuzey-magaza`,
    laravel: `${P.demo}/laravel-api`,
    wp: `${P.demo}/wp-blog`,
    unicode: `${FX}/Proje Ğüş klasörü`,
    odd: `${FX}/odd`,
    empty: `${FX}/empty`,
    missing: `${FX}/does-not-exist`,
  };

  const stage = Number(sessionStorage.getItem("ferry.e2e.stage") ?? "-1");
  const next = () => {
    leaving = true;
    const wanted = ["kuzey", "kuzey", "laravel", "wp", "wp", "unicode", "unicode", "odd", "empty", "missing", "missing"][stage + 1];
    if (wanted) {
      const now = Date.now();
      const order = [projects[wanted], ...Object.values(projects).filter((p) => p !== projects[wanted])];
      localStorage.setItem("ferry.recent", JSON.stringify(order.map((path, i) => ({ path, name: path.split("/").pop(), framework: "generic", openedAt: now - i * 1000 }))));
    }
    sessionStorage.setItem("ferry.e2e.stage", String(stage + 1));
    location.reload();
  };

  const openRecent = async (path) => {
    const overview = () => text().includes("Hangi sürüme geçiyorsunuz?") || text().includes("Which version");
    const start = Date.now();
    while (!overview()) {
      if (Date.now() - start > 60000) throw new Error("timeout: overview");
      const item = await until(
        () => $$(".recent__item").find((b) => !b.disabled && b.querySelector(".recent__path")?.textContent === path),
        `recent ${path}`,
      );
      item.click();
      await until(() => overview() || !item.isConnected || item.disabled, "open started", 5000).catch(() => {});
      await until(() => overview() || !$$(".recent__item").some((b) => b.disabled), "opened", 30000).catch(() => {});
    }
  };
  const scan = async () => {
    await click("Taramayı başlat");
    await until(() => $(".score-ring"), "scan result", 120000);
    return Number($(".score-ring__number")?.textContent);
  };
  const preview = async () => {
    await nav("Düzeltmeler");
    await click("Değişiklikleri hesapla", undefined, 10000);
    await until(() => $(".change-item") || text().includes("Yapılacak değişiklik yok"), "preview", 300000);
    const files = $$(".change-item__file").map((e) => e.textContent);
    const all = $(".change-list__header input");
    if (all && !all.checked) {
      all.click();
      await until(() => text().includes(`${files.length} / ${files.length} dosya seçili`), "select every file");
    }
    return files;
  };
  const applyAndConfirm = async (expected) => {
    const before = seen.length;
    await click("uygula", $(".apply-bar"));
    await click("Uygula", await dialog());
    await until(() => seen.slice(before).some((t) => /güncellendi|amadı/.test(t)), "apply toast", 180000);
    noErrorToast(before, expected);
  };

  const scenarios = [
    async function setup() {
      localStorage.clear();
      const now = Date.now();
      localStorage.setItem(
        "ferry.recent",
        JSON.stringify([projects.missing, ...Object.values(projects).filter((p) => p !== projects.missing)].slice(0, 6).map((path, i) => ({ path, name: path.split("/").pop(), framework: "generic", openedAt: now - i * 1000 }))),
      );
      localStorage.setItem("ferry.target", JSON.stringify("8.4"));
      localStorage.setItem("ferry.lang", JSON.stringify("tr"));
      localStorage.setItem("ferry.theme", JSON.stringify("light"));
      for (const p of Object.values(projects)) {
        for (const s of await invoke("list_snapshots", { path: p }).catch(() => [])) await invoke("delete_snapshot", { path: p, id: s.id });
      }
    },

    async function kuzey() {
      await openRecent(projects.kuzey);
      await emit(`  chip=${$(".project-chip__name")?.textContent} platform=${text().match(/Platform\s*\n?\s*(\S+)/)?.[1]}`);
      const findingsOpen = () => text().includes("Henüz tarama yapılmadı");
      check(await settles(() => findingsOpen() || (key("2"), false)), "Cmd+2 should open empty findings");
      key("1");
      await sleep(300);
      const score = await scan();
      await emit(`  score=${score} findings=${$(".nav__count")?.textContent}`);
      const rescan = async (version) => {
        radio(version).click();
        await until(() => $(".score__meta .badge")?.textContent === `PHP ${version}` && $(".score-ring"), `rescan for ${version}`, 120000);
        return Number($(".score-ring__number")?.textContent);
      };
      const score85 = await rescan("8.5");
      await emit(`  8.5 score=${score85}`);
      await rescan("8.4");

      await nav("Bulgular");
      const all = await until(() => $$(".finding").length, "findings");
      await click("Elle düzelt");
      const manual = $$(".finding").length;
      await click("Elle düzelt");
      check($$(".finding").length === all, "filter toggle restores list");
      const search = $('input[type="search"], input[aria-label="Bulgularda ara"]');
      search.focus();
      document.execCommand("insertText", false, "zzzz-no-match");
      await until(() => text().includes("Eşleşen bulgu yok"), "no-match empty state");
      search.select();
      document.execCommand("delete");
      await until(() => $$(".finding").length === all, "search cleared");
      const firstSel = $(".finding[aria-selected=true]")?.textContent;
      $(".finding").focus();
      $(".finding").dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true }));
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true }));
      await sleep(300);
      await emit(`  findings all=${all} manual=${manual} arrowMoved=${$(".finding[aria-selected=true]")?.textContent !== firstSel}`);
      const detail = $('[aria-label="Bulgu ayrıntısı"]');
      check(detail && detail.innerText.length > 50, "finding detail visible");

      await nav("Bağımlılıklar");
      await sleep(500);
      await emit(`  deps: ${$("main")?.innerText.split("\n").slice(0, 6).join(" / ")}`);

      const files = await preview();
      await emit(`  preview ${files.length}: ${files.join(", ")}`);
      check(files.length >= 2, "kuzey preview should change several files");
      $$(".change-item input[type=checkbox]")[0].click();
      check(await settles(() => text().includes(`${files.length - 1} / ${files.length} dosya seçili`)), "unchecking one file updates count");
      await applyAndConfirm(/sözdizimi hatası/);
      const left = $$(".change-item__file").map((e) => e.textContent);
      check(left.length === 1 && left[0] === files[0], `unapplied file stays listed: ${left}`);

      await nav("Geçmiş");
      const row = await until(() => $$("tbody tr")[0], "history row");
      await click("İncele", row);
      const review = await dialog();
      await until(() => review.innerText.includes(`${files.length - 1} dosya`), "review meta");
      key("Escape", false);
      document.activeElement.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
      await until(() => !$('[role="dialog"]'), "Escape closes review");

      await emit(`MODIFY ${projects.kuzey}/${files[1]}`);
      await sleep(3000);
      await click("Geri yükle", $$("tbody tr")[0]);
      const confirm = await dialog();
      await until(() => confirm.innerText.includes("uygulamadan sonra değiştirilmiş"), "conflict warning", 10000);
      check(confirm.innerText.includes(files[1]), "conflict lists edited file");
      const before = seen.length;
      await click("Geri yükle", confirm);
      await until(() => seen.slice(before).includes("Geri yüklendi"), "restored toast", 30000);
      await until(() => !$('[role="dialog"]'), "restore dialog closed");

      await until(() => $$("tbody tr").some((r) => r.innerText.includes("Geri yükleme öncesi")), "edited file kept in a backup");

      for (let n = $$("tbody tr").length; n > 0; n--) {
        emit("  deleting snapshot");
        (await until(() => $('tbody tr button[aria-label="Yedeği sil"]'), "delete button")).click();
        emit("  confirming delete");
        await click("Sil", await dialog());
        await until(() => $$("tbody tr").length < n, "snapshot deleted");
      }
      emit("  waiting for empty history");
      await until(() => text().includes("Henüz bir değişiklik uygulanmadı"), "history empty after delete");
    },

    async function laravel() {
      await openRecent(projects.laravel);
      check(/Laravel/.test($(".project-chip")?.textContent ?? text()), "laravel detected");
      await scan();
      const files = await preview();
      await emit(`  preview: ${files.join(", ")}`);
      check(files.length > 0, "laravel preview");
      await applyAndConfirm();
      await until(() => text().includes("Hepsi uygulandı"), "all applied state");
      await nav("Geçmiş");
      await click("Geri yükle", await until(() => $$("tbody tr")[0], "history row"));
      const before = seen.length;
      await click("Geri yükle", await dialog());
      await until(() => seen.slice(before).includes("Geri yüklendi"), "restored", 30000);
      await until(() => !$('[role="dialog"]'), "restore dialog closed");
    },

    async function stopPreview() {
      await openRecent(projects.wp);
      await scan();
      await nav("Düzeltmeler");
      await click("Değişiklikleri hesapla", undefined, 10000);
      const before = seen.length;
      const finished = () => $(".change-item") || text().includes("Yapılacak değişiklik yok");
      await until(() => button("Durdur") || finished(), "Stop button or results", 10000);
      const stopped = !!button("Durdur");
      if (stopped) {
        button("Durdur").click();
        await until(() => button("Değişiklikleri hesapla") || finished(), "calculate button back after stop", 15000);
      }
      await sleep(1000);
      noErrorToast(before);
      const raced = !button("Değişiklikleri hesapla");
      if (raced) await emit("  Rector finished before Stop took effect");
      else check(!$(".change-item"), "no stale results after stop");
      const files = raced ? $$(".change-item__file").map((e) => e.textContent) : await preview();
      await emit(`  after stop, preview ${files.length}: ${files.join(", ")}`);
      check(files.length > 0, "preview works after a stop");

      const report = `${FX}/report.md`;
      await invoke("export_report", { path: projects.wp, destination: report });
      await emit(`  report written ${report}`);
    },

    async function modernize() {
      await openRecent(projects.wp);
      await scan();
      const compat = await preview();
      radio("Modernleştirme").click();
      if (button("Durdur")) check(button("uygula", $(".apply-bar")) === undefined, "apply is disabled while recalculating");
      await until(() => !$$("button").some((b) => b.textContent.includes("Durdur")), "modern preview", 300000);
      check(await settles(() => radio("Modernleştirme").getAttribute("aria-checked") === "true"), "modernize stays selected");
      const modern = $$(".change-item__file").map((e) => e.textContent);
      await emit(`  compat=${compat.length} modern=${modern.length}: ${modern.join(", ")}`);
      check(modern.length >= compat.length, "modernize covers at least the compat files");
      await applyAndConfirm();
      await nav("Geçmiş");
      const row = await until(() => $$("tbody tr")[0], "history row");
      check(row.innerText.includes("Modernleştirme"), "history label says modernize");
      await click("Geri yükle", row);
      const before = seen.length;
      await click("Geri yükle", await dialog());
      await until(() => seen.slice(before).includes("Geri yüklendi"), "restored", 30000);
      await until(() => !$('[role="dialog"]'), "restore dialog closed");
    },

    async function hunkSelection() {
      await openRecent(projects.unicode);
      await scan();
      const files = await preview();
      check(files.length > 0, "changes to choose from");
      const hunk = await until(() => $(".diff__choice"), "hunk checkbox");
      const applyLabel = () => $(".apply-bar button").textContent;
      const on = hunk.checked;
      hunk.click();
      await until(() => $(".diff__choice").checked !== on, "hunk toggled");
      if (!on) await emit(`  after ticking: ${applyLabel()}`);
    },

    async function unicodePath() {
      await openRecent(projects.unicode);
      const score = await scan();
      await emit(`  score=${score} findings=${$(".nav__count")?.textContent}`);
      const files = await preview();
      await emit(`  preview: ${files.join(", ")}`);
      check(files.includes("src/a.php"), "unicode project preview");
      await applyAndConfirm();
      await emit(`DUMP ${projects.unicode}/src/a.php`);
      await sleep(1500);
      await nav("Geçmiş");
      await click("Geri yükle", await until(() => $$("tbody tr")[0], "history row"));
      const confirm = await dialog();
      await sleep(800);
      check(!confirm.innerText.includes("uygulamadan sonra"), "no conflict warning when untouched");
      const before = seen.length;
      await click("Geri yükle", confirm);
      await until(() => seen.slice(before).includes("Geri yüklendi"), "restored", 30000);
      await until(() => !$('[role="dialog"]'), "restore dialog closed");
    },

    async function oddProject() {
      await openRecent(projects.odd);
      const score = await scan();
      await emit(`  score=${score} findings=${$(".nav__count")?.textContent}`);
      await emit(`  overview: ${$("main").innerText.replace(/\n+/g, " / ").slice(0, 900)}`);
      check(text().includes("big.php"), "oversized file is reported");
      await nav("Bulgular");
      await sleep(400);
      const findFiles = [...new Set($("[role=listbox]").innerText.match(/[\w/.-]+\.php/g) ?? [])];
      await emit(`  finding files: ${findFiles.join(", ")}`);
      check(!findFiles.some((f) => f?.startsWith("vendor/")), "vendor must be skipped");
      check(!findFiles.some((f) => f?.includes("loop/")), "symlink loop must not be followed");
      await nav("Bağımlılıklar");
      await sleep(500);
      await emit(`  deps: ${$("main").innerText.replace(/\n+/g, " / ").slice(0, 500)}`);
      check(text().includes("acme/dev"), "dev package listed");
      radio("Sadece üretim").click();
      check(await settles(() => !text().includes("acme/dev") && text().includes("acme/x")), "production filter hides dev packages");
      radio("Tümü").click();
      const files = await preview();
      await emit(`  preview: ${files.join(", ")}`);
      await emit(`  changes screen: ${$("main").innerText.replace(/\n+/g, " / ").slice(0, 600)}`);
      if (files.length) {
        await applyAndConfirm();
        await nav("Geçmiş");
        await click("Geri yükle", await until(() => $$("tbody tr")[0], "history row"));
        const before = seen.length;
        await click("Geri yükle", await dialog());
        await until(() => seen.slice(before).includes("Geri yüklendi"), "restored", 30000);
      await until(() => !$('[role="dialog"]'), "restore dialog closed");
      }
    },

    async function emptyProject() {
      await openRecent(projects.empty);
      const score = await scan();
      await emit(`  score=${score} text=${$("main").innerText.replace(/\n+/g, " / ").slice(0, 400)}`);
      check(text().includes("Tarama eksik kaldı"), "an empty folder isn't reported as ready");
      const files = await preview();
      check(files.length === 0, "empty project has no changes");
      check(text().includes("Yapılacak değişiklik yok"), "nothing state");
    },

    async function runtimeRoot() {
      await openRecent(projects.unicode);
      await scan();
      await invoke("import_runtime_log", { path: projects.unicode, log: `${FX}/php_errors.log` });
      await click("Tekrar tara");
      await sleep(1000);
      await until(() => button("Tekrar tara"), "rescan done");
      await nav("Bulgular");
      await until(() => text().includes("php_errors.log:"), "log summary");
      const summary = () => $(".check-log").closest(".callout").innerText;
      check(/0 mesaj proje dışındaki/.test(summary()), `the guess ties the message to the project: ${summary()}`);
      $(".check-log").open = true;
      $('input[name="roots"]').value = "/srv/app";
      await click("Uygula");
      await until(() => text().includes("Sunucudaki proje kökü: /srv/app"), "root shown");
      await until(() => /1 mesaj proje dışındaki/.test(summary()), "message moved outside");
      await click("Log'u kaldır");
    },

    async function dockerCompare() {
      if (!P.docker) return scenarioSkipped();
      const before = seen.length;
      await openRecent(projects.unicode);
      await scan();
      await nav("Gerçek test");
      await click("Testi başlat");
      await until(() => /PHP 8\.\d sürümünde yeni: [1-9]/.test(text()), "new messages on the target", 300000);
      await emit(`  docker: ${$("main").innerText.replace(/\n+/g, " / ").slice(300, 900)}`);
      noErrorToast(before);
      await click("Bulgularda göster");
      await until(() => text().includes("Docker · PHP"), "merged into the findings");
      await click("Log'u kaldır");
      await nav("Gerçek test");
      await click("Düzeltmeleri hazırla");
      await until(() => text().includes("geçici kopyada dene"), "fixes prepared", 120000);
      await click("geçici kopyada dene");
      await until(() => text().includes("Kapandı") && text().includes("projeye yaz"), "trial result", 300000);
      await emit(`  trial: ${$(".docker-delta").innerText.replace(/\s+/g, " ")}`);
      noErrorToast(before);
      const backups = (await invoke("list_snapshots", { path: projects.unicode })).length;
      const runs = (await invoke("docker_history", { path: projects.unicode })).length;
      await click("projeye yaz");
      await click("Projeye yaz", await dialog());
      await until(async () => !$('[role="dialog"]'), "fixes written", 30000);
      await until(() => !text().includes("Kapandı"), "trial closed", 30000);
      check((await invoke("list_snapshots", { path: projects.unicode })).length === backups + 1, "a backup was taken");
      check((await invoke("docker_history", { path: projects.unicode })).length === runs + 1, "the trial became the latest comparison");
      await until(() => /PHP 8\.\d sürümünde yeni: 0/.test(text()), "no new messages after writing", 30000);
      noErrorToast(before);
    },

    async function dockerStopAndClean() {
      if (!P.docker) return scenarioSkipped();
      const before = seen.length;
      const images = (await invoke("docker_image_usage")).images;
      await openRecent(projects.unicode);
      await scan();
      await nav("Gerçek test");
      await click("Testi başlat");
      await click("Durdur", undefined, 30000);
      await until(() => button("Testi başlat"), "start button back after stop", 60000);
      noErrorToast(before);
      check(!text().includes("sürümünde yeni"), "no result after a stop");
      const clean = $$(".docker-option").find((l) => l.textContent.includes("bitince"));
      clean.querySelector("input").click();
      await click("Testi başlat");
      await until(() => text().includes("Kurulan PHP eklentileri"), "extension installed", 600000);
      noErrorToast(before);
      const after = (await invoke("docker_image_usage")).images;
      await emit(`  images before=${images} after=${after}`);
      check(after <= images, "the run removed the images it used");
    },

    async function dockerDatabase() {
      if (!P.docker) return scenarioSkipped();
      const before = seen.length;
      await openRecent(projects.unicode);
      await scan();
      await nav("Gerçek test");
      const select = await until(() => $$('[role="combobox"]').find((s) => s.getAttribute("aria-label") === "Veritabanı"), "database choices");
      select.click();
      const option = await until(() => $$('[role="option"]').find((o) => o.textContent.includes("database/dump.sql")), "database dump");
      option.click();
      await click("Testi başlat");
      await until(() => text().includes("İstekler sonrası"), "data comparison", 900000);
      noErrorToast(before);
    },

    async function missingProject() {
      const before = seen.length;
      const item = await until(() => $$(".recent__item").find((b) => b.querySelector(".recent__path")?.textContent === projects.missing), "missing recent");
      item.click();
      await until(() => seen.length > before, "error toast for missing folder", 10000);
      await emit(`  stays on welcome=${text().includes("Kodunuz yeni sürüme hazır olsun")}`);
    },

    async function contrast() {
      await openRecent(projects.kuzey);
      await scan();
      const still = document.createElement("style");
      still.textContent = "*,*::before,*::after{transition:none!important;animation:none!important}";
      document.head.appendChild(still);
      const vibrancy = document.documentElement.dataset.vibrancy;
      document.documentElement.dataset.vibrancy = "false";
      const failures = [];
      for (const [mode, label] of [["light", "Açık"], ["dark", "Koyu"]]) {
        key(",");
        await until(() => radio(label), "theme radio");
        radio(label).click();
        await until(() => document.documentElement.dataset.theme === mode, `${mode} theme`);
        await sleep(500);
        for (const [page, k] of [["overview", "1"], ["findings", "2"], ["fixes", "3"], ["dependencies", "4"], ["history", "5"]]) {
          key(k);
          await sleep(700);
          const result = await window.axe.run(document, { runOnly: ["color-contrast"] });
          for (const v of result.violations) for (const n of v.nodes) failures.push(`${mode}/${page} ${n.target.join(" ")} ${n.any[0]?.data?.contrastRatio}`);
        }
      }
      key(",");
      await until(() => radio("Sistem"), "system theme radio");
      radio("Sistem").click();
      still.remove();
      if (vibrancy) document.documentElement.dataset.vibrancy = vibrancy;
      check(failures.length === 0, `contrast: ${failures.join("; ")}`);
    },

    async function settings() {
      key(",");
      await until(() => text().includes("Görünüm"), "settings via Cmd+,");
      radio("Koyu").click();
      check(await settles(() => document.documentElement.dataset.theme === "dark"), "dark theme");
      radio("English").click();
      await until(() => text().includes("Appearance"), "english");
      check(document.documentElement.lang === "en", "lang attr");
      const bad = await invoke("inspect_project", { path: projects.missing }).catch((e) => String(e));
      await emit(`  backend error in en: ${bad}`);
      check(/not found/i.test(bad), "backend errors follow language");
      radio("Türkçe").click();
      await until(() => text().includes("Görünüm"), "back to turkish");
      radio("Açık").click();
      const before = seen.length;
      await click("Yeniden kontrol et");
      await sleep(2500);
      noErrorToast(before);
      await emit(`  tools: ${$("main").innerText.replace(/\n+/g, " / ").slice(0, 500)}`);
      const orig = await invoke("get_settings");
      await invoke("save_settings", { settings: { phpPath: "/nope/php" } });
      const status = await invoke("toolchain_status");
      await emit(`  bogus php -> ${JSON.stringify(status).slice(0, 300)}`);
      await invoke("save_settings", { settings: orig });
    },
  ];

  if (stage < 0) {
    sessionStorage.setItem("ferry.e2e.stage", "0");
    leaving = true;
    location.reload();
    return;
  }
  if (stage >= scenarios.length) {
    await emit("DONE");
    return;
  }
  await sleep(800);
  const s = scenarios[stage];
  await emit(`STEP ${s.name}`);
  let skipped = false;
  scenarioSkipped = () => (skipped = true);
  try {
    await s();
    await emit(`${skipped ? "SKIP" : "PASS"} ${s.name}`);
  } catch (e) {
    await emit(`FAIL ${s.name}: ${e && e.message ? e.message : e}`);
    await emit(`  screen: ${document.body.innerText.replace(/\n+/g, " / ").slice(0, 600)}`);
  }
  next();
})();
