import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { DiffViewer, wordHighlight } from "../components/DiffViewer";
import { ScoreRing } from "../components/ScoreRing";
import { DICTS } from "../i18n";
import { dependencyBlockers, formatDuration, formatRelative, joinPath, scoreHeadline, scoreTone } from "../lib/labels";
import { filterFindings } from "../screens/Findings";
import { commonTitle } from "../screens/Overview";
import type { FileDiff, Finding } from "../lib/types";

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
    ruleId: "x",
    message: { tr: "", en: "" },
    since: "8.0",
    replacement: null,
    snippet: "code();",
  };
  const findings: Finding[] = [
    { ...base, id: "1", file: "src/a.php", fix: "auto", severity: "breaking", title: { tr: "each()", en: "each()" } },
    { ...base, id: "2", file: "src/b.php", fix: "manual", severity: "deprecated", title: { tr: "Örtük nullable", en: "Implicitly nullable" } },
    { ...base, id: "3", file: "lib/c.php", fix: "manual", severity: "breaking", title: { tr: "mysql_query()", en: "mysql_query()" } },
  ];
  const ids = (list: Finding[]) => list.map((f) => f.id);

  it("combines fix, severity and query filters", () => {
    expect(ids(filterFindings(findings, "all", "all", ""))).toEqual(["1", "2", "3"]);
    expect(ids(filterFindings(findings, "manual", "all", ""))).toEqual(["2", "3"]);
    expect(ids(filterFindings(findings, "manual", "breaking", ""))).toEqual(["3"]);
    expect(ids(filterFindings(findings, "all", "all", "SRC/"))).toEqual(["1", "2"]);
    expect(ids(filterFindings(findings, "all", "all", "implicitly"))).toEqual(["2"]);
    expect(ids(filterFindings(findings, "all", "all", "örtük"))).toEqual(["2"]);
    expect(ids(filterFindings(findings, "auto", "deprecated", ""))).toEqual([]);
  });
});

describe("helpers", () => {
  it("counts an incompatible root constraint as a blocker", () => {
    expect(dependencyBlockers(null)).toEqual({ packages: 0, root: false, total: 0 });
    expect(
      dependencyBlockers({ target: "8.4", rootConstraint: ">=7.1 <8.0", rootStatus: "incompatible", packages: [], hasLockFile: false }),
    ).toEqual({ packages: 0, root: true, total: 1 });
  });

  it("keeps the platform separator of the project root", () => {
    expect(joinPath("/Users/demo/site/", "src/a.php")).toBe("/Users/demo/site/src/a.php");
    expect(joinPath("C:\\xampp\\htdocs\\site", "src/a.php")).toBe("C:\\xampp\\htdocs\\site\\src\\a.php");
  });
});
