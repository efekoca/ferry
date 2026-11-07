import type { CSSProperties, ReactNode } from "react";
import { Tooltip } from "../components/ui";
import { useI18n } from "../i18n";
import { columnOf, dayLabels, moment, niceMax, smoothPath, spreadOut } from "../lib/history";
import type { HistoryEntry } from "../lib/types";

const WIDTH = 800;
const LINE_HEIGHT = 176;
const BAR_HEIGHT = 84;

export interface ChartMark {
  at: number;
  tone: "good" | "bad";
  icon: ReactNode;
  text: string;
}

function Grid({ ticks, height }: { ticks: number[]; height: number }) {
  const top = ticks[ticks.length - 1];
  return ticks.map((tick) => {
    const y = (1 - tick / top) * height;
    return (
      <span key={tick}>
        <span className="plot__grid" style={{ top: y }} />
        <span className="plot__y" style={{ top: y }}>
          {tick}
        </span>
      </span>
    );
  });
}

function Bar({ scan, scale }: { scan: HistoryEntry; scale: number }) {
  const parts: [string, number][] = scan.counts
    ? [
        ["behavior", scan.counts.behavior],
        ["deprecated", scan.counts.deprecated],
        ["breaking", scan.counts.breaking],
      ]
    : [["unknown", scan.total]];
  return (
    <span className="plot__bar">
      {parts
        .filter(([, count]) => count > 0)
        .map(([kind, count]) => (
          <i key={kind} className={`plot__seg plot__seg--${kind}`} style={{ height: Math.max(2, count * scale) }} />
        ))}
    </span>
  );
}

function ScanTip({ scan, flip }: { scan: HistoryEntry; flip: boolean }) {
  const { t } = useI18n();
  const rows: [string, number | undefined, string][] = [
    [t.history.score, scan.score, "score"],
    [t.history.findings, scan.total, "total"],
    [t.severity.breaking, scan.counts?.breaking, "breaking"],
    [t.severity.deprecated, scan.counts?.deprecated, "deprecated"],
    [t.severity.behavior, scan.counts?.behavior, "behavior"],
  ];
  return (
    <span className={`plot__tip${flip ? " plot__tip--left" : ""}`}>
      <span className="plot__when">{moment(scan.at * 1000, t)}</span>
      <span className="plot__read">
        {rows
          .filter(([, value]) => value !== undefined)
          .map(([label, value, kind]) => (
            <span key={kind} className="plot__row">
              <i className={`plot__key plot__key--${kind}`} />
              <b>{value}</b>
              <span>{label}</span>
            </span>
          ))}
      </span>
    </span>
  );
}

export function HistoryChart({ scans, marks }: { scans: HistoryEntry[]; marks: ChartMark[] }) {
  const { t } = useI18n();
  const n = scans.length;
  const xs = scans.map((_, i) => ((i + 0.5) / n) * WIDTH);
  const ys = scans.map((s) => (1 - s.score / 100) * LINE_HEIGHT);
  const line = smoothPath(xs, ys);
  const top = niceMax(Math.max(...scans.map((s) => s.total)));
  const lefts = spreadOut(
    marks.map((mark) => ((columnOf(mark.at, scans) + 0.5) / n) * 100),
    3,
  );
  const labels = dayLabels(scans, t);
  return (
    <div className="plot" style={{ "--n": n } as CSSProperties} role="img" aria-label={t.progress.trend(scans.map((s) => s.score).join(", "))}>
      <div className="plot__panel plot__panel--line">
        <Grid ticks={[0, 50, 100]} height={LINE_HEIGHT} />
        <svg className="plot__svg" viewBox={`0 0 ${WIDTH} ${LINE_HEIGHT}`} preserveAspectRatio="none" aria-hidden>
          <path className="plot__area" d={`${line} L${xs[n - 1]} ${LINE_HEIGHT} L${xs[0]} ${LINE_HEIGHT} Z`} />
          <path className="plot__line" d={line} />
        </svg>
      </div>
      <div className="plot__lane" aria-hidden>
        {marks.map((mark, i) => (
          <span key={i} className="plot__pin-at" style={{ left: `${lefts[i]}%` }}>
            <Tooltip className={`plot__pin plot__pin--${mark.tone}`} text={mark.text} align={lefts[i] > 80 ? "end" : "center"}>
              {mark.icon}
            </Tooltip>
          </span>
        ))}
      </div>
      <div className="plot__panel plot__panel--bars">
        <Grid ticks={[0, top / 2, top]} height={BAR_HEIGHT} />
      </div>
      <div className="plot__hits" aria-hidden>
        {scans.map((scan, i) => (
          <div key={i} className="plot__hit">
            <span className="plot__dot" style={{ top: ys[i] }} />
            {(i === 0 || i === n - 1) && (
              <span className="plot__value" style={{ top: ys[i] }}>
                {scan.score}
              </span>
            )}
            <Bar scan={scan} scale={BAR_HEIGHT / top} />
            <ScanTip scan={scan} flip={i >= n / 2} />
          </div>
        ))}
      </div>
      <div className="plot__x" aria-hidden>
        {labels.map((label, i) => (
          <span key={i}>{label}</span>
        ))}
      </div>
    </div>
  );
}

export function HistoryTable({ scans }: { scans: HistoryEntry[] }) {
  const { t } = useI18n();
  const number = (value: number | undefined) => value ?? "–";
  return (
    <div className="plot-table">
      <table className="table">
        <thead>
          <tr>
            <th scope="col">{t.history.scan}</th>
            <th scope="col">{t.history.score}</th>
            <th scope="col">{t.history.findings}</th>
            <th scope="col">{t.severity.breaking}</th>
            <th scope="col">{t.severity.deprecated}</th>
            <th scope="col">{t.severity.behavior}</th>
          </tr>
        </thead>
        <tbody>
          {[...scans].reverse().map((scan, i) => (
            <tr key={i}>
              <td>{moment(scan.at * 1000, t)}</td>
              <td className="tabular">{scan.score}</td>
              <td className="tabular">{scan.total}</td>
              <td className="tabular">{number(scan.counts?.breaking)}</td>
              <td className="tabular">{number(scan.counts?.deprecated)}</td>
              <td className="tabular">{number(scan.counts?.behavior)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
