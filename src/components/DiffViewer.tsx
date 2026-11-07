import { Fragment, type CSSProperties, type ReactNode } from "react";
import type { DiffLine, FileDiff } from "../lib/types";
import { useI18n } from "../i18n";
import { highlight } from "../lib/code";

export function wordDiff(a: string, b: string): [boolean[], boolean[]] {
  const x = a.match(TOKEN) ?? [];
  const y = b.match(TOKEN) ?? [];
  if (x.length * y.length > MAX_CELLS) return edges(a, b);
  const table = Array.from({ length: x.length + 1 }, () => new Uint16Array(y.length + 1));
  for (let i = x.length - 1; i >= 0; i--) {
    for (let j = y.length - 1; j >= 0; j--) {
      table[i][j] = x[i] === y[j] ? table[i + 1][j + 1] + 1 : Math.max(table[i + 1][j], table[i][j + 1]);
    }
  }
  const left = new Array<boolean>(a.length).fill(false);
  const right = new Array<boolean>(b.length).fill(false);
  let i = 0;
  let j = 0;
  let at = 0;
  let bt = 0;
  while (i < x.length || j < y.length) {
    if (i < x.length && j < y.length && x[i] === y[j]) {
      at += x[i++].length;
      bt += y[j++].length;
    } else if (j < y.length && (i === x.length || table[i][j + 1] >= table[i + 1][j])) {
      right.fill(true, bt, bt + y[j].length);
      bt += y[j++].length;
    } else {
      left.fill(true, at, at + x[i].length);
      at += x[i++].length;
    }
  }
  return [tidy(left, a), tidy(right, b)];
}

const TOKEN = /\s+|[A-Za-z0-9_$]+|./g;
const MAX_CELLS = 40000;

function edges(a: string, b: string): [boolean[], boolean[]] {
  let start = 0;
  while (start < a.length && start < b.length && a[start] === b[start]) start++;
  let end = 0;
  while (end < a.length - start && end < b.length - start && a[a.length - 1 - end] === b[b.length - 1 - end]) end++;
  const mark = (text: string) => Array.from(text, (_, i) => i >= start && i < text.length - end);
  return [mark(a), mark(b)];
}

const isSpace = (c: string) => /\s/.test(c);

function tidy(marked: boolean[], text: string): boolean[] {
  const out = [...marked];
  for (let i = 1; i < text.length - 1; i++) {
    if (out[i] || !isSpace(text[i])) continue;
    let before = i;
    while (before >= 0 && isSpace(text[before])) before--;
    let after = i;
    while (after < text.length && isSpace(text[after])) after++;
    if (before >= 0 && after < text.length && out[before] && out[after]) out[i] = true;
  }
  let i = 0;
  while (i < text.length) {
    if (!out[i]) {
      i++;
      continue;
    }
    let end = i;
    while (end < text.length && out[end]) end++;
    let first = i;
    while (first < end && isSpace(text[first])) out[first++] = false;
    let last = end - 1;
    while (last >= first && isSpace(text[last])) out[last--] = false;
    i = end;
  }
  return out;
}

function pairs(lines: DiffLine[]): Map<number, string> {
  const counterpart = new Map<number, string>();
  let i = 0;
  while (i < lines.length) {
    if (lines[i].kind !== "del") {
      i++;
      continue;
    }
    const delStart = i;
    while (i < lines.length && lines[i].kind === "del") i++;
    const addStart = i;
    while (i < lines.length && lines[i].kind === "add") i++;
    const n = Math.min(addStart - delStart, i - addStart);
    for (let k = 0; k < n; k++) {
      counterpart.set(delStart + k, lines[addStart + k].text);
      counterpart.set(addStart + k, lines[delStart + k].text);
    }
  }
  return counterpart;
}

function renderText(text: string, other: string | undefined, php: boolean): ReactNode {
  if (!text) return " ";
  const marked = other === undefined ? [] : other === text ? [] : wordDiff(text, other)[0];
  return highlight(text, marked, php);
}

function indentOf(text: string): number {
  let width = 0;
  for (const c of text) {
    if (c === " ") width += 1;
    else if (c === "\t") width += 4;
    else break;
  }
  return width;
}

const SIGN = { add: "+", del: "−", context: " " } as const;

export interface HunkChoice {
  isOn(hunk: number): boolean;
  toggle(hunk: number): void;
}

type SplitRow = { left?: DiffLine; right?: DiffLine; leftAt: number; rightAt: number };

export function splitRows(lines: DiffLine[]): SplitRow[] {
  const rows: SplitRow[] = [];
  let i = 0;
  while (i < lines.length) {
    if (lines[i].kind === "context") {
      rows.push({ left: lines[i], right: lines[i], leftAt: i, rightAt: i });
      i++;
      continue;
    }
    const start = i;
    while (i < lines.length && lines[i].kind === "del") i++;
    const middle = i;
    while (i < lines.length && lines[i].kind === "add") i++;
    for (let k = 0; k < Math.max(middle - start, i - middle); k++) {
      const leftAt = start + k;
      const rightAt = middle + k;
      rows.push({ left: leftAt < middle ? lines[leftAt] : undefined, right: rightAt < i ? lines[rightAt] : undefined, leftAt, rightAt });
    }
  }
  return rows;
}

function SplitDiff({ diff, php, choice }: { diff: FileDiff; php: boolean; choice?: HunkChoice }) {
  const { t } = useI18n();
  const side = (line: DiffLine | undefined, number: number | null | undefined, other: string | undefined) => (
    <>
      <td className={`num ${line?.kind ?? "gap"}`}>{number ?? ""}</td>
      <td className={`text ${line?.kind ?? "gap"}`} style={{ "--indent": line ? indentOf(line.text) : 0 } as CSSProperties}>
        {line ? renderText(line.text, other, php) : " "}
      </td>
    </>
  );
  return (
    <table className="diff__split">
      <colgroup>
        <col className="diff__num" />
        <col />
        <col className="diff__num" />
        <col />
      </colgroup>
      <tbody>
        {diff.hunks.map((hunk, h) => {
          const counterpart = pairs(hunk.lines);
          return (
            <Fragment key={h}>
              <tr className="diff__hunk">
                <td colSpan={4}>
                  {choice && (
                    <input type="checkbox" className="checkbox diff__choice" aria-label={t.changes.applyHunk(hunk.oldStart)} checked={choice.isOn(h)} onChange={() => choice.toggle(h)} />
                  )}
                  {t.changes.hunkLine(hunk.oldStart)}
                </td>
              </tr>
              {splitRows(hunk.lines).map((row, i) => (
                <tr key={i}>
                  {side(row.left, row.left?.oldNo, counterpart.get(row.leftAt))}
                  {side(row.right, row.right?.newNo, counterpart.get(row.rightAt))}
                </tr>
              ))}
            </Fragment>
          );
        })}
      </tbody>
    </table>
  );
}

export function DiffViewer({ diff, file, choice, bar, split = false }: { diff: FileDiff; file: string; choice?: HunkChoice; bar?: ReactNode; split?: boolean }) {
  const { t } = useI18n();
  const php = /\.(php|phtml|inc)$/i.test(file);
  if (split) {
    return (
      <div className="diff" role="region" aria-label={t.changes.diffLabel(file)}>
        {bar && <div className="diff__bar">{bar}</div>}
        <SplitDiff diff={diff} php={php} choice={choice} />
      </div>
    );
  }
  return (
    <div className="diff" role="region" aria-label={t.changes.diffLabel(file)}>
      {bar && <div className="diff__bar">{bar}</div>}
      <table>
        <colgroup>
          <col className="diff__num" />
          <col className="diff__num" />
          <col className="diff__sign" />
          <col />
        </colgroup>
        <tbody>
          {diff.hunks.map((hunk, h) => {
            const counterpart = pairs(hunk.lines);
            return (
              <Fragment key={h}>
                <tr className="diff__hunk">
                  <td colSpan={4}>
                    {choice && (
                      <input
                        type="checkbox"
                        className="checkbox diff__choice"
                        aria-label={t.changes.applyHunk(hunk.oldStart)}
                        checked={choice.isOn(h)}
                        onChange={() => choice.toggle(h)}
                      />
                    )}
                    {t.changes.hunkLine(hunk.oldStart)}
                  </td>
                </tr>
                {hunk.lines.map((line, i) => (
                  <tr key={i} className={line.kind} data-kind={line.kind}>
                    <td className="num">{line.oldNo ?? ""}</td>
                    <td className="num">{line.newNo ?? ""}</td>
                    <td className="sign" aria-hidden>
                      {SIGN[line.kind]}
                    </td>
                    <td className="text" style={{ "--indent": indentOf(line.text) } as CSSProperties}>
                      <span className="sr-only">
                        {line.kind === "add" ? t.changes.added : line.kind === "del" ? t.changes.removed : ""}
                      </span>
                      {renderText(line.text, counterpart.get(i), php)}
                    </td>
                  </tr>
                ))}
              </Fragment>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
