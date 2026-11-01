import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import App from "../App";
import { mockApi } from "../lib/mock";

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
    api.snapshotConflicts = async () => ["includes/helpers.php"];
    const restored: string[][] = [];
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
    expect(restored).toEqual([["includes/helpers.php"]]);
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

  it("supports keyboard selection in the version picker", async () => {
    const user = setup();
    await user.click(await screen.findByRole("button", { name: "Proje klasörü seçin" }));
    const current = await screen.findByRole("radio", { name: "8.4" });
    current.focus();
    await user.keyboard("{ArrowRight}");
    expect(screen.getByRole("radio", { name: "8.5" })).toHaveAttribute("aria-checked", "true");
  });
});
