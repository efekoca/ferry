import type { DockerEvent, FileChange } from "./types";

export const containerPath = (path: string) => path.replace(/^\/app\//, "");

export const isPin = (change: FileChange) => change.rules.some((r) => r.name.startsWith("server."));

export function touches(event: DockerEvent, change: FileChange) {
  if (containerPath(event.path) !== change.file) return false;
  return change.diff.hunks.some((hunk) => {
    const lines = hunk.lines.flatMap((l) => (l.oldNo === null ? [] : [l.oldNo]));
    return lines.length > 0 && event.line >= Math.min(...lines) && event.line <= Math.max(...lines);
  });
}
