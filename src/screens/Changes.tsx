import { useEffect, useMemo, useState } from "react";
import { CircleAlert, CircleCheck, Download, GitCompareArrows, RotateCw, ShieldCheck, Sparkles, Wand2 } from "lucide-react";
import { DiffViewer } from "../components/DiffViewer";
import { ConfirmDialog } from "../components/Modal";
import { PageHeader } from "../components/Shell";
import { Button, Callout, Card, EmptyState, Progress, RiskBadge, RuleChips, Segmented } from "../components/ui";
import { useI18n } from "../i18n";
import { useStore } from "../lib/store";
import type { FileChange, RectorMode } from "../lib/types";

export function Changes() {
  const { t } = useI18n();
  const { toolchain, installRector, installingRector, preview, previewMode, previewing, loadPreview, applyChanges, applying, target, setView } =
    useStore();
  const [mode, setMode] = useState<RectorMode>(previewMode);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [active, setActive] = useState<string | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [appliedAll, setAppliedAll] = useState(false);

  const modes = [
    { value: "compat" as const, label: t.changes.compat },
    { value: "modernize" as const, label: t.changes.modernize },
  ];

  useEffect(() => {
    setSelected(new Set(preview?.changes.map((c) => c.file) ?? []));
    if (preview?.changes.length) setAppliedAll(false);
    setActive((cur) => (preview?.changes.some((c) => c.file === cur) ? cur : (preview?.changes[0]?.file ?? null)));
  }, [preview]);

  const changes = useMemo(() => preview?.changes ?? [], [preview]);
  const current = changes.find((c) => c.file === active) ?? null;
  const chosen = useMemo(() => changes.filter((c) => selected.has(c.file)), [changes, selected]);
  const totals = chosen.reduce((sum, c) => ({ add: sum.add + c.diff.additions, del: sum.del + c.diff.deletions }), { add: 0, del: 0 });
  const allSelected = changes.length > 0 && chosen.length === changes.length;

  const toggle = (file: string) =>
    setSelected((s) => {
      const next = new Set(s);
      if (next.has(file)) next.delete(file);
      else next.add(file);
      return next;
    });

  const header = (
    <PageHeader
      title={t.changes.title}
      subtitle={preview ? t.changes.subtitle(preview.target, changes.length) : `PHP ${target}`}
      actions={
        preview && (
          <>
            <Segmented
              label={t.changes.mode}
              options={modes}
              value={mode}
              onChange={(m) => {
                setMode(m);
                loadPreview(m);
              }}
              disabled={previewing}
            />
            <Button variant="secondary" size="sm" icon={<RotateCw size={14} />} onClick={() => loadPreview(mode)} loading={previewing}>
              {t.changes.recalculate}
            </Button>
          </>
        )
      }
    />
  );

  if (!toolchain) {
    return (
      <>
        {header}
        <div className="page-body">
          <Progress value={null} label={t.changes.checkingTools} />
        </div>
      </>
    );
  }

  if (!toolchain.php) {
    return (
      <>
        {header}
        <EmptyState
          icon={<CircleAlert size={24} />}
          title={t.changes.noPhp}
          action={
            <Button variant="primary" onClick={() => setView("settings")}>
              {t.changes.openSettings}
            </Button>
          }
        >
          {t.changes.noPhpText}
        </EmptyState>
      </>
    );
  }

  if (!toolchain.rector) {
    return (
      <>
        {header}
        <EmptyState
          icon={<Download size={24} />}
          title={t.changes.needRector}
          action={
            <Button variant="primary" icon={<Download size={15} />} onClick={installRector} loading={installingRector} disabled={!toolchain.composer}>
              {installingRector ? t.changes.installing : t.changes.installRector}
            </Button>
          }
        >
          {toolchain.composer ? t.changes.needRectorText : t.changes.needComposerText}
        </EmptyState>
      </>
    );
  }

  if (!preview) {
    return (
      <>
        {header}
        <div className="page-scroll">
          <div className="page-body stack" style={{ maxWidth: 760 }}>
            <Card bodyClassName="card__body stack">
              <div className="row">
                <Wand2 size={18} style={{ color: "var(--accent)" }} />
                <h2 style={{ fontSize: "var(--text-lg)", fontWeight: 600 }}>{t.changes.introTitle(target)}</h2>
              </div>
              <p className="muted">{t.changes.introText}</p>
              <div>
                <Segmented label={t.changes.mode} options={modes} value={mode} onChange={setMode} disabled={previewing} />
              </div>
              <p className="muted" style={{ fontSize: "var(--text-sm)" }}>
                {mode === "compat" ? t.changes.compatText : t.changes.modernizeText}
              </p>
              {previewing ? (
                <div className="stack" style={{ gap: 8 }}>
                  <Progress value={null} label={t.changes.calculating} />
                  <span className="subtle" style={{ fontSize: "var(--text-sm)" }}>
                    {t.changes.calculatingHint}
                  </span>
                </div>
              ) : (
                <div>
                  <Button variant="primary" icon={<Sparkles size={15} />} onClick={() => loadPreview(mode)}>
                    {t.changes.calculate}
                  </Button>
                </div>
              )}
            </Card>
          </div>
        </div>
      </>
    );
  }

  if (changes.length === 0) {
    return (
      <>
        {header}
        {appliedAll ? (
          <EmptyState
            icon={<CircleCheck size={24} />}
            title={t.changes.allApplied}
            action={
              <Button variant="secondary" onClick={() => setView("findings")}>
                {t.changes.seeRemaining}
              </Button>
            }
          >
            {t.changes.allAppliedText}
          </EmptyState>
        ) : (
          <EmptyState icon={<CircleCheck size={24} />} title={t.changes.nothing}>
            {preview.mode === "compat" ? t.changes.nothingCompat(preview.target) : t.changes.nothingModern(preview.target)}
          </EmptyState>
        )}
        {preview.errors.length > 0 && (
          <div className="page-body">
            <ErrorList errors={preview.errors} />
          </div>
        )}
      </>
    );
  }

  return (
    <>
      {header}
      <div className="split split--changes">
        <aside className="pane pane--side" aria-label={t.changes.changedFiles}>
          <label className="change-list__header">
            <input
              type="checkbox"
              className="checkbox"
              checked={allSelected}
              ref={(el) => {
                if (el) el.indeterminate = chosen.length > 0 && !allSelected;
              }}
              onChange={() => setSelected(allSelected ? new Set() : new Set(changes.map((c) => c.file)))}
              aria-label={t.changes.selectAll}
            />
            <span>{t.changes.selected(chosen.length, changes.length)}</span>
          </label>
          <div className="change-list" role="listbox" aria-label={t.changes.files}>
            {changes.map((c) => (
              <ChangeItem
                key={c.file}
                change={c}
                checked={selected.has(c.file)}
                active={c.file === active}
                onToggle={() => toggle(c.file)}
                onSelect={() => setActive(c.file)}
              />
            ))}
          </div>
          <div className="apply-bar">
            {preview.errors.length > 0 && <ErrorList errors={preview.errors} compact />}
            <div className="row" style={{ justifyContent: "space-between" }}>
              <span className="subtle" style={{ fontSize: "var(--text-sm)" }}>
                <span className="stat-add">+{totals.add}</span> <span className="stat-del">−{totals.del}</span> {t.common.lines}
              </span>
              <Button variant="primary" disabled={chosen.length === 0} loading={applying} onClick={() => setConfirming(true)}>
                {chosen.length === changes.length ? t.changes.applyAll : t.changes.applySome(chosen.length)}
              </Button>
            </div>
          </div>
        </aside>

        <section className="diff-pane" aria-label={t.changes.details}>
          {current ? (
            <>
              <div className="diff-header">
                <span className="diff-header__file" title={current.file}>
                  {current.file}
                </span>
                <span className="stat-add">+{current.diff.additions}</span>
                <span className="stat-del">−{current.diff.deletions}</span>
                <span style={{ marginLeft: "auto" }}>
                  <RiskBadge fix={current.fix} />
                </span>
              </div>
              <RuleChips rules={current.rules} />
              {current.fix === "likely" && (
                <div style={{ padding: "var(--space-3) var(--space-6) 0" }}>
                  <Callout tone="info" icon={<ShieldCheck size={15} />}>
                    {t.changes.likelyNote}
                  </Callout>
                </div>
              )}
              <DiffViewer diff={current.diff} file={current.file} />
            </>
          ) : (
            <EmptyState icon={<GitCompareArrows size={24} />} title={t.changes.pickFile} />
          )}
        </section>
      </div>

      <ConfirmDialog
        open={confirming}
        icon={<Wand2 size={20} />}
        title={t.changes.confirmTitle(chosen.length)}
        confirmLabel={t.common.apply}
        loading={applying}
        onCancel={() => setConfirming(false)}
        onConfirm={async () => {
          const all = chosen.length === changes.length;
          const ok = await applyChanges(chosen.map((c) => c.file));
          if (ok) {
            setConfirming(false);
            setAppliedAll(all);
          }
        }}
      >
        <span className="tabular">{t.changes.confirmText(totals.add, totals.del)}</span>
      </ConfirmDialog>
    </>
  );
}

function ChangeItem({
  change,
  checked,
  active,
  onToggle,
  onSelect,
}: {
  change: FileChange;
  checked: boolean;
  active: boolean;
  onToggle: () => void;
  onSelect: () => void;
}) {
  const { t } = useI18n();
  return (
    <div
      className="change-item"
      role="option"
      aria-selected={active}
      tabIndex={0}
      onClick={onSelect}
      onKeyDown={(e) => {
        if (e.key === "Enter") onSelect();
        if (e.key === " ") {
          e.preventDefault();
          onToggle();
        }
      }}
    >
      <input
        type="checkbox"
        className="checkbox"
        checked={checked}
        onChange={onToggle}
        onClick={(e) => e.stopPropagation()}
        aria-label={t.changes.selectFile(change.file)}
      />
      <div className="change-item__body">
        <div className="change-item__file" title={change.file}>
          <bdi>{change.file}</bdi>
        </div>
        <div className="change-item__meta">
          <RiskBadge fix={change.fix} />
          <span className="stat-add">+{change.diff.additions}</span>
          <span className="stat-del">−{change.diff.deletions}</span>
          <span className="subtle" style={{ fontSize: "var(--text-xs)" }}>
            {t.common.rules(change.rules.length)}
          </span>
        </div>
      </div>
    </div>
  );
}

function ErrorList({ errors, compact = false }: { errors: string[]; compact?: boolean }) {
  const { t } = useI18n();
  return (
    <Callout tone="warn" icon={<CircleAlert size={15} />}>
      <strong>{t.changes.failed(errors.length)}</strong>
      {compact ? (
        <div className="selectable" style={{ marginTop: 2 }}>
          {errors[0]}
        </div>
      ) : (
        <ul style={{ margin: "6px 0 0", paddingLeft: 16 }} className="selectable">
          {errors.slice(0, 8).map((e) => (
            <li key={e}>{e}</li>
          ))}
        </ul>
      )}
    </Callout>
  );
}
