import { useEffect, useMemo, useState } from "react";
import { CircleAlert, CircleCheck, CircleStop, Download, GitBranch, GitCompareArrows, Pin, RotateCw, ShieldCheck, Sparkles, Wand2 } from "lucide-react";
import { DiffViewer, type HunkChoice } from "../components/DiffViewer";
import { ConfirmDialog } from "../components/Modal";
import { PageHeader } from "../components/Shell";
import { Splitter } from "../components/Splitter";
import { Button, Callout, Card, CommandLine, EmptyState, Progress, RiskBadge, RuleChips, Segmented } from "../components/ui";
import { useI18n } from "../i18n";
import { useStore } from "../lib/store";
import type { FileChange, RectorMode } from "../lib/types";

export function Changes() {
  const { t, loc } = useI18n();
  const { toolchain, installRector, installingRector, preview, previewMode, previewing, loadPreview, cancelPreview, applyChanges, applying, target, setView, api, project } =
    useStore();
  const [dirty, setDirty] = useState<string[]>([]);
  const [unclean, setUnclean] = useState<string[]>([]);
  const [ownBranch, setOwnBranch] = useState(false);
  const [mode, setMode] = useState<RectorMode>(previewMode);
  const [excluded, setExcluded] = useState<{ previewId: number | null; files: Set<string> }>({ previewId: null, files: new Set() });
  const [active, setActive] = useState<string | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [appliedAll, setAppliedAll] = useState(false);

  const modes = [
    { value: "compat" as const, label: t.changes.compat },
    { value: "modernize" as const, label: t.changes.modernize },
    { value: "pins" as const, label: t.changes.pins },
  ];
  const modeIcon = { compat: ShieldCheck, modernize: Sparkles, pins: Pin };
  const modeText = { compat: t.changes.compatText, modernize: t.changes.modernizeText, pins: t.changes.pinsText };
  const nothingText = { compat: t.changes.nothingCompat, modernize: t.changes.nothingModern, pins: t.changes.nothingPins };
  const pinsInstead = (
    <Button variant="secondary" icon={<Pin size={15} />} onClick={() => setMode("pins")}>
      {t.changes.pinsInstead}
    </Button>
  );

  useEffect(() => {
    if (preview?.changes.length) setAppliedAll(false);
  }, [preview]);

  const changes = useMemo(() => preview?.changes ?? [], [preview]);
  const off = useMemo(
    () =>
      preview && excluded.previewId === preview.id ? excluded.files : new Set(changes.filter((c) => c.fix !== "auto").map((c) => c.file)),
    [preview, excluded, changes],
  );
  const [viewed, setViewed] = useState<{ previewId: number | null; files: Set<string> }>({ previewId: null, files: new Set() });
  const seen = preview && viewed.previewId === preview.id ? viewed.files : null;
  const selected = useMemo(() => new Set(changes.filter((c) => !off?.has(c.file)).map((c) => c.file)), [changes, off]);
  const current = changes.find((c) => c.file === active) ?? changes[0] ?? null;
  useEffect(() => {
    if (!preview || !current || seen?.has(current.file)) return;
    setViewed({ previewId: preview.id, files: new Set([...(seen ?? []), current.file]) });
  }, [preview, current, seen]);
  const chosen = useMemo(() => changes.filter((c) => selected.has(c.file)), [changes, selected]);
  const [skips, setSkips] = useState<{ previewId: number | null; files: Record<string, number[]> }>({ previewId: null, files: {} });
  const skipped = useMemo(() => (preview && skips.previewId === preview.id ? skips.files : {}), [preview, skips]);
  const totals = chosen.reduce(
    (sum, c) => {
      const lines = c.diff.hunks.filter((_, h) => !skipped[c.file]?.includes(h)).flatMap((h) => h.lines);
      return { add: sum.add + lines.filter((l) => l.kind === "add").length, del: sum.del + lines.filter((l) => l.kind === "del").length };
    },
    { add: 0, del: 0 },
  );
  const allSelected = changes.length > 0 && chosen.length === changes.length;
  const reviewChosen = chosen.filter((c) => c.fix !== "auto");
  const unseen = reviewChosen.filter((c) => !seen?.has(c.file)).length;

  const saveSkips = (file: string, hunks: number[]) => setSkips({ previewId: preview?.id ?? null, files: { ...skipped, [file]: hunks } });
  const hunkChoice = (change: FileChange): HunkChoice => ({
    isOn: (h) => selected.has(change.file) && !skipped[change.file]?.includes(h),
    toggle: (h) => {
      const count = change.diff.hunks.length;
      const others = change.diff.hunks.map((_, i) => i).filter((i) => i !== h);
      const current = skipped[change.file] ?? [];
      if (!selected.has(change.file)) {
        setSelected(new Set([...selected, change.file]));
        saveSkips(change.file, others);
      } else if (current.includes(h)) {
        saveSkips(change.file, current.filter((i) => i !== h));
      } else if (current.length + 1 === count) {
        setSelected(new Set([...selected].filter((f) => f !== change.file)));
        saveSkips(change.file, []);
      } else {
        saveSkips(change.file, [...current, h]);
      }
    },
  });

  const setSelected = (next: Set<string>) =>
    setExcluded({ previewId: preview?.id ?? null, files: new Set(changes.filter((c) => !next.has(c.file)).map((c) => c.file)) });

  const ruleOptions = useMemo(() => {
    const map = new Map<string, { title: string; files: string[] }>();
    for (const c of changes) {
      for (const r of c.rules) {
        const entry = map.get(r.class) ?? { title: loc(r.title), files: [] };
        if (!entry.files.includes(c.file)) entry.files.push(c.file);
        map.set(r.class, entry);
      }
    }
    return [...map.entries()].sort((a, b) => b[1].files.length - a[1].files.length);
  }, [changes, loc]);
  const narrow = (value: string) => {
    if (value === "all") setSelected(new Set(changes.map((c) => c.file)));
    else if (value === "auto") setSelected(new Set(changes.filter((c) => c.fix === "auto").map((c) => c.file)));
    else setSelected(new Set(ruleOptions.find(([cls]) => cls === value)?.[1].files ?? []));
  };
  const commitMessage = useMemo(() => {
    const counts = new Map<string, number>();
    for (const c of chosen) for (const r of c.rules) counts.set(loc(r.title), (counts.get(loc(r.title)) ?? 0) + 1);
    const lines = [...counts.entries()].sort((a, b) => b[1] - a[1]).map(([title, n]) => `- ${title} (${n})`);
    return [t.git.subject(preview?.target ?? target, chosen.length, preview?.mode === "pins"), "", ...lines].join("\n");
  }, [chosen, loc, preview, target, t]);
  const openConfirm = () => {
    setDirty([]);
    setUnclean([]);
    setOwnBranch(Boolean(project?.isGitRepo && project.config.git.branchPerBatch));
    setConfirming(true);
    if (api && project) {
      api
        .gitStatus(project.root, chosen.map((c) => c.file))
        .then((s) => setDirty(s.dirty))
        .catch(() => {});
      if (project.config.git.requireClean)
        api
          .gitStatus(project.root, [])
          .then((s) => setUnclean(s.dirty))
          .catch(() => {});
    }
  };

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

  if (!toolchain.php && mode !== "pins") {
    return (
      <>
        {header}
        <EmptyState
          icon={<CircleAlert size={24} />}
          title={t.changes.noPhp}
          action={
            <div className="row">
              <Button variant="primary" onClick={() => setView("settings")}>
                {t.changes.openSettings}
              </Button>
              {pinsInstead}
            </div>
          }
        >
          {t.changes.noPhpText}
        </EmptyState>
      </>
    );
  }

  if (!toolchain.rector && mode !== "pins") {
    return (
      <>
        {header}
        <EmptyState
          icon={<Download size={24} />}
          title={t.changes.needRector}
          action={
            <div className="row">
              <Button variant="primary" icon={<Download size={15} />} onClick={installRector} loading={installingRector} disabled={!toolchain.composer}>
                {installingRector ? t.changes.installing : t.changes.installRector}
              </Button>
              {pinsInstead}
            </div>
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
              <p className="muted">{mode === "pins" ? t.changes.introPins : t.changes.introText}</p>
              {mode !== "pins" && <p className="hint">{t.changes.introTrust}</p>}
              <div className="mode-cards" role="radiogroup" aria-label={t.changes.mode}>
                {modes.map((m) => {
                  const Icon = modeIcon[m.value];
                  return (
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
                      <Icon size={15} aria-hidden />
                      {m.label}
                      {m.value === "compat" && <span className="mode-card__tag">{t.changes.recommended}</span>}
                    </span>
                    <span className="mode-card__text">{modeText[m.value]}</span>
                  </label>
                  );
                })}
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
            {nothingText[preview.mode](preview.target)}
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
          <div className="change-list__narrow">
            <select title={t.git.byRuleNote} className="input" aria-label={t.git.selectBy} value="" onChange={(e) => narrow(e.target.value)}>
              <option value="" disabled>
                {t.git.selectBy}
              </option>
              <option value="all">{t.git.selectAll}</option>
              <option value="auto">{t.git.onlyAuto}</option>
              {ruleOptions.map(([cls, r]) => (
                <option key={cls} value={cls}>
                  {t.git.byRule(r.title, r.files.length)}
                </option>
              ))}
            </select>
          </div>
          <ul className="change-list" aria-label={t.changes.files}>
            {changes.map((c) => (
              <ChangeItem
                key={c.file}
                change={c}
                seen={!!seen?.has(c.file)}
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
              <Button variant="primary" disabled={chosen.length === 0 || previewing} loading={applying} onClick={openConfirm}>
                {chosen.length === changes.length ? t.changes.applyAll : t.changes.applySome(chosen.length)}
              </Button>
            </div>
          </div>
        </aside>

        <Splitter id="changes" side="start" label={t.common.resizePanes} />
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
              <DiffViewer diff={current.diff} file={current.file} choice={hunkChoice(current)} />
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
        confirmDisabled={unclean.length > 0 || (ownBranch && dirty.length > 0)}
        onCancel={() => setConfirming(false)}
        onConfirm={async () => {
          const all = chosen.length === changes.length;
          const ok = await applyChanges(chosen.map((c) => c.file), skipped, false, ownBranch ? commitMessage : undefined);
          if (ok) {
            setConfirming(false);
            setAppliedAll(all);
          }
        }}
      >
        <span className="tabular">{t.changes.confirmText(totals.add, totals.del)}</span>{" "}
        <span>{t.changes.confirmKinds(chosen.length - reviewChosen.length, reviewChosen.length)}</span>
        {unseen > 0 && (
          <Callout tone="warn" icon={<ShieldCheck size={15} />}>
            {t.changes.unseen(unseen)}
          </Callout>
        )}
        {dirty.length > 0 && (
          <Callout tone="warn" icon={<GitBranch size={15} />}>
            <strong>{t.git.dirtyTitle(dirty.length)}</strong> {t.git.dirtyText}
            <div className="mono selectable dirty-list">{dirty.slice(0, 5).join(", ")}</div>
          </Callout>
        )}
        {unclean.length > 0 && (
          <Callout tone="danger" icon={<GitBranch size={15} />}>
            <strong>{t.git.uncleanTitle(unclean.length)}</strong> {t.git.uncleanText}
            <div className="mono selectable dirty-list">{unclean.slice(0, 5).join(", ")}</div>
          </Callout>
        )}
        {project?.isGitRepo && (
          <label className="git-batch">
            <input type="checkbox" className="checkbox" checked={ownBranch} onChange={(e) => setOwnBranch(e.target.checked)} />
            <span>{t.git.ownBranch}</span>
          </label>
        )}
        {project?.isGitRepo && (
          <details className="commit-message">
            <summary>{t.git.message}</summary>
            <CommandLine command={commitMessage} />
          </details>
        )}
      </ConfirmDialog>
    </>
  );
}

function ChangeItem({
  change,
  seen,
  checked,
  active,
  onToggle,
  onSelect,
}: {
  change: FileChange;
  seen: boolean;
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
          {change.fix !== "auto" && !seen && <span className="change-item__unseen">{t.changes.notViewed}</span>}
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
