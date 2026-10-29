import { useEffect, useMemo, useState, type ReactNode } from "react";
import { ArrowRight, CircleCheck, ClipboardList, FileCode2, FolderSearch, Info, Lightbulb, Search, Sparkles } from "lucide-react";
import { PageHeader } from "../components/Shell";
import { useToast } from "../components/Toast";
import { Badge, Button, Callout, EmptyState, RiskBadge, SeverityBadge, SeverityIcon } from "../components/ui";
import { useI18n } from "../i18n";
import { errorMessage } from "../lib/api";
import { joinPath } from "../lib/labels";
import { useStore } from "../lib/store";
import type { Finding, FixKind, Severity } from "../lib/types";

type FixFilter = FixKind | "all";
type SeverityFilter = Severity | "all";

const FIX_TONE = { auto: "success", likely: "info", manual: "warn" } as const;
const FIX_ICON: Record<FixKind, ReactNode> = {
  auto: <Sparkles size={15} />,
  likely: <Lightbulb size={15} />,
  manual: <Info size={15} />,
};

export function filterFindings(findings: Finding[], fix: FixFilter, severity: SeverityFilter, query: string): Finding[] {
  const q = query.trim().toLowerCase();
  const hit = (s: string) => s.toLowerCase().includes(q);
  return findings.filter(
    (f) =>
      (fix === "all" || f.fix === fix) &&
      (severity === "all" || f.severity === severity) &&
      (!q || hit(f.file) || hit(f.title.tr) || hit(f.title.en) || hit(f.snippet)),
  );
}

export function Findings() {
  const { t, loc } = useI18n();
  const { scan, setView, project, api, runScan, scanning } = useStore();
  const [fix, setFix] = useState<FixFilter>("all");
  const [severity, setSeverity] = useState<SeverityFilter>("all");
  const [query, setQuery] = useState("");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const toast = useToast();

  const findings = useMemo(() => scan?.findings ?? [], [scan]);
  const visible = useMemo(() => filterFindings(findings, fix, severity, query), [findings, fix, severity, query]);
  const groups = useMemo(() => {
    const map = new Map<string, Finding[]>();
    for (const f of visible) map.set(f.file, [...(map.get(f.file) ?? []), f]);
    return [...map.entries()];
  }, [visible]);
  const selected = visible.find((f) => f.id === selectedId) ?? visible[0] ?? null;

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.key === "ArrowDown" || e.key === "ArrowUp") || (e.target as HTMLElement)?.tagName === "INPUT") return;
      if (!visible.length) return;
      e.preventDefault();
      const i = selected ? visible.indexOf(selected) : -1;
      const next = visible[Math.max(0, Math.min(visible.length - 1, i + (e.key === "ArrowDown" ? 1 : -1)))];
      setSelectedId(next.id);
      document.getElementById(`finding-${next.id}`)?.scrollIntoView({ block: "nearest" });
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [visible, selected]);

  if (!scan) {
    return (
      <>
        <PageHeader title={t.findings.title} />
        <EmptyState
          icon={<ClipboardList size={24} />}
          title={t.findings.noScan}
          action={
            <Button variant="primary" onClick={runScan} loading={scanning}>
              {t.overview.startScan}
            </Button>
          }
        >
          {t.findings.noScanText}
        </EmptyState>
      </>
    );
  }

  const countFix = (k: FixKind) => findings.filter((f) => f.fix === k).length;
  const countSev = (k: Severity) => findings.filter((f) => f.severity === k).length;

  return (
    <>
      <PageHeader title={t.findings.title} subtitle={t.findings.subtitle(scan.target, findings.length)} />
      {findings.length === 0 ? (
        <EmptyState icon={<CircleCheck size={24} />} title={t.findings.clean}>
          {t.findings.cleanText(scan.target)}
        </EmptyState>
      ) : (
        <div className="split">
          <div className="pane">
            <div className="toolbar">
              <div className="chip-group" role="group" aria-label={t.findings.fixType}>
                <Chip pressed={fix === "all"} onClick={() => setFix("all")} label={t.common.all} count={findings.length} />
                {(["auto", "likely", "manual"] as const).map((k) => (
                  <Chip key={k} pressed={fix === k} onClick={() => setFix(fix === k ? "all" : k)} label={t.fix[k]} count={countFix(k)} />
                ))}
              </div>
              <div className="chip-group" role="group" aria-label={t.findings.severity} style={{ marginLeft: 8 }}>
                {(["breaking", "deprecated", "behavior"] as const).map((k) => (
                  <Chip key={k} pressed={severity === k} onClick={() => setSeverity(severity === k ? "all" : k)} label={t.severity[k]} count={countSev(k)} />
                ))}
              </div>
              <div className="search" style={{ marginLeft: "auto" }}>
                <Search size={14} />
                <input
                  className="input"
                  placeholder={t.findings.search}
                  aria-label={t.findings.searchLabel}
                  value={query}
                  onChange={(e) => setQuery(e.target.value)}
                />
              </div>
            </div>

            {visible.length === 0 ? (
              <EmptyState icon={<FolderSearch size={24} />} title={t.findings.noMatch}>
                {t.findings.noMatchText}
              </EmptyState>
            ) : (
              <div role="listbox" aria-label={t.findings.list} style={{ paddingBottom: "var(--space-6)" }}>
                {groups.map(([file, items]) => (
                  <section key={file} className="file-group" role="group" aria-label={file}>
                    <header className="file-group__header">
                      <FileCode2 size={14} />
                      <span className="file-group__name" title={file}>
                        {file}
                      </span>
                      <span>{items.length}</span>
                    </header>
                    {items.map((f) => (
                      <button
                        key={f.id}
                        id={`finding-${f.id}`}
                        role="option"
                        aria-selected={selected?.id === f.id}
                        className="finding"
                        onClick={() => setSelectedId(f.id)}
                      >
                        <span className="finding__line">
                          <SeverityIcon severity={f.severity} />
                          {f.line}
                        </span>
                        <span className="finding__main">
                          <div className="finding__title">{loc(f.title)}</div>
                          <div className="finding__snippet">{f.snippet}</div>
                        </span>
                        <span className="finding__badges">
                          <RiskBadge fix={f.fix} />
                        </span>
                      </button>
                    ))}
                  </section>
                ))}
              </div>
            )}
          </div>

          <aside className="pane pane--side" aria-label={t.findings.details}>
            {selected && (
              <div className="detail" key={selected.id}>
                <div>
                  <div className="row" style={{ marginBottom: 8 }}>
                    <SeverityBadge severity={selected.severity} />
                    <RiskBadge fix={selected.fix} />
                    <Badge mono>PHP {selected.since}</Badge>
                  </div>
                  <h2 className="selectable">{loc(selected.title)}</h2>
                  <div className="detail__location selectable">
                    {selected.file}:{selected.line}:{selected.column}
                  </div>
                </div>

                <div className="detail__section">
                  <h3>{t.findings.code}</h3>
                  <pre className="code">{selected.snippet}</pre>
                </div>

                <div className="detail__section">
                  <h3>{t.findings.why}</h3>
                  <p className="selectable">{loc(selected.message)}</p>
                </div>

                {selected.replacement && (
                  <div className="detail__section">
                    <h3>{t.findings.instead}</h3>
                    <pre className="code code--suggest">{loc(selected.replacement)}</pre>
                  </div>
                )}

                <Callout tone={FIX_TONE[selected.fix]} icon={FIX_ICON[selected.fix]}>
                  <strong>{t.fix[selected.fix]}:</strong> {t.fixDesc[selected.fix]}
                </Callout>

                <div className="row" style={{ flexWrap: "wrap" }}>
                  {selected.fix !== "manual" && (
                    <Button variant="primary" onClick={() => setView("changes")}>
                      {t.findings.previewFix} <ArrowRight size={14} />
                    </Button>
                  )}
                  {project && api && (
                    <Button
                      variant="secondary"
                      onClick={() =>
                        api
                          .revealPath(joinPath(project.root, selected.file))
                          .catch((e) => toast.push({ tone: "error", title: t.toast.revealFailed, description: errorMessage(e, t.common.genericError) }))
                      }
                    >
                      {t.findings.reveal}
                    </Button>
                  )}
                </div>
                <div className="subtle mono" style={{ fontSize: "var(--text-xs)" }}>
                  {selected.ruleId}
                </div>
              </div>
            )}
          </aside>
        </div>
      )}
    </>
  );
}

function Chip({ pressed, onClick, label, count }: { pressed: boolean; onClick: () => void; label: string; count: number }) {
  return (
    <button type="button" className="chip" aria-pressed={pressed} onClick={onClick}>
      {label}
      <span className="chip__count">{count}</span>
    </button>
  );
}
