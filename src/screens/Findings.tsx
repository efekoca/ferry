import { useDeferredValue, useEffect, useMemo, useState, type ReactNode } from "react";
import { Activity, ArrowRight, CircleCheck, FileCheck2, FlaskConical, ClipboardList, EyeOff, FileCode2, FolderSearch, Info, Lightbulb, Search, Sparkles, SquareArrowOutUpRight, Undo2, X } from "lucide-react";
import { ConfirmDialog } from "../components/Modal";
import { PageHeader } from "../components/Shell";
import { useToast } from "../components/Toast";
import { Badge, Button, Callout, EmptyState, RiskBadge, SeverityBadge, SeverityIcon } from "../components/ui";
import { useI18n } from "../i18n";
import { errorMessage } from "../lib/api";
import { commonTitle, joinPath } from "../lib/labels";
import { NO_FILTER, useStore, type FindingsFilter } from "../lib/store";
import type { Finding, FixKind, LocalPhp, Severity } from "../lib/types";

const PAGE = 300;

const FIX_TONE = { auto: "success", likely: "info", manual: "warn" } as const;
const FIX_ICON: Record<FixKind, ReactNode> = {
  auto: <Sparkles size={15} />,
  likely: <Lightbulb size={15} />,
  manual: <Info size={15} />,
};

export function filterFindings(findings: Finding[], { fix, severity, rule, query }: FindingsFilter): Finding[] {
  const q = query.trim().toLowerCase();
  const hit = (s: string) => s.toLowerCase().includes(q);
  return findings.filter(
    (f) =>
      (!fix || f.fix === fix) &&
      (!severity || f.severity === severity) &&
      (!rule || f.ruleId === rule) &&
      (!q || hit(f.file) || hit(f.title.tr) || hit(f.title.en) || hit(f.snippet)),
  );
}

export function Findings() {
  const { t, loc } = useI18n();
  const { scan, setView, project, api, runScan, scanning, findingsFilter: filter, setFindingsFilter, ignoreFinding, unignoreFinding, pruneIgnores, importRuntimeLog, clearRuntimeLog, runLocalCheck, target } = useStore();
  const [localPhp, setLocalPhp] = useState<LocalPhp[]>([]);
  const [checking, setChecking] = useState<"lint" | "tests" | null>(null);
  const [confirmTests, setConfirmTests] = useState(false);
  useEffect(() => {
    api?.localPhpVersions().then(setLocalPhp).catch(() => {});
  }, [api]);
  const local = localPhp.find((p) => p.line === target);
  const check = async (kind: "lint" | "tests") => {
    setChecking(kind);
    await runLocalCheck(kind);
    setChecking(null);
    setConfirmTests(false);
  };
  const [onlyRuntime, setOnlyRuntime] = useState(false);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [showIgnored, setShowIgnored] = useState(false);
  const [ignoring, setIgnoring] = useState<{ id: string; reason: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const toast = useToast();

  const ignoredList = useMemo(() => scan?.ignored ?? [], [scan]);
  const reasons = useMemo(() => new Map(ignoredList.map((i) => [i.finding.id, i.reason])), [ignoredList]);
  const viewingIgnored = showIgnored && ignoredList.length > 0;
  const findings = useMemo(
    () => (viewingIgnored ? ignoredList.map((i) => i.finding) : scan?.findings ?? []),
    [scan, viewingIgnored, ignoredList],
  );
  const deferredFilter = useDeferredValue(filter);
  const runtimeCount = useMemo(() => findings.filter((f) => f.runtimeHits > 0).length, [findings]);
  const runtimeFilter = onlyRuntime && runtimeCount > 0;
  const visible = useMemo(
    () => filterFindings(findings, deferredFilter).filter((f) => !runtimeFilter || f.runtimeHits > 0),
    [findings, deferredFilter, runtimeFilter],
  );
  const [limit, setLimit] = useState({ for: visible, count: PAGE });
  const shown = limit.for === visible ? limit.count : PAGE;
  const showMore = (count: number) => setLimit({ for: visible, count });
  const update = (patch: Partial<FindingsFilter>) => setFindingsFilter({ ...filter, ...patch });
  const filtered = filter.fix || filter.severity || filter.rule || filter.query;
  const ruleTitle = useMemo(() => {
    const titles = [...new Set(findings.filter((f) => f.ruleId === filter.rule).map((f) => loc(f.title)))];
    return titles.length ? commonTitle(titles) : null;
  }, [findings, filter.rule, loc]);
  const groups = useMemo(() => {
    const map = new Map<string, Finding[]>();
    for (const f of visible.slice(0, shown)) {
      const list = map.get(f.file);
      if (list) list.push(f);
      else map.set(f.file, [f]);
    }
    return [...map.entries()];
  }, [visible, shown]);
  const selected = visible.find((f) => f.id === selectedId) ?? visible[0] ?? null;

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      const inList = !!target?.closest?.("[role=listbox]");
      const step = ({ ArrowDown: 1, ArrowUp: -1 } as Record<string, number>)[e.key];
      const jump = inList ? ({ Home: -Infinity, End: Infinity } as Record<string, number>)[e.key] : undefined;
      if (step === undefined && jump === undefined) return;
      if (target?.closest?.("input, textarea, select, .pane--side, [role=dialog]")) return;
      if (!visible.length) return;
      e.preventDefault();
      const i = selected ? visible.indexOf(selected) : -1;
      const j = Math.max(0, Math.min(visible.length - 1, jump ?? i + step));
      const next = visible[j];
      if (j >= shown) showMore(Math.max(shown + PAGE, j + 1));
      setSelectedId(next.id);
      requestAnimationFrame(() => {
        const el = document.getElementById(`finding-${next.id}`);
        el?.scrollIntoView({ block: "nearest" });
        if (inList) el?.focus();
      });
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  if (!scan) {
    return (
      <>
        <PageHeader title={t.findings.title} />
        <EmptyState
          icon={<ClipboardList size={24} />}
          title={t.findings.noScan}
          action={
            <Button variant="primary" onClick={() => runScan()} loading={scanning}>
              {t.overview.startScan}
            </Button>
          }
        >
          {t.findings.noScanText}
        </EmptyState>
      </>
    );
  }

  const confirmIgnore = async () => {
    if (!ignoring) return;
    setBusy(true);
    const done = await ignoreFinding(ignoring.id, ignoring.reason);
    setBusy(false);
    if (done) setIgnoring(null);
  };
  const openInEditor = (f: Finding) =>
    project &&
    api
      ?.openInEditor(project.root, f.file, f.line, f.column)
      .catch((e) => toast.push({ tone: "error", title: t.toast.editorFailed, description: errorMessage(e, t.common.genericError) }));
  const ignoredToggle = ignoredList.length > 0 && (
    <Button variant="ghost" size="sm" icon={viewingIgnored ? <Undo2 size={13} /> : <EyeOff size={13} />} onClick={() => setShowIgnored(!viewingIgnored)}>
      {viewingIgnored ? t.findings.backToFindings : t.findings.ignored(ignoredList.length)}
    </Button>
  );
  const selectedReason = selected ? reasons.get(selected.id) : undefined;
  const runtime = scan?.runtime;
  const runtimeNotice = runtime && (
    <Callout tone="info" icon={<Activity size={15} />}>
      <div className="row row--wrap">
        <span>{t.findings.runtimeSummary(runtime.file, runtime.events, runtime.confirmed, runtime.added, runtime.outside)}</span>
        <Button size="sm" variant="ghost" onClick={() => void clearRuntimeLog()}>
          {t.findings.clearLog}
        </Button>
      </div>
    </Callout>
  );
  const stale = scan?.staleIgnores ?? [];
  const staleNotice = stale.length > 0 && (
    <Callout tone="warn" icon={<EyeOff size={15} />}>
      <div className="row row--wrap">
        <span>{t.findings.staleIgnores(stale.length)}</span>
        <Button size="sm" variant="secondary" onClick={() => void pruneIgnores()}>
          {t.findings.pruneIgnores}
        </Button>
      </div>
    </Callout>
  );

  const countFix = (k: FixKind) => findings.filter((f) => f.fix === k).length;
  const countSev = (k: Severity) => findings.filter((f) => f.severity === k).length;

  return (
    <>
      <PageHeader
        title={t.findings.title}
        subtitle={viewingIgnored ? t.findings.ignoredTitle(findings.length) : t.findings.subtitle(scan.target, findings.length)}
      />
      {findings.length === 0 ? (
        <EmptyState icon={<CircleCheck size={24} />} title={t.findings.clean} action={ignoredToggle || undefined}>
          {t.findings.cleanText(scan.target)}
        </EmptyState>
      ) : (
        <div className="split">
          <div className="pane">
            <div className="toolbar">
              <div className="search">
                <Search size={14} aria-hidden />
                <input
                  className="input"
                  type="search"
                  placeholder={t.findings.search}
                  aria-label={t.findings.searchLabel}
                  value={filter.query}
                  onChange={(e) => update({ query: e.target.value })}
                />
              </div>
              <div className="toolbar__filters">
                <div className="chip-group" role="group" aria-label={t.findings.fixType}>
                  {(["auto", "likely", "manual"] as const).map((k) => (
                    <Chip key={k} pressed={filter.fix === k} onClick={() => update({ fix: filter.fix === k ? null : k })} label={t.fix[k]} count={countFix(k)} />
                  ))}
                </div>
                <div className="chip-group" role="group" aria-label={t.findings.severity}>
                  {(["breaking", "deprecated", "behavior"] as const).map((k) => (
                    <Chip key={k} pressed={filter.severity === k} onClick={() => update({ severity: filter.severity === k ? null : k })} label={t.severity[k]} count={countSev(k)} />
                  ))}
                </div>
                {runtimeCount > 0 && (
                  <Chip pressed={runtimeFilter} onClick={() => setOnlyRuntime(!runtimeFilter)} label={t.findings.runtimeOnly} count={runtimeCount} />
                )}
                {ruleTitle && (
                  <button type="button" className="chip chip--removable" onClick={() => update({ rule: null })} aria-label={t.findings.removeRule(ruleTitle)}>
                    {ruleTitle}
                    <X size={12} aria-hidden />
                  </button>
                )}
              </div>
              {staleNotice}
              {runtimeNotice}
              <div className="toolbar__summary">
                <span aria-live="polite">{t.findings.showing(visible.length, findings.length)}</span>
                {filtered && (
                  <Button variant="ghost" size="sm" onClick={() => setFindingsFilter(NO_FILTER)}>
                    {t.findings.clearFilters}
                  </Button>
                )}
                {ignoredToggle}
              </div>
              <div className="toolbar__actions">
                <Button variant="ghost" size="sm" icon={<Activity size={13} />} onClick={() => void importRuntimeLog()}>
                  {t.findings.importLog}
                </Button>
                {local && (
                  <>
                    <Button variant="ghost" size="sm" icon={<FileCheck2 size={13} />} loading={checking === "lint"} disabled={!!checking} onClick={() => void check("lint")}>
                      {t.findings.lintWith(local.version)}
                    </Button>
                    <Button variant="ghost" size="sm" icon={<FlaskConical size={13} />} disabled={!!checking} onClick={() => setConfirmTests(true)}>
                      {t.findings.testWith(local.version)}
                    </Button>
                  </>
                )}
              </div>
            </div>

            {visible.length === 0 ? (
              <EmptyState
                icon={<FolderSearch size={24} />}
                title={t.findings.noMatch}
                action={
                  <Button variant="secondary" onClick={() => setFindingsFilter(NO_FILTER)}>
                    {t.findings.clearFilters}
                  </Button>
                }
              >
                {t.findings.noMatchText}
              </EmptyState>
            ) : (
              <>
                <div role="listbox" aria-label={t.findings.list} className="finding-list">
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
                          tabIndex={selected?.id === f.id ? 0 : -1}
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
                            {f.runtimeHits > 0 && <Badge tone="accent">{t.findings.runtimeSeen(f.runtimeHits)}</Badge>}
                            <RiskBadge fix={f.fix} />
                          </span>
                        </button>
                      ))}
                    </section>
                  ))}
                </div>
                {visible.length > shown && (
                  <div className="finding-more">
                    <Button variant="secondary" onClick={() => showMore(shown + PAGE)}>
                      {t.findings.showMore(Math.min(PAGE, visible.length - shown), visible.length - shown)}
                    </Button>
                  </div>
                )}
              </>
            )}
          </div>

          <aside className="pane pane--side" aria-label={t.findings.details}>
            {selected && (
              <div className="detail" key={selected.id}>
                <div>
                  <div className="row detail__badges">
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
                  {selected.runtimeHits > 0 && !selected.ruleId.startsWith("runtime.") && (
                    <p className="detail__runtime">
                      <Activity size={13} aria-hidden /> {t.findings.runtimeDetail(selected.runtimeHits)}
                    </p>
                  )}
                </div>

                {selected.replacement && (
                  <div className="detail__section">
                    <h3>{t.findings.instead}</h3>
                    <pre className="code code--suggest">{loc(selected.replacement)}</pre>
                  </div>
                )}

                {selectedReason !== undefined ? (
                  <Callout tone="info" icon={<EyeOff size={15} />}>
                    <strong>{t.findings.ignoredReason}:</strong> <span className="selectable">{selectedReason}</span>
                  </Callout>
                ) : (
                  <Callout tone={FIX_TONE[selected.fix]} icon={FIX_ICON[selected.fix]}>
                    <strong>{t.fix[selected.fix]}:</strong> {t.fixDesc[selected.fix]}
                  </Callout>
                )}

                <div className="row row--wrap">
                  {selectedReason === undefined && selected.fix !== "manual" && (
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
                  {project && api && (
                    <Button variant="secondary" icon={<SquareArrowOutUpRight size={13} />} onClick={() => openInEditor(selected)}>
                      {t.findings.openInEditor}
                    </Button>
                  )}
                  {selectedReason === undefined ? (
                    <Button variant="ghost" icon={<EyeOff size={13} />} onClick={() => setIgnoring({ id: selected.id, reason: "" })}>
                      {t.findings.ignore}
                    </Button>
                  ) : (
                    <Button variant="ghost" icon={<Undo2 size={13} />} onClick={() => void unignoreFinding(selected.id)}>
                      {t.findings.unignore}
                    </Button>
                  )}
                </div>
                <div className="detail__rule">
                  {selected.ruleId}
                </div>
              </div>
            )}
          </aside>
        </div>
      )}
      <ConfirmDialog
        open={confirmTests}
        icon={<FlaskConical size={20} />}
        title={t.findings.testTitle}
        confirmLabel={t.findings.testRun}
        loading={checking === "tests"}
        onConfirm={() => void check("tests")}
        onCancel={() => setConfirmTests(false)}
      >
        {t.findings.testText(local?.version ?? target)}
      </ConfirmDialog>
      <ConfirmDialog
        open={!!ignoring}
        icon={<EyeOff size={20} />}
        title={t.findings.ignoreTitle}
        confirmLabel={t.findings.ignore}
        loading={busy}
        confirmDisabled={!ignoring?.reason.trim()}
        onConfirm={confirmIgnore}
        onCancel={() => setIgnoring(null)}
      >
        <p>{t.findings.ignoreText}</p>
        <label className="field">
          <span className="field__label">{t.findings.ignoreReason}</span>
          <textarea
            className="input"
            rows={3}
            value={ignoring?.reason ?? ""}
            placeholder={t.findings.ignorePlaceholder}
            onChange={(e) => setIgnoring((v) => v && { ...v, reason: e.target.value })}
          />
        </label>
      </ConfirmDialog>
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
