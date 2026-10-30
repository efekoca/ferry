import { useEffect, useLayoutEffect, useRef, useState, type ReactElement } from "react";
import {
  Bot,
  Check,
  CircleAlert,
  CircleCheck,
  CircleHelp,
  Container,
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
  Sun,
  Trash2,
} from "lucide-react";
import { DiffViewer } from "../components/DiffViewer";
import { ConfirmDialog, Sheet } from "../components/Modal";
import { PageHeader } from "../components/Shell";
import { Badge, Button, Callout, Card, EmptyState, RiskBadge, RuleChips, Segmented } from "../components/ui";
import { useToast } from "../components/Toast";
import { LANGS, useI18n, type Lang } from "../i18n";
import { errorMessage } from "../lib/api";
import { formatRelative } from "../lib/labels";
import { useStore, type Theme } from "../lib/store";
import type { DependencyStatus, FileChange, Snapshot } from "../lib/types";

const DEP_TONE: Record<DependencyStatus, "danger" | "auto" | "neutral"> = {
  incompatible: "danger",
  compatible: "auto",
  unknown: "neutral",
};

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
  const { deps, scan, runScan, scanning, target } = useStore();
  const [showDev, setShowDev] = useState(true);

  if (!deps) {
    return (
      <>
        <PageHeader title={t.deps.title} />
        <EmptyState
          icon={<Package size={24} />}
          title={t.deps.notChecked}
          action={
            !scan && (
              <Button variant="primary" onClick={runScan} loading={scanning}>
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

  const packages = deps.packages.filter((p) => showDev || !p.dev);
  const bad = deps.packages.filter((p) => p.status === "incompatible").length;

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
              <strong>{t.deps.rootBlockedTitle(deps.target)}</strong> {t.deps.rootBlockedText(deps.rootConstraint ?? "", deps.target)}
            </Callout>
          )}
          {!deps.hasLockFile ? (
            <Callout tone="warn" icon={<CircleAlert size={16} />}>
              {t.deps.noLock}
            </Callout>
          ) : bad > 0 ? (
            <Callout tone="danger" icon={<CircleAlert size={16} />}>
              <strong>{t.deps.badTitle(bad, deps.target)}</strong> {t.deps.badText}{" "}
              <code className="selectable">composer update vendor/package --with-all-dependencies</code>
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
              <span style={{ marginLeft: "auto" }}>
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
                    <th style={{ width: 140 }}>{t.deps.status}</th>
                  </tr>
                </thead>
                <tbody>
                  {packages.map((p) => (
                    <tr key={p.name}>
                      <td>
                        <span className="mono selectable">{p.name}</span> {p.dev && <Badge>dev</Badge>}
                      </td>
                      <td className="mono subtle">{p.version}</td>
                      <td>
                        <span className="mono">{p.phpConstraint ?? <span className="subtle">{t.deps.unspecified}</span>}</span>
                        {p.note && <div className="dep-note">{loc(p.note)}</div>}
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
        </div>
      </div>
    </>
  );
}

export function History() {
  const { t } = useI18n();
  const { api, project, snapshots, restoreSnapshot, deleteSnapshot, deleteAllSnapshots } = useStore();
  const [clearing, setClearing] = useState(false);
  const [pending, setPending] = useState<{ snapshot: Snapshot; action: "restore" | "delete" } | null>(null);
  const [reviewing, setReviewing] = useState<Snapshot | null>(null);
  const [busy, setBusy] = useState(false);
  const [conflicts, setConflicts] = useState<string[]>([]);
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
                    <th style={{ width: 260 }} />
                  </tr>
                </thead>
                <tbody>
                  {snapshots.map((s) => (
                    <tr key={s.id}>
                      <td>
                        <div className="row">
                          <FileClock size={15} className="subtle" />
                          <span style={{ fontWeight: 500 }}>{s.label}</span>
                        </div>
                      </td>
                      <td className="muted" title={new Date(s.createdAt).toLocaleString(t.locale)}>
                        {formatRelative(s.createdAt, t)}
                      </td>
                      <td className="muted tabular" title={s.files.map((f) => f.path).join("\n")}>
                        {s.files.length}
                      </td>
                      <td>
                        <div className="row" style={{ justifyContent: "flex-end" }}>
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
          if (pending.action === "restore") await restoreSnapshot(pending.snapshot.id);
          else await deleteSnapshot(pending.snapshot.id);
          setBusy(false);
          setPending(null);
        }}
      >
        {deleting ? t.history.deleteText : t.history.restoreText(pending?.snapshot.files.length ?? 0, pending?.snapshot.label ?? "")}
        {!deleting && conflicts.length > 0 && (
          <div style={{ marginTop: "var(--space-3)" }}>
            <Callout tone="warn" icon={<CircleAlert size={15} />}>
              <strong>{t.history.conflictTitle(conflicts.length)}</strong>
              <ul className="conflict-list">
                {conflicts.map((f) => (
                  <li key={f}>
                    <bdi>{f}</bdi>
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
  <span className="row" style={{ gap: 6 }}>
    {icon}
    {label}
  </span>
);

export function SettingsScreen() {
  const { t, lang, setLang } = useI18n();
  const { theme, setTheme, toolchain, refreshToolchain, installRector, installingRector, api } = useStore();
  const toast = useToast();
  const [checking, setChecking] = useState(false);

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
    try {
      const path = await api.pickPhpBinary(t.dialogs.pickPhp);
      if (!path) return;
      await api.saveSettings({ phpPath: path });
      await recheck();
      toast.push({ tone: "success", title: t.toast.phpSaved, description: path });
    } catch (e) {
      toast.push({ tone: "error", title: t.toast.saveFailed, description: errorMessage(e, t.common.genericError) });
    }
  };

  const resetPhp = async () => {
    if (!api) return;
    try {
      await api.saveSettings({ phpPath: null });
      await recheck();
    } catch (e) {
      toast.push({ tone: "error", title: t.toast.saveFailed, description: errorMessage(e, t.common.genericError) });
    }
  };

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
        <div className="page-body stack" style={{ maxWidth: 820 }}>
          <Card title={t.settings.appearance} bodyClassName="">
            <div style={{ paddingTop: "var(--space-2)" }}>
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
            <div style={{ paddingTop: "var(--space-2)" }}>
              {tools.map((tool) => (
                <div key={tool.name} className="tool">
                  <span className={`dot ${tool.info ? "dot--ok" : tool.required ? "dot--error" : "dot--warn"}`} aria-hidden />
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
          </Card>

          {toolchain && (
            <p className="subtle selectable" style={{ fontSize: "var(--text-sm)" }}>
              {t.settings.footnote(toolchain.toolsDir)}
            </p>
          )}
        </div>
      </div>
    </>
  );
}

export function Roadmap({ kind }: { kind: "runtime" | "ai" }) {
  const { t } = useI18n();
  const content = t.roadmap[kind];
  return (
    <>
      <PageHeader title={content.title} />
      <div className="page-scroll">
        <div className="roadmap">
          <div className="roadmap__icon">{kind === "runtime" ? <Container size={26} /> : <Bot size={26} />}</div>
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
