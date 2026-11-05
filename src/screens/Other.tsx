import { useEffect, useLayoutEffect, useRef, useState, type ReactElement } from "react";
import {
  Bot,
  Check,
  CircleAlert,
  CircleCheck,
  CircleHelp,
  Eye,
  FileClock,
  FolderOpen,
  History as HistoryIcon,
  Info,
  Monitor,
  Moon,
  Package,
  RefreshCw,
  RotateCcw,
  Search,
  Server,
  Sun,
  Trash2,
  ShieldAlert,
} from "lucide-react";
import { DiffViewer } from "../components/DiffViewer";
import { ConfirmDialog, Sheet } from "../components/Modal";
import { PageHeader } from "../components/Shell";
import { Badge, Button, Callout, Card, CommandLine, EmptyState, RiskBadge, RuleChips, Segmented } from "../components/ui";
import { PHP2SHELL_CVES, isVulnerable, patchedFor } from "../lib/security";
import { useToast } from "../components/Toast";
import { LANGS, useI18n, type Lang } from "../i18n";
import { errorMessage } from "../lib/api";
import { formatBytes, formatRelative } from "../lib/labels";
import { useStore, type Theme } from "../lib/store";
import { compareProbe, parseProbe } from "../lib/probe";
import type { BackupUsage, DockerImageUsage, DependencyReport, DependencyStatus, ExtensionUse, FileChange, Settings, Snapshot, SnapshotConflict, VersionSuggestion, WpLatest } from "../lib/types";

const DEP_TONE: Record<DependencyStatus, "danger" | "auto" | "neutral"> = {
  incompatible: "danger",
  compatible: "auto",
  unknown: "neutral",
};

const DEP_ORDER: Record<DependencyStatus, number> = { incompatible: 0, unknown: 1, compatible: 2 };

const DEP_ICON: Record<DependencyStatus, ReactElement> = {
  incompatible: <CircleAlert size={11} />,
  compatible: <CircleCheck size={11} />,
  unknown: <CircleHelp size={11} />,
};

function DepBadge({ status }: { status: DependencyStatus }) {
  const { t } = useI18n();
  return (
    <Badge tone={DEP_TONE[status]}>
      {DEP_ICON[status]}
      {t.dep[status]}
    </Badge>
  );
}

export function Dependencies() {
  const { t, loc } = useI18n();
  const { deps, scan, runScan, scanning, target, api } = useStore();
  const [showDev, setShowDev] = useState(true);
  const [suggestions, setSuggestions] = useState<{ for: DependencyReport; list: VersionSuggestion[] } | null>(null);
  const [looking, setLooking] = useState(false);
  const [latest, setLatest] = useState<{ for: DependencyReport; list: WpLatest[] } | null>(null);
  const [checkingWp, setCheckingWp] = useState(false);
  const toast = useToast();

  if (!deps) {
    return (
      <>
        <PageHeader title={t.deps.title} />
        <EmptyState
          icon={<Package size={24} />}
          title={t.deps.notChecked}
          action={
            !scan && (
              <Button variant="primary" onClick={() => runScan()} loading={scanning}>
                {t.overview.startScan}
              </Button>
            )
          }
        >
          {t.deps.notCheckedText(target)}
        </EmptyState>
      </>
    );
  }

  const packages = deps.packages
    .filter((p) => showDev || !p.dev)
    .sort((a, b) => DEP_ORDER[a.status] - DEP_ORDER[b.status] || Number(a.dev) - Number(b.dev) || a.name.localeCompare(b.name));
  const incompatible = packages.filter((p) => p.status === "incompatible");
  const bad = incompatible.length;
  const unknown = packages.filter((p) => p.status === "unknown").length;
  const command = `composer update ${incompatible.map((p) => p.name).join(" ")} --with-all-dependencies`;
  const found = suggestions?.for === deps ? suggestions.list : null;
  const suggestionFor = (name: string) => found?.find((s) => s.name === name);
  const findVersions = async () => {
    if (!api) return;
    setLooking(true);
    try {
      const list = await api.findCompatibleVersions(deps.target, incompatible.map((p) => ({ name: p.name, version: p.version })));
      setSuggestions({ for: deps, list });
    } catch (e) {
      toast.push({ tone: "error", title: t.deps.findFailed, description: errorMessage(e, t.common.genericError) });
    } finally {
      setLooking(false);
    }
  };
  const wpLatest = latest?.for === deps ? latest.list : null;
  const breakingIn = (dir: string) => scan?.findings.filter((f) => f.severity === "breaking" && f.file.startsWith(`${dir}/`)).length ?? 0;
  const checkWordpress = async () => {
    if (!api) return;
    setCheckingWp(true);
    try {
      setLatest({ for: deps, list: await api.wordpressLatest(deps.wordpress.map((c) => ({ kind: c.kind, slug: c.slug }))) });
    } catch (e) {
      toast.push({ tone: "error", title: t.deps.wpFailed, description: errorMessage(e, t.common.genericError) });
    } finally {
      setCheckingWp(false);
    }
  };
  const undeclared = deps.extensions.filter((e) => !e.declared && !e.removed).map((e) => `"ext-${e.name}": "*"`);

  return (
    <>
      <PageHeader
        title={t.deps.title}
        subtitle={t.deps.subtitle(deps.target, deps.packages.length)}
        actions={
          <Segmented
            label={t.deps.packageType}
            value={showDev ? "all" : "prod"}
            onChange={(v) => setShowDev(v === "all")}
            options={[
              { value: "all", label: t.common.all },
              { value: "prod", label: t.deps.productionOnly },
            ]}
          />
        }
      />
      <div className="page-scroll">
        <div className="page-body stack">
          {deps.rootStatus === "incompatible" && (
            <Callout tone="danger" icon={<CircleAlert size={16} />}>
              <strong>{t.deps.rootBlockedTitle(deps.target)}</strong>{" "}
              {deps.rootConflict && deps.rootConstraint === null
                ? t.deps.rootConflictText(deps.rootConflict, deps.target)
                : deps.rootConflict
                  ? `${t.deps.rootBlockedText(deps.rootConstraint ?? "", deps.target)} ${t.deps.rootConflictText(deps.rootConflict, deps.target)}`
                  : t.deps.rootBlockedText(deps.rootConstraint ?? "", deps.target)}
            </Callout>
          )}
          {deps.lockUnreadable ? (
            <Callout tone="warn" icon={<CircleAlert size={16} />}>
              {t.deps.lockUnreadable}
            </Callout>
          ) : !deps.hasLockFile ? (
            <Callout tone="warn" icon={<CircleAlert size={16} />}>
              {t.deps.noLock}
            </Callout>
          ) : bad > 0 ? (
            <Callout tone="danger" icon={<CircleAlert size={16} />}>
              <strong>{t.deps.badTitle(bad, deps.target)}</strong> {t.deps.badText}
              <CommandLine command={command} />
              {!found && api && (
                <div className="row row--wrap dep-lookup">
                  <Button size="sm" variant="secondary" icon={<Search size={13} />} loading={looking} onClick={findVersions}>
                    {t.deps.findVersions}
                  </Button>
                  <span className="subtle">{t.deps.findVersionsNote}</span>
                </div>
              )}
            </Callout>
          ) : unknown > 0 ? (
            <Callout tone="warn" icon={<CircleAlert size={16} />}>
              {t.deps.someUnknown(unknown, deps.target)}
            </Callout>
          ) : (
            <Callout tone="info" icon={<CircleCheck size={16} />}>
              {t.deps.allGood(deps.target)}
            </Callout>
          )}

          {deps.rootConstraint && (
            <Card bodyClassName="card__body row">
              <span className="muted">{t.deps.rootConstraint}</span>
              <code>{deps.rootConstraint}</code>
              <span className="push-end">
                <DepBadge status={deps.rootStatus} />
              </span>
            </Card>
          )}

          {packages.length > 0 && (
            <Card bodyClassName="">
              <table className="table">
                <thead>
                  <tr>
                    <th>{t.deps.package}</th>
                    <th>{t.deps.version}</th>
                    <th>{t.deps.requirement}</th>
                    <th className="table__status">{t.deps.status}</th>
                  </tr>
                </thead>
                <tbody>
                  {packages.map((p) => (
                    <tr key={p.name} data-status={p.status}>
                      <td>
                        <span className="mono selectable">{p.name}</span> {p.dev && <Badge>dev</Badge>}
                      </td>
                      <td className="mono subtle">{p.version}</td>
                      <td>
                        <span className="mono">{p.phpConstraint ?? <span className="subtle">{t.deps.unspecified}</span>}</span>
                        {p.note && <div className="dep-note">{loc(p.note)}</div>}
                        {found && p.status === "incompatible" && (
                          <div className="dep-note">
                            {suggestionFor(p.name) ? (
                              <>
                                {t.deps.compatibleFrom(suggestionFor(p.name)!.version)}
                                {suggestionFor(p.name)!.majorChange && <Badge tone="manual">{t.deps.majorChange}</Badge>}
                              </>
                            ) : (
                              t.deps.noneFound
                            )}
                          </div>
                        )}
                      </td>
                      <td>
                        <DepBadge status={p.status} />
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </Card>
          )}

          {deps.wordpress.length > 0 && (
            <Card title={t.deps.wpTitle} bodyClassName="">
              <p className="card__intro muted">{t.deps.wpText}</p>
              <table className="table" aria-label={t.deps.wpTitle}>
                <thead>
                  <tr>
                    <th>{t.deps.wpName}</th>
                    <th>{t.deps.wpInstalled}</th>
                    <th className="table__num">{t.deps.wpBreaking}</th>
                    {wpLatest && <th>{t.deps.wpLatest}</th>}
                  </tr>
                </thead>
                <tbody>
                  {deps.wordpress.map((c) => {
                    const l = wpLatest?.find((x) => x.kind === c.kind && x.slug === c.slug);
                    const breaking = breakingIn(c.dir);
                    return (
                      <tr key={`${c.kind}:${c.slug}`}>
                        <td>
                          {c.name} <Badge>{c.kind === "plugin" ? t.deps.wpPlugin : t.deps.wpTheme}</Badge>
                          <div className="dep-note mono">{c.dir}</div>
                        </td>
                        <td>
                          <span className="mono">{c.version ?? "—"}</span>
                          {c.requiresPhp && <div className="dep-note">{t.deps.wpRequires(c.requiresPhp)}</div>}
                        </td>
                        <td className="table__num tabular">{breaking > 0 ? <Badge tone="danger">{breaking}</Badge> : <span className="subtle">0</span>}</td>
                        {wpLatest && (
                          <td>
                            {!l ? (
                              <span className="subtle">{t.deps.wpNotListed}</span>
                            ) : l.closed ? (
                              <Badge tone="danger">{t.deps.wpClosed}</Badge>
                            ) : (
                              <>
                                <span className="mono">{l.version}</span>
                                {l.requiresPhp && <div className="dep-note">{t.deps.wpRequires(l.requiresPhp)}</div>}
                              </>
                            )}
                          </td>
                        )}
                      </tr>
                    );
                  })}
                </tbody>
              </table>
              {!wpLatest && api && (
                <div className="row row--wrap dep-lookup card__intro">
                  <Button size="sm" variant="secondary" icon={<Search size={13} />} loading={checkingWp} onClick={checkWordpress}>
                    {t.deps.wpCheck}
                  </Button>
                  <span className="subtle">{t.deps.wpCheckNote}</span>
                </div>
              )}
            </Card>
          )}

          <ServerProbe target={deps.target} extensions={deps.extensions} />

          {deps.extensions.length > 0 && (
            <Card title={t.deps.extensions} bodyClassName="">
              <p className="card__intro muted">{t.deps.extensionsText}</p>
              <table className="table" aria-label={t.deps.extensions}>
                <thead>
                  <tr>
                    <th>{t.deps.extension}</th>
                    <th>{t.deps.usedAt}</th>
                    <th className="table__num">{t.deps.uses}</th>
                    <th>{t.deps.declared}</th>
                    <th>{t.deps.local}</th>
                  </tr>
                </thead>
                <tbody>
                  {deps.extensions.map((e) => (
                    <tr key={e.name}>
                      <td>
                        <span className="mono selectable">{e.name}</span> {e.removed && <Badge tone="danger">{t.deps.removedExt}</Badge>}
                      </td>
                      <td>
                        {e.uses > 0 ? (
                          <>
                            <span className="mono">{e.symbol}</span>
                            <div className="dep-note mono">
                              {e.file}:{e.line}
                            </div>
                          </>
                        ) : (
                          <span className="subtle">{t.deps.requiredBy((e.requiredBy ?? []).join(", "))}</span>
                        )}
                        {e.dev && <Badge tone="neutral">{t.deps.devOnly}</Badge>}
                      </td>
                      <td className="table__num tabular">{e.uses > 0 ? e.uses : "—"}</td>
                      <td>{e.declared ? t.deps.yes : <span className="subtle">{t.deps.no}</span>}</td>
                      <td>{e.available === null ? "—" : e.available ? t.deps.yes : <Badge tone="danger">{t.deps.no}</Badge>}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
              {undeclared.length > 0 && <p className="card__intro subtle">{t.deps.addExt(undeclared.join(", "))}</p>}
            </Card>
          )}
        </div>
      </div>
    </>
  );
}

function ServerProbe({ target, extensions }: { target: string; extensions: ExtensionUse[] }) {
  const { t } = useI18n();
  const { api, project } = useStore();
  const toast = useToast();
  const key = project ? `ferry.probe:${project.root}` : null;
  const [token, setToken] = useState<string | null>(null);
  const [text, setText] = useState(() => {
    try {
      return (key && localStorage.getItem(key)) || "";
    } catch {
      return "";
    }
  });
  const probe = text.trim() ? parseProbe(text) : null;
  const report = probe && compareProbe(probe, target, extensions);
  const remember = (value: string) => {
    setText(value);
    try {
      if (key) localStorage.setItem(key, value);
    } catch {
      /* storage can be unavailable */
    }
  };
  const create = async () => {
    if (!api) return;
    try {
      const dest = await api.pickProbeFile("ferry-probe.php");
      if (!dest) return;
      setToken(await api.writeProbe(dest));
    } catch (e) {
      toast.push({ tone: "error", title: t.deps.probeFailed, description: errorMessage(e, t.common.genericError) });
    }
  };
  return (
    <Card title={t.deps.probeTitle} bodyClassName="card__body stack">
      <p className="muted">{t.deps.probeText}</p>
      <div>
        <Button size="sm" variant="secondary" icon={<Server size={13} />} onClick={create}>
          {t.deps.probeCreate}
        </Button>
      </div>
      {token && (
        <div className="stack stack--tight">
          <span className="subtle">{t.deps.probeOpen}</span>
          <CommandLine command={`https://example.com/ferry-probe.php?token=${token}`} />
        </div>
      )}
      <label className="field">
        <span className="field__label">{t.deps.probePaste}</span>
        <textarea className="input mono" rows={3} value={text} placeholder={t.deps.probePlaceholder} onChange={(e) => remember(e.target.value)} />
      </label>
      {text.trim() && !probe && <Callout tone="warn" icon={<CircleAlert size={15} />}>{t.deps.probeInvalid}</Callout>}
      {report && (
        <>
          <table className="table" aria-label={t.deps.probeTitle}>
            <tbody>
              <tr>
                <th scope="row">{t.deps.probePhp}</th>
                <td>
                  <span className="mono">{report.php}</span>{" "}
                  {report.belowTarget && <Badge tone="danger">{t.deps.probeBelow(target)}</Badge>}{" "}
                  {report.vulnerable && <Badge tone="danger">{t.deps.probeVulnerable}</Badge>}
                </td>
              </tr>
              <tr>
                <th scope="row">{t.deps.probeSapi}</th>
                <td>
                  <span className="mono">{report.sapi}</span>
                  {report.server && <span className="subtle"> · {report.server}</span>}
                  {report.fpm && <div className="dep-note">{t.deps.probeFpm}</div>}
                </td>
              </tr>
              <tr>
                <th scope="row">{t.deps.probeMissing}</th>
                <td>
                  {report.missing.length ? (
                    <span className="row row--wrap">
                      {report.missing.map((m) => (
                        <Badge key={m} tone="danger">
                          {m}
                        </Badge>
                      ))}
                    </span>
                  ) : (
                    <span className="subtle">{t.deps.probeNone}</span>
                  )}
                </td>
              </tr>
              <tr>
                <th scope="row">{t.deps.probeDisabled}</th>
                <td className="mono">{report.disabled.length ? report.disabled.join(", ") : <span className="subtle">{t.deps.probeNone}</span>}</td>
              </tr>
              <tr>
                <th scope="row">{t.deps.probeMemory}</th>
                <td className="mono">{report.memoryLimit ?? "—"}</td>
              </tr>
            </tbody>
          </table>
          <div>
            <Button size="sm" variant="ghost" onClick={() => remember("")}>
              {t.deps.probeClear}
            </Button>
          </div>
        </>
      )}
    </Card>
  );
}

export function History() {
  const { t } = useI18n();
  const { api, project, snapshots, restoreSnapshot, deleteSnapshot, deleteAllSnapshots, dockerHistory } = useStore();
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

  return (
    <>
      <PageHeader
        title={t.history.title}
        subtitle={snapshots.length ? t.history.subtitle(snapshots.length) : undefined}
        actions={
          snapshots.length > 1 && (
            <Button variant="ghost" size="sm" icon={<Trash2 size={14} />} onClick={() => setClearing(true)}>
              {t.history.deleteAll}
            </Button>
          )
        }
      />
      <div className="page-scroll">
        <div className="page-body stack">
          {dockerHistory.length > 0 && (
            <Card title={t.history.dockerTitle}>
              <ul className="docker-events">
                {dockerHistory.map((run) => (
                  <li key={run.at}>
                    <Badge tone={run.regressed ? "danger" : run.testsCompared ? "auto" : "neutral"}>{t.overview.readiness.dockerState[run.regressed ? "failed" : run.testsCompared ? "passed" : "partial"]}</Badge>
                    <code>
                      PHP {run.source} → {run.target}
                    </code>
                    <span>{formatRelative(run.at * 1000, t)}</span>
                    <span>{t.history.dockerCounts(run.differingPages, run.pages, run.newFailures, run.dataChanges)}</span>
                  </li>
                ))}
              </ul>
            </Card>
          )}
          {snapshots.length === 0 ? (
            <EmptyState icon={<HistoryIcon size={24} />} title={t.history.empty}>
              {t.history.emptyText}
            </EmptyState>
          ) : (
            <Card bodyClassName="">
              <table className="table">
                <thead>
                  <tr>
                    <th>{t.history.change}</th>
                    <th>{t.history.when}</th>
                    <th>{t.history.files}</th>
                    <th className="table__actions">
                      <span className="sr-only">{t.history.actions}</span>
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {snapshots.map((s) => (
                    <tr key={s.id}>
                      <td>
                        <div className="row">
                          <FileClock size={15} className="subtle" />
                          <span className="strong">{s.label}</span>
                        </div>
                      </td>
                      <td className="muted" title={new Date(s.createdAt).toLocaleString(t.locale)}>
                        {formatRelative(s.createdAt, t)}
                      </td>
                      <td className="muted tabular" title={s.files.map((f) => f.path).join("\n")}>
                        {s.files.length}
                      </td>
                      <td>
                        <div className="row row--end">
                          <Button size="sm" variant="ghost" icon={<Eye size={13} />} onClick={() => setReviewing(s)}>
                            {t.history.review}
                          </Button>
                          <Button size="sm" icon={<RotateCcw size={13} />} onClick={() => setPending({ snapshot: s, action: "restore" })}>
                            {t.history.restore}
                          </Button>
                          <Button
                            size="sm"
                            variant="ghost"
                            iconOnly
                            aria-label={t.history.delete}
                            onClick={() => setPending({ snapshot: s, action: "delete" })}
                          >
                            <Trash2 size={14} />
                          </Button>
                        </div>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </Card>
          )}
        </div>
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

const withIcon = (icon: ReactElement, label: string) => (
  <span className="row row--tight">
    {icon}
    {label}
  </span>
);

const EDITORS = [
  ["vscode", "Visual Studio Code"],
  ["cursor", "Cursor"],
  ["phpstorm", "PhpStorm"],
  ["sublime", "Sublime Text"],
  ["zed", "Zed"],
] as const;

function ProjectSettings() {
  const { t } = useI18n();
  const { project, saveConfig } = useStore();
  const [exclude, setExclude] = useState(project?.config.exclude.join("\n") ?? "");
  if (!project) return null;
  const list = exclude.split("\n").map((d) => d.trim()).filter(Boolean);
  const pinned = project.config.target;
  return (
    <Card title={t.settings.projectTitle(project.name)}>
      <p className="subtle">{t.settings.projectNote}</p>
      {pinned && <p>{t.settings.projectTarget(pinned)}</p>}
      <label className="docker-field">
        <span>{t.settings.projectExclude}</span>
        <textarea className="input" rows={3} placeholder="legacy&#10;tools/old" value={exclude} onChange={(e) => setExclude(e.target.value)} />
      </label>
      <div className="docker-save">
        <Button variant="secondary" size="sm" onClick={() => void saveConfig({ ...project.config, exclude: list })}>
          {t.settings.projectSave}
        </Button>
      </div>
    </Card>
  );
}

export function SettingsScreen() {
  const { t, lang, setLang } = useI18n();
  const { theme, setTheme, toolchain, refreshToolchain, installRector, installingRector, api, refreshSnapshots } = useStore();
  const toast = useToast();
  const [checking, setChecking] = useState(false);
  const [settings, setSettings] = useState<Settings>({ phpPath: null, snapshotLimit: null, editor: null });
  const [usage, setUsage] = useState<BackupUsage | null>(null);
  const [clearing, setClearing] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [images, setImages] = useState<DockerImageUsage | null>(null);
  const [clearingImages, setClearingImages] = useState(false);

  useEffect(() => {
    api?.getSettings().then(setSettings).catch(() => {});
    api?.backupUsage().then(setUsage).catch(() => {});
    api?.dockerImageUsage().then(setImages).catch(() => {});
  }, [api]);

  const imagesSize = images ? formatBytes(images.bytes, t.locale) : "";
  const removeImages = async () => {
    if (!api) return;
    setDeleting(true);
    try {
      await api.removeDockerImages();
      toast.push({ tone: "success", title: t.settings.imagesDone });
      setClearingImages(false);
    } catch (e) {
      toast.push({ tone: "error", title: t.toast.deleteFailed, description: errorMessage(e, t.common.genericError) });
    } finally {
      setDeleting(false);
      api.dockerImageUsage().then(setImages).catch(() => {});
    }
  };

  const size = usage ? formatBytes(usage.bytes, t.locale) : "";
  const deleteAll = async () => {
    if (!api) return;
    setDeleting(true);
    try {
      await api.deleteAllBackups();
      toast.push({ tone: "success", title: t.settings.deleteAllDone });
      setClearing(false);
      void refreshSnapshots();
    } catch (e) {
      toast.push({ tone: "error", title: t.toast.deleteFailed, description: errorMessage(e, t.common.genericError) });
    } finally {
      setDeleting(false);
      api.backupUsage().then(setUsage).catch(() => {});
    }
  };

  const latest = useRef(settings);
  latest.current = settings;
  const queue = useRef(Promise.resolve(true));
  const save = (patch: Partial<Settings>) => {
    const run = async () => {
      if (!api) return false;
      const next = { ...latest.current, ...patch };
      try {
        await api.saveSettings(next);
        latest.current = next;
        setSettings(next);
        return true;
      } catch (e) {
        toast.push({ tone: "error", title: t.toast.saveFailed, description: errorMessage(e, t.common.genericError) });
        return false;
      }
    };
    queue.current = queue.current.then(run, run);
    return queue.current;
  };

  const themes = [
    { value: "system" as Theme, label: withIcon(<Monitor size={13} />, t.settings.system) },
    { value: "light" as Theme, label: withIcon(<Sun size={13} />, t.settings.light) },
    { value: "dark" as Theme, label: withIcon(<Moon size={13} />, t.settings.dark) },
  ];

  const recheck = async () => {
    setChecking(true);
    await refreshToolchain();
    setChecking(false);
  };

  const choosePhp = async () => {
    if (!api) return;
    let path: string | null;
    try {
      path = await api.pickPhpBinary(t.dialogs.pickPhp);
    } catch (e) {
      toast.push({ tone: "error", title: t.toast.saveFailed, description: errorMessage(e, t.common.genericError) });
      return;
    }
    if (!path || !(await save({ phpPath: path }))) return;
    await recheck();
    toast.push({ tone: "success", title: t.toast.phpSaved, description: path });
  };

  const resetPhp = async () => {
    if (await save({ phpPath: null })) await recheck();
  };

  const limits = [
    { value: "none", label: t.settings.unlimited },
    { value: "10", label: "10" },
    { value: "30", label: "30" },
    { value: "100", label: "100" },
  ] as const;
  type LimitValue = (typeof limits)[number]["value"];
  const limitValue = (settings.snapshotLimit === null ? "none" : String(settings.snapshotLimit)) as LimitValue;

  const phpVersion = toolchain?.php?.version ?? null;
  const tools = [
    { name: "PHP", info: toolchain?.php, required: true },
    { name: "Composer", info: toolchain?.composer, required: false },
    { name: "Rector", info: toolchain?.rector, required: true },
    { name: "Docker", info: toolchain?.docker, required: false },
  ];

  return (
    <>
      <PageHeader title={t.settings.title} />
      <div className="page-scroll">
        <div className="page-body page-body--narrow stack">
          <ProjectSettings />
          <Card title={t.settings.appearance} bodyClassName="">
            <div className="card__list">
              <div className="setting">
                <div className="setting__text">
                  <div className="setting__title">{t.settings.language}</div>
                  <div className="setting__desc">{t.settings.languageText}</div>
                </div>
                <Segmented<Lang> label={t.settings.language} options={LANGS} value={lang} onChange={setLang} />
              </div>
              <div className="setting">
                <div className="setting__text">
                  <div className="setting__title">{t.settings.theme}</div>
                  <div className="setting__desc">{t.settings.themeText}</div>
                </div>
                <Segmented label={t.settings.theme} options={themes} value={theme} onChange={setTheme} />
              </div>
              <div className="setting">
                <div className="setting__text">
                  <div className="setting__title">{t.settings.editor}</div>
                  <div className="setting__desc">{t.settings.editorText}</div>
                </div>
                <select
                  className="input"
                  aria-label={t.settings.editor}
                  value={settings.editor ?? ""}
                  onChange={(e) => void save({ editor: e.target.value || null })}
                >
                  <option value="">{t.settings.editorSystem}</option>
                  {EDITORS.map(([value, label]) => (
                    <option key={value} value={value}>
                      {label}
                    </option>
                  ))}
                </select>
              </div>
            </div>
          </Card>

          <Card
            title={t.settings.tools}
            actions={
              <Button size="sm" variant="ghost" icon={<RefreshCw size={13} />} onClick={recheck} loading={checking}>
                {t.settings.recheck}
              </Button>
            }
            bodyClassName=""
          >
            <div className="card__list">
              {tools.map((tool) => (
                <div key={tool.name} className="tool">
                  <span
                    className={`dot ${tool.info ? (tool.name === "PHP" && phpVersion && isVulnerable(phpVersion) ? "dot--error" : "dot--ok") : tool.required ? "dot--error" : "dot--warn"}`}
                    aria-hidden
                  />
                  <span className="tool__name">{tool.name}</span>
                  <span className="tool__path selectable" title={tool.info?.path}>
                    {tool.info ? tool.info.path : t.settings.notFound}
                  </span>
                  {tool.info?.version && <Badge mono>{tool.info.version}</Badge>}
                  {tool.name === "PHP" && (
                    <>
                      <Button size="sm" icon={<FolderOpen size={13} />} onClick={choosePhp}>
                        {t.settings.choose}
                      </Button>
                      <Button size="sm" variant="ghost" onClick={resetPhp}>
                        {t.settings.auto}
                      </Button>
                    </>
                  )}
                  {tool.name === "Rector" && (
                    <Button size="sm" onClick={installRector} loading={installingRector} disabled={!toolchain?.composer}>
                      {tool.info ? t.settings.update : t.settings.install}
                    </Button>
                  )}
                </div>
              ))}
            </div>
            {phpVersion && isVulnerable(phpVersion) && (
              <div className="tool-warning">
                <Callout tone="danger" icon={<ShieldAlert size={15} aria-hidden />}>
                  {t.settings.vulnerablePhp(phpVersion, PHP2SHELL_CVES, patchedFor(phpVersion))}
                </Callout>
              </div>
            )}
          </Card>

          <Card title={t.settings.backups} bodyClassName="">
            <div className="card__list">
              <div className="setting">
                <div className="setting__text">
                  <div className="setting__title">{t.settings.keep}</div>
                  <div className="setting__desc">{t.settings.keepText}</div>
                </div>
                <Segmented<LimitValue>
                  label={t.settings.keep}
                  options={limits}
                  value={limits.some((l) => l.value === limitValue) ? limitValue : "none"}
                  onChange={(v) => save({ snapshotLimit: v === "none" ? null : Number(v) })}
                />
              </div>
              <div className="setting">
                <div className="setting__text">
                  <div className="setting__title">{t.settings.usage}</div>
                  <div className="setting__desc">
                    {usage && usage.snapshots > 0 ? t.settings.usageText(usage.snapshots, usage.projects, size) : t.settings.usageEmpty}
                  </div>
                </div>
                <Button size="sm" variant="danger" icon={<Trash2 size={13} />} onClick={() => setClearing(true)} disabled={!usage?.snapshots}>
                  {t.settings.deleteAll}
                </Button>
              </div>
              <div className="setting">
                <div className="setting__text">
                  <div className="setting__title">{t.settings.images}</div>
                  <div className="setting__desc">
                    {!images ? t.settings.imagesUnknown : images.images > 0 ? t.settings.imagesText(images.images, imagesSize) : t.settings.imagesEmpty}
                  </div>
                </div>
                <Button size="sm" variant="danger" icon={<Trash2 size={13} />} onClick={() => setClearingImages(true)} disabled={!images?.images}>
                  {t.settings.imagesDelete}
                </Button>
              </div>
            </div>
          </Card>

          <ConfirmDialog
            open={clearing}
            tone="danger"
            icon={<Trash2 size={20} />}
            title={t.settings.deleteAllTitle}
            confirmLabel={t.history.deleteConfirm}
            loading={deleting}
            onCancel={() => setClearing(false)}
            onConfirm={deleteAll}
          >
            {t.settings.deleteAllText(usage?.snapshots ?? 0, size)}
          </ConfirmDialog>

          <ConfirmDialog
            open={clearingImages}
            tone="danger"
            icon={<Trash2 size={20} />}
            title={t.settings.imagesTitle}
            confirmLabel={t.history.deleteConfirm}
            loading={deleting}
            onCancel={() => setClearingImages(false)}
            onConfirm={removeImages}
          >
            {t.settings.imagesConfirm(images?.images ?? 0, imagesSize)}
          </ConfirmDialog>

          {toolchain && (
            <p className="hint selectable">
              {t.settings.footnote(toolchain.toolsDir)}
            </p>
          )}
        </div>
      </div>
    </>
  );
}

export function Roadmap() {
  const { t } = useI18n();
  const content = t.roadmap.ai;
  return (
    <>
      <PageHeader title={content.title} />
      <div className="page-scroll">
        <div className="roadmap">
          <div className="roadmap__icon">
            <Bot size={26} />
          </div>
          <Badge tone="accent">{t.common.soon}</Badge>
          <h2>{content.title}</h2>
          <p>{content.text}</p>
          <ul>
            {content.items.map((i) => (
              <li key={i}>
                <Check size={14} />
                {i}
              </li>
            ))}
          </ul>
        </div>
      </div>
    </>
  );
}
