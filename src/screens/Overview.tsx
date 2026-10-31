import { useState, type ReactNode } from "react";
import {
  ArrowRight,
  Check,
  CircleAlert,
  Download,
  FileCode2,
  Layers,
  Package,
  Play,
  RotateCw,
  ScanSearch,
  ShieldCheck,
  Sparkles,
  Wrench,
  ShieldAlert,
} from "lucide-react";
import { ConfirmDialog } from "../components/Modal";
import { PageHeader } from "../components/Shell";
import { ScoreRing } from "../components/ScoreRing";
import { Badge, Button, Callout, Card, Progress, RiskBadge, Segmented } from "../components/ui";
import { useI18n } from "../i18n";
import { commonTitle, dependencyBlockers, FRAMEWORK_LABEL, formatDuration, formatNumber, scoreHeadline, scoreTone } from "../lib/labels";
import { useStore } from "../lib/store";
import { BLOCKED_TARGETS, PATCHED, PHP2SHELL_CVES, isBlockedTarget } from "../lib/security";
import { TARGETS, type Finding, type FixKind, type Target } from "../lib/types";

function topRules(findings: Finding[], loc: (f: Finding) => string) {
  const map = new Map<string, { titles: string[]; fix: Finding["fix"]; since: string; count: number }>();
  for (const f of findings) {
    const title = loc(f);
    const cur = map.get(f.ruleId);
    if (cur) {
      cur.count++;
      if (!cur.titles.includes(title)) cur.titles.push(title);
    } else {
      map.set(f.ruleId, { titles: [title], fix: f.fix, since: f.since, count: 1 });
    }
  }
  return [...map.entries()]
    .sort((a, b) => b[1].count - a[1].count)
    .slice(0, 6)
    .map(([id, r]) => [id, { ...r, title: commonTitle(r.titles) }] as const);
}

export function Overview() {
  const { t, loc } = useI18n();
  const { project, target, setTarget, scan, scanning, progress, runScan, deps, setView, showFindings, toolchain, exportReport, preview } =
    useStore();
  const [pendingTarget, setPendingTarget] = useState<Target | null>(null);
  if (!project) return null;

  const chooseTarget = (next: Target) => {
    if (next === target) return;
    if (preview && preview.changes.length > 0) setPendingTarget(next);
    else setTarget(next);
  };

  const pct = progress && progress.total > 0 ? (progress.done / progress.total) * 100 : null;
  const s = scan?.summary;
  const blockers = dependencyBlockers(deps);
  const frameworkBlockers = scan?.findings.filter((f) => f.ruleId.startsWith("framework.") && f.ruleId.endsWith("_version")) ?? [];
  const blockerText = blockers.root
    ? t.overview.rootBlocked(deps?.rootConstraint ?? "", target, blockers.packages)
    : t.overview.packagesBlocked(blockers.packages, target);
  const fmt = (n: number) => formatNumber(n, t.locale);
  const versionHint = project.currentVersion ? t.overview.currentHint(project.currentVersion) : t.overview.currentUnknown;
  const skipped = project.excludedDirs.length > 0 ? t.overview.skipped(project.excludedDirs.join(", ")) : t.overview.skippedVendor;

  return (
    <>
      <PageHeader
        title={t.overview.title}
        subtitle={project.root}
        actions={
          scan && (
            <>
              <Button variant="ghost" size="sm" icon={<Download size={14} />} onClick={exportReport}>
                {t.overview.exportReport}
              </Button>
              <Button variant="secondary" size="sm" icon={<RotateCw size={14} />} onClick={() => runScan()} loading={scanning}>
                {t.overview.rescan}
              </Button>
            </>
          )
        }
      />
      <div className="page-scroll">
        <div className="page-body stack">
          <Card bodyClassName="facts">
            <Fact label={t.overview.platform} value={FRAMEWORK_LABEL[project.framework]} />
            <Fact label={t.overview.phpFiles} value={fmt(project.phpFiles)} />
            <Fact label={t.overview.currentVersion} value={project.currentVersion ? `PHP ${project.currentVersion}` : t.common.unknown} />
            <Fact label={t.overview.constraint} value={project.phpConstraint ?? "—"} mono />
          </Card>

          <Card bodyClassName="target">
            <div className="target__text">
              <h2>{t.overview.targetTitle}</h2>
              <p>
                {versionHint}
                {skipped}
              </p>
            </div>
            <div className="target__controls">
              <Segmented<Target>
                label={t.overview.targetLabel}
                size="lg"
                value={target}
                onChange={chooseTarget}
                disabled={scanning}
                options={TARGETS.map((v) =>
                  isBlockedTarget(v) ? { value: v, label: v, disabled: true, describedBy: "target-blocked" } : { value: v, label: v },
                )}
              />
              {!scan && (
                <Button variant="primary" size="lg" icon={<Play size={16} />} onClick={() => runScan()} loading={scanning}>
                  {t.overview.startScan}
                </Button>
              )}
            </div>
            <p className="target__note" id="target-blocked">
              <ShieldAlert size={14} aria-hidden />
              <span>{t.overview.blockedTarget(BLOCKED_TARGETS.join(", "), PHP2SHELL_CVES, Object.values(PATCHED))}</span>
            </p>
          </Card>

          {scanning && (
            <Card bodyClassName="scan-progress">
              <div className="row">
                <ScanSearch size={16} className="subtle" />
                <strong>{t.overview.scanning(target)}</strong>
              </div>
              <Progress value={pct} label={t.overview.scanProgress} />
              <div className="scan-progress__meta">
                <span className="scan-progress__file">{progress?.file || t.overview.preparing}</span>
                <span className="tabular">
                  {fmt(progress?.done ?? 0)} / {fmt(progress?.total ?? project.phpFiles)}
                </span>
              </div>
            </Card>
          )}

          {!scan && !scanning && (
            <Card bodyClassName="">
              <div className="steps">
                <Step state="active" icon={<ScanSearch size={14} />} title={t.overview.steps.scan.title} desc={t.overview.steps.scan.text} />
                <Step icon={<Sparkles size={14} />} title={t.overview.steps.fix.title} desc={t.overview.steps.fix.text} />
                <Step icon={<Package size={14} />} title={t.overview.steps.deps.title} desc={t.overview.steps.deps.text} />
                <Step icon={<Layers size={14} />} title={t.overview.steps.live.title} desc={t.overview.steps.live.text} badge={t.common.soon} />
              </div>
            </Card>
          )}

          {scan && s && (
            <>
              <Card className="score-card" bodyClassName="score">
                <div className="score__glow" data-tone={scoreTone(s.score)} aria-hidden />
                <ScoreRing score={s.score} />
                <div className="score__text">
                  <div className="score__meta">
                    <Badge tone="accent">PHP {scan.target}</Badge>
                    <span>
                      {t.overview.scanMeta(fmt(scan.filesScanned), fmt(scan.linesScanned), formatDuration(scan.durationMs, t.locale))}
                    </span>
                  </div>
                  <h2>{scoreHeadline(s.score, s.total, t)}</h2>
                  <p>{s.total === 0 ? t.overview.clean : t.overview.summary(s.total, s.filesAffected, s.auto + s.likely)}</p>
                </div>
                {s.total > 0 && (
                  <div className="fix-tiles">
                    {(["auto", "likely", "manual"] as const).map((kind) => (
                      <FixTile key={kind} kind={kind} n={s[kind]} total={s.total} onClick={() => showFindings({ fix: kind })} />
                    ))}
                  </div>
                )}
              </Card>

              {frameworkBlockers.map((f) => {
                const [lead, ...rest] = loc(f.message).split(/(?<=\.) /);
                return (
                  <Callout key={f.id} tone={f.severity === "breaking" ? "danger" : "warn"} icon={<CircleAlert size={16} />}>
                    <strong>{lead}</strong> {rest.join(" ")}
                  </Callout>
                );
              })}

              <div className="grid-4">
                <Metric label={t.severity.breaking} value={s.breaking} hint={t.overview.metrics.breakingHint} tone="danger" onClick={() => showFindings({ severity: "breaking" })} />
                <Metric label={t.severity.deprecated} value={s.deprecated} hint={t.overview.metrics.deprecatedHint} tone="manual" onClick={() => showFindings({ severity: "deprecated" })} />
                <Metric label={t.severity.behavior} value={s.behavior} hint={t.overview.metrics.behaviorHint} tone="neutral" onClick={() => showFindings({ severity: "behavior" })} />
                <Metric
                  label={t.overview.metrics.blockers}
                  value={deps ? blockers.total : "—"}
                  hint={
                    blockers.root
                      ? t.overview.metrics.rootIncluded
                      : deps?.hasLockFile
                        ? t.overview.metrics.ofPackages(deps.packages.length)
                        : t.overview.metrics.noLock
                  }
                  tone={!deps ? "neutral" : blockers.total > 0 ? "danger" : "auto"}
                  onClick={() => setView("dependencies")}
                />
              </div>

              <div className="grid-2">
                <Card
                  title={t.overview.commonIssues}
                  actions={
                    s.total > 0 && (
                      <Button variant="ghost" size="sm" onClick={() => showFindings()}>
                        {t.overview.viewAll} <ArrowRight size={13} />
                      </Button>
                    )
                  }
                  bodyClassName="rule-list"
                >
                  {s.total === 0 ? (
                    <div className="card__body muted">{t.overview.noIssues}</div>
                  ) : (
                    topRules(scan.findings, (f) => loc(f.title)).map(([id, r]) => (
                      <button key={id} type="button" className="rule-row" onClick={() => showFindings({ rule: id })}>
                        <span className="rule-row__title">{r.title}</span>
                        <Badge mono>PHP {r.since}</Badge>
                        <RiskBadge fix={r.fix} />
                        <span className="rule-row__count">{r.count}</span>
                      </button>
                    ))
                  )}
                </Card>

                <Card title={t.overview.nextSteps} bodyClassName="">
                  <div className="steps steps--card">
                    <Step state="done" icon={<Check size={14} />} title={t.overview.scanDone} desc={t.overview.classified(s.total)} />
                    {frameworkBlockers.map((f) => (
                      <Step
                        key={f.id}
                        state="active"
                        icon={<CircleAlert size={14} />}
                        title={t.overview.upgradeFramework(loc(f.title).replace(/\s+[\d.]+$/, ""))}
                        desc={f.replacement ? loc(f.replacement) : loc(f.message)}
                        action={
                          <Button size="sm" variant="ghost" onClick={() => showFindings({ rule: f.ruleId })}>
                            {t.overview.inspect}
                          </Button>
                        }
                      />
                    ))}
                    <Step
                      state={preview ? "done" : "active"}
                      icon={preview ? <Check size={14} /> : <Sparkles size={14} />}
                      title={t.overview.reviewFixes}
                      desc={toolchain && !toolchain.rector ? t.overview.installRectorFirst : t.overview.reviewFixesText}
                      action={
                        <Button size="sm" variant={preview ? "secondary" : "primary"} onClick={() => setView("changes")}>
                          {t.nav.fixes}
                        </Button>
                      }
                    />
                    <Step
                      state={blockers.total > 0 ? "active" : "done"}
                      icon={blockers.total > 0 ? <CircleAlert size={14} /> : <Check size={14} />}
                      title={t.overview.updateDeps}
                      desc={blockers.total > 0 ? blockerText : t.overview.noBlockers}
                      action={
                        <Button size="sm" variant="ghost" onClick={() => setView("dependencies")}>
                          {t.overview.inspect}
                        </Button>
                      }
                    />
                    <Step
                      state={s.manual > 0 ? undefined : "done"}
                      icon={s.manual > 0 ? <FileCode2 size={14} /> : <Check size={14} />}
                      title={t.overview.resolveManual}
                      desc={t.overview.manualLeft(s.manual)}
                      action={
                        s.manual > 0 && (
                          <Button size="sm" variant="ghost" onClick={() => showFindings({ fix: "manual" })}>
                            {t.overview.inspect}
                          </Button>
                        )
                      }
                    />
                  </div>
                </Card>
              </div>

              {scan.errors.length > 0 && (
                <Callout tone="warn" icon={<CircleAlert size={16} />}>
                  {t.overview.unreadable(
                    scan.errors.length,
                    scan.errors.slice(0, 3).map((e) => e.file).join(", ") + (scan.errors.length > 3 ? "…" : ""),
                  )}
                </Callout>
              )}
            </>
          )}
        </div>
      </div>

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
    </>
  );
}

function Fact({ label, value, mono = false }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="fact">
      <div className="fact__label">{label}</div>
      <div className={`fact__value ${mono ? "mono" : "tabular"}`} title={value}>
        {value}
      </div>
    </div>
  );
}

const FIX_TILE_ICON = {
  auto: <Sparkles size={14} />,
  likely: <ShieldCheck size={14} />,
  manual: <Wrench size={14} />,
} as const;

function FixTile({ kind, n, total, onClick }: { kind: FixKind; n: number; total: number; onClick: () => void }) {
  const { t } = useI18n();
  const share = total ? n / total : 0;
  return (
    <button type="button" className="fix-tile" data-kind={kind} onClick={onClick} title={t.fixDesc[kind]}>
      <span className="fix-tile__icon" aria-hidden>
        {FIX_TILE_ICON[kind]}
      </span>
      <span className="fix-tile__label">{t.fix[kind]}</span>
      <span className="fix-tile__count tabular">{n}</span>
      <span className="fix-tile__bar" aria-hidden>
        <span style={{ width: `${share * 100}%` }} />
      </span>
      <span className="fix-tile__pct tabular">
        {new Intl.NumberFormat(t.locale, { style: "percent" }).format(share)}
      </span>
    </button>
  );
}

function Metric({
  label,
  value,
  hint,
  tone,
  onClick,
}: {
  label: string;
  value: number | string;
  hint: string;
  tone: "danger" | "manual" | "neutral" | "auto";
  onClick: () => void;
}) {
  return (
    <button type="button" className="card card--interactive metric" onClick={onClick}>
      <div className="metric__label">
        <span className={`dot dot--${tone}`} aria-hidden />
        {label}
      </div>
      <div className="metric__value">{value}</div>
      <div className="metric__hint">{hint}</div>
    </button>
  );
}

function Step({
  state,
  icon,
  title,
  desc,
  action,
  badge,
}: {
  state?: "done" | "active";
  icon: ReactNode;
  title: string;
  desc: string;
  action?: ReactNode;
  badge?: string;
}) {
  return (
    <div className="step">
      <span className={`step__icon ${state ? `step__icon--${state}` : ""}`}>{icon}</span>
      <div className="step__text">
        <div className="step__title">
          {title} {badge && <Badge>{badge}</Badge>}
        </div>
        <div className="step__desc">{desc}</div>
      </div>
      {action}
    </div>
  );
}
