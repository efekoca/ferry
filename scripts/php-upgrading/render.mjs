// Renders docs/coverage.json into docs/COVERAGE.md:
// node scripts/php-upgrading/render.mjs
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const items = JSON.parse(readFileSync(join(root, "docs", "coverage.json"), "utf8"));
const STATUSES = ["covered", "partial", "gap", "runtime", "n/a"];
const LABEL = { covered: "Covered", partial: "Partial", gap: "Gap", runtime: "Runtime only", "n/a": "Not applicable" };
const versions = [...new Set(items.map((i) => i.version))];
const cell = (s) => s.replace(/\|/g, "\\|").replace(/</g, "&lt;");
const count = (list, status) => list.filter((i) => i.status === status).length;

const out = [
  "# PHP upgrade coverage",
  "",
  "Every backward incompatible change, deprecation, removed extension and removed ini setting listed in php-src's",
  "`UPGRADING` files from PHP 7.0 to 8.5, and what Ferry does about it. Each covered item was checked by scanning a",
  "minimal snippet with `ferry-cli scan --target 8.5`.",
  "",
  "- **Covered**: Ferry reports code affected by the change.",
  "- **Partial**: some forms are reported, others that could be found statically are not.",
  "- **Gap**: could be found from the source but isn't reported yet.",
  "- **Runtime only**: depends on values or types that are only known when the code runs.",
  "- **Not applicable**: doesn't affect application code (C API, build options, SAPIs, internals).",
  "",
  "The data lives in `docs/coverage.json`; this page is generated from it with",
  "`node scripts/php-upgrading/render.mjs`.",
  "",
  `| Version | ${STATUSES.map((s) => LABEL[s]).join(" | ")} | Total |`,
  `| --- | ${STATUSES.map(() => "---:").join(" | ")} | ---: |`,
  ...versions.map((v) => {
    const list = items.filter((i) => i.version === v);
    return `| [${v}](#php-${v.replace(".", "")}) | ${STATUSES.map((s) => count(list, s)).join(" | ")} | ${list.length} |`;
  }),
  `| **All** | ${STATUSES.map((s) => `**${count(items, s)}**`).join(" | ")} | **${items.length}** |`,
  "",
];

for (const v of versions) {
  out.push(`## PHP ${v}`, "", `Source: [UPGRADING](https://github.com/php/php-src/blob/PHP-${v}/UPGRADING)`, "");
  out.push("| Area | Change | Status | Rules | Note |", "| --- | --- | --- | --- | --- |");
  for (const i of items.filter((i) => i.version === v)) {
    const rules = (i.rules ?? []).map((r) => `\`${r}\``).join("<br>");
    out.push(`| ${cell(i.ext ?? i.section)} | ${cell(i.change)} | ${LABEL[i.status]} | ${rules} | ${cell(i.note)} |`);
  }
  out.push("");
}

writeFileSync(join(root, "docs", "COVERAGE.md"), out.join("\n"));
