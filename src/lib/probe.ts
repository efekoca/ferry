import type { ExtensionUse } from "./types";
import { isVulnerable } from "./security";

export interface ProbeData {
  php: string;
  sapi: string;
  os: string;
  server: string | null;
  extensions: string[];
  ini: Record<string, string | false>;
}

export interface ProbeReport {
  php: string;
  belowTarget: boolean;
  vulnerable: boolean;
  sapi: string;
  fpm: boolean;
  server: string | null;
  missing: string[];
  disabled: string[];
  memoryLimit: string | null;
}

const MAX_PROBE = 512 * 1024;

const text = (v: unknown, max = 200): string | null => (typeof v === "string" && v.length <= max ? v : null);

export function parseProbe(input: string): ProbeData | null {
  if (input.length > MAX_PROBE) return null;
  const start = input.indexOf("{");
  const end = input.lastIndexOf("}");
  if (start < 0 || end < start) return null;
  let data: unknown;
  try {
    data = JSON.parse(input.slice(start, end + 1));
  } catch {
    return null;
  }
  if (!data || typeof data !== "object" || Array.isArray(data)) return null;
  const d = data as Record<string, unknown>;
  const php = text(d.php, 40);
  if (d.ferryProbe !== 1 || !php || !/^\d+\.\d+(\.\d+)?/.test(php)) return null;
  if (!Array.isArray(d.extensions) || d.extensions.length > 1000 || !d.extensions.every((e) => text(e, 100) !== null)) return null;
  const ini: Record<string, string | false> = {};
  if (d.ini && typeof d.ini === "object" && !Array.isArray(d.ini)) {
    for (const [key, value] of Object.entries(d.ini as Record<string, unknown>)) {
      if (value === false) ini[key] = false;
      else if (text(value, 4000) !== null) ini[key] = value as string;
    }
  }
  return {
    php,
    sapi: text(d.sapi) ?? "",
    os: text(d.os) ?? "",
    server: text(d.server),
    extensions: d.extensions as string[],
    ini,
  };
}

const minor = (v: string) => (v.match(/^(\d+)\.(\d+)/) ?? []).slice(1).map(Number);

export function compareProbe(probe: ProbeData, target: string, used: ExtensionUse[]): ProbeReport {
  const [pm, pn] = minor(probe.php);
  const [tm, tn] = minor(target);
  const normalize = (name: string) => name.trim().toLowerCase().replace(/[- ]/g, "_").replace(/^zend_/, "");
  const loaded = new Set(probe.extensions.map(normalize));
  const disabled = String(probe.ini.disable_functions || "")
    .split(",")
    .map((f) => f.trim())
    .filter(Boolean);
  return {
    php: probe.php,
    belowTarget: pm < tm || (pm === tm && pn < tn),
    vulnerable: isVulnerable(probe.php),
    sapi: probe.sapi,
    fpm: /fpm|cgi/i.test(probe.sapi),
    server: probe.server,
    missing: used.filter((e) => !e.dev && !loaded.has(normalize(e.name))).map((e) => e.name),
    disabled,
    memoryLimit: probe.ini.memory_limit || null,
  };
}
