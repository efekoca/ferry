import { useEffect, useMemo, useState } from "react";
import { ArrowRight, CircleAlert, CircleCheck, CircleStop, Download, FileCode2, GitBranch, GitCompareArrows, ListChecks, Pin, RotateCw, ShieldCheck, Sparkles, Wand2 } from "lucide-react";
import { DiffViewer, type HunkChoice } from "../components/DiffViewer";
import { ConfirmDialog } from "../components/Modal";
import { PageHeader, usePageTitle } from "../components/Shell";
import { Splitter } from "../components/Splitter";
import { Button, Callout, CommandLine, EmptyState, Progress, RiskBadge, RuleChips, Segmented, Select } from "../components/ui";
import { useI18n } from "../i18n";
import { useStore } from "../lib/store";
import { isPin } from "../lib/fixes";
import { stepsBetween, type FileChange, type RectorMode } from "../lib/types";

const ALL_STEPS = "all";
const DIFF_VIEW = "ferry.diffView";


function storedView() {
  try {
    return localStorage.getItem(DIFF_VIEW) === "split" ? "split" : "unified";
  } catch {
    return "unified";
  }
}

export function Changes() {
  const { t, loc } = useI18n();
  const { toolchain, installRector, installingRector, preview, previewMode, previewing, loadPreview, clearPreview, cancelPreview, skippedHunks: skipped, skipHunks, applyChanges, applying, target, setView, api, project } =
    useStore();
  const [dirty, setDirty] = useState<string[]>([]);
  const [unclean, setUnclean] = useState<string[]>([]);
  const [ownBranch, setOwnBranch] = useState(false);
  const [kinds, setKinds] = useState({ compat: previewMode !== "pins", modernize: previewMode === "modernize", pins: previewMode === "pins" });
  const mode: RectorMode = kinds.modernize ? "modernize" : kinds.compat ? "compat" : "pins";
  const withPins = kinds.pins && mode !== "pins";
  const [scope, setScope] = useState<"all" | "code" | "pins">("all");
  const [diffView, setDiffView] = useState<"unified" | "split">(storedView);
  usePageTitle(t.changes.title);
  const [excluded, setExcluded] = useState<{ previewId: number | null; files: Set<string> }>({ previewId: null, files: new Set() });
  const [active, setActive] = useState<string | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [appliedAll, setAppliedAll] = useState(false);

  const steps = useMemo(() => stepsBetween(project?.currentVersion ?? null, target), [project, target]);
  const [pickedStep, setPickedStep] = useState(() => (preview && preview.target !== target ? preview.target : ""));
  const step = mode !== "pins" && steps.includes(pickedStep) ? pickedStep : undefined;
  const nextStep = step ? (steps[steps.indexOf(step) + 1] ?? target) : null;
  const goToStep = (value: string) => {
    setPickedStep(value);
    void loadPreview(mode, steps.includes(value) ? value : undefined, withPins);
  };
  const stepPicker = steps.length > 0 && mode !== "pins" && (
    <div className="step-picker">
      <span>{t.changes.step}</span>
      <Select
        label={t.changes.step}
        value={step ?? ALL_STEPS}
        disabled={previewing}
        options={[{ value: ALL_STEPS, label: t.changes.stepAll(target) }, ...steps.map((v) => ({ value: v, label: t.changes.stepTo(v) }))]}
        onChange={(v) => {
          const picked = v === ALL_STEPS ? "" : v;
          if (preview) goToStep(picked);
          else setPickedStep(picked);
        }}
      />
    </div>
  );
  const nextStepButton = nextStep && (
    <Button variant="primary" onClick={() => goToStep(nextStep)}>
      {t.changes.nextStep(nextStep)}
    </Button>
  );

  const options = [
    { value: "compat" as const, label: t.changes.compat, text: t.changes.compatShort, icon: ShieldCheck },
    { value: "modernize" as const, label: t.changes.modernize, text: t.changes.modernizeShort, icon: Sparkles },
    { value: "pins" as const, label: t.changes.pins, text: t.changes.pinsShort, icon: Pin },
  ];
  const nothingText = { compat: t.changes.nothingCompat, modernize: t.changes.nothingModern, pins: t.changes.nothingPins };
  const onlyPins = () => setKinds({ compat: false, modernize: false, pins: true });
  const toggleKind = (kind: "compat" | "modernize" | "pins") =>
    setKinds((k) => {
      const on = !k[kind];
      if (kind === "modernize") return { ...k, modernize: on, compat: on || k.compat };
      if (kind === "compat") return { ...k, compat: on, modernize: on && k.modernize };
      return { ...k, pins: on };
    });
  const nothingChosen = !kinds.compat && !kinds.modernize && !kinds.pins;
  const pinsInstead = (
    <Button variant="secondary" icon={<Pin size={15} />} onClick={onlyPins}>
      {t.changes.pinsInstead}
    </Button>
  );

  useEffect(() => {
    if (preview?.changes.length) setAppliedAll(false);
  }, [preview]);

  const changes = useMemo(() => preview?.changes ?? [], [preview]);
  const off = useMemo(
    () =>
      preview && excluded.previewId === preview.id
        ? excluded.files
        : new Set(changes.filter((c) => c.fix !== "auto" && !isPin(c)).map((c) => c.file)),
    [preview, excluded, changes],
  );
  const [viewed, setViewed] = useState<{ previewId: number | null; files: Set<string> }>({ previewId: null, files: new Set() });
  const seen = preview && viewed.previewId === preview.id ? viewed.files : null;
  const selected = useMemo(() => new Set(changes.filter((c) => !off?.has(c.file)).map((c) => c.file)), [changes, off]);
  const pinCount = useMemo(() => changes.filter(isPin).length, [changes]);
  const mixed = pinCount > 0 && pinCount < changes.length;
  const shownChanges = useMemo(() => (!mixed || scope === "all" ? changes : changes.filter((c) => isPin(c) === (scope === "pins"))), [changes, mixed, scope]);
  const groups = useMemo(() => {
    const code = shownChanges.filter((c) => !isPin(c));
    return [
      { key: "auto", label: t.fix.auto, items: code.filter((c) => c.fix === "auto") },
      { key: "review", label: t.changes.reviewGroup, items: code.filter((c) => c.fix !== "auto") },
      { key: "pins", label: t.changes.pins, items: shownChanges.filter(isPin) },
    ].filter((g) => g.items.length > 0);
  }, [shownChanges, t]);
  const current = shownChanges.find((c) => c.file === active) ?? shownChanges[0] ?? null;
  useEffect(() => {
    if (!preview || !current || seen?.has(current.file)) return;
    setViewed({ previewId: preview.id, files: new Set([...(seen ?? []), current.file]) });
  }, [preview, current, seen]);
  const chosen = useMemo(() => changes.filter((c) => selected.has(c.file)), [changes, selected]);
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

  const hunkChoice = (change: FileChange): HunkChoice => ({
    isOn: (h) => selected.has(change.file) && !skipped[change.file]?.includes(h),
    toggle: (h) => {
      const count = change.diff.hunks.length;
      const others = change.diff.hunks.map((_, i) => i).filter((i) => i !== h);
      const current = skipped[change.file] ?? [];
      if (!selected.has(change.file)) {
        setSelected(new Set([...selected, change.file]));
        skipHunks(change.file, others);
      } else if (current.includes(h)) {
        skipHunks(change.file, current.filter((i) => i !== h));
      } else if (current.length + 1 === count) {
        setSelected(new Set([...selected].filter((f) => f !== change.file)));
        skipHunks(change.file, []);
      } else {
        skipHunks(change.file, [...current, h]);
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
    const pins = chosen.filter(isPin).length;
    const kind = pins === 0 ? "code" : pins === chosen.length ? "pins" : "mixed";
    return [t.git.subject(preview?.target ?? target, chosen.length, kind), "", ...lines].join("\n");
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
            {mixed && (
              <Segmented
                label={t.changes.scope}
                value={scope}
                onChange={setScope}
                options={[
                  { value: "all", label: <>{t.changes.scopeAll} <span className="segmented__count">{changes.length}</span></> },
                  { value: "code", label: <>{mode === "modernize" ? t.changes.modernize : t.changes.compat} <span className="segmented__count">{changes.length - pinCount}</span></> },
                  { value: "pins", label: <>{t.changes.pins} <span className="segmented__count">{pinCount}</span></> },
                ]}
              />
            )}
            {stepPicker}
            {previewing && (
              <Button variant="ghost" size="sm" icon={<CircleStop size={14} />} onClick={cancelPreview}>
                {t.changes.stop}
              </Button>
            )}
            <Button variant="ghost" size="sm" icon={<ListChecks size={14} />} disabled={previewing} onClick={clearPreview}>
              {t.changes.changeChoice}
            </Button>
            <Button variant="secondary" size="sm" icon={<RotateCw size={14} />} onClick={() => loadPreview(mode, step, withPins)} loading={previewing}>
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
        <h1 className="sr-only">{t.changes.title}</h1>
        <div className="overview-start fixes-start" data-tauri-drag-region>
          <h2 className="page-title">{t.changes.startTitle}</h2>
          <p className="page-lead">{t.changes.startLead}</p>
          <div className="opts" role="group" aria-label={t.changes.mode}>
            {options.map((o) => (
              <label key={o.value} className="opt" data-checked={kinds[o.value]}>
                <input type="checkbox" className="checkbox opt__check" checked={kinds[o.value]} disabled={previewing} onChange={() => toggleKind(o.value)} aria-label={o.label} />
                <span className="opt__icon" aria-hidden>
                  <o.icon size={15} />
                </span>
                <span>
                  <span className="opt__title">{o.label}</span>
                  <span className="opt__text">{o.text}</span>
                </span>
              </label>
            ))}
          </div>
          {stepPicker}
          {previewing ? (
            <div className="fixes-start__progress">
              <Progress value={null} label={t.changes.calculating} />
              <div className="row row--between">
                <span className="hint">{t.changes.calculatingHint}</span>
                <Button variant="secondary" size="sm" icon={<CircleStop size={14} />} onClick={cancelPreview}>
                  {t.changes.stop}
                </Button>
              </div>
            </div>
          ) : (
            <Button variant="primary" size="cta" disabled={nothingChosen} onClick={() => loadPreview(mode, step, withPins)}>
              {t.changes.calculate}
              <span className="btn__chip">
                <ArrowRight size={15} aria-hidden />
              </span>
            </Button>
          )}
          {mode !== "pins" && <p className="hint">{t.changes.introTrust}</p>}
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
              <div className="row">
                {nextStepButton}
                <Button variant="secondary" onClick={() => setView("findings")}>
                  {t.changes.seeRemaining}
                </Button>
              </div>
            }
          >
            {t.changes.allAppliedText}
          </EmptyState>
        ) : (
          <EmptyState icon={<CircleCheck size={24} />} title={t.changes.nothing} action={nextStepButton || undefined}>
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
            <Select
              block
              title={t.git.byRuleNote}
              label={t.git.selectBy}
              value=""
              options={[
                { value: "all", label: t.git.selectAll },
                { value: "auto", label: t.git.onlyAuto },
                ...ruleOptions.map(([cls, r]) => ({ value: cls, label: t.git.byRule(r.title, r.files.length) })),
              ]}
              onChange={narrow}
            />
          </div>
          <div className="change-list" role="list" aria-label={t.changes.files}>
            {groups.map((group) => (
              <div key={group.key} role="presentation">
                <div className="change-group">{group.label}</div>
                {group.items.map((c) => (
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
              </div>
            ))}
          </div>
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
              <div className="diff-rules">
                <RiskBadge fix={current.fix} />
                <RuleChips rules={current.rules} />
              </div>
              {current.fix === "likely" && (
                <div className="diff-note">
                  <Callout tone="info" icon={<ShieldCheck size={15} />}>
                    {t.changes.likelyNote}
                  </Callout>
                </div>
              )}
              <DiffViewer
                diff={current.diff}
                file={current.file}
                choice={hunkChoice(current)}
                split={diffView === "split"}
                bar={
                  <>
                    <FileCode2 size={14} aria-hidden />
                    <span className="diff__path" title={current.file}>
                      {current.file.slice(0, current.file.lastIndexOf("/") + 1)}
                      <b>{current.file.slice(current.file.lastIndexOf("/") + 1)}</b>
                    </span>
                    <span className="stat-add">+{current.diff.additions}</span>
                    <span className="stat-del">−{current.diff.deletions}</span>
                    <Segmented
                      size="sm"
                      label={t.changes.diffView}
                      value={diffView}
                      onChange={(view) => {
                        setDiffView(view);
                        try {
                          localStorage.setItem(DIFF_VIEW, view);
                        } catch {
                          return;
                        }
                      }}
                      options={[
                        { value: "unified", label: t.changes.unified },
                        { value: "split", label: t.changes.split },
                      ]}
                    />
                  </>
                }
              />
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
    <div className="change-item" role="listitem" data-active={active} onClick={onSelect}>
      <input
        type="checkbox"
        className="checkbox"
        checked={checked}
        onChange={onToggle}
        onClick={(e) => e.stopPropagation()}
        aria-label={t.changes.selectFile(change.file)}
      />
      <button type="button" className="change-item__body" aria-current={active || undefined} onClick={onSelect}>
        <span className="change-item__file" title={change.file}>
          {change.file.slice(0, change.file.lastIndexOf("/") + 1)}
          <b>{change.file.slice(change.file.lastIndexOf("/") + 1)}</b>
        </span>
        <span className="stat-add">+{change.diff.additions}</span>
        <span className="stat-del">−{change.diff.deletions}</span>
        {change.fix !== "auto" && !seen && (
          <span className="change-item__unseen" title={t.changes.notViewed}>
            <span className="sr-only">{t.changes.notViewed}</span>
          </span>
        )}
      </button>
    </div>
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
