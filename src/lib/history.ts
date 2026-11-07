import type { Dict } from "../i18n";
import type { CheckSummary, DockerSummary, HistoryEntry, Snapshot } from "./types";

export type HistoryRange = "7" | "30" | "all";

type EventBody =
  | { kind: "scan"; at: number; entry: HistoryEntry }
  | { kind: "fix"; at: number; snapshot: Snapshot }
  | { kind: "real"; at: number; run: DockerSummary }
  | { kind: "check"; at: number; run: CheckSummary };

export type HistoryEvent = EventBody & { key: string };

const DAY = 86_400_000;

export const rangeStart = (range: HistoryRange, now = Date.now()) => (range === "all" ? 0 : now - Number(range) * DAY);

export function historyEvents(scans: HistoryEntry[], snapshots: Snapshot[], real: DockerSummary[], checks: CheckSummary[]): HistoryEvent[] {
  const events: EventBody[] = [
    ...scans.map((entry) => ({ kind: "scan" as const, at: entry.at * 1000, entry })),
    ...snapshots.map((snapshot) => ({ kind: "fix" as const, at: snapshot.createdAt, snapshot })),
    ...real.map((run) => ({ kind: "real" as const, at: run.at * 1000, run })),
    ...checks.map((run) => ({ kind: "check" as const, at: run.at * 1000, run })),
  ];
  const seen = new Map<string, number>();
  return events
    .sort((a, b) => b.at - a.at)
    .map((event) => {
      const base = `${event.kind}-${event.at}`;
      const repeat = seen.get(base) ?? 0;
      seen.set(base, repeat + 1);
      return { ...event, key: repeat ? `${base}-${repeat}` : base };
    });
}

export function smoothPath(xs: number[], ys: number[]) {
  const n = xs.length;
  if (n === 0) return "";
  if (n === 1) return `M${xs[0]} ${ys[0]}`;
  const slopes = ys.slice(0, -1).map((y, i) => (ys[i + 1] - y) / (xs[i + 1] - xs[i]));
  const tangents = ys.map((_, i) => {
    if (i === 0) return slopes[0];
    if (i === n - 1) return slopes[n - 2];
    const [a, b] = [slopes[i - 1], slopes[i]];
    return a * b <= 0 ? 0 : (2 * a * b) / (a + b);
  });
  const round = (v: number) => Number(v.toFixed(1));
  let path = `M${round(xs[0])} ${round(ys[0])}`;
  for (let i = 0; i < n - 1; i++) {
    const third = (xs[i + 1] - xs[i]) / 3;
    path += ` C${round(xs[i] + third)} ${round(ys[i] + tangents[i] * third)} ${round(xs[i + 1] - third)} ${round(ys[i + 1] - tangents[i + 1] * third)} ${round(xs[i + 1])} ${round(ys[i + 1])}`;
  }
  return path;
}

export function niceMax(value: number) {
  const scale = 10 ** Math.max(0, Math.floor(Math.log10(Math.max(value, 1))) - 1);
  return Math.ceil(value / (2 * scale)) * 2 * scale || 2;
}

export function columnOf(at: number, scans: HistoryEntry[]) {
  const last = scans.length - 1;
  if (at < scans[0].at * 1000) return -0.3;
  for (let i = 0; i < last; i++) {
    const [from, to] = [scans[i].at * 1000, scans[i + 1].at * 1000];
    if (at < to) return i + (at - from) / (to - from);
  }
  return last + 0.3;
}

export function spreadOut(positions: number[], gap: number, max = 100) {
  const order = positions.map((_, i) => i).sort((a, b) => positions[a] - positions[b]);
  const placed = [...positions];
  order.forEach((index, i) => {
    if (i > 0) placed[index] = Math.max(placed[index], placed[order[i - 1]] + gap);
  });
  for (let i = order.length - 1; i >= 0; i--) {
    const limit = i === order.length - 1 ? max : placed[order[i + 1]] - gap;
    placed[order[i]] = Math.min(placed[order[i]], limit);
  }
  return placed;
}

const sameDay = (a: Date, b: Date) => a.toDateString() === b.toDateString();

export function dayName(at: number, t: Dict, month: "long" | "short" = "long", now = Date.now()) {
  const day = new Date(at);
  if (sameDay(day, new Date(now))) return t.history.today;
  if (sameDay(day, new Date(now - DAY))) return t.history.yesterday;
  return day.toLocaleDateString(t.locale, { day: "numeric", month });
}

export const clock = (at: number, t: Dict) => new Date(at).toLocaleTimeString(t.locale, { hour: "2-digit", minute: "2-digit" });

export const moment = (at: number, t: Dict) => `${dayName(at, t)} ${clock(at, t)}`;

export function dayLabels(scans: HistoryEntry[], t: Dict, room = 8) {
  const every = Math.ceil(scans.length / room);
  let shown = -every;
  let day = "";
  return scans.map((scan, i) => {
    const key = new Date(scan.at * 1000).toDateString();
    if (key === day || i - shown < every) return "";
    day = key;
    shown = i;
    return dayName(scan.at * 1000, t, "short");
  });
}
