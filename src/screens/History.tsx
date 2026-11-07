import { useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { ArrowDown, ArrowUp, CircleAlert, Container, History as HistoryIcon, Info, ListChecks, RotateCcw, Search, Trash2, Wrench } from "lucide-react";
import { DiffViewer } from "../components/DiffViewer";
import { ConfirmDialog, Sheet } from "../components/Modal";
import { PageHeader } from "../components/Shell";
import { Button, Callout, Card, EmptyState, RiskBadge, RuleChips, Segmented, Tooltip } from "../components/ui";
import { useToast } from "../components/Toast";
import { useI18n, type Dict } from "../i18n";
import { errorMessage } from "../lib/api";
import { clock, dayName, historyEvents, moment, rangeStart, type HistoryEvent, type HistoryRange } from "../lib/history";
import { formatRelative } from "../lib/labels";
import { useStore } from "../lib/store";
import type { CheckSummary, DockerSummary, FileChange, HistoryEntry, Snapshot, SnapshotConflict } from "../lib/types";
import { HistoryChart, HistoryTable, type ChartMark } from "./HistoryChart";

const TIMELINE_LIMIT = 8;

type Tone = "neutral" | "good" | "bad" | "warn";

interface ScanChange {
  at: number;
  text: string;
}

const dockerIssues = (run: DockerSummary) => run.differingPages + run.newFailures + run.dataChanges;

function dockerState(run: DockerSummary, t: Dict): { tone: "good" | "bad" | "warn"; text: string } {
  const states = t.overview.readiness.dockerState;
  if (run.regressed) return { tone: "bad", text: dockerIssues(run) > 0 ? t.history.checkFailed(dockerIssues(run)) : states.failed };
  if (run.incomplete) return { tone: "warn", text: states.incomplete };
  return run.testsCompared ? { tone: "good", text: states.passed } : { tone: "warn", text: states.partial };
}

function checkState(run: CheckSummary, t: Dict): { tone: Tone; text: string } {
  if (run.passed) return { tone: "good", text: t.history.checkState(true, false) };
  return { tone: run.timedOut ? "warn" : "bad", text: run.failed > 0 ? t.history.checkFailed(run.failed) : t.history.checkState(false, run.timedOut) };
}

function Delta({ change, upIsGood, text }: { change: number | null; upIsGood: boolean; text: (n: number, up: boolean) => string }) {
  if (!change) return null;
  return (
    <span className={`kpi__delta kpi__delta--${change > 0 === upIsGood ? "good" : "bad"}`}>
      {change > 0 ? <ArrowUp size={12} aria-hidden /> : <ArrowDown size={12} aria-hidden />}
      {text(Math.abs(change), change > 0)}
    </span>
  );
}

function ProgressCard({ scans, marks }: { scans: HistoryEntry[]; marks: ChartMark[] }) {
  const { t } = useI18n();
  const [view, setView] = useState<"chart" | "table">("chart");
  const last = scans[scans.length - 1];
  const change = (pick: (s: HistoryEntry) => number | undefined) => {
    const [from, to] = [pick(scans[0]), pick(last)];
    return scans.length > 1 && from !== undefined && to !== undefined ? to - from : null;
  };
  const moved = (n: number, up: boolean) => (up ? t.history.more(n) : t.history.less(n));
  const kpis: { label: string; value: number | undefined; delta: ReactNode }[] = [
    { label: t.history.kpiScore, value: last.score, delta: <Delta change={change((s) => s.score)} upIsGood text={t.history.points} /> },
    { label: t.history.kpiFindings, value: last.total, delta: <Delta change={change((s) => s.total)} upIsGood={false} text={moved} /> },
    { label: t.history.kpiCritical, value: last.counts?.breaking, delta: <Delta change={change((s) => s.counts?.breaking)} upIsGood={false} text={moved} /> },
    { label: t.history.kpiFiles, value: last.counts?.files, delta: <Delta change={change((s) => s.counts?.files)} upIsGood={false} text={moved} /> },
  ];
  return (
    <section className="card" aria-label={t.progress.title}>
      <dl className="kpis">
        {kpis.map((kpi) => (
          <div key={kpi.label} className="kpi">
            <dt>{kpi.label}</dt>
            <dd>
              <span className="kpi__value">{kpi.value ?? "–"}</span>
              {kpi.delta}
            </dd>
          </div>
        ))}
      </dl>
      {scans.length < 2 ? (
        <p className="history-note">{t.progress.first}</p>
      ) : (
        <>
          <div className="plot-head">
            {view === "chart" && (
              <ul className="legend">
                <li>
                  <i className="legend__line" />
                  {t.history.kpiScore}
                </li>
                <li>
                  <i className="legend__key legend__key--breaking" />
                  {t.severity.breaking}
                </li>
                <li>
                  <i className="legend__key legend__key--deprecated" />
                  {t.severity.deprecated}
                </li>
                <li>
                  <i className="legend__key legend__key--behavior" />
                  {t.severity.behavior}
                </li>
              </ul>
            )}
            <div className="plot-head__end">
              <Segmented
                size="sm"
                label={t.history.view}
                value={view}
                onChange={setView}
                options={[
                  { value: "chart", label: t.history.chart },
                  { value: "table", label: t.history.table },
                ]}
              />
            </div>
          </div>
          {view === "chart" ? <HistoryChart scans={scans} marks={marks} /> : <HistoryTable scans={scans} />}
        </>
      )}
    </section>
  );
}

function EventIcon({ event }: { event: HistoryEvent }) {
  const { t } = useI18n();
  if (event.kind === "fix") return <span className="badge-icon badge-icon--good"><Wrench size={15} /></span>;
  if (event.kind === "scan") return <span className="badge-icon"><Search size={15} /></span>;
  const tone = event.kind === "real" ? dockerState(event.run, t).tone : checkState(event.run, t).tone;
  const icon = event.kind === "real" ? <Container size={15} /> : <ListChecks size={15} />;
  return <span className={`badge-icon badge-icon--${tone}`}>{icon}</span>;
}

function eventText(event: HistoryEvent, t: Dict, since: ScanChange | null): { title: string; sub: string; hint?: string } {
  if (event.kind === "scan") {
    const { at, target, total, score } = event.entry;
    return { title: t.history.scan, sub: [t.history.scanLine(target, total, score), at === since?.at && since.text].filter(Boolean).join(" · ") };
  }
  if (event.kind === "fix") return { title: event.snapshot.label, sub: t.history.fileCount(event.snapshot.files.length), hint: event.snapshot.files.map((f) => f.path).join("\n") };
  if (event.kind === "real") {
    const { run } = event;
    return { title: t.nav.runtime, sub: `PHP ${run.source} → ${run.target} · ${dockerState(run, t).text}`, hint: t.history.dockerCounts(run.differingPages, run.pages, run.newFailures, run.dataChanges) };
  }
  return { title: event.run.kind === "tests" ? t.history.tests : t.history.lint, sub: `PHP ${event.run.target} · ${checkState(event.run, t).text}` };
}

function Timeline({ events, since, onRestore }: { events: HistoryEvent[]; since: ScanChange | null; onRestore: (snapshot: Snapshot) => void }) {
  const { t } = useI18n();
  const [all, setAll] = useState(false);
  const shown = all ? events : events.slice(0, TIMELINE_LIMIT);
  const days = useMemo(() => {
    const groups: { key: string; name: string; items: HistoryEvent[] }[] = [];
    for (const event of shown) {
      const key = new Date(event.at).toDateString();
      const last = groups[groups.length - 1];
      if (last?.key === key) last.items.push(event);
      else groups.push({ key, name: dayName(event.at, t), items: [event] });
    }
    return groups;
  }, [shown, t]);
  return (
    <section className="card" aria-label={t.history.timeline}>
      <ol className="timeline">
        {days.map((day) => (
          <li key={day.key}>
            <div className="timeline__day">{day.name}</div>
            <ol className="timeline__items">
              {day.items.map((event) => {
                const { title, sub, hint } = eventText(event, t, since);
                return (
                  <li key={event.key} className="line line--soft" title={hint}>
                    <EventIcon event={event} />
                    <span className="line__main">
                      <span className="line__title">{title}</span>
                      <span className="line__sub">{sub}</span>
                    </span>
                    <span className="line__end">
                      {event.kind === "fix" && (
                        <Button size="sm" className="line__reveal" aria-label={t.history.restoreLabel(event.snapshot.label)} onClick={() => onRestore(event.snapshot)}>
                          {t.history.restore}
                        </Button>
                      )}
                      <span>{clock(event.at, t)}</span>
                    </span>
                  </li>
                );
              })}
            </ol>
          </li>
        ))}
      </ol>
      {!all && events.length > TIMELINE_LIMIT && (
        <div className="timeline__more">
          <Button variant="ghost" size="sm" onClick={() => setAll(true)}>
            {t.history.moreRecords(events.length - TIMELINE_LIMIT)}
          </Button>
        </div>
      )}
    </section>
  );
}

function Backups({
  snapshots,
  onReview,
  onRestore,
  onDelete,
  onClear,
}: {
  snapshots: Snapshot[];
  onReview: (snapshot: Snapshot) => void;
  onRestore: (snapshot: Snapshot) => void;
  onDelete: (snapshot: Snapshot) => void;
  onClear: () => void;
}) {
  const { t } = useI18n();
  return (
    <Card title={t.history.backups} bodyClassName="lines-card">
      {snapshots.length === 0 ? (
        <div className="history-empty">
          <strong>{t.history.empty}</strong>
          <p>{t.history.emptyText}</p>
        </div>
      ) : (
        <ul className="lines" aria-label={t.history.backups}>
          {snapshots.map((s) => {
            const [name, ...details] = s.label.split(" · ");
            return (
            <li key={s.id} className="line history-backup">
              <button className="line__main line__open" aria-label={t.history.reviewLabel(s.label)} title={s.files.map((f) => f.path).join("\n")} onClick={() => onReview(s)}>
                <span className="line__title">{name}</span>
                <span className="line__sub">{[...details, moment(s.createdAt, t), t.history.fileCount(s.files.length)].join(" · ")}</span>
              </button>
              <span className="line__end">
                <Button size="sm" variant="ghost" iconOnly className="line__reveal" aria-label={t.history.delete} onClick={() => onDelete(s)}>
                  <Trash2 size={14} />
                </Button>
                <Button size="sm" onClick={() => onRestore(s)}>
                  {t.history.restore}
                </Button>
              </span>
            </li>
            );
          })}
        </ul>
      )}
      {snapshots.length > 1 && (
        <div className="lines-card__foot">
          <Button variant="ghost" size="sm" className="danger-text" onClick={onClear}>
            {t.history.deleteAll}
          </Button>
        </div>
      )}
    </Card>
  );
}

function LatestChecks({ real, checks }: { real: DockerSummary[]; checks: CheckSummary[] }) {
  const { t } = useI18n();
  const newest = <T extends { at: number }>(list: T[]) => list.reduce<T | null>((best, run) => (!best || run.at > best.at ? run : best), null);
  const docker = newest(real);
  const lint = newest(checks.filter((c) => c.kind === "lint"));
  const tests = newest(checks.filter((c) => c.kind === "tests"));
  const rows = [
    docker && { key: "real", title: t.nav.runtime, sub: `PHP ${docker.source} → ${docker.target} · ${moment(docker.at * 1000, t)}`, ...dockerState(docker, t) },
    lint && { key: "lint", title: t.history.lint, sub: `PHP ${lint.target} · ${moment(lint.at * 1000, t)}`, ...checkState(lint, t) },
    tests && { key: "tests", title: t.history.tests, sub: `PHP ${tests.target} · ${moment(tests.at * 1000, t)}`, ...checkState(tests, t) },
  ].flatMap((row) => (row ? [row] : []));
  if (rows.length === 0) return null;
  const dots: Record<Tone, string> = { neutral: "neutral", good: "ok", bad: "danger", warn: "warn" };
  return (
    <Card
      title={
        <Tooltip text={t.history.checksNote} align="start">
          <span tabIndex={0}>{t.history.checks}</span>
        </Tooltip>
      }
      bodyClassName="lines-card lines-card--fill"
    >
      <ul className="lines">
        {rows.map((row) => (
          <li key={row.key} className="line">
            <span className={`dot dot--${dots[row.tone]}`} aria-hidden />
            <span className="line__main">
              <span className="line__title">{row.title}</span>
              <span className="line__sub">{row.sub}</span>
            </span>
            <span className="line__end">{row.text}</span>
          </li>
        ))}
      </ul>
    </Card>
  );
}

export function History() {
  const { t } = useI18n();
  const { api, project, target, scan, scanHistory, snapshots, restoreSnapshot, deleteSnapshot, deleteAllSnapshots, dockerHistory, checkHistory } = useStore();
  const [range, setRange] = useState<HistoryRange>("all");
  const [clearing, setClearing] = useState(false);
  const [pending, setPending] = useState<{ snapshot: Snapshot; action: "restore" | "delete" } | null>(null);
  const [reviewing, setReviewing] = useState<Snapshot | null>(null);
  const [busy, setBusy] = useState(false);
  const [conflicts, setConflicts] = useState<SnapshotConflict[]>([]);
  const deleting = pending?.action === "delete";
  const restoringId = pending?.action === "restore" ? pending.snapshot.id : null;

  useEffect(() => {
    setConflicts([]);
    if (!api || !project || !restoringId) return;
    let live = true;
    api
      .snapshotConflicts(project.root, restoringId)
      .then((files) => live && setConflicts(files))
      .catch(() => {});
    return () => {
      live = false;
    };
  }, [api, project, restoringId]);

  const start = useMemo(() => rangeStart(range), [range]);
  const allScans = useMemo(() => scanHistory.filter((e) => e.target === target), [scanHistory, target]);
  const scans = useMemo(() => allScans.filter((e) => e.at * 1000 >= start), [allScans, start]);
  const events = useMemo(() => historyEvents(scans, snapshots, dockerHistory, checkHistory).filter((e) => e.at >= start), [scans, snapshots, dockerHistory, checkHistory, start]);
  const marks = useMemo(
    () =>
      [...events].reverse().flatMap((event): ChartMark[] => {
        if (event.kind === "fix") return [{ at: event.at, tone: "good", icon: <Wrench size={12} />, text: `${event.snapshot.label} · ${t.history.fileCount(event.snapshot.files.length)}` }];
        if (event.kind === "real") return [{ at: event.at, tone: dockerState(event.run, t).tone, icon: <Container size={12} />, text: `${t.nav.runtime} · ${dockerState(event.run, t).text}` }];
        return [];
      }),
    [events, t],
  );
  const since = useMemo((): ScanChange | null => {
    const progress = scan?.target === target ? scan.progress : null;
    const at = progress?.history[progress.history.length - 1]?.at;
    const text = [progress?.fixedSinceLast && t.progress.resolved(progress.fixedSinceLast), progress?.newSinceLast && t.progress.added(progress.newSinceLast)].filter(Boolean).join(" · ");
    return at !== undefined && text ? { at, text } : null;
  }, [scan, target, t]);
  const nothing = allScans.length === 0 && snapshots.length === 0 && dockerHistory.length === 0 && checkHistory.length === 0;

  return (
    <>
      <PageHeader
        title={t.history.title}
        subtitle={allScans.length ? `PHP ${target}` : undefined}
        actions={
          allScans.length > 0 && (
            <Segmented
              size="sm"
              label={t.history.range}
              value={range}
              onChange={setRange}
              options={[
                { value: "7", label: t.history.range7 },
                { value: "30", label: t.history.range30 },
                { value: "all", label: t.history.rangeAll },
              ]}
            />
          )
        }
      />
      <div className="page-scroll">
        {nothing ? (
          <div className="page-body">
            <EmptyState icon={<HistoryIcon size={24} />} title={t.history.empty}>
              {t.history.emptyText}
            </EmptyState>
          </div>
        ) : (
          <div className="page-body stack">
            {scans.length > 0 ? <ProgressCard scans={scans} marks={marks} /> : allScans.length > 0 && <p className="history-note history-note--bare">{t.history.noScans}</p>}
            <div className="grid-rail">
              {events.length > 0 && <Timeline events={events} since={since} onRestore={(snapshot) => setPending({ snapshot, action: "restore" })} />}
              <div className="grid-rail__side">
                <Backups
                  snapshots={snapshots}
                  onReview={setReviewing}
                  onRestore={(snapshot) => setPending({ snapshot, action: "restore" })}
                  onDelete={(snapshot) => setPending({ snapshot, action: "delete" })}
                  onClear={() => setClearing(true)}
                />
                <LatestChecks real={dockerHistory} checks={checkHistory} />
              </div>
            </div>
          </div>
        )}
      </div>

      <SnapshotReview
        snapshot={reviewing}
        onClose={() => setReviewing(null)}
        onRestore={(snapshot) => {
          setReviewing(null);
          setPending({ snapshot, action: "restore" });
        }}
      />

      <ConfirmDialog
        open={!!pending}
        tone="danger"
        icon={deleting ? <Trash2 size={20} /> : <RotateCcw size={20} />}
        title={deleting ? t.history.deleteTitle : t.history.restoreTitle}
        confirmLabel={deleting ? t.history.deleteConfirm : t.history.restore}
        loading={busy}
        onCancel={() => setPending(null)}
        onConfirm={async () => {
          if (!pending) return;
          setBusy(true);
          if (pending.action === "restore") await restoreSnapshot(pending.snapshot.id, conflicts);
          else await deleteSnapshot(pending.snapshot.id);
          setBusy(false);
          setPending(null);
        }}
      >
        {deleting ? t.history.deleteText : t.history.restoreText(pending?.snapshot.files.length ?? 0, pending?.snapshot.label ?? "")}
        {!deleting && conflicts.length > 0 && (
          <div className="dialog-callout">
            <Callout tone="warn" icon={<CircleAlert size={15} />}>
              <strong>{t.history.conflictTitle(conflicts.length)}</strong>
              <ul className="conflict-list">
                {conflicts.map((c) => (
                  <li key={c.file}>
                    <bdi>{c.file}</bdi>
                  </li>
                ))}
              </ul>
              {t.history.conflictText}
            </Callout>
          </div>
        )}
      </ConfirmDialog>

      <ConfirmDialog
        open={clearing}
        tone="danger"
        icon={<Trash2 size={20} />}
        title={t.history.deleteAllTitle(snapshots.length)}
        confirmLabel={t.history.deleteConfirm}
        loading={busy}
        onCancel={() => setClearing(false)}
        onConfirm={async () => {
          setBusy(true);
          await deleteAllSnapshots();
          setBusy(false);
          setClearing(false);
        }}
      >
        {t.history.deleteAllText}
      </ConfirmDialog>
    </>
  );
}

function SnapshotReview({
  snapshot,
  onClose,
  onRestore,
}: {
  snapshot: Snapshot | null;
  onClose: () => void;
  onRestore: (snapshot: Snapshot) => void;
}) {
  const { t } = useI18n();
  const { api, project } = useStore();
  const toast = useToast();
  const [changes, setChanges] = useState<FileChange[] | null>(null);
  const [active, setActive] = useState<string | null>(null);
  const closeRef = useRef(onClose);
  useLayoutEffect(() => {
    closeRef.current = onClose;
  });
  const id = snapshot?.id;
  const root = project?.root;

  useEffect(() => {
    setChanges(null);
    setActive(null);
    if (!id || !api || !root) return;
    let live = true;
    api
      .snapshotChanges(root, id)
      .then((list) => {
        if (!live) return;
        setChanges(list);
        setActive(list[0]?.file ?? null);
      })
      .catch((e) => {
        if (!live) return;
        toast.push({ tone: "error", title: t.history.reviewFailed, description: errorMessage(e, t.common.genericError) });
        closeRef.current();
      });
    return () => {
      live = false;
    };
  }, [id, api, root, toast, t]);

  const current = changes?.find((c) => c.file === active) ?? null;
  const legacy = !!changes?.length && changes.every((c) => c.rules.length === 0);

  return (
    <Sheet
      open={!!snapshot}
      title={snapshot?.label ?? t.history.reviewTitle}
      meta={snapshot && t.history.reviewMeta(formatRelative(snapshot.createdAt, t), snapshot.files.length)}
      onClose={onClose}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            {t.common.close}
          </Button>
          <Button icon={<RotateCcw size={14} />} onClick={() => snapshot && onRestore(snapshot)}>
            {t.history.restore}
          </Button>
        </>
      }
    >
      {!changes ? (
        <div className="sheet__loading">
          <span className="spinner" aria-hidden />
          <span>{t.history.reviewLoading}</span>
        </div>
      ) : (
        <div className="review">
          <nav className="review__files" aria-label={t.changes.files}>
            {changes.map((c) => (
              <button
                key={c.file}
                className="review__file"
                aria-current={c.file === active ? "true" : undefined}
                onClick={() => setActive(c.file)}
                title={c.file}
              >
                <span className="review__name">{c.file}</span>
                <span className="review__stats">
                  <span className="stat-add">+{c.diff.additions}</span>
                  <span className="stat-del">−{c.diff.deletions}</span>
                </span>
              </button>
            ))}
          </nav>
          <section className="review__diff" aria-label={t.changes.details}>
            {legacy && (
              <div className="review__note">
                <Callout tone="neutral" icon={<Info size={15} />}>
                  {t.history.legacyNote}
                </Callout>
              </div>
            )}
            {current && (
              <>
                {current.rules.length > 0 && (
                  <div className="review__rules">
                    <RiskBadge fix={current.fix} />
                    <RuleChips rules={current.rules} />
                  </div>
                )}
                {current.diff.hunks.length ? (
                  <DiffViewer diff={current.diff} file={current.file} />
                ) : (
                  <p className="muted review__empty">{t.history.noDiff}</p>
                )}
              </>
            )}
          </section>
        </div>
      )}
    </Sheet>
  );
}
