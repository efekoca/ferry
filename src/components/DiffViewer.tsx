import { Fragment, type ReactNode } from "react";
import type { DiffLine, FileDiff } from "../lib/types";
import { useI18n } from "../i18n";

export function wordHighlight(text: string, other: string): [string, string, string] {
  let start = 0;
  while (start < text.length && start < other.length && text[start] === other[start]) start++;
  let end = 0;
  while (
    end < text.length - start &&
    end < other.length - start &&
    text[text.length - 1 - end] === other[other.length - 1 - end]
  ) {
    end++;
  }
  return [text.slice(0, start), text.slice(start, text.length - end), text.slice(text.length - end)];
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

function renderText(text: string, other: string | undefined): ReactNode {
  if (other === undefined) return text || " ";
  const [pre, mid, post] = wordHighlight(text, other);
  if (!mid) return text || " ";
  return (
    <>
      {pre}
      <mark>{mid}</mark>
      {post}
    </>
  );
}

const SIGN = { add: "+", del: "−", context: " " } as const;

export interface HunkChoice {
  isOn(hunk: number): boolean;
  toggle(hunk: number): void;
}

export function DiffViewer({ diff, file, choice }: { diff: FileDiff; file: string; choice?: HunkChoice }) {
  const { t } = useI18n();
  return (
    <div className="diff" role="region" aria-label={t.changes.diffLabel(file)}>
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
                    @@ −{hunk.oldStart} +{hunk.newStart} @@
                  </td>
                </tr>
                {hunk.lines.map((line, i) => (
                  <tr key={i} className={line.kind} data-kind={line.kind}>
                    <td className="num">{line.oldNo ?? ""}</td>
                    <td className="num">{line.newNo ?? ""}</td>
                    <td className="sign" aria-hidden>
                      {SIGN[line.kind]}
                    </td>
                    <td className="text">
                      <span className="sr-only">
                        {line.kind === "add" ? t.changes.added : line.kind === "del" ? t.changes.removed : ""}
                      </span>
                      {renderText(line.text, counterpart.get(i))}
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
