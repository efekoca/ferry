import type { DockerEvent, FileChange, Finding, FixOption } from "./types";

/** The option a finding uses: the one chosen for it, else the recommended one. */
export function optionFor(finding: Finding, choices: Readonly<Record<string, string>>): FixOption | undefined {
  return finding.options?.find((o) => o.id === choices[finding.id]) ?? finding.options?.find((o) => o.recommended);
}

/** The finding as its chosen option leaves it: the fix kind becomes that option's. */
export function withChoice(finding: Finding, choices: Readonly<Record<string, string>>): Finding {
  const option = optionFor(finding, choices);
  return option && option.fix !== finding.fix ? { ...finding, fix: option.fix } : finding;
}

export const containerPath = (path: string) => path.replace(/^\/app\//, "");

export const isPin = (change: FileChange) => change.rules.some((r) => r.name.startsWith("server."));

export function touches(event: DockerEvent, change: FileChange) {
  if (containerPath(event.path) !== change.file) return false;
  return change.diff.hunks.some((hunk) => {
    const lines = hunk.lines.flatMap((l) => (l.oldNo === null ? [] : [l.oldNo]));
    return lines.length > 0 && event.line >= Math.min(...lines) && event.line <= Math.max(...lines);
  });
}
