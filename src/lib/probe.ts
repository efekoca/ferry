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

export function parseProbe(text: string): ProbeData | null {
  const start = text.indexOf("{");
  const end = text.lastIndexOf("}");
  if (start < 0 || end < start) return null;
  try {
    const data = JSON.parse(text.slice(start, end + 1));
    if (data?.ferryProbe !== 1 || typeof data.php !== "string" || !Array.isArray(data.extensions)) return null;
    return { php: data.php, sapi: String(data.sapi ?? ""), os: String(data.os ?? ""), server: data.server ?? null, extensions: data.extensions.map(String), ini: data.ini ?? {} };
  } catch {
    return null;
  }
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
