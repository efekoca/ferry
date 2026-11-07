import { touches } from "../lib/fixes";
import type { CheckSummary, DockerEvent, DockerSummary, FileChange, HistoryEntry, Snapshot } from "../lib/types";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";
import { DiffViewer, splitRows, wordDiff } from "../components/DiffViewer";
import { Select, TagInput } from "../components/ui";
import { DICTS, I18nProvider } from "../i18n";
import { highlight } from "../lib/code";
import { columnOf, dayLabels, historyEvents, niceMax, rangeStart, smoothPath, spreadOut } from "../lib/history";
import { squarify } from "../lib/treemap";
import { filesHoldingHalf, fileStats } from "../screens/Overview";
import { commonTitle, dependencyBlockers, formatBytes, formatDuration, formatRelative, joinPath, scoreHeadline, scoreTone } from "../lib/labels";
import { NO_FILTER, type FindingsFilter } from "../lib/store";
import { filterFindings, prioritizeFindings } from "../screens/Findings";
import { relatedFindings } from "../screens/FindingDetail";
import { RULE_NOTES } from "../i18n/rules";
import { isVulnerable, patchedFor } from "../lib/security";
import type { ExtensionUse, FileDiff, Finding } from "../lib/types";
import { compareProbe, parseProbe } from "../lib/probe";
import { ErrorBoundary } from "../components/ErrorBoundary";
import { fireEvent } from "@testing-library/react";

const { tr, en } = DICTS;

describe("dictionaries", () => {
  it("have the same keys in both languages", () => {
    const keys = (o: object, prefix = ""): string[] =>
      Object.entries(o).flatMap(([k, v]) =>
        v && typeof v === "object" && !Array.isArray(v) ? keys(v, `${prefix}${k}.`) : [`${prefix}${k}`],
      );
    expect(keys(en).sort()).toEqual(keys(tr).sort());
  });
});

const changed = (text: string, marks: boolean[]) => [...text].filter((_, i) => marks[i]).join("");

describe("wordDiff", () => {
  it("isolates the changed span", () => {
    const [before, after] = wordDiff("f(Foo $a = null)", "f(?Foo $a = null)");
    expect(changed("f(Foo $a = null)", before)).toBe("");
    expect(changed("f(?Foo $a = null)", after)).toBe("?");
    expect(wordDiff("same", "same")[0].some(Boolean)).toBe(false);
    expect(changed("abc", wordDiff("abc", "xyz")[0])).toBe("abc");
  });

  it("keeps an inserted word in one piece without its trailing space", () => {
    const text = "    public function label($row)";
    expect(changed(text, wordDiff("    function label($row)", text)[1])).toBe("public");
  });

  it("marks only the tokens that changed in a long line", () => {
    const before = "return $row[status] == 'paid' ? 'yes' : $row[status];";
    const after = "return $row['status'] == 'paid' ? 'yes' : $row['status'];";
    expect(changed(after, wordDiff(before, after)[1])).toBe("''''");
  });
});

describe("DiffViewer", () => {
  const diff: FileDiff = {
    additions: 1,
    deletions: 1,
    hunks: [
      {
        oldStart: 3,
        newStart: 3,
        lines: [
          { kind: "context", oldNo: 3, newNo: 3, text: "{" },
          { kind: "del", oldNo: 4, newNo: null, text: "    return utf8_encode($s);" },
          { kind: "add", oldNo: null, newNo: 4, text: "    return mb_convert_encoding($s, 'UTF-8', 'ISO-8859-1');" },
        ],
      },
    ],
  };

  it("renders numbered rows with kinds and intra-line marks", () => {
    const { container } = render(<DiffViewer diff={diff} file="a.php" />);
    expect(screen.getByRole("region", { name: "a.php değişiklikleri" })).toBeInTheDocument();
    const rows = container.querySelectorAll("tr[data-kind]");
    expect([...rows].map((r) => r.getAttribute("data-kind"))).toEqual(["context", "del", "add"]);
    expect(container.querySelector("tr.del mark")?.textContent).toBe("utf8_encode");
    expect(container.querySelector("tr.add mark")?.textContent).toBe("mb_convert_encoding");
    expect(container.querySelector(".diff__hunk")?.textContent).toContain("Satır 3");
  });
});

describe("labels", () => {
  it("maps scores to tones and headlines", () => {
    expect(scoreTone(95)).toBe("auto");
    expect(scoreTone(60)).toBe("manual");
    expect(scoreTone(10)).toBe("danger");
    expect(scoreHeadline(100, 0, tr)).toBe("Hedef sürüme hazır");
    expect(scoreHeadline(100, 0, tr, true)).toBe("Tarama eksik kaldı");
    expect(scoreHeadline(30, 12, en)).toBe("A substantial migration");
  });

  it("formats relative times per language", () => {
    const now = 1_000_000_000_000;
    expect(formatRelative(now - 10_000, tr, now)).toBe("az önce");
    expect(formatRelative(now - 5 * 60_000, tr, now)).toBe("5 dakika önce");
    expect(formatRelative(now - 3 * 3_600_000, en, now)).toBe("3 hours ago");
    expect(formatRelative(now - 2 * 86_400_000, en, now)).toBe("2 days ago");
  });

  it("formats durations", () => {
    expect(formatDuration(0, "tr-TR")).toBe("< 1 ms");
    expect(formatDuration(1500, "tr-TR")).toBe("1,5 sn");
    expect(formatDuration(1500, "en-US")).toBe("1.5 s");
  });


  it("groups function families", () => {
    expect(commonTitle(["mysql_connect()", "mysql_query()"])).toBe("mysql_*()");
    expect(commonTitle(["each()", "each()"])).toBe("each()");
  });
});

describe("filterFindings", () => {
  const base: Omit<Finding, "id" | "file" | "fix" | "severity" | "title"> = {
    line: 1,
    column: 1,
    runtimeHits: 0,
    ruleId: "x",
    message: { tr: "", en: "" },
    since: "8.0",
    replacement: null,
    snippet: "code();",
  };
  const findings: Finding[] = [
    { ...base, id: "1", file: "src/a.php", fix: "auto", severity: "breaking", title: { tr: "each()", en: "each()" } },
    { ...base, id: "2", file: "src/b.php", fix: "manual", severity: "deprecated", title: { tr: "Örtük nullable", en: "Implicitly nullable" } },
    { ...base, id: "3", file: "lib/c.php", ruleId: "removed.mysql", fix: "manual", severity: "breaking", title: { tr: "mysql_query()", en: "mysql_query()" } },
  ];
  const ids = (list: Finding[]) => list.map((f) => f.id);

  it("combines fix, severity, rule and query filters", () => {
    const f = (patch: Partial<FindingsFilter>) => ids(filterFindings(findings, { ...NO_FILTER, ...patch }));
    expect(f({})).toEqual(["1", "2", "3"]);
    expect(f({ fix: "manual" })).toEqual(["2", "3"]);
    expect(f({ fix: "manual", severity: "breaking" })).toEqual(["3"]);
    expect(f({ query: "SRC/" })).toEqual(["1", "2"]);
    expect(f({ query: "implicitly" })).toEqual(["2"]);
    expect(f({ query: "örtük" })).toEqual(["2"]);
    expect(f({ fix: "auto", severity: "deprecated" })).toEqual([]);
    expect(f({ rule: "removed.mysql" })).toEqual(["3"]);
  });

  it("keeps files together and lists findings seen at runtime first", () => {
    const at = (id: string, file: string, runtimeHits = 0): Finding => ({ ...findings[0], id, file, runtimeHits });
    expect(ids(prioritizeFindings([at("1", "a.php"), at("2", "b.php"), at("3", "a.php")]))).toEqual(["1", "3", "2"]);
    expect(ids(prioritizeFindings([at("1", "a.php"), at("2", "b.php"), at("3", "c.php", 2), at("4", "c.php"), at("5", "b.php", 1)]))).toEqual(["3", "4", "5", "2", "1"]);
  });
});

describe("helpers", () => {
  it("counts an incompatible root constraint as a blocker", () => {
    expect(dependencyBlockers(null)).toEqual({ packages: 0, root: false, total: 0 });
    expect(
      dependencyBlockers({ target: "8.4", rootConstraint: ">=7.1 <8.0", rootStatus: "incompatible", packages: [], hasLockFile: false, extensions: [], wordpress: [] }),
    ).toEqual({ packages: 0, root: true, total: 1 });
  });

  it("keeps the platform separator of the project root", () => {
    expect(joinPath("/Users/demo/site/", "src/a.php")).toBe("/Users/demo/site/src/a.php");
    expect(joinPath("C:\\xampp\\htdocs\\site", "src/a.php")).toBe("C:\\xampp\\htdocs\\site\\src\\a.php");
  });
});

describe("isVulnerable", () => {
  it("flags every branch below its patched release and every unsupported branch", () => {
    for (const v of ["5.4.45", "7.4.33", "8.0.30", "8.1.34", "8.2.34", "8.3.35", "8.4.26", "8.5.11", "8.4.0"]) expect(isVulnerable(v), v).toBe(true);
    for (const v of ["8.2.35", "8.3.36", "8.4.27", "8.5.12", "8.5.13-dev", "8.6.0", "5.3.29", "unknown"]) expect(isVulnerable(v), v).toBe(false);
    expect(patchedFor("8.3.1")).toBe("8.3.36");
    expect(patchedFor("7.4.33")).toBe("8.4.27");
  });
});

describe("formatBytes", () => {
  it("picks a readable unit", () => {
    expect(formatBytes(512, "en-US")).toBe("512 byte");
    expect(formatBytes(1536, "en-US")).toBe("1.5 kB");
    expect(formatBytes(5 * 1024 * 1024, "en-US")).toBe("5 MB");
    expect(formatBytes(3 * 1024 ** 3, "tr-TR")).toBe("3 GB");
  });
});

describe("server probe", () => {
  const output = `{"ferryProbe":1,"php":"8.2.10","sapi":"fpm-fcgi","os":"Linux","server":"nginx","extensions":["Core","curl","mysqli"],"ini":{"memory_limit":"256M","disable_functions":"exec, shell_exec"}}`;
  const ext = (name: string): ExtensionUse => ({ name, uses: 1, file: "a.php", line: 1, symbol: `${name}()`, declared: false, removed: false, available: null });

  it("parses the probe output and compares it with the target", () => {
    expect(parseProbe("garbage")).toBeNull();
    const probe = parseProbe(`HTTP/1.1 200 OK\n\n${output}`)!;
    const report = compareProbe(probe, "8.4", [ext("curl"), ext("intl"), ext("gd")]);
    expect(report).toMatchObject({ php: "8.2.10", belowTarget: true, vulnerable: true, fpm: true, missing: ["intl", "gd"], disabled: ["exec", "shell_exec"], memoryLimit: "256M" });
    expect(compareProbe({ ...probe, php: "8.4.27" }, "8.4", []).belowTarget).toBe(false);
    const required = { ...ext("intl"), uses: 0, requiredBy: ["composer.json"] };
    const devOnly = { ...ext("xdebug"), uses: 0, dev: true, requiredBy: ["composer.json (require-dev)"] };
    const withOpcache = { ...probe, extensions: [...probe.extensions, "Zend OPcache"] };
    expect(compareProbe(withOpcache, "8.4", [required, devOnly, ext("opcache")]).missing).toEqual(["intl"]);
  });
});

describe("probe validation", () => {
  it("drops values of the wrong type and rejects malformed output", () => {
    const odd = parseProbe(JSON.stringify({ ferryProbe: 1, php: "8.4.27", extensions: [], server: { unexpected: true }, ini: { memory_limit: { x: 1 }, display_errors: false, upload_max_filesize: "8M" } }))!;
    expect(odd.server).toBeNull();
    expect(odd.ini).toEqual({ display_errors: false, upload_max_filesize: "8M" });
    expect(parseProbe(JSON.stringify({ ferryProbe: 1, php: "eight", extensions: [] }))).toBeNull();
    expect(parseProbe(JSON.stringify({ ferryProbe: 1, php: "8.4.27", extensions: [{}] }))).toBeNull();
    expect(parseProbe(JSON.stringify({ ferryProbe: 1, php: "8.4.27", extensions: ["x".repeat(600 * 1024)] }))).toBeNull();
    expect(parseProbe("[1,2]")).toBeNull();
  });
});

describe("ErrorBoundary", () => {
  it("shows the fallback and recovers", () => {
    let broken = true;
    const Child = () => {
      if (broken) throw new Error("boom");
      return <p>fine</p>;
    };
    const spy = vi.spyOn(console, "error").mockImplementation(() => {});
    render(
      <ErrorBoundary fallback={(e, reset) => <button onClick={reset}>{e.message}</button>}>
        <Child />
      </ErrorBoundary>,
    );
    broken = false;
    fireEvent.click(screen.getByRole("button", { name: "boom" }));
    expect(screen.getByText("fine")).toBeInTheDocument();
    spy.mockRestore();
  });
});


describe("touches", () => {
  const change = {
    file: "a.php",
    rules: [],
    fix: "auto",
    diff: { additions: 1, deletions: 1, hunks: [{ oldStart: 8, newStart: 8, lines: [{ kind: "context", oldNo: 8, newNo: 8, text: "" }, { kind: "del", oldNo: 9, newNo: null, text: "" }, { kind: "add", oldNo: null, newNo: 9, text: "" }, { kind: "context", oldNo: 10, newNo: 10, text: "" }] }] },
  } as FileChange;
  const at = (path: string, line: number) => ({ path, line, level: "fatal", message: "", count: 1 }) as DockerEvent;

  it("matches an event inside a hunk of the same file", () => {
    expect(touches(at("/app/a.php", 9), change)).toBe(true);
    expect(touches(at("/app/a.php", 11), change)).toBe(false);
    expect(touches(at("/app/b.php", 9), change)).toBe(false);
  });
});

describe("Select", () => {
  const options = [
    { value: "a", label: "Alfa" },
    { value: "b", label: "Beta", disabled: true },
    { value: "c", label: "Gama" },
  ];

  function Picker({ onChange }: { onChange: (value: string) => void }) {
    const [value, setValue] = useState("a");
    return (
      <Select
        label="Seçim"
        value={value}
        options={options}
        onChange={(v) => {
          setValue(v);
          onChange(v);
        }}
      />
    );
  }

  it("moves past disabled options with the keyboard and picks with Enter", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<I18nProvider><Picker onChange={onChange} /></I18nProvider>);
    const box = screen.getByRole("combobox", { name: "Seçim" });
    box.focus();
    await user.keyboard("{ArrowDown}");
    expect(box).toHaveAttribute("aria-expanded", "true");
    await user.keyboard("{ArrowDown}{Enter}");
    expect(onChange).toHaveBeenCalledWith("c");
    expect(box).toHaveAttribute("aria-expanded", "false");
    expect(box).toHaveFocus();
    expect(box).toHaveTextContent("Gama");
  });

  it("closes on Escape without changing the value and ignores a disabled click", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<I18nProvider><Picker onChange={onChange} /></I18nProvider>);
    const box = screen.getByRole("combobox", { name: "Seçim" });
    await user.click(box);
    await user.click(screen.getByRole("option", { name: "Beta" }));
    expect(screen.getByRole("listbox")).toBeInTheDocument();
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    expect(onChange).not.toHaveBeenCalled();
  });

  it("jumps to an option by typing and closes on an outside click", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<I18nProvider><Picker onChange={onChange} /><button>dış</button></I18nProvider>);
    const box = screen.getByRole("combobox", { name: "Seçim" });
    box.focus();
    await user.keyboard("{Enter}g{Enter}");
    expect(onChange).toHaveBeenCalledWith("c");
    await user.click(box);
    await user.click(screen.getByRole("button", { name: "dış" }));
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
  });
});

describe("TagInput", () => {
  function Tags({ onChange }: { onChange: (values: string[]) => void }) {
    const [values, setValues] = useState(["legacy"]);
    return (
      <TagInput
        label="Klasörler"
        values={values}
        removeLabel={(v) => `${v} çıkar`}
        onChange={(next) => {
          setValues(next);
          onChange(next);
        }}
      />
    );
  }

  it("adds on Enter, ignores duplicates and removes with Backspace or the button", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<Tags onChange={onChange} />);
    const field = screen.getByRole("textbox", { name: "Klasörler" });
    await user.type(field, "tools{Enter}");
    expect(onChange).toHaveBeenLastCalledWith(["legacy", "tools"]);
    onChange.mockClear();
    await user.type(field, "tools{Enter}");
    expect(onChange).not.toHaveBeenCalled();
    await user.keyboard("{Backspace}");
    expect(onChange).toHaveBeenLastCalledWith(["legacy"]);
    await user.click(screen.getByRole("button", { name: "legacy çıkar" }));
    expect(onChange).toHaveBeenLastCalledWith([]);
  });

  it("splits a pasted list and keeps what was typed when the field loses focus", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<Tags onChange={onChange} />);
    const field = screen.getByRole("textbox", { name: "Klasörler" });
    await user.click(field);
    await user.paste("a\nb, c");
    expect(onChange).toHaveBeenLastCalledWith(["legacy", "a", "b", "c"]);
    await user.type(field, "d");
    await user.tab();
    expect(onChange).toHaveBeenLastCalledWith(["legacy", "a", "b", "c", "d"]);
  });
});

describe("highlight", () => {
  it("colours PHP and keeps the source text intact", () => {
    const { container } = render(<code>{highlight("$a = strlen('x'); // ok")}</code>);
    expect(container.textContent).toBe("$a = strlen('x'); // ok");
    expect(container.querySelector(".tok-variable")?.textContent).toBe("$a");
    expect(container.querySelector(".tok-function")?.textContent).toBe("strlen");
    expect(container.querySelector(".tok-string")?.textContent).toBe("'x'");
    expect(container.querySelector(".tok-comment")?.textContent).toBe("// ok");
  });

  it("leaves other files plain", () => {
    const { container } = render(<code>{highlight("FROM php:8.4", [], false)}</code>);
    expect(container.querySelector("[class^=tok-]")).toBeNull();
  });
});

describe("squarify", () => {
  const items = [47, 22, 19, 18, 9, 9, 8, 8, 7, 7, 7, 7, 6, 5, 4, 4, 3, 1, 1, 1, 1, 1].map((value) => ({ value }));

  it("gives each file an area in proportion to its findings and fills the box", () => {
    const tiles = squarify(items, 730, 770);
    const total = items.reduce((a, b) => a + b.value, 0);
    expect(tiles).toHaveLength(items.length);
    expect(tiles.reduce((a, t) => a + t.width * t.height, 0)).toBeCloseTo(730 * 770, 3);
    for (const tile of tiles) {
      expect((tile.width * tile.height) / (730 * 770)).toBeCloseTo(tile.item.value / total, 6);
      expect(tile.x + tile.width).toBeLessThanOrEqual(730.001);
      expect(tile.y + tile.height).toBeLessThanOrEqual(770.001);
    }
  });

  it("does not overlap tiles and keeps them from getting thin", () => {
    const tiles = squarify(items, 730, 770);
    for (const a of tiles) {
      expect(Math.max(a.width / a.height, a.height / a.width)).toBeLessThan(5);
      for (const b of tiles) {
        if (a === b) continue;
        const apart = a.x + a.width <= b.x + 0.001 || b.x + b.width <= a.x + 0.001 || a.y + a.height <= b.y + 0.001 || b.y + b.height <= a.y + 0.001;
        expect(apart).toBe(true);
      }
    }
  });

  it("ignores empty values and an empty box", () => {
    expect(squarify([{ value: 0 }], 100, 100)).toEqual([]);
    expect(squarify([{ value: 3 }], 0, 100)).toEqual([]);
    expect(squarify([{ value: 3 }, { value: 0 }], 100, 100)).toHaveLength(1);
  });
});

describe("fileStats", () => {
  const finding = (file: string, severity: Finding["severity"]) => ({ file, severity }) as Finding;
  const stats = fileStats([finding("a.php", "breaking"), finding("b.php", "deprecated"), finding("b.php", "breaking"), finding("b.php", "behavior"), finding("c.php", "behavior")]);

  it("counts severities per file with the heaviest first", () => {
    expect(stats.map((s) => s.file)).toEqual(["b.php", "a.php", "c.php"]);
    expect(stats[0]).toMatchObject({ value: 3, breaking: 1, deprecated: 1, behavior: 1 });
  });

  it("finds how few files hold half of the findings", () => {
    expect(filesHoldingHalf(stats)).toBe(1);
    expect(filesHoldingHalf(fileStats([finding("a", "breaking"), finding("b", "breaking"), finding("c", "breaking"), finding("d", "breaking")]))).toBe(2);
    expect(filesHoldingHalf([])).toBe(0);
  });
});

describe("rule notes", () => {
  it("has text for both languages", () => {
    for (const [id, note] of Object.entries(RULE_NOTES)) {
      expect(note.why.tr.length, id).toBeGreaterThan(40);
      expect(note.why.en.length, id).toBeGreaterThan(40);
    }
  });
});

describe("relatedFindings", () => {
  const finding = (id: string, ruleId: string, file: string) => ({ id, ruleId, file }) as Finding;
  const list = [
    finding("a", "removed.set_magic_quotes_runtime", "a.php"),
    finding("b", "removed.get_magic_quotes_gpc", "b.php"),
    finding("c", "removed.magic_quotes_runtime", "a.php"),
    finding("d", "removed.each", "a.php"),
    finding("e", "removed.set_magic_quotes_runtime", "c.php"),
  ];

  it("joins rules that share a note and puts the same file first", () => {
    expect(relatedFindings(list[0], list).map((f) => f.id)).toEqual(["c", "b", "e"]);
  });

  it("keeps to the same rule when there is no note", () => {
    const plain = [finding("a", "x.one", "a.php"), finding("b", "x.two", "a.php"), finding("c", "x.one", "b.php")];
    expect(relatedFindings(plain[0], plain).map((f) => f.id)).toEqual(["c"]);
  });

  it("stops at the limit", () => {
    expect(relatedFindings(list[0], list, 1)).toHaveLength(1);
  });
});

describe("splitRows", () => {
  const line = (kind: "context" | "add" | "del", text: string) => ({ kind, text, oldNo: 1, newNo: 1 });

  it("pairs removed and added lines and keeps context on both sides", () => {
    const rows = splitRows([line("context", "a"), line("del", "b"), line("del", "c"), line("add", "d"), line("context", "e")]);
    expect(rows.map((r) => [r.left?.text, r.right?.text])).toEqual([["a", "a"], ["b", "d"], ["c", undefined], ["e", "e"]]);
  });

  it("handles an addition with nothing removed", () => {
    expect(splitRows([line("add", "x")]).map((r) => [r.left?.text, r.right?.text])).toEqual([[undefined, "x"]]);
  });
});

describe("history chart helpers", () => {
  const scan = (at: number) => ({ at, target: "8.4", score: 1, total: 1 }) as HistoryEntry;

  it("rounds the bar axis up to an even, readable top", () => {
    expect([0, 1, 7, 21, 36, 195, 1234].map(niceMax)).toEqual([2, 2, 8, 22, 36, 200, 1400]);
  });

  it("never overshoots between two points", () => {
    const path = smoothPath([0, 10, 20, 30], [100, 100, 0, 0]);
    const ys = [...path.matchAll(/C[\d.]+ ([\d.-]+) [\d.]+ ([\d.-]+) [\d.]+ ([\d.-]+)/g)].flatMap((m) => m.slice(1).map(Number));
    expect(Math.max(...ys)).toBeLessThanOrEqual(100);
    expect(Math.min(...ys)).toBeGreaterThanOrEqual(0);
    expect(smoothPath([5], [6])).toBe("M5 6");
    expect(smoothPath([], [])).toBe("");
  });

  it("places an event between the scans around it", () => {
    const scans = [scan(100), scan(200), scan(400)];
    expect(columnOf(150_000, scans)).toBeCloseTo(0.5);
    expect(columnOf(300_000, scans)).toBeCloseTo(1.5);
    expect(columnOf(50_000, scans)).toBeLessThan(0);
    expect(columnOf(900_000, scans)).toBeGreaterThan(2);
  });

  it("keeps marks apart and inside the chart", () => {
    expect(spreadOut([10, 10, 50], 3)).toEqual([10, 13, 50]);
    expect(spreadOut([99, 100, 100], 3)).toEqual([94, 97, 100]);
    expect(spreadOut([50, 10], 3)).toEqual([50, 10]);
  });

  it("names each day once and thins labels when scans are dense", () => {
    const t = DICTS.tr;
    const noon = new Date(2026, 10, 2, 12).getTime() / 1000;
    const scans = [noon, noon + 60, noon + 86400, noon + 86460].map(scan);
    expect(dayLabels(scans, t).filter(Boolean)).toHaveLength(2);
    const dense = Array.from({ length: 30 }, (_, i) => scan(noon + i * 86400));
    expect(dayLabels(dense, t).filter(Boolean).length).toBeLessThanOrEqual(8);
  });

  it("measures ranges from now", () => {
    expect(rangeStart("all")).toBe(0);
    expect(rangeStart("7", 8 * 86_400_000)).toBe(86_400_000);
  });

  it("merges events newest first in milliseconds", () => {
    const events = historyEvents(
      [scan(10)],
      [{ id: "a", createdAt: 30_000, projectRoot: "/", label: "x", files: [] } as Snapshot],
      [{ at: 20 } as DockerSummary],
      [{ at: 5 } as CheckSummary],
    );
    expect(events.map((e) => [e.kind, e.at])).toEqual([["fix", 30_000], ["real", 20_000], ["scan", 10_000], ["check", 5_000]]);
  });
});
