(async () => {
  const emit = (m) => window.__TAURI_INTERNALS__.invoke("plugin:event|emit", { event: "e2e://log", payload: String(m) });
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  const until = async (fn, what, ms = 30000) => {
    const start = Date.now();
    while (Date.now() - start < ms) {
      const v = fn();
      if (v) return v;
      await sleep(100);
    }
    throw new Error(`zaman aşımı: ${what}`);
  };
  const button = (text, scope = document) =>
    [...scope.querySelectorAll("button")].find((b) => !b.disabled && b.textContent.trim().includes(text));
  const click = async (text, scope, ms) => (await until(() => button(text, scope), `"${text}" butonu`, ms)).click();
  const nav = (text) => click(text, document.querySelector('nav[aria-label="Bölümler"]'));
  const text = () => document.body.innerText;
  const step = async (name) => {
    await emit(`STEP ${name}`);
    await sleep(1800);
  };

  const root = window.__FERRY_E2E_ROOT__;
  if (!sessionStorage.getItem("ferry.e2e")) {
    sessionStorage.setItem("ferry.e2e", "1");
    localStorage.clear();
    localStorage.setItem(
      "ferry.recent",
      JSON.stringify([{ path: root, name: root.split(/[\\/]/).pop(), framework: "generic", openedAt: Date.now() }]),
    );
    localStorage.setItem("ferry.target", JSON.stringify("8.4"));
    localStorage.setItem("ferry.lang", JSON.stringify("tr"));
    location.reload();
    return;
  }

  try {
    await step("welcome");
    (await until(() => document.querySelector(".recent__item"), "son projeler")).click();
    await until(() => text().includes("Hangi sürüme geçiyorsunuz?"), "genel bakış");
    await emit(`project ${document.querySelector(".project-chip__name")?.textContent}`);

    await click("Taramayı başlat");
    await until(() => document.querySelector(".overview-score"), "tarama sonucu", 60000);
    await emit(`scan score=${document.querySelector(".overview-score")?.dataset.score} findings=${document.querySelector(".nav__count")?.textContent}`);
    await step("overview");

    await nav("Bulgular");
    const rows = await until(() => document.querySelectorAll(".finding").length || 0, "bulgu listesi");
    await emit(`findings rows=${rows}`);
    await step("findings");

    await nav("Düzeltmeler");
    await until(() => button("Rector'ı kur") || button("Değişiklikleri hesapla"), "düzeltmeler ekranı");
    if (button("Rector'ı kur")) {
      await emit("installing rector");
      await click("Rector'ı kur");
      await until(() => button("Değişiklikleri hesapla"), "Rector kurulumu", 600000);
      await emit("rector installed");
    }
    await click("Değişiklikleri hesapla");
    await until(() => document.querySelector(".change-item") || text().includes("Yapılacak değişiklik yok"), "önizleme", 300000);
    const files = [...document.querySelectorAll(".change-item__file")].map((e) => e.textContent);
    await emit(`preview files=${JSON.stringify(files)}`);
    await step("changes");

    await click("uygula", document.querySelector(".apply-bar"));
    const dialog = await until(() => document.querySelector('[role="dialog"]'), "onay penceresi");
    await step("confirm");
    await click("Uygula", dialog);
    await until(() => text().includes("dosya güncellendi"), "uygulama bildirimi", 60000);
    await emit("applied");
    await step("applied");

    await nav("Geçmiş");
    await click("Geri yükle", await until(() => document.querySelector("table"), "geçmiş tablosu"));
    await click("Geri yükle", await until(() => document.querySelector('[role="dialog"]'), "geri yükleme onayı"));
    await until(() => text().includes("Geri yüklendi"), "geri yükleme bildirimi", 60000);
    await emit("restored");
    await step("history");
    await emit("DONE");
  } catch (e) {
    await emit(`FAIL ${e && e.message ? e.message : e}`);
  }
})();
