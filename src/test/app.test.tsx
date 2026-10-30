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
    await user.click(screen.getByRole("button", { name: /Taramayı başlat/ }));
    await screen.findByRole("img", { name: /Hazırlık skoru/ });
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("changing the target version clears stale results", async () => {
    const user = setup();
    await openAndScan(user);
    await user.click(screen.getByRole("radio", { name: "8.1" }));
    expect(screen.queryByRole("img", { name: /Hazırlık skoru/ })).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /Taramayı başlat/ }));
    await screen.findByRole("img", { name: /Hazırlık skoru/ });
    expect(screen.getByText("PHP 8.1", { selector: ".badge" })).toBeInTheDocument();
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

    await user.type(screen.getByRole("textbox", { name: "Bulgularda ara" }), "strftime");
    expect(within(list).getAllByRole("option")).toHaveLength(1);
    const detail = screen.getByRole("complementary", { name: "Bulgu ayrıntısı" });
    expect(within(detail).getByRole("heading", { name: "strftime()" })).toBeInTheDocument();
    expect(within(detail).getByText(/admin\/settings-page.php:88/)).toBeInTheDocument();
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
    await user.type(screen.getByRole("textbox", { name: "Search findings" }), "nullable");
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
    await user.click(screen.getByRole("radio", { name: "8.1" }));
    expect(cancels).toBeGreaterThan(before);
  });

  it("warns before restoring files edited after the apply", async () => {
    const user = userEvent.setup();
    const api = mockApi({ delay: 0 });
    api.snapshotConflicts = async () => ["includes/helpers.php"];
    render(<App api={api} />);
    await openAndScan(user);
    await user.click(nav(/Geçmiş/));
    const row = (await screen.findAllByRole("row"))[1];
    await user.click(within(row).getByRole("button", { name: /Geri yükle/ }));
    const dialog = await screen.findByRole("dialog");
    expect(await within(dialog).findByText("Bu dosya uygulamadan sonra değiştirilmiş:")).toBeInTheDocument();
    expect(within(dialog).getByText("includes/helpers.php")).toBeInTheDocument();
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

  it("supports keyboard selection in the version picker", async () => {
    const user = setup();
    await user.click(await screen.findByRole("button", { name: "Proje klasörü seçin" }));
    const current = await screen.findByRole("radio", { name: "8.4" });
    current.focus();
    await user.keyboard("{ArrowRight}");
    expect(screen.getByRole("radio", { name: "8.5" })).toHaveAttribute("aria-checked", "true");
  });
});
