import type { ReactNode } from "react";

const KEYWORDS = new Set(
  "abstract and array as break case catch class clone const continue declare default do echo else elseif empty enum extends false final finally fn for foreach function global goto if implements include include_once instanceof insteadof interface isset list match namespace new null or parent print private protected public readonly require require_once return self static switch throw trait true try unset use var while xor yield".split(
    " ",
  ),
);

const PART = /(\/\/.*$|#(?!\[).*$|\/\*.*?\*\/)|('(?:[^'\\]|\\.)*'|"(?:[^"\\]|\\.)*")|(\$[A-Za-z_][A-Za-z0-9_]*)|(\b\d+(?:\.\d+)?\b)|([A-Za-z_][A-Za-z0-9_]*)/g;

const MAX_LINE = 2000;

function classes(code: string): string[] {
  const out = new Array<string>(code.length).fill("");
  for (const m of code.matchAll(PART)) {
    let kind = "";
    if (m[1]) kind = "tok-comment";
    else if (m[2]) kind = "tok-string";
    else if (m[3]) kind = "tok-variable";
    else if (m[4]) kind = "tok-number";
    else if (KEYWORDS.has(m[5].toLowerCase())) kind = "tok-keyword";
    else if (code[m.index + m[0].length] === "(") kind = "tok-function";
    out.fill(kind, m.index, m.index + m[0].length);
  }
  return out;
}

export function highlight(code: string, marked: boolean[] = [], php = true): ReactNode {
  const kinds = php && code.length <= MAX_LINE ? classes(code) : new Array<string>(code.length).fill("");
  const nodes: ReactNode[] = [];
  let i = 0;
  while (i < code.length) {
    const inMark = !!marked[i];
    let end = i + 1;
    while (end < code.length && !!marked[end] === inMark) end++;
    const inner: ReactNode[] = [];
    let j = i;
    while (j < end) {
      let k = j + 1;
      while (k < end && kinds[k] === kinds[j]) k++;
      const text = code.slice(j, k);
      inner.push(kinds[j] ? <span key={j} className={kinds[j]}>{text}</span> : text);
      j = k;
    }
    nodes.push(inMark ? <mark key={i}>{inner}</mark> : <span key={i}>{inner}</span>);
    i = end;
  }
  return nodes;
}
