import { useLayoutEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import { Check, CircleAlert, CircleStop, Columns3, Download, MoreHorizontal, Play, RotateCw, ScanSearch, ShieldCheck } from "lucide-react";
import { ConfirmDialog, Sheet } from "../components/Modal";
import { usePageTitle } from "../components/Shell";
import { ActionMenu, Button, Callout, Card, EmptyState, Progress, Segmented, Select, Tooltip } from "../components/ui";
import { useI18n } from "../i18n";
import { commonTitle, dependencyBlockers, formatDuration, formatNumber, isSettled, readiness, scoreHeadline, scoreTone, firstFew } from "../lib/labels";
import { useStore } from "../lib/store";
import { BLOCKED_TARGETS, PATCHED, PHP2SHELL_CVES, isBlockedTarget } from "../lib/security";
import { TARGETS, type Finding, type Severity, type Target, type TargetSummary } from "../lib/types";
import { squarify, type Tile } from "../lib/treemap";
import { useToast } from "../components/Toast";
import { errorMessage } from "../lib/api";

function topRules(findings: Finding[], loc: (f: Finding) => string) {
  const map = new Map<string, { titles: string[]; severity: Severity; count: number }>();
  for (const f of findings) {
    const title = loc(f);
    const cur = map.get(f.ruleId);
    if (cur) {
      cur.count++;
      if (!cur.titles.includes(title)) cur.titles.push(title);
    } else {
      map.set(f.ruleId, { titles: [title], severity: f.severity, count: 1 });
    }
  }
  return [...map.entries()]
    .sort((a, b) => b[1].count - a[1].count)
    .slice(0, 5)
    .map(([id, r]) => [id, { ...r, title: commonTitle(r.titles) }] as const);
}

export interface FileStat {
  file: string;
  value: number;
  breaking: number;
  deprecated: number;
  behavior: number;
}

export function fileStats(findings: Finding[]): FileStat[] {
  const files = new Map<string, FileStat>();
  for (const f of findings) {
    const stat = files.get(f.file) ?? { file: f.file, value: 0, breaking: 0, deprecated: 0, behavior: 0 };
    stat.value++;
    stat[f.severity]++;
    files.set(f.file, stat);
  }
  return [...files.values()].sort((a, b) => b.value - a.value || a.file.localeCompare(b.file));
}

export function filesHoldingHalf(files: FileStat[]): number {
  const total = files.reduce((a, f) => a + f.value, 0);
  let sum = 0;
  for (let i = 0; i < files.length; i++) {
    sum += files[i].value;
    if (sum * 2 >= total) return i + 1;
  }
  return files.length;
}

const MAX_TILES = 36;

export function Overview() {
  const { t, loc } = useI18n();
  const { dockerStep, project, target, setTarget, scan, scanning, progress, runScan, cancelScan, deps, setView, showFindings, exportReport, preview, dockerHistory } = useStore();
  const [pendingTarget, setPendingTarget] = useState<Target | null>(null);
  const [comparing, setComparing] = useState(false);
  usePageTitle(t.overview.title);
  if (!project) return null;

  const chooseTarget = (next: Target) => {
    if (next === target) return;
    if (preview && preview.changes.length > 0) setPendingTarget(next);
    else setTarget(next);
  };

  const pct = progress && progress.total > 0 ? (progress.done / progress.total) * 100 : null;
  const fmt = (n: number) => formatNumber(n, t.locale);
  const versionHint = project.currentVersion ? t.overview.currentHint(project.currentVersion) : t.overview.currentUnknown;
  const skipped = project.excludedDirs.length > 0 ? t.overview.skipped(project.excludedDirs.join(", ")) : t.overview.skippedVendor;
  const blockedText = t.overview.blockedTarget(BLOCKED_TARGETS.join(", "), PHP2SHELL_CVES, Object.values(PATCHED));

  const dialogs = (
    <>
      <ConfirmDialog
        open={pendingTarget !== null}
        icon={<RotateCw size={20} />}
        title={t.overview.switchTitle(pendingTarget ?? target)}
        confirmLabel={t.overview.switchConfirm}
        onCancel={() => setPendingTarget(null)}
        onConfirm={() => {
          if (pendingTarget) setTarget(pendingTarget);
          setPendingTarget(null);
        }}
      >
        {t.overview.switchText(preview?.changes.length ?? 0)}
      </ConfirmDialog>
      <Sheet open={comparing} title={t.compare.title} onClose={() => setComparing(false)}>
        <CompareCard
          root={project.root}
          current={target}
          onPick={(next) => {
            setComparing(false);
            chooseTarget(next);
          }}
        />
      </Sheet>
    </>
  );

  if (scanning || !scan) {
    return (
      <>
        <div className="overview-start" data-tauri-drag-region>
          <h1 className="sr-only">{t.overview.title}</h1>
          {scanning ? (
            <div className="overview-scan" role="status">
              <div className="row">
                <ScanSearch size={16} className="subtle" />
                <strong>{t.overview.scanning(target)}</strong>
                <Button variant="ghost" size="sm" icon={<CircleStop size={14} />} onClick={cancelScan} className="scan-progress__stop">
                  {t.overview.stopScan}
                </Button>
              </div>
              <Progress value={pct} label={t.overview.scanProgress} />
              <div className="scan-progress__meta">
                <span className="scan-progress__file">{progress?.file || t.overview.preparing}</span>
                <span className="tabular">
                  {fmt(progress?.done ?? 0)} / {fmt(progress?.total ?? project.phpFiles)}
                </span>
              </div>
            </div>
          ) : (
            <>
              <h2 className="page-title">{t.overview.targetTitle}</h2>
              <p className="page-lead">
                {versionHint}
                {skipped}
              </p>
              <Segmented<Target>
                label={t.overview.targetLabel}
                size="lg"
                value={target}
                onChange={chooseTarget}
                disabled={!!dockerStep}
                options={TARGETS.map((v) => (isBlockedTarget(v) ? { value: v, label: v, disabled: true, tip: blockedText } : { value: v, label: v }))}
              />
              <Button variant="primary" size="lg" icon={<Play size={16} />} onClick={() => runScan()}>
                {t.overview.startScan}
              </Button>
            </>
          )}
        </div>
        {dialogs}
      </>
    );
  }

  const s = scan.summary;
  const ready = readiness(scan, deps, dockerHistory);
  const settled = isSettled(ready);
  const baseTone = scoreTone(s.score);
  const tone = ready.blockers > 0 && baseTone === "auto" ? "manual" : baseTone;
  const blockers = dependencyBlockers(deps);
  const frameworkBlockers = scan.findings.filter((f) => f.ruleId.startsWith("framework.") && f.ruleId.endsWith("_version"));
  const files = fileStats(scan.findings);
  const half = filesHoldingHalf(files);
  const concentrated = files.length >= 3 && half <= 6 && half < files.length;
  const heaviest = files[0];
  const heaviestName = heaviest?.file.slice(heaviest.file.lastIndexOf("/") + 1);
  const r = t.overview.readiness;
  const depsText = {
    blocked: blockers.root ? t.overview.rootBlocked(deps?.rootConstraint ?? "", target, blockers.packages) : t.overview.packagesBlocked(blockers.packages, target),
    unknown: r.depsUnknown(ready.deps.count),
    verified: r.depsVerified,
    missing: r.depsMissing,
    unreadable: r.depsUnreadable,
    pending: r.depsPending,
  }[ready.deps.state];
  const steps: { key: string; title: string; state?: "done" | "next" | "blocked"; count?: number; hint: string; onClick: () => void }[] = [
    {
      key: "scan",
      title: t.overview.stepScan,
      state: ready.unscanned ? "blocked" : "done",
      count: ready.unscanned || undefined,
      hint: ready.unscanned ? r.coveragePartial(ready.unscanned) : t.overview.scanMeta(fmt(scan.filesScanned), fmt(scan.linesScanned), formatDuration(scan.durationMs, t.locale)),
      onClick: () => (ready.unscanned ? document.getElementById("unscanned")?.scrollIntoView({ block: "center" }) : showFindings()),
    },
    ...frameworkBlockers.map((f) => ({
      key: f.id,
      title: t.overview.upgradeFramework(loc(f.title).replace(/\s+[\d.]+$/, "")),
      state: "blocked" as const,
      hint: f.replacement ? loc(f.replacement) : loc(f.message),
      onClick: () => showFindings({ rule: f.ruleId }),
    })),
    {
      key: "fixes",
      title: t.nav.fixes,
      state: "next",
      count: s.auto + s.likely || undefined,
      hint: t.overview.reviewFixesText,
      onClick: () => setView("changes"),
    },
    {
      key: "deps",
      title: t.nav.dependencies,
      state: ready.deps.state === "blocked" ? "blocked" : ready.deps.state === "verified" ? "done" : undefined,
      count: ready.deps.count || undefined,
      hint: depsText,
      onClick: () => setView("dependencies"),
    },
    {
      key: "real",
      title: t.nav.runtime,
      state: ready.docker === "passed" ? "done" : ready.docker === "failed" ? "blocked" : undefined,
      hint: r.dockerState[ready.docker],
      onClick: () => setView("runtime"),
    },
  ];

  return (
    <>
      <h1 className="sr-only">{t.overview.title}</h1>
      <div className="map-page">
        <div className="map-rail">
          <div className="map-bar" data-tauri-drag-region>
            <Select<Target>
              className="overview-target"
              label={t.overview.targetLabel}
              value={target}
              icon={<span className="overview-target__hint">{t.overview.targetShort}</span>}
              disabled={!!dockerStep} title={dockerStep ? t.overview.busyRealTest : undefined}
              options={TARGETS.map((v) => (isBlockedTarget(v) ? { value: v, label: `PHP ${v}`, disabled: true, hint: t.overview.blockedHint } : { value: v, label: `PHP ${v}` }))}
              onChange={chooseTarget}
            />
            <Tooltip className="map-bar__push" place="down" text={`${t.overview.rescan} · ${t.overview.scanMeta(fmt(scan.filesScanned), fmt(scan.linesScanned), formatDuration(scan.durationMs, t.locale))}`}>
              <Button variant="ghost" iconOnly onClick={() => runScan()}>
                <RotateCw size={15} aria-hidden />
                <span className="sr-only">{t.overview.rescan}</span>
              </Button>
            </Tooltip>
            <ActionMenu
              label={t.overview.more}
              icon={<MoreHorizontal size={15} aria-hidden />}
              items={[
                { label: t.overview.exportReport, icon: <Download size={14} aria-hidden />, onClick: exportReport },
                { label: t.compare.title, icon: <Columns3 size={14} aria-hidden />, onClick: () => setComparing(true) },
              ]}
            />
          </div>

          <Tooltip align="start" text={t.overview.scoreNote} className="overview-score-tip">
            <span className={`badge badge--${tone} overview-score`} role="img" tabIndex={0} aria-label={t.scoreLabel(s.score)} data-score={s.score}>
              {t.overview.scorePill(s.score)}
            </span>
          </Tooltip>
          <h2 className="page-title map-rail__title">{concentrated ? t.overview.concentrated(half) : scoreHeadline(s.score, s.total, t, scan.errors.length > 0, settled, ready.ignoredBreaking)}</h2>
          <p className="page-lead">
            {heaviest ? (
              <>
                {t.overview.heaviest} <code>{heaviestName}</code>: {t.overview.heaviestCount(heaviest.value, heaviest.breaking)}
              </>
            ) : scan.errors.length > 0 ? (
              t.overview.cleanPartial
            ) : (
              t.overview.clean
            )}
          </p>
          {s.total > 0 && (
            <div>
              <Button variant="primary" size="lg" onClick={() => setView("changes")}>
                {t.overview.startFixes}
              </Button>
            </div>
          )}

          {s.total > 0 && (
            <ul className="map-counts">
              {(["breaking", "deprecated", "behavior"] as const).map((severity) => (
                <li key={severity}>
                  <button type="button" onClick={() => showFindings({ severity })}>
                    <span className="map-counts__name">
                      <span className={`dot dot--${severity}`} aria-hidden />
                      {t.severity[severity]}
                    </span>
                    <span className="map-counts__value">{fmt(s[severity])}</span>
                  </button>
                </li>
              ))}
            </ul>
          )}

          {s.total > 0 && (
            <section className="slot" aria-label={t.overview.commonIssues}>
              <div className="slot__head">
                <h3 className="slot__title">{t.overview.commonIssues}</h3>
                <button type="button" className="link" onClick={() => showFindings()}>
                  {t.overview.viewAll}
                </button>
              </div>
              <div className="slot__list">
                {topRules(scan.findings, (f) => loc(f.title)).map(([id, rule]) => (
                  <button key={id} type="button" className="krow" onClick={() => showFindings({ rule: id })}>
                    <span className={`dot dot--${rule.severity}`} aria-hidden />
                    <span className="krow__name">{rule.title}</span>
                    <span className="krow__value">{rule.count}</span>
                  </button>
                ))}
              </div>
            </section>
          )}

          <ol className="map-steps" aria-label={t.overview.stepsLabel}>
            {steps.map((step, i) => (
              <li key={step.key} data-state={step.state}>
                <button type="button" title={step.hint} onClick={step.onClick}>
                  <span className="chain__mark" aria-hidden>
                    {step.state === "done" ? <Check size={10} strokeWidth={2.6} /> : i + 1}
                  </span>
                  <span className="map-steps__title">{step.title}</span>
                  {step.count !== undefined && <small>{fmt(step.count)}</small>}
                </button>
              </li>
            ))}
          </ol>
        </div>

        <div className="map-main">
          {scan.errors.length > 0 && (
            <div id="unscanned">
              <Callout tone="warn" icon={<CircleAlert size={16} />}>
                {t.overview.unreadable(scan.errors.length, firstFew(scan.errors.map((e) => e.file), 3))}
              </Callout>
            </div>
          )}
          {frameworkBlockers.map((f) => {
            const [lead, ...rest] = loc(f.message).split(/(?<=\.) /);
            return (
              <Callout key={f.id} tone={f.severity === "breaking" ? "danger" : "warn"} icon={<CircleAlert size={16} />}>
                <strong>{lead}</strong> {rest.join(" ")}
              </Callout>
            );
          })}
          {files.length > 0 ? (
            <FileMap files={files} onOpen={(file) => showFindings(file ? { query: file } : {})} />
          ) : (
            <EmptyState icon={<ShieldCheck size={24} />} title={t.overview.noIssues} />
          )}
        </div>
      </div>
      {dialogs}
    </>
  );
}

type MapItem = { value: number; stat: FileStat | null; files: number };

function FileMap({ files, onOpen }: { files: FileStat[]; onOpen: (file: string | null) => void }) {
  const { t } = useI18n();
  const box = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ width: 720, height: 720 });
  useLayoutEffect(() => {
    const el = box.current;
    if (!el) return;
    const read = () => {
      if (el.clientWidth > 0 && el.clientHeight > 0) setSize({ width: el.clientWidth, height: el.clientHeight });
    };
    read();
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(read);
    observer.observe(el);
    return () => observer.disconnect();
  }, []);
  const tiles = useMemo(() => {
    const rest = files.slice(MAX_TILES);
    const items: MapItem[] = files.slice(0, MAX_TILES).map((stat) => ({ value: stat.value, stat, files: 1 }));
    if (rest.length > 0) items.push({ value: rest.reduce((a, f) => a + f.value, 0), stat: null, files: rest.length });
    return squarify(items, size.width, size.height);
  }, [files, size]);
  return (
    <div className="fmap" ref={box} role="group" aria-label={t.overview.mapLabel}>
      {tiles.map((tile) => (
        <FileTile key={tile.item.stat?.file ?? "rest"} tile={tile} size={size} onOpen={onOpen} />
      ))}
    </div>
  );
}

function FileTile({ tile, size, onOpen }: { tile: Tile<MapItem>; size: { width: number; height: number }; onOpen: (file: string | null) => void }) {
  const { t } = useI18n();
  const { stat, value, files } = tile.item;
  const big = tile.width >= 150 && tile.height >= 96;
  const mid = tile.width >= 92 && tile.height >= 58;
  const cut = stat ? stat.file.lastIndexOf("/") + 1 : 0;
  const parts = stat
    ? (["breaking", "deprecated", "behavior"] as const)
        .filter((k) => stat[k] > 0)
        .map((k) => `${stat[k]} ${t.severity[k].toLocaleLowerCase(t.locale)}`)
        .join(", ")
    : "";
  const label = stat ? `${stat.file} · ${t.overview.fileSummary(value, parts)}` : t.overview.moreFiles(files);
  const center = tile.x + tile.width / 2;
  const tip = ["tip__box", tile.y < 60 && "tip__box--down", center < 130 ? "tip__box--start" : center > size.width - 130 ? "tip__box--end" : false].filter(Boolean).join(" ");
  const style: CSSProperties = {
    left: `${(tile.x / size.width) * 100}%`,
    top: `${(tile.y / size.height) * 100}%`,
    width: `${(tile.width / size.width) * 100}%`,
    height: `${(tile.height / size.height) * 100}%`,
  };
  const heat = stat ? (0.34 * stat.breaking) / value : 0;
  return (
    <button type="button" className={`fmap__tile tip ${big ? "fmap__tile--big" : mid ? "" : "fmap__tile--tiny"}`} style={style} aria-label={label} onClick={() => onOpen(stat?.file ?? null)}>
      <span className="fmap__body" style={{ "--heat": heat } as CSSProperties}>
        {big && stat && <span className="fmap__dir">{stat.file.slice(0, cut)}</span>}
        {(big || mid) && <span className="fmap__name">{stat ? stat.file.slice(cut) : t.overview.moreFiles(files)}</span>}
        <span className="fmap__count">{value}</span>
        {stat && (
          <span className="fmap__mix" aria-hidden>
            {(["breaking", "deprecated", "behavior"] as const)
              .filter((k) => stat[k] > 0)
              .map((k) => (
                <i key={k} className={`dot--${k}`} style={{ flex: `${stat[k]} 1 0` }} />
              ))}
          </span>
        )}
      </span>
      <span className={tip} role="tooltip">
        {label}
      </span>
    </button>
  );
}

function CompareCard({ root, current, onPick }: { root: string; current: Target; onPick: (t: Target) => void }) {
  const { t } = useI18n();
  const { api, scan } = useStore();
  const toast = useToast();
  const [rows, setRows] = useState<{ root: string; scan: unknown; list: TargetSummary[] } | null>(null);
  const [loading, setLoading] = useState(false);
  const list = rows?.root === root && rows.scan === scan ? rows.list : null;
  const run = async () => {
    if (!api) return;
    setLoading(true);
    try {
      setRows({ root, scan, list: await api.compareTargets(root) });
    } catch (e) {
      toast.push({ tone: "error", title: t.compare.failed, description: errorMessage(e, t.common.genericError) });
    } finally {
      setLoading(false);
    }
  };
  return (
    <Card bodyClassName="">
      {!list ? (
        <div className="compare__intro">
          <p className="muted">{t.compare.text}</p>
          <Button variant="secondary" size="sm" icon={<Columns3 size={14} />} loading={loading} onClick={run}>
            {t.compare.run}
          </Button>
        </div>
      ) : (
        <table className="table" aria-label={t.compare.title}>
          <thead>
            <tr>
              <th>{t.compare.target}</th>
              <th className="table__num">{t.compare.score}</th>
              <th className="table__num">{t.severity.breaking}</th>
              <th className="table__num">{t.severity.deprecated}</th>
              <th className="table__num">{t.fix.manual}</th>
              <th className="table__num">{t.compare.packages}</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {list.map((r) => {
              const target = r.target as Target;
              return (
                <tr key={r.target} aria-current={target === current ? "true" : undefined} className={target === current ? "compare__current" : undefined}>
                  <td className="mono">
                    PHP {r.target}
                    {r.incomplete && !r.error && <div className="subtle">{t.compare.incomplete}</div>}
                  </td>
                  {r.error ? (
                    <td colSpan={5} className="subtle">
                      {r.error}
                    </td>
                  ) : (
                    <>
                      <td className="table__num tabular">{r.summary.score}</td>
                      <td className="table__num tabular">{r.summary.breaking}</td>
                      <td className="table__num tabular">{r.summary.deprecated}</td>
                      <td className="table__num tabular">{r.summary.manual}</td>
                      <td className="table__num tabular">
                        {r.blockingPackages}
                        {r.rootBlocks && <span className="subtle"> {t.compare.plusRoot}</span>}
                      </td>
                    </>
                  )}
                  <td className="table__num">
                    {target === current ? (
                      <span className="subtle">{t.compare.current}</span>
                    ) : (
                      <Button size="sm" variant="ghost" onClick={() => onPick(target)}>
                        {t.compare.pick}
                      </Button>
                    )}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      )}
    </Card>
  );
}
