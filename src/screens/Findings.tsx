import { useDeferredValue, useEffect, useMemo, useState } from "react";
import { Activity, ArrowRight, CircleCheck, CircleStop, ClipboardList, EyeOff, FileCheck2, FileCode2, FlaskConical, FolderSearch, MoreHorizontal, Search, SquareArrowOutUpRight, Undo2, X } from "lucide-react";
import { ConfirmDialog } from "../components/Modal";
import { PageHeader } from "../components/Shell";
import { Splitter } from "../components/Splitter";
import { useToast } from "../components/Toast";
import { ActionMenu, Badge, Button, Callout, EmptyState, RiskBadge, Segmented, Select, SeverityIcon, TextArea, type MenuAction } from "../components/ui";
import { useI18n } from "../i18n";
import { errorMessage } from "../lib/api";
import { commonTitle, joinPath, fold } from "../lib/labels";
import { NO_FILTER, useStore, type FindingsFilter } from "../lib/store";
import type { Finding, FixKind, LocalPhp, Severity } from "../lib/types";
import { FindingDetail } from "./FindingDetail";

const PAGE = 300;

export function filterFindings(findings: Finding[], { fix, severity, rule, query }: FindingsFilter): Finding[] {
  const q = fold(query.trim());
  const hit = (s: string) => fold(s).includes(q);
  return findings.filter(
    (f) =>
      (!fix || f.fix === fix) &&
      (!severity || f.severity === severity) &&
      (!rule || f.ruleId === rule) &&
      (!q || hit(f.file) || hit(f.title.tr) || hit(f.title.en) || hit(f.snippet)),
  );
}

/** List order: files with findings seen at runtime first, then scan order, keeping each file's findings together. */
export function prioritizeFindings(findings: Finding[]): Finding[] {
  const rank = new Map<string, number>();
  findings.forEach((f, i) => {
    const runtime = f.runtimeHits > 0 ? -findings.length : 0;
    rank.set(f.file, Math.min(rank.get(f.file) ?? i, i + runtime));
  });
  const seen = (f: Finding) => (f.runtimeHits > 0 ? 0 : 1);
  return [...findings].sort((a, b) => rank.get(a.file)! - rank.get(b.file)! || seen(a) - seen(b));
}

export function Findings() {
  const { t, loc } = useI18n();
  const { scan, setView, project, api, runScan, scanning, findingsFilter: filter, setFindingsFilter, ignoreFinding, unignoreFinding, pruneIgnores, importRuntimeLog, setRuntimeRoots, clearRuntimeLog, runLocalCheck, checking, cancelLocalCheck, target } = useStore();
  const [localPhp, setLocalPhp] = useState<LocalPhp[]>([]);
  const [confirmTests, setConfirmTests] = useState(false);
  useEffect(() => {
    api?.localPhpVersions().then(setLocalPhp).catch(() => {});
  }, [api]);
  const local = localPhp.find((p) => p.line === target);
  const [onlyRuntime, setOnlyRuntime] = useState(false);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [showIgnored, setShowIgnored] = useState(false);
  const [ignoring, setIgnoring] = useState<{ id: string; reason: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const toast = useToast();

  const ignoredList = useMemo(() => scan?.ignored ?? [], [scan]);
  const reasons = useMemo(() => new Map(ignoredList.map((i) => [i.finding.id, i.reason])), [ignoredList]);
  const inlineIgnores = useMemo(() => new Set(ignoredList.filter((i) => i.inline).map((i) => i.finding.id)), [ignoredList]);
  const viewingIgnored = showIgnored && ignoredList.length > 0;
  const findings = useMemo(
    () => (viewingIgnored ? ignoredList.map((i) => i.finding) : scan?.findings ?? []),
    [scan, viewingIgnored, ignoredList],
  );
  const deferredFilter = useDeferredValue(filter);
  const runtimeCount = useMemo(() => findings.filter((f) => f.runtimeHits > 0).length, [findings]);
  const runtimeFilter = onlyRuntime && runtimeCount > 0;
  const visible = useMemo(
    () => prioritizeFindings(filterFindings(findings, deferredFilter).filter((f) => !runtimeFilter || f.runtimeHits > 0)),
    [findings, deferredFilter, runtimeFilter],
  );
  const [limit, setLimit] = useState({ for: visible, first: 0, count: PAGE });
  const selectedAt = selectedId ? visible.findIndex((f) => f.id === selectedId) : -1;
  const saved = limit.for === visible ? limit : { first: 0, count: PAGE };
  const outside = selectedAt >= 0 && (selectedAt < saved.first || selectedAt >= saved.count);
  const first = outside ? Math.max(0, selectedAt - PAGE / 2) : saved.first;
  const shown = outside ? Math.min(visible.length, first + PAGE) : saved.count;
  const showRange = (from: number, count: number) => setLimit({ for: visible, first: from, count });
  const update = (patch: Partial<FindingsFilter>) => setFindingsFilter({ ...filter, ...patch });
  const filtered = filter.fix || filter.severity || filter.rule || filter.query || runtimeFilter;
  const clearFilters = () => {
    setFindingsFilter(NO_FILTER);
    setOnlyRuntime(false);
  };
  const ruleTitle = useMemo(() => {
    const titles = [...new Set(findings.filter((f) => f.ruleId === filter.rule).map((f) => loc(f.title)))];
    return titles.length ? commonTitle(titles) : null;
  }, [findings, filter.rule, loc]);
  const groups = useMemo(() => {
    const map = new Map<string, Finding[]>();
    for (const f of visible.slice(first, shown)) {
      const list = map.get(f.file);
      if (list) list.push(f);
      else map.set(f.file, [f]);
    }
    return [...map.entries()];
  }, [visible, first, shown]);
  const selected = visible.find((f) => f.id === selectedId) ?? visible[0] ?? null;

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      const inList = !!target?.closest?.("[role=listbox]");
      const step = ({ ArrowDown: 1, ArrowUp: -1 } as Record<string, number>)[e.key];
      const jump = inList ? ({ Home: -Infinity, End: Infinity } as Record<string, number>)[e.key] : undefined;
      if (step === undefined && jump === undefined) return;
      if (target?.closest?.("input, textarea, [role=combobox], [role=menu], .pane--side, [role=dialog]")) return;
      if (!visible.length) return;
      e.preventDefault();
      const i = selected ? visible.indexOf(selected) : -1;
      const j = Math.max(0, Math.min(visible.length - 1, jump ?? i + step));
      const next = visible[j];
      if (j < first || j >= shown) {
        const from = Math.max(0, Math.min(j - PAGE / 2, visible.length - PAGE));
        showRange(from, Math.min(visible.length, from + PAGE));
      }
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
  const lastCheck = runtime?.check;
  const runtimeNotice = runtime && (
    <Callout tone={(lastCheck && !lastCheck.passed) || runtime.outsideFatals > 0 ? "warn" : "info"} icon={<Activity size={15} />}>
      <div className="row row--wrap">
        <span>
          {lastCheck && <strong className="check-result">{t.findings.checkResult(lastCheck)} </strong>}
          {t.findings.runtimeSummary(runtime.file, runtime.events, runtime.confirmed, runtime.added, runtime.outside)}
          {runtime.outsideFatals > 0 && ` ${t.findings.outsideFatals(runtime.outsideFatals)}`}
        </span>
        <Button size="sm" variant="ghost" onClick={() => void clearRuntimeLog()}>
          {t.findings.clearLog}
        </Button>
      </div>
      {!lastCheck && !runtime.file.startsWith("Docker") && (
        <details className="check-log">
          <summary>{runtime.roots.length > 0 ? t.findings.runtimeRootActive(runtime.roots.join(", ")) : t.findings.runtimeRoot}</summary>
          <form
            key={runtime.roots.join()}
            className="runtime-root"
            onSubmit={(e) => {
              e.preventDefault();
              void setRuntimeRoots(String(new FormData(e.currentTarget).get("roots") ?? ""));
            }}
          >
            <input className="input" name="roots" aria-label={t.findings.runtimeRoot} placeholder="/var/www/site" defaultValue={runtime.roots.join(", ")} />
            <Button type="submit" size="sm" variant="secondary">
              {t.findings.runtimeRootApply}
            </Button>
          </form>
          <p className="runtime-root__hint">{t.findings.runtimeRootHint}</p>
        </details>
      )}
      {lastCheck?.log && (
        <details className="check-log">
          <summary>{t.findings.showOutput}</summary>
          <code className="check-log__command">{lastCheck.command}</code>
          <pre>{lastCheck.log}</pre>
        </details>
      )}
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

  const moreItems: MenuAction[] = [
    { label: t.findings.importLog, icon: <Activity size={14} aria-hidden />, onClick: () => void importRuntimeLog() },
    ...(checking === "tests" ? [{ label: t.findings.stopTests, icon: <CircleStop size={14} aria-hidden />, onClick: cancelLocalCheck }] : []),
    ...(local
      ? [
          { label: t.findings.lintWith(local.version), icon: <FileCheck2 size={14} aria-hidden />, disabled: !!checking, onClick: () => void runLocalCheck("lint") },
          { label: t.findings.testWith(local.version), icon: <FlaskConical size={14} aria-hidden />, disabled: !!checking, onClick: () => setConfirmTests(true) },
        ]
      : []),
  ];
  const moreMenu = <ActionMenu label={t.overview.more} icon={checking ? <span className="spinner" aria-hidden /> : <MoreHorizontal size={15} aria-hidden />} items={moreItems} />;

  const countFix = (k: FixKind) => findings.filter((f) => f.fix === k).length;
  const countSev = (k: Severity) => findings.filter((f) => f.severity === k).length;

  return (
    <>
      <PageHeader
        title={t.findings.title}
        subtitle={viewingIgnored ? t.findings.ignoredTitle(findings.length) : t.findings.subtitle(scan.target, findings.length)}
        actions={
          <>
            {findings.length > 0 && (
              <Segmented<Severity | "all">
                label={t.findings.severity}
                value={filter.severity ?? "all"}
                onChange={(severity) => update({ severity: severity === "all" ? null : severity })}
                options={[
                  { value: "all", label: t.findings.all },
                  ...(["breaking", "deprecated", "behavior"] as const).map((k) => ({
                    value: k,
                    label: (
                      <>
                        <span className={`dot dot--${k}`} aria-hidden />
                        {t.severity[k]} <span className="segmented__count">{countSev(k)}</span>
                      </>
                    ),
                  })),
                ]}
              />
            )}
            {moreMenu}
          </>
        }
      />
      {findings.length === 0 ? (
        <div className="findings-empty">
          {staleNotice}
          {runtimeNotice}
          <EmptyState
            icon={<CircleCheck size={24} />}
            title={t.findings.clean}
            action={
              <div className="row row--wrap findings-empty__actions">
                {ignoredToggle}
              </div>
            }
          >
            {t.findings.cleanText(scan.target)}
          </EmptyState>
        </div>
      ) : (
        <div className="split">
          <div className="pane">
            <div className="toolbar">
              <div className="toolbar__row">
                <label className="search">
                  <Search size={14} aria-hidden />
                  <input
                    className="input"
                    type="search"
                    placeholder={t.findings.search}
                    aria-label={t.findings.searchLabel}
                    value={filter.query}
                    onChange={(e) => update({ query: e.target.value })}
                  />
                </label>
                <Select<FixKind | "all">
                  className="toolbar__select"
                  label={t.findings.fixType}
                  value={filter.fix ?? "all"}
                  options={[{ value: "all", label: t.findings.allKinds }, ...(["auto", "likely", "manual"] as const).map((k) => ({ value: k, label: `${t.fix[k]} ${countFix(k)}` }))]}
                  onChange={(fix) => update({ fix: fix === "all" ? null : fix })}
                />
              </div>
              {(runtimeCount > 0 || ruleTitle) && (
                <div className="toolbar__filters">
                  {runtimeCount > 0 && <Chip pressed={runtimeFilter} onClick={() => setOnlyRuntime(!runtimeFilter)} label={t.findings.runtimeOnly} count={runtimeCount} />}
                  {ruleTitle && (
                    <button type="button" className="chip chip--removable" onClick={() => update({ rule: null })} aria-label={t.findings.removeRule(ruleTitle)}>
                      {ruleTitle}
                      <X size={12} aria-hidden />
                    </button>
                  )}
                </div>
              )}
              {staleNotice}
              {runtimeNotice}
              {(filtered || ignoredList.length > 0) && (
                <div className="toolbar__summary">
                  <span aria-live="polite">{t.findings.showing(visible.length, findings.length)}</span>
                  {filtered && (
                    <Button variant="ghost" size="sm" onClick={clearFilters}>
                      {t.findings.clearFilters}
                    </Button>
                  )}
                  {ignoredToggle}
                </div>
              )}
            </div>

            {visible.length === 0 ? (
              <EmptyState
                icon={<FolderSearch size={24} />}
                title={t.findings.noMatch}
                action={
                  <Button variant="secondary" onClick={clearFilters}>
                    {t.findings.clearFilters}
                  </Button>
                }
              >
                {t.findings.noMatchText}
              </EmptyState>
            ) : (
              <>
                {first > 0 && (
                  <div className="finding-more">
                    <Button variant="secondary" onClick={() => showRange(Math.max(0, first - PAGE), shown)}>
                      {t.findings.showEarlier(Math.min(PAGE, first), first)}
                    </Button>
                  </div>
                )}
                <div role="listbox" aria-label={t.findings.list} className="finding-list">
                  {groups.map(([file, items]) => (
                    <section key={file} className="file-group" role="group" aria-label={file}>
                      <header className="file-group__header">
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
                          <SeverityIcon severity={f.severity} />
                          <span className="finding__main">
                            <div className="finding__title">{loc(f.title)}</div>
                            <div className="finding__snippet">{f.snippet}</div>
                          </span>
                          <span className="finding__badges">
                            {f.runtimeHits > 0 && <Badge tone="accent">{t.findings.runtimeSeen(f.runtimeHits)}</Badge>}
                            {f.fix !== "manual" && <RiskBadge fix={f.fix} />}
                            <span className="finding__line">{f.line}</span>
                          </span>
                        </button>
                      ))}
                    </section>
                  ))}
                </div>
                {visible.length > shown && (
                  <div className="finding-more">
                    <Button variant="secondary" onClick={() => showRange(first, shown + PAGE)}>
                      {t.findings.showMore(Math.min(PAGE, visible.length - shown), visible.length - shown)}
                    </Button>
                  </div>
                )}
              </>
            )}
          </div>

          <Splitter id="findings" side="end" label={t.common.resizePanes} narrowAt="(max-width: 900px)" />
          <aside className="pane pane--side" aria-label={t.findings.details}>
            {selected && (
              <FindingDetail
                finding={selected}
                findings={visible}
                current={project?.currentVersion ?? null}
                target={scan.target}
                root={project?.root}
                api={api}
                onSelect={setSelectedId}
                notice={
                  selectedReason !== undefined && (
                    <Callout tone="info" icon={<EyeOff size={15} />}>
                      <strong>{t.findings.ignoredReason}:</strong> <span className="selectable">{selectedReason || "—"}</span>
                      {inlineIgnores.has(selected.id) && <div>{t.findings.inlineIgnore}</div>}
                    </Callout>
                  )
                }
                footer={
                  <>
                    {selectedReason === undefined && selected.fix !== "manual" && (
                      <Button variant="primary" onClick={() => setView("changes")}>
                        {t.findings.previewFix} <ArrowRight size={14} />
                      </Button>
                    )}
                    {project && api && (
                      <Button variant="secondary" icon={<SquareArrowOutUpRight size={13} />} onClick={() => openInEditor(selected)}>
                        {t.findings.openInEditor}
                      </Button>
                    )}
                    {project && api && (
                      <Button
                        variant="ghost"
                        icon={<FileCode2 size={13} />}
                        onClick={() =>
                          api
                            .revealPath(joinPath(project.root, selected.file))
                            .catch((e) => toast.push({ tone: "error", title: t.toast.revealFailed, description: errorMessage(e, t.common.genericError) }))
                        }
                      >
                        {t.findings.reveal}
                      </Button>
                    )}
                    {selectedReason === undefined ? (
                      <Button variant="ghost" className="push-end" icon={<EyeOff size={13} />} onClick={() => setIgnoring({ id: selected.id, reason: "" })}>
                        {t.findings.ignore}
                      </Button>
                    ) : inlineIgnores.has(selected.id) ? null : (
                      <Button variant="ghost" className="push-end" icon={<Undo2 size={13} />} onClick={() => void unignoreFinding(selected.id)}>
                        {t.findings.unignore}
                      </Button>
                    )}
                  </>
                }
              />
            )}
          </aside>
        </div>
      )}
      <ConfirmDialog
        open={confirmTests}
        icon={<FlaskConical size={20} />}
        title={t.findings.testTitle}
        confirmLabel={t.findings.testRun}
        onConfirm={() => {
          setConfirmTests(false);
          void runLocalCheck("tests");
        }}
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
        <TextArea
          className="field"
          label={t.findings.ignoreReason}
          rows={3}
          value={ignoring?.reason ?? ""}
          placeholder={t.findings.ignorePlaceholder}
          onChange={(e) => setIgnoring((v) => v && { ...v, reason: e.target.value })}
        />
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
