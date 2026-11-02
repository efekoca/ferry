import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import axe from "axe-core";
import { describe, expect, it } from "vitest";
import App from "../App";
import { mockApi } from "../lib/mock";

async function violations(context: Element = document.body) {
  const result = await axe.run(context, {
    rules: { "color-contrast": { enabled: false }, region: { enabled: false } },
  });
  return result.violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).join(", ")}`);
}

const nav = (name: RegExp) => within(screen.getByRole("navigation", { name: "Bölümler" })).getByRole("button", { name });

describe("accessibility", () => {
  it("has no axe violations on any screen or dialog", async () => {
    const user = userEvent.setup();
    render(<App api={mockApi({ delay: 0 })} />);
    const found: Record<string, string[]> = {};
    const check = async (name: string) => {
      found[name] = await violations();
    };

    await screen.findByRole("heading", { name: "Kodunuz yeni sürüme hazır olsun" });
    await check("welcome");
    await user.click(screen.getByRole("button", { name: "Proje klasörü seçin" }));
    await screen.findByRole("heading", { name: "Genel bakış" });
    await check("overview before scan");
    await user.click(screen.getByRole("button", { name: /Taramayı başlat/ }));
    await screen.findByRole("img", { name: /Hazırlık skoru/ });
    await check("overview");
    await user.click(nav(/Bulgular/));
    await screen.findByRole("listbox", { name: "Bulgu listesi" });
    await check("findings");
    await user.click(nav(/Düzeltmeler/));
    await screen.findByRole("button", { name: /Değişiklikleri hesapla/ });
    await check("fixes intro");
    await user.click(screen.getByRole("button", { name: /Değişiklikleri hesapla/ }));
    await screen.findByRole("list", { name: "Dosyalar" });
    await check("fixes");
    await user.click(screen.getByRole("checkbox", { name: "Tümünü seç" }));
    await user.click(screen.getByRole("button", { name: "Tümünü uygula" }));
    await screen.findByRole("dialog");
    await check("apply dialog");
    await user.click(within(screen.getByRole("dialog")).getByRole("button", { name: "Uygula" }));
    await screen.findByText("4 dosya güncellendi");
    await user.click(nav(/Bağımlılıklar/));
    await screen.findAllByRole("table");
    await check("dependencies");
    await user.click(nav(/Geçmiş/));
    const row = (await screen.findAllByRole("row"))[1];
    await check("history");
    await user.click(within(row).getByRole("button", { name: /İncele/ }));
    await screen.findByRole("dialog", { name: /Uyumluluk/ });
    await check("history review");
    await user.keyboard("{Escape}");
    await user.click(screen.getByRole("button", { name: /Ayarlar/ }));
    await screen.findByRole("heading", { name: "Ayarlar" });
    await check("settings");

    expect(Object.fromEntries(Object.entries(found).filter(([, v]) => v.length))).toEqual({});
  });

  it("moves through findings with the keyboard using a single tab stop", async () => {
    const user = userEvent.setup();
    render(<App api={mockApi({ delay: 0 })} />);
    await user.click(await screen.findByRole("button", { name: "Proje klasörü seçin" }));
    await user.click(await screen.findByRole("button", { name: /Taramayı başlat/ }));
    await screen.findByRole("img", { name: /Hazırlık skoru/ });
    await user.click(nav(/Bulgular/));
    const options = within(await screen.findByRole("listbox", { name: "Bulgu listesi" })).getAllByRole("option");
    expect(options.filter((o) => o.tabIndex === 0)).toHaveLength(1);

    options[0].focus();
    await user.keyboard("{ArrowDown}");
    await waitFor(() => expect(options[1]).toHaveFocus());
    expect(options[1]).toHaveAttribute("aria-selected", "true");
    await user.keyboard("{End}");
    await waitFor(() => expect(options.at(-1)).toHaveFocus());
    await user.keyboard("{Home}");
    await waitFor(() => expect(options[0]).toHaveFocus());
    expect(document.title).toBe("Bulgular · Ferry");
  });

  it("describes confirmation dialogs with their body text", async () => {
    const user = userEvent.setup();
    render(<App api={mockApi({ delay: 0 })} />);
    await user.click(await screen.findByRole("button", { name: "Proje klasörü seçin" }));
    await user.click(await screen.findByRole("button", { name: /Taramayı başlat/ }));
    await screen.findByRole("img", { name: /Hazırlık skoru/ });
    await user.click(nav(/Düzeltmeler/));
    await user.click(await screen.findByRole("button", { name: /Değişiklikleri hesapla/ }));
    await user.click(await screen.findByRole("checkbox", { name: "Tümünü seç" }));
    await user.click(await screen.findByRole("button", { name: "Tümünü uygula" }));
    expect(await screen.findByRole("dialog")).toHaveAccessibleDescription(/satır değişecek/);
  });
});
