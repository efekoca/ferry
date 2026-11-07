import { useEffect, useState, type CSSProperties, type ReactNode } from "react";
import { Activity, FileCode2 } from "lucide-react";
import { Badge, RiskBadge, SeverityBadge, SeverityIcon } from "../components/ui";
import { useI18n } from "../i18n";
import { RULE_NOTES } from "../i18n/rules";
import type { Api } from "../lib/api";
import { highlight } from "../lib/code";
import type { Finding, SourceLine } from "../lib/types";

const VERSIONS = ["5.3", "5.4", "5.5", "5.6", "7.0", "7.1", "7.2", "7.3", "7.4", "8.0", "8.1", "8.2", "8.3", "8.4", "8.5"];

export function relatedFindings(selected: Finding, findings: readonly Finding[], limit = 4): Finding[] {
  const why = RULE_NOTES[selected.ruleId]?.why;
  const same = (f: Finding) => f.ruleId === selected.ruleId || (!!why && RULE_NOTES[f.ruleId]?.why === why);
  const others = findings.filter((f) => f.id !== selected.id && same(f));
  const nearby = others.filter((f) => f.file === selected.file);
  return [...nearby, ...others.filter((f) => f.file !== selected.file)].slice(0, limit);
}

const looksLikeCode = (text: string) => /[()$[\]=>:]/.test(text) || /^\S+$/.test(text);

function CodeContext({ finding, root, api }: { finding: Finding; root?: string; api: Api | null }) {
  const [loaded, setLoaded] = useState<{ id: string; lines: SourceLine[] } | null>(null);
  useEffect(() => {
    if (!api || !root) return;
    let live = true;
    api
      .sourceLines(root, finding.file, finding.line)
      .then((lines) => live && setLoaded({ id: finding.id, lines }))
      .catch(() => {});
    return () => {
      live = false;
    };
  }, [api, root, finding.id, finding.file, finding.line]);
  const fresh = loaded?.id === finding.id && loaded.lines.some((l) => l.number === finding.line) ? loaded.lines : null;
  const lines = fresh ?? [{ number: finding.line, text: finding.snippet }];
  const php = /\.(php|phtml|inc)$/i.test(finding.file);
  const cut = finding.file.lastIndexOf("/") + 1;
  return (
    <figure className="code-view">
      <figcaption className="code-view__bar">
        <FileCode2 size={14} aria-hidden />
        <span className="code-view__path selectable">
          {finding.file.slice(0, cut)}
          <b>{finding.file.slice(cut)}</b>:{finding.line}
        </span>
      </figcaption>
      <div className="code-view__body selectable">
        {lines.map((line) => (
          <div key={line.number} className={`code-line ${line.number === finding.line ? "code-line--hit" : ""}`}>
            <span className="code-line__number">{line.number}</span>
            <span className="code-line__text" style={{ "--indent": line.text.length - line.text.trimStart().length } as CSSProperties}>
              {line.text ? highlight(line.text, [], php) : " "}
            </span>
          </div>
        ))}
      </div>
    </figure>
  );
}

function VersionPath({ finding, current, target }: { finding: Finding; current: string | null; target: string }) {
  const { t } = useI18n();
  const deprecated = RULE_NOTES[finding.ruleId]?.deprecated;
  const rank = (v: string | null | undefined) => (v ? VERSIONS.indexOf(v) : -1);
  const marks = [deprecated, finding.since, current].map(rank).filter((r) => r >= 0);
  const from = Math.min(...marks);
  const to = rank(target);
  if (rank(finding.since) < 0 || to < 0 || from >= to) return null;
  const at = (v: string) => Math.min(100, ((rank(v) - from) / (to - from)) * 100);
  const since = at(finding.since);
  const breaking = finding.severity === "breaking";
  const stage = breaking ? t.findings.lifeRemoved : finding.severity === "deprecated" ? t.findings.lifeDeprecated : t.findings.lifeChanged;
  return (
    <section className="detail__block">
      <h3>{t.findings.path}</h3>
      <div className="life">
        <div className="life__top">
          {deprecated && breaking && (
            <span>
              <b>{deprecated}</b> {t.findings.lifeDeprecated}
            </span>
          )}
          <span>
            <b>{finding.since}</b> {stage}
          </span>
        </div>
        <div className="life__rail" aria-hidden>
          {deprecated && breaking && <i className="life__part life__part--deprecated" style={{ left: `${at(deprecated)}%`, width: `${since - at(deprecated)}%` }} />}
          <i className={`life__part life__part--${finding.severity}`} style={{ left: `${since}%`, right: 0 }} />
          {current && rank(current) >= 0 && <i className="life__pin" style={{ left: `${at(current)}%` }} />}
          <i className="life__pin" style={{ left: "100%" }} />
        </div>
        <div className="life__bottom">
          {current && (
            <span>
              <b>{current}</b> {t.findings.lifeProject}
            </span>
          )}
          <span className="life__end">
            <b>{target}</b> {t.findings.lifeTarget}
          </span>
        </div>
      </div>
    </section>
  );
}

export function FindingDetail({
  finding,
  findings,
  current,
  target,
  root,
  api,
  notice,
  footer,
  onSelect,
}: {
  finding: Finding;
  findings: readonly Finding[];
  current: string | null;
  target: string;
  root?: string;
  api: Api | null;
  notice?: ReactNode;
  footer: ReactNode;
  onSelect: (id: string) => void;
}) {
  const { t, loc, lang } = useI18n();
  const note = RULE_NOTES[finding.ruleId];
  const related = relatedFindings(finding, findings);
  const replacement = finding.replacement ? loc(finding.replacement) : null;
  return (
    <div className="detail" key={finding.id}>
      <div className="detail__scroll">
        <header className="detail__head">
          <div className="row detail__badges">
            <SeverityBadge severity={finding.severity} />
            <RiskBadge fix={finding.fix} />
            <Badge mono>PHP {finding.since}</Badge>
          </div>
          <h2 className="selectable">{loc(finding.title)}</h2>
          <p className="selectable">{loc(finding.message)}</p>
        </header>

        <CodeContext finding={finding} root={root} api={api} />

        {notice}

        <section className="detail__block">
          <h3>{t.findings.howToFix}</h3>
          {replacement && (looksLikeCode(replacement) ? <pre className="code code--suggest">{replacement}</pre> : <p className="selectable">{replacement}</p>)}
          <p>{t.fixDesc[finding.fix]}</p>
        </section>

        <section className="detail__block">
          <h3>{t.findings.whatHappens}</h3>
          <p>{t.findings.effect[finding.severity]}</p>
          {finding.runtimeHits > 0 && !finding.ruleId.startsWith("runtime.") && (
            <p className="detail__runtime">
              <Activity size={13} aria-hidden /> {t.findings.runtimeDetail(finding.runtimeHits)}
            </p>
          )}
        </section>

        {note && (
          <section className="detail__block">
            <h3>{t.findings.whyChanged}</h3>
            <p className="selectable">{note.why[lang]}</p>
          </section>
        )}

        <VersionPath finding={finding} current={current} target={target} />

        {related.length > 0 && (
          <section className="detail__block">
            <h3>{t.findings.related}</h3>
            <div className="detail__related">
              {related.map((f) => (
                <button key={f.id} type="button" onClick={() => onSelect(f.id)}>
                  <SeverityIcon severity={f.severity} />
                  <span className="detail__related-title">{loc(f.title)}</span>
                  <span className="detail__related-at">
                    {f.file === finding.file ? "" : f.file.slice(f.file.lastIndexOf("/") + 1)}:{f.line}
                  </span>
                </button>
              ))}
            </div>
          </section>
        )}

        <div className="detail__rule selectable">{finding.ruleId}</div>
      </div>
      <div className="detail__foot">{footer}</div>
    </div>
  );
}
