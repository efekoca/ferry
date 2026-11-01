import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { DiffViewer, wordHighlight } from "../components/DiffViewer";
import { ScoreRing } from "../components/ScoreRing";
import { DICTS } from "../i18n";
import { commonTitle, dependencyBlockers, formatBytes, formatDuration, formatRelative, joinPath, scoreHeadline, scoreTone } from "../lib/labels";
import { NO_FILTER, type FindingsFilter } from "../lib/store";
import { filterFindings } from "../screens/Findings";
import { isVulnerable, patchedFor } from "../lib/security";
import type { ExtensionUse, FileDiff, Finding } from "../lib/types";
import { compareProbe, parseProbe } from "../lib/probe";

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

describe("wordHighlight", () => {
  it("isolates the changed span", () => {
    expect(wordHighlight("f(Foo $a = null)", "f(?Foo $a = null)")).toEqual(["f(", "", "Foo $a = null)"]);
    expect(wordHighlight("f(?Foo $a = null)", "f(Foo $a = null)")).toEqual(["f(", "?", "Foo $a = null)"]);
    expect(wordHighlight("same", "same")).toEqual(["same", "", ""]);
    expect(wordHighlight("abc", "xyz")).toEqual(["", "abc", ""]);
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
    expect(container.querySelector("tr.del mark")?.textContent).toBe("utf8_encode($s");
    expect(container.querySelector("tr.add mark")?.textContent).toBe("mb_convert_encoding($s, 'UTF-8', 'ISO-8859-1'");
    expect(container.querySelector(".diff__hunk")?.textContent).toContain("−3 +3");
  });
});

describe("labels", () => {
  it("maps scores to tones and headlines", () => {
    expect(scoreTone(95)).toBe("auto");
    expect(scoreTone(60)).toBe("manual");
    expect(scoreTone(10)).toBe("danger");
    expect(scoreHeadline(100, 0, tr)).toBe("Hedef sürüme hazır");
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

  it("score ring exposes an accessible label", () => {
    render(<ScoreRing score={73} />);
    expect(screen.getByRole("img", { name: "Hazırlık skoru: 73/100" })).toHaveAttribute("data-tone", "manual");
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
});

describe("helpers", () => {
  it("counts an incompatible root constraint as a blocker", () => {
    expect(dependencyBlockers(null)).toEqual({ packages: 0, root: false, total: 0 });
    expect(
      dependencyBlockers({ target: "8.4", rootConstraint: ">=7.1 <8.0", rootStatus: "incompatible", packages: [], hasLockFile: false, extensions: [] }),
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
  });
});
