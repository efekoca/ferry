import { useEffect, useMemo, useState } from "react";
import { CircleAlert, CircleCheck, CircleStop, Download, GitCompareArrows, RotateCw, ShieldCheck, Sparkles, Wand2 } from "lucide-react";
import { DiffViewer } from "../components/DiffViewer";
import { ConfirmDialog } from "../components/Modal";
import { PageHeader } from "../components/Shell";
import { Button, Callout, Card, EmptyState, Progress, RiskBadge, RuleChips, Segmented } from "../components/ui";
import { useI18n } from "../i18n";
import { useStore } from "../lib/store";
import type { FileChange, RectorMode, RectorPreview } from "../lib/types";

export function Changes() {
  const { t } = useI18n();
  const { toolchain, installRector, installingRector, preview, previewMode, previewing, loadPreview, cancelPreview, applyChanges, applying, target, setView } =
    useStore();
  const [mode, setMode] = useState<RectorMode>(previewMode);
  const [excluded, setExcluded] = useState<{ preview: RectorPreview | null; files: Set<string> }>({ preview: null, files: new Set() });
  const [active, setActive] = useState<string | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [appliedAll, setAppliedAll] = useState(false);

  const modes = [
    { value: "compat" as const, label: t.changes.compat },
    { value: "modernize" as const, label: t.changes.modernize },
  ];

  useEffect(() => {
    if (preview?.changes.length) setAppliedAll(false);
  }, [preview]);

  const changes = useMemo(() => preview?.changes ?? [], [preview]);
  const off = excluded.preview === preview ? excluded.files : null;
  const selected = useMemo(() => new Set(changes.filter((c) => !off?.has(c.file)).map((c) => c.file)), [changes, off]);
  const current = changes.find((c) => c.file === active) ?? changes[0] ?? null;
  const chosen = useMemo(() => changes.filter((c) => selected.has(c.file)), [changes, selected]);
  const totals = chosen.reduce((sum, c) => ({ add: sum.add + c.diff.additions, del: sum.del + c.diff.deletions }), { add: 0, del: 0 });
  const allSelected = changes.length > 0 && chosen.length === changes.length;

  const setSelected = (next: Set<string>) =>
    setExcluded({ preview, files: new Set(changes.filter((c) => !next.has(c.file)).map((c) => c.file)) });

  const toggle = (file: string) => {
    const next = new Set(selected);
    if (next.has(file)) next.delete(file);
    else next.add(file);
    setSelected(next);
  };

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
                const previous = mode;
                setMode(m);
                loadPreview(m).then((ok) => {
                  if (!ok) setMode(previous);
                });
              }}
              disabled={previewing}
            />
            {previewing && (
              <Button variant="ghost" size="sm" icon={<CircleStop size={14} />} onClick={cancelPreview}>
                {t.changes.stop}
              </Button>
            )}
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
          <div className="page-body page-body--narrow stack">
            <Card bodyClassName="card__body stack">
              <div className="intro__title">
                <Wand2 size={18} aria-hidden />
                <h2>{t.changes.introTitle(target)}</h2>
              </div>
              <p className="muted">{t.changes.introText}</p>
              <div className="mode-cards" role="radiogroup" aria-label={t.changes.mode}>
                {modes.map((m) => (
                  <label key={m.value} className="mode-card" data-checked={mode === m.value}>
                    <input
                      type="radio"
                      name="rector-mode"
                      className="sr-only"
                      value={m.value}
                      checked={mode === m.value}
                      disabled={previewing}
                      onChange={() => setMode(m.value)}
                    />
                    <span className="mode-card__title">
                      {m.value === "compat" ? <ShieldCheck size={15} aria-hidden /> : <Sparkles size={15} aria-hidden />}
                      {m.label}
                      {m.value === "compat" && <span className="mode-card__tag">{t.changes.recommended}</span>}
                    </span>
                    <span className="mode-card__text">{m.value === "compat" ? t.changes.compatText : t.changes.modernizeText}</span>
                  </label>
                ))}
              </div>
              {previewing ? (
                <div className="stack stack--tight">
                  <Progress value={null} label={t.changes.calculating} />
                  <div className="row row--between">
                    <span className="hint">{t.changes.calculatingHint}</span>
                    <Button variant="secondary" size="sm" icon={<CircleStop size={14} />} onClick={cancelPreview}>
                      {t.changes.stop}
                    </Button>
                  </div>
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
          <ul className="change-list" aria-label={t.changes.files}>
            {changes.map((c) => (
              <ChangeItem
                key={c.file}
                change={c}
                checked={selected.has(c.file)}
                active={c.file === current?.file}
                onToggle={() => toggle(c.file)}
                onSelect={() => setActive(c.file)}
              />
            ))}
          </ul>
          <div className="apply-bar">
            {preview.errors.length > 0 && <ErrorList errors={preview.errors} compact />}
            <div className="row row--between">
              <span className="hint">
                <span className="stat-add">+{totals.add}</span> <span className="stat-del">−{totals.del}</span> {t.common.lines}
              </span>
              <Button variant="primary" disabled={chosen.length === 0 || previewing} loading={applying} onClick={() => setConfirming(true)}>
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
                <span className="push-end">
                  <RiskBadge fix={current.fix} />
                </span>
              </div>
              <RuleChips rules={current.rules} />
              {current.fix === "likely" && (
                <div className="diff-note">
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
    <li className="change-item" data-active={active} onClick={onSelect}>
      <input
        type="checkbox"
        className="checkbox"
        checked={checked}
        onChange={onToggle}
        onClick={(e) => e.stopPropagation()}
        aria-label={t.changes.selectFile(change.file)}
      />
      <button type="button" className="change-item__body" aria-current={active || undefined} onClick={onSelect}>
        <div className="change-item__file" title={change.file}>
          <bdi>{change.file}</bdi>
        </div>
        <div className="change-item__meta">
          <RiskBadge fix={change.fix} />
          <span className="stat-add">+{change.diff.additions}</span>
          <span className="stat-del">−{change.diff.deletions}</span>
          <span className="meta">
            {t.common.rules(change.rules.length)}
          </span>
        </div>
      </button>
    </li>
  );
}

function ErrorList({ errors, compact = false }: { errors: string[]; compact?: boolean }) {
  const { t } = useI18n();
  return (
    <Callout tone="warn" icon={<CircleAlert size={15} />}>
      <strong>{t.changes.failed(errors.length)}</strong>
      {compact ? (
        <div className="selectable error-list__first">
          {errors[0]}
        </div>
      ) : (
        <ul className="error-list selectable">
          {errors.slice(0, 8).map((e) => (
            <li key={e}>{e}</li>
          ))}
        </ul>
      )}
    </Callout>
  );
}
