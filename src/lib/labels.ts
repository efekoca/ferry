import type { Dict } from "../i18n";
import type { DependencyReport } from "./types";

export const formatNumber = (n: number, locale: string) => new Intl.NumberFormat(locale).format(n);

export function formatDuration(ms: number, locale: string): string {
  if (ms < 1) return "< 1 ms";
  if (ms < 1000) return `${ms} ms`;
  const s = (ms / 1000).toLocaleString(locale, { maximumFractionDigits: 1 });
  return locale.startsWith("tr") ? `${s} sn` : `${s} s`;
}

export function formatBytes(bytes: number, locale: string): string {
  const units = ["byte", "kilobyte", "megabyte", "gigabyte"] as const;
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit++;
  }
  return new Intl.NumberFormat(locale, { style: "unit", unit: units[unit], unitDisplay: "short", maximumFractionDigits: unit ? 1 : 0 }).format(value);
}

export function formatRelative(ts: number, t: Dict, now = Date.now()): string {
  const min = Math.round(Math.max(0, now - ts) / 60000);
  if (min < 1) return t.time.justNow;
  const rtf = new Intl.RelativeTimeFormat(t.locale, { numeric: "auto" });
  if (min < 60) return rtf.format(-min, "minute");
  const h = Math.round(min / 60);
  if (h < 24) return rtf.format(-h, "hour");
  const d = Math.round(h / 24);
  if (d < 7) return rtf.format(-d, "day");
  return new Date(ts).toLocaleDateString(t.locale, { day: "numeric", month: "long", year: "numeric" });
}

export function scoreTone(score: number): "auto" | "manual" | "danger" {
  if (score >= 80) return "auto";
  if (score >= 50) return "manual";
  return "danger";
}

export function scoreHeadline(score: number, total: number, t: Dict, incomplete = false): string {
  if (total === 0) return incomplete ? t.overview.headline.incomplete : t.overview.headline.ready;
  if (score >= 80) return t.overview.headline.small;
  if (score >= 50) return t.overview.headline.medium;
  return t.overview.headline.large;
}

export const FRAMEWORK_LABEL = {
  wordpress: "WordPress",
  laravel: "Laravel",
  symfony: "Symfony",
  drupal: "Drupal",
  magento: "Magento",
  prestashop: "PrestaShop",
  joomla: "Joomla",
  moodle: "Moodle",
  mediawiki: "MediaWiki",
  phpbb: "phpBB",
  generic: "PHP",
} as const;

export const isMac = () =>
  typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);

export const modKey = () => (isMac() ? "⌘" : "Ctrl");

export function joinPath(root: string, rel: string): string {
  const sep = root.includes("\\") && !root.includes("/") ? "\\" : "/";
  return `${root.replace(/[\\/]+$/, "")}${sep}${rel.split("/").join(sep)}`;
}

export function dependencyBlockers(deps: DependencyReport | null) {
  const packages = deps?.packages.filter((p) => p.status === "incompatible").length ?? 0;
  const root = deps?.rootStatus === "incompatible";
  return { packages, root, total: packages + (root ? 1 : 0) };
}

export function commonTitle(titles: string[]): string {
  if (titles.every((t) => t === titles[0])) return titles[0];
  let prefix = titles[0];
  for (const t of titles) while (!t.startsWith(prefix)) prefix = prefix.slice(0, -1);
  return prefix ? `${prefix}*()` : titles[0];
}
