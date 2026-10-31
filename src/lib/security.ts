import type { Target } from "./types";

export const PHP2SHELL_CVES = "CVE-2026-109618, CVE-2026-109619";

export const PATCHED: Readonly<Record<string, string>> = { "8.2": "8.2.35", "8.3": "8.3.36", "8.4": "8.4.27", "8.5": "8.5.12" };

export const BLOCKED_TARGETS: readonly Target[] = ["8.1"];

export const isBlockedTarget = (target: string) => BLOCKED_TARGETS.includes(target as Target);

const parts = (version: string) => (version.match(/^(\d+)\.(\d+)(?:\.(\d+))?/) ?? []).slice(1).map((n) => Number(n ?? 0));

export function isVulnerable(version: string): boolean {
  const [major, minor, patch] = parts(version);
  if (major === undefined || minor === undefined) return false;
  const branch = `${major}.${minor}`;
  const fixed = PATCHED[branch];
  if (fixed) return patch < parts(fixed)[2];
  return (major === 5 && minor >= 4) || major === 7 || (major === 8 && minor <= 1);
}

export function patchedFor(version: string): string {
  const [major, minor] = parts(version);
  return PATCHED[`${major}.${minor}`] ?? PATCHED["8.4"];
}
