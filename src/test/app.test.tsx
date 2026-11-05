import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import App from "../App";
import { mockApi } from "../lib/mock";
import type { SnapshotConflict } from "../lib/types";

function setup() {
  const user = userEvent.setup();
  render(<App api={mockApi({ delay: 0 })} />);
  return user;
}

const nav = (name: RegExp) => within(screen.getByRole("navigation", { name: "Bölümler" })).getByRole("button", { name });

async function openAndScan(user: ReturnType<typeof userEvent.setup>) {
  await user.click(await screen.findByRole("button", { name: "Proje klasörü seçin" }));
  await screen.findByRole("heading", { name: "Genel bakış" });
  await user.click(screen.getByRole("button", { name: /Taramayı başlat/ }));
  await screen.findByRole("img", { name: /Hazırlık skoru/ });
}

describe("Ferry app", () => {
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
    expect(screen.getByText("Ciddi bir geçiş gerekiyor")).toBeInTheDocument();
    expect(JSON.parse(localStorage.getItem("ferry.recent")!)[0].name).toBe("kuzey-magaza");
    expect(screen.getByText("mysql_*()")).toBeInTheDocument();
    expect(screen.getByRole("alert")).toHaveTextContent("WordPress 6.2, PHP 8.4 ile çalışmaz.");
    expect(screen.getByText("WordPress sürümünü yükseltin")).toBeInTheDocument();
  });

  it("shows what still stands between the project and the target", async () => {
    const user = setup();
    await openAndScan(user);
    const list = screen.getByRole("list", { name: "Hazırlık durumu" });
    expect(within(list).getByText("Geçişi engelleyenler").parentElement).toHaveAttribute("data-state", "bad");
    expect(within(list).getByText("Tüm dosyalar")).toBeInTheDocument();
    expect(within(list).getByRole("button", { name: "3 engel" })).toBeInTheDocument();
    expect(within(list).getByRole("button", { name: "Çalıştırılmadı" })).toBeInTheDocument();
    await user.click(within(list).getByRole("button", { name: "3 engel" }));
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
    await user.click(screen.getByRole("radio", { name: "8.2" }));
    expect(await screen.findByText("PHP 8.2", { selector: ".score__meta .badge" })).toBeInTheDocument();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("rescans when the target changes after a scan", async () => {
    const user = setup();
    await openAndScan(user);
    await user.click(screen.getByRole("radio", { name: "8.3" }));
    expect(await screen.findByText("PHP 8.3", { selector: ".score__meta .badge" })).toBeInTheDocument();
    expect(screen.queryByText("PHP 8.4", { selector: ".score__meta .badge" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Taramayı başlat/ })).not.toBeInTheDocument();
  });

  it("keeps PHP 8.1 out of reach and explains why", async () => {
    const user = setup();
    await openAndScan(user);
    const blocked = screen.getByRole("radio", { name: "8.1" });
    expect(blocked).toBeDisabled();
    expect(blocked).toHaveAccessibleDescription(/CVE-2026-109618.*8\.2\.35, 8\.3\.36, 8\.4\.27, 8\.5\.12/);
    await user.click(screen.getByRole("radio", { name: "8.2" }));
    await screen.findByText("PHP 8.2", { selector: ".score__meta .badge" });
    screen.getByRole("radio", { name: "8.2" }).focus();
    await user.keyboard("{ArrowLeft}");
    expect(await screen.findByText("PHP 8.5", { selector: ".score__meta .badge" })).toBeInTheDocument();
    expect(blocked).toHaveAttribute("aria-checked", "false");
  });

  it("falls back to the default target when PHP 8.1 was stored", async () => {
    localStorage.setItem("ferry.target", JSON.stringify("8.1"));
    const user = setup();
    await openAndScan(user);
    expect(screen.getByRole("radio", { name: "8.4" })).toHaveAttribute("aria-checked", "true");
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
    await user.click(await screen.findByRole("button", { name: /Değişiklikleri hesapla/ }));
    await screen.findByRole("list", { name: "Dosyalar" });
    await user.click(nav(/Genel bakış/));
    await user.click(screen.getByRole("radio", { name: "8.3" }));
    const dialog = await screen.findByRole("dialog", { name: "PHP 8.3 hedefine geçilsin mi?" });
    await user.click(within(dialog).getByRole("button", { name: "Vazgeç" }));
    expect(screen.getByRole("radio", { name: "8.4" })).toHaveAttribute("aria-checked", "true");
    await user.click(screen.getByRole("radio", { name: "8.3" }));
    await user.click(within(await screen.findByRole("dialog")).getByRole("button", { name: "Hedefi değiştir" }));
    expect(await screen.findByText("PHP 8.3", { selector: ".score__meta .badge" })).toBeInTheDocument();
    await user.click(nav(/Düzeltmeler/));
    expect(await screen.findByRole("button", { name: /Değişiklikleri hesapla/ })).toBeInTheDocument();
  });

  it("opens findings filtered from the overview", async () => {
    const user = setup();
    await openAndScan(user);
    await user.click(screen.getByRole("button", { name: /^Kritik/ }));
    const list = await screen.findByRole("listbox", { name: "Bulgu listesi" });
    within(list)
      .getAllByRole("option")
      .forEach((o) => expect(within(o).getByText("Kritik")).toBeInTheDocument());
    expect(screen.getByRole("button", { name: /^Kritik/, pressed: true })).toBeInTheDocument();

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

    await user.click(screen.getByRole("button", { name: /^Elle düzelt/ }));
    const manual = within(list).getAllByRole("option");
    expect(manual.length).toBeLessThan(all);
    manual.forEach((o) => expect(within(o).getByText("Elle düzelt")).toBeInTheDocument());

    await user.type(screen.getByRole("searchbox", { name: "Bulgularda ara" }), "strftime");
    expect(within(list).getAllByRole("option")).toHaveLength(1);
    const detail = screen.getByRole("complementary", { name: "Bulgu ayrıntısı" });
    expect(within(detail).getByRole("heading", { name: "strftime()" })).toBeInTheDocument();
    expect(within(detail).getByText(/admin\/settings-page.php:88/)).toBeInTheDocument();
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
    expect(await screen.findByRole("heading", { name: "PHP eklentileri" })).toBeInTheDocument();
    expect(screen.getByText("PHP'den çıkarıldı")).toBeInTheDocument();
    expect(screen.getByText(/composer.json'a eklenebilir: "ext-mysqli": "\*", "ext-gd": "\*"/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Uyumlu sürümleri bul" }));
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

  it("shows progress since the previous scan", async () => {
    const user = setup();
    await openAndScan(user);
    expect(screen.getByText(/İlk tarama/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /Tekrar tara/ }));
    expect(await screen.findByText("Değişiklik yok")).toBeInTheDocument();
    expect(screen.getByRole("img", { name: /Hazırlık puanının taramalar boyunca değişimi/ })).toBeInTheDocument();
  });

  it("merges an imported runtime log into the findings", async () => {
    const user = setup();
    await openAndScan(user);
    await user.click(nav(/Bulgular/));
    const all = (await screen.findAllByRole("option")).length;
    await user.click(screen.getByRole("button", { name: "Log içe aktar" }));
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
    await user.click(screen.getByRole("button", { name: "Log içe aktar" }));
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
    expect(screen.getByRole("button", { name: "Log içe aktar" })).toBeInTheDocument();
    expect(await screen.findByRole("button", { name: "PHP 8.4.27 ile lint et" })).toBeInTheDocument();
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
    await user.click(await screen.findByRole("button", { name: /Değişiklikleri hesapla/ }));
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
    await user.click(await screen.findByRole("button", { name: /Değişiklikleri hesapla/ }));
    await screen.findByRole("list", { name: "Dosyalar" });
    await user.selectOptions(screen.getByRole("combobox", { name: "Seçimi daralt" }), "Yalnızca otomatik düzeltmeler");
    const autoCount = screen.getByText(/ \/ 4 dosya seçili/).textContent;
    expect(autoCount).not.toBe("4 / 4 dosya seçili");
    await user.selectOptions(screen.getByRole("combobox", { name: "Seçimi daralt" }), "Tüm dosyalar");
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
    await user.click(await screen.findByRole("button", { name: "PHP 8.4.27 ile lint et" }));
    expect(await screen.findByText(/php -l · PHP 8.4: 31 farklı mesaj/)).toBeInTheDocument();
    expect(screen.getByText("128 dosya lint edildi, sözdizimi hatası yok.")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Testleri PHP 8.4.27 ile çalıştır" }));
    const dialog = await screen.findByRole("dialog", { name: "Projenin testleri çalıştırılsın mı?" });
    await user.click(within(dialog).getByRole("button", { name: "Testleri çalıştır" }));
    expect(await screen.findByText(/PHPUnit · PHP 8.4: 31 farklı mesaj/)).toBeInTheDocument();
    expect(screen.getByText("Testler geçti.")).toBeInTheDocument();
  });

  it("compares two versions in Docker and feeds the findings", async () => {
    const user = userEvent.setup();
    render(<App api={mockApi({ delay: 0 })} />);
    await openAndScan(user);
    await user.click(nav(/Docker karşılaştırması/));
    await user.click(await screen.findByRole("button", { name: "Karşılaştırmayı başlat" }));
    expect(await screen.findByText("PHP 8.4 sürümünde yeni sorunlar var.")).toBeInTheDocument();
    expect(screen.getByText("Error: Call to undefined function each()")).toBeInTheDocument();
    expect(screen.getByText("1 test başarısız")).toBeInTheDocument();
    expect(screen.getByText("CartTest::testTotal")).toBeInTheDocument();
    expect(screen.getByText("PHP 8.4 sürümünde farklı sayfalar: 1 / 2")).toBeInTheDocument();
    expect(screen.getByText("/cart.php")).toBeInTheDocument();
    expect(screen.getAllByText("mysqli")).toHaveLength(2);
    await user.click(screen.getByRole("button", { name: "Bulgularda göster" }));
    expect(await screen.findByText(/Docker · PHP .* → 8.4: 31 farklı mesaj/)).toBeInTheDocument();
    await user.click(nav(/Genel bakış/));
    expect(await screen.findByRole("button", { name: "Yeni sorunlar var" })).toBeInTheDocument();
    await user.click(nav(/Geçmiş/));
    expect(await screen.findByText("Docker karşılaştırmaları")).toBeInTheDocument();
    expect(screen.queryByText("Sunucudaki proje kökü")).not.toBeInTheDocument();
  });

  it("applies only the hunks left on", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    const apply = vi.spyOn(api, "applyChanges");
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Düzeltmeler/));
    await user.click(await screen.findByRole("button", { name: "Değişiklikleri hesapla" }));
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
    await user.click(nav(/Docker karşılaştırması/));
    await user.selectOptions(await screen.findByRole("combobox", { name: "Veritabanı" }), await screen.findByRole("option", { name: "Canlı kopya: magaza (127.0.0.1)" }));
    expect(screen.getByText(/Canlı veritabanınıza yazılmaz/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Karşılaştırmayı başlat" }));
    expect(await screen.findByText("İstekler sonrası farklı tablolar: 1")).toBeInTheDocument();
    expect(screen.getByText("orders")).toBeInTheDocument();
  });

  it("asks before copying a database from a server", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    const compare = vi.spyOn(api, "dockerCompare");
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Docker karşılaştırması/));
    await user.selectOptions(await screen.findByRole("combobox", { name: "Veritabanı" }), "ssh");
    await user.type(screen.getByRole("combobox", { name: "SSH sunucusu" }), "prod");
    await user.type(screen.getByRole("textbox", { name: "Veritabanı adı" }), "magaza");
    await user.type(screen.getByRole("textbox", { name: "Veritabanı kullanıcısı" }), "app");
    await user.click(screen.getByRole("button", { name: "Karşılaştırmayı başlat" }));
    const dialog = await screen.findByRole("dialog", { name: "Sunucudaki veri kopyalansın mı?" });
    expect(within(dialog).getByText(/prod sunucusundaki magaza/)).toBeInTheDocument();
    expect(compare).not.toHaveBeenCalled();
    await user.click(within(dialog).getByRole("button", { name: "Kopyala ve karşılaştır" }));
    await waitFor(() => expect(compare).toHaveBeenCalled());
    expect(compare.mock.calls[0][6]).toEqual({ kind: "ssh", host: "prod", engine: "mysql", name: "magaza", user: "app" });
  });

  it("tries fixes in a copy and writes them only after the result is shown", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    const compare = vi.spyOn(api, "dockerCompare");
    const apply = vi.spyOn(api, "applyChanges");
    const keep = vi.spyOn(api, "keepDockerTrial");
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Docker karşılaştırması/));
    await user.click(await screen.findByRole("button", { name: "Karşılaştırmayı başlat" }));
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
    expect(apply.mock.calls[0][4]).toBeUndefined();
  });

  it("opens with the project's saved settings and saves them back", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0, config: { target: "8.3", exclude: ["legacy"], baseline: null, docker: { source: "8.0", extensions: false, pages: ["/shop"], dump: null, prepare: "make assets", test: "vendor/bin/pest" }, git: { requireClean: false, branchPerBatch: false } } });
    const compare = vi.spyOn(api, "dockerCompare");
    const save = vi.spyOn(api, "saveProjectConfig");
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Docker karşılaştırması/));
    expect(await screen.findByDisplayValue("vendor/bin/pest")).toBeInTheDocument();
    expect(screen.getByDisplayValue("make assets")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Karşılaştırmayı başlat" }));
    await waitFor(() => expect(compare).toHaveBeenCalled());
    const call = compare.mock.calls[0];
    expect([call[1], call[2], call[3], call[5], call[7], call[8]]).toEqual(["8.0", "8.3", false, ["/shop"], "make assets", "vendor/bin/pest"]);
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
    await user.click(nav(/Docker karşılaştırması/));
    await user.click(await screen.findByRole("button", { name: "Karşılaştırmayı başlat" }));
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
    await user.click(nav(/Docker karşılaştırması/));
    await user.selectOptions(await screen.findByRole("combobox", { name: "Veritabanı" }), "ssh");
    await user.type(screen.getByRole("combobox", { name: "SSH sunucusu" }), "prod");
    await user.type(screen.getByRole("textbox", { name: "Veritabanı adı" }), "magaza");
    await user.type(screen.getByRole("textbox", { name: "Veritabanı kullanıcısı" }), "app");
    await user.click(screen.getByRole("button", { name: "Karşılaştırmayı başlat" }));
    await user.click(within(await screen.findByRole("dialog")).getByRole("button", { name: "Kopyala ve karşılaştır" }));
    await user.click(await screen.findByRole("button", { name: "Düzeltmeleri hazırla" }));
    await user.click(await screen.findByRole("button", { name: /geçici kopyada dene/ }));
    const dialog = await screen.findByRole("dialog", { name: "Sunucudaki veri kopyalansın mı?" });
    expect(within(dialog).getByText(/prod sunucusundaki magaza/)).toBeInTheDocument();
    expect(compare).toHaveBeenCalledTimes(1);
    await user.click(within(dialog).getByRole("button", { name: "Kopyala ve karşılaştır" }));
    await waitFor(() => expect(compare).toHaveBeenCalledTimes(2));
  });

  it("removes prepared Docker images from the settings", async () => {
    const user = userEvent.setup();
    render(<App api={mockApi({ delay: 0 })} />);
    await openAndScan(user);
    await user.click(nav(/Docker karşılaştırması/));
    await user.click(await screen.findByRole("button", { name: "Karşılaştırmayı başlat" }));
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
    await user.click(await screen.findByRole("button", { name: "Log içe aktar" }));
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
    await user.click(await screen.findByRole("button", { name: "Testleri PHP 8.4.27 ile çalıştır" }));
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
    await user.click(screen.getByRole("button", { name: "Karşılaştır" }));
    const table = await screen.findByRole("table", { name: "Hedefleri karşılaştır" });
    expect(within(table).getByText("Bazı dosyalar taranamadı")).toBeInTheDocument();
    expect(within(table).getByText("Bu hedef karşılaştırılamadı: boom")).toBeInTheDocument();
  });

  it("compares targets and switches to one", async () => {
    const user = setup();
    await openAndScan(user);
    await user.click(screen.getByRole("button", { name: "Karşılaştır" }));
    const table = await screen.findByRole("table", { name: "Hedefleri karşılaştır" });
    expect(within(table).getAllByRole("row")).toHaveLength(5);
    expect(within(table).getByText("Seçili")).toBeInTheDocument();
    await user.click(within(table).getAllByRole("button", { name: "Bu hedefe geç" })[0]);
    await waitFor(() => expect(screen.getByRole("radio", { name: "8.2" })).toHaveAttribute("aria-checked", "true"));
  });

  it("saves the editor used to open findings", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    render(<App api={api} />);
    await user.click(await screen.findByRole("button", { name: /Ayarlar/ }));
    await user.selectOptions(await screen.findByRole("combobox", { name: "Editör" }), "PhpStorm");
    await waitFor(async () => expect((await api.getSettings()).editor).toBe("phpstorm"));
  });

  it("renders long finding lists in pages and extends them on demand", async () => {
    const user = userEvent.setup();
    render(<App api={mockApi({ delay: 0, scale: 40 })} />);
    await openAndScan(user);
    await user.click(nav(/Bulgular/));
    const list = await screen.findByRole("listbox", { name: "Bulgu listesi" });
    expect(within(list).getAllByRole("option")).toHaveLength(300);
    expect(screen.getByText("840 bulgu")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "300 bulgu daha göster (540 kaldı)" }));
    expect(within(list).getAllByRole("option")).toHaveLength(600);

    const options = within(list).getAllByRole("option");
    await user.click(options[599]);
    await user.keyboard("{ArrowDown}");
    await waitFor(() => expect(within(list).getAllByRole("option")).toHaveLength(840));
    expect(within(list).getAllByRole("option")[600]).toHaveAttribute("aria-selected", "true");
    expect(screen.queryByRole("button", { name: /daha göster/ })).not.toBeInTheDocument();
  });

  it("previews, applies and restores changes", async () => {
    const user = setup();
    await openAndScan(user);
    await user.click(nav(/Düzeltmeler/));
    await user.click(await screen.findByRole("button", { name: /Değişiklikleri hesapla/ }));
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
    const row = (await screen.findByText(/Uyumluluk düzeltmeleri · PHP 8.4/)).closest("tr")!;
    await user.click(within(row).getByRole("button", { name: /İncele/ }));
    const review = await screen.findByRole("dialog", { name: /Uyumluluk düzeltmeleri/ });
    expect(await within(review).findByText("3 dosya", { exact: false })).toBeInTheDocument();
    expect(within(review).getByRole("region", { name: /değişiklikleri/ })).toBeInTheDocument();
    expect(within(review).queryByRole("button", { name: /helpers\.php/ })).not.toBeInTheDocument();
    await user.click(within(review).getByRole("button", { name: /Geri yükle/ }));
    await user.click(within(await screen.findByRole("dialog")).getByRole("button", { name: "Geri yükle" }));
    expect(await screen.findByText("Geri yüklendi")).toBeInTheDocument();
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
    await api.saveSettings({ phpPath: "/opt/php", snapshotLimit: null, editor: null });
    render(<App api={api} />);
    await user.click(await screen.findByRole("button", { name: /Ayarlar/ }));
    const group = await screen.findByRole("radiogroup", { name: "Saklanacak yedek" });
    await user.click(within(group).getByRole("radio", { name: "30" }));
    await waitFor(async () => expect(await api.getSettings()).toEqual({ phpPath: "/opt/php", snapshotLimit: 30, editor: null }));
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
    await user.selectOptions(screen.getByRole("combobox", { name: "Editör" }), "vscode");
    await waitFor(async () => expect(await api.getSettings()).toEqual({ phpPath: null, snapshotLimit: 30, editor: "vscode" }));
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
    await user.click(await screen.findByRole("button", { name: /Değişiklikleri hesapla/ }));
    await user.click(await screen.findByRole("checkbox", { name: "Tümünü seç" }));
    await user.click(await screen.findByRole("button", { name: "Tümünü uygula" }));
    await user.click(within(await screen.findByRole("dialog")).getByRole("button", { name: "Uygula" }));
    const alert = await screen.findByText("4 dosya güncellendi, 1 dosyada yeni sözdizimi hatası var");
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
    expect(screen.getByText("A substantial migration")).toBeInTheDocument();

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
    await waitFor(() => expect(screen.getByRole("radio", { name: "8.4" })).toBeDisabled());
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
    await user.click(await screen.findByRole("button", { name: /Değişiklikleri hesapla/ }));
    await user.click(await screen.findByRole("button", { name: /Durdur/ }));
    expect(await screen.findByRole("button", { name: /Değişiklikleri hesapla/ })).toBeEnabled();
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
    await user.click(await screen.findByRole("button", { name: /Değişiklikleri hesapla/ }));
    await screen.findByRole("button", { name: /Durdur/ });
    const before = cancels;
    await user.click(nav(/Genel bakış/));
    await user.click(screen.getByRole("radio", { name: "8.3" }));
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
    const row = (await screen.findAllByRole("row"))[1];
    await user.click(within(row).getByRole("button", { name: /Geri yükle/ }));
    const dialog = await screen.findByRole("dialog");
    expect(await within(dialog).findByText("Bu dosya uygulamadan sonra değiştirilmiş:")).toBeInTheDocument();
    expect(within(dialog).getByText("includes/helpers.php")).toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "Geri yükle" }));
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
    const row = (await screen.findAllByRole("row"))[1];
    await user.click(within(row).getByRole("button", { name: /Geri yükle/ }));
    await user.click(within(await screen.findByRole("dialog")).getByRole("button", { name: "Geri yükle" }));
    expect(await screen.findByText("Geri yüklendi")).toBeInTheDocument();
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    expect(scans).toBe(2);
  });

  it("deletes every snapshot of the project after confirming", async () => {
    const user = setup();
    await openAndScan(user);
    await user.click(nav(/Düzeltmeler/));
    await user.click(await screen.findByRole("button", { name: /Değişiklikleri hesapla/ }));
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
    await user.click(await screen.findByRole("button", { name: /Değişiklikleri hesapla/ }));
    await user.click(nav(/Genel bakış/));
    await user.click(screen.getByRole("button", { name: /Tekrar tara/ }));
    await screen.findByRole("img", { name: /Hazırlık skoru/ });
    resolve(undefined as never);
    await user.click(nav(/Düzeltmeler/));
    expect(await screen.findByRole("list", { name: "Dosyalar" })).toBeInTheDocument();
  });

  it("focuses cancel first in destructive dialogs", async () => {
    const user = setup();
    await openAndScan(user);
    await user.click(nav(/Geçmiş/));
    const row = (await screen.findAllByRole("row"))[1];
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

  it("limits the dependency warning to the packages shown", async () => {
    const user = setup();
    await openAndScan(user);
    await user.click(nav(/Bağımlılıklar/));
    expect(await screen.findByText("3 paket PHP 8.4 ile çalışmıyor.")).toBeInTheDocument();
    expect(screen.getByText(/composer update .*phpunit\/phpunit/)).toBeInTheDocument();
    await user.click(screen.getByRole("radio", { name: "Sadece üretim" }));
    expect(screen.getByText("2 paket PHP 8.4 ile çalışmıyor.")).toBeInTheDocument();
    expect(screen.queryByText(/phpunit\/phpunit/)).not.toBeInTheDocument();
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
    await user.click(await screen.findByRole("button", { name: "Değişiklikleri hesapla" }));
    const hunk = await screen.findByRole("checkbox", { name: /satırdaki değişikliği uygula/ });
    await user.click(hunk);
    await user.click(screen.getByRole("button", { name: "Tümünü uygula" }));
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
    await user.click(await screen.findByRole("button", { name: "Değişiklikleri hesapla" }));
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
    await user.click(await screen.findByRole("button", { name: "Değişiklikleri hesapla" }));
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
    await user.selectOptions(await screen.findByRole("combobox", { name: "Adım" }), "8.0");
    await user.click(screen.getByRole("button", { name: "Değişiklikleri hesapla" }));
    await waitFor(() => expect(preview).toHaveBeenCalled());
    expect(preview.mock.calls[0][1]).toBe("8.0");
    await user.click(await screen.findByRole("checkbox", { name: "Tümünü seç" }));
    await user.click(await screen.findByRole("button", { name: "Tümünü uygula" }));
    const dialog = await screen.findByRole("dialog");
    await user.click(within(dialog).getByRole("button", { name: "Uygula" }));
    await user.click(await screen.findByRole("button", { name: "PHP 8.1 adımına geç" }));
    await waitFor(() => expect(preview.mock.calls.at(-1)?.[1]).toBe("8.1"));
  });
});
