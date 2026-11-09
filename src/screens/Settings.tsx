import { useEffect, useRef, useState } from "react";
import { FolderOpen, RefreshCw, ShieldAlert, Trash2, X } from "lucide-react";
import { ConfirmDialog } from "../components/Modal";
import { PageHeader } from "../components/Shell";
import { Badge, Button, Callout, Card, Segmented, Select, Switch, TagInput, TextField, Tooltip } from "../components/ui";
import { useToast } from "../components/Toast";
import { LANGS, useI18n, type Lang } from "../i18n";
import { errorMessage } from "../lib/api";
import { formatBytes, relativeTo } from "../lib/labels";
import { PHP2SHELL_CVES, isVulnerable, patchedFor } from "../lib/security";
import { useStore, type Theme } from "../lib/store";
import type { BackupUsage, DockerImageUsage, Settings } from "../lib/types";

const SYSTEM_EDITOR = "system";

const EDITORS = [
  ["vscode", "Visual Studio Code"],
  ["cursor", "Cursor"],
  ["phpstorm", "PhpStorm"],
  ["sublime", "Sublime Text"],
  ["zed", "Zed"],
] as const;

function Label({ text, hint }: { text: string; hint: string }) {
  return (
    <span className="line__main">
      <Tooltip text={hint} align="start">
        <span className="setting-label" tabIndex={0}>
          {text}
        </span>
      </Tooltip>
    </span>
  );
}

function EditorCommand({ value, onSave }: { value: string; onSave: (command: string) => void }) {
  const { t } = useI18n();
  const [command, setCommand] = useState(value);
  useEffect(() => setCommand(value), [value]);
  const missing = command.trim() !== "" && !command.includes("{file}");
  return (
    <div className="setting-command">
      <TextField
        label={t.settings.editorCommand}
        mono
        placeholder="emacsclient -n +{line}:{column} {file}"
        spellCheck={false}
        aria-invalid={missing}
        value={command}
        onChange={(e) => setCommand(e.target.value)}
        onBlur={() => onSave(command.trim())}
      />
      <p className={missing ? "field-error" : "hint"}>{missing ? t.settings.editorCommandMissing : t.settings.editorCommandHint}</p>
    </div>
  );
}

function ProjectSettings() {
  const { t } = useI18n();
  const { api, project, saveConfig } = useStore();
  const toast = useToast();
  const [exclude, setExclude] = useState(project?.config.exclude ?? []);
  const [requireClean, setRequireClean] = useState(project?.config.git.requireClean ?? false);
  const [branchPerBatch, setBranchPerBatch] = useState(project?.config.git.branchPerBatch ?? false);
  const [keepTarget, setKeepTarget] = useState(true);
  if (!project) return null;
  const pinned = keepTarget ? project.config.target : null;
  const pickFolder = async () => {
    if (!api) return;
    try {
      const path = await api.pickProjectFolder(t.settings.projectExcludePick);
      if (!path) return;
      const folder = relativeTo(project.root, path);
      if (!folder) toast.push({ tone: "error", title: t.settings.projectExcludeOutside });
      else if (!exclude.includes(folder)) setExclude([...exclude, folder]);
    } catch (e) {
      toast.push({ tone: "error", title: t.toast.saveFailed, description: errorMessage(e, t.common.genericError) });
    }
  };
  return (
    <Card className="settings-fill" title={project.name} actions={
        pinned && (
          <span className="pinned-target">
            <Badge>{t.settings.projectTarget(pinned)}</Badge>
            <Tooltip text={t.settings.projectTargetClear} align="end">
              <Button variant="ghost" size="sm" iconOnly aria-label={t.settings.projectTargetClear} onClick={() => setKeepTarget(false)}>
                <X size={12} />
              </Button>
            </Tooltip>
          </span>
        )
      } bodyClassName="card__body setting-project">
      <TagInput
        label={t.settings.projectExclude}
        values={exclude}
        placeholder={t.settings.projectExcludeAdd}
        removeLabel={t.settings.projectExcludeRemove}
        onChange={setExclude}
        end={
          <Tooltip text={t.settings.projectExcludePick} align="end">
            <Button variant="ghost" size="sm" iconOnly aria-label={t.settings.projectExcludePick} onClick={pickFolder}>
              <FolderOpen size={14} />
            </Button>
          </Tooltip>
        }
      />
      {project.isGitRepo && (
        <>
          <label className="setting-toggle">
            {t.settings.projectRequireClean}
            <Switch checked={requireClean} onChange={(e) => setRequireClean(e.target.checked)} />
          </label>
          <label className="setting-toggle">
            {t.settings.projectBranchPerBatch}
            <Switch checked={branchPerBatch} onChange={(e) => setBranchPerBatch(e.target.checked)} />
          </label>
        </>
      )}
      <div className="setting-project__end">
        <Tooltip text={t.settings.projectNote} align="end">
          <Button onClick={() => void saveConfig({ ...project.config, target: pinned, exclude, git: { requireClean, branchPerBatch } })}>{t.settings.projectSave}</Button>
        </Tooltip>
      </div>
    </Card>
  );
}

export function SettingsScreen() {
  const { t, lang, setLang } = useI18n();
  const { theme, setTheme, toolchain, refreshToolchain, installRector, installingRector, previewing, api, project, refreshSnapshots } = useStore();
  const toast = useToast();
  const [checking, setChecking] = useState(false);
  const [settings, setSettings] = useState<Settings>({ phpPath: null, snapshotLimit: null, editor: null, editorCommand: null });
  const [usage, setUsage] = useState<BackupUsage | null>(null);
  const [clearing, setClearing] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [images, setImages] = useState<DockerImageUsage | null>(null);
  const [clearingImages, setClearingImages] = useState(false);
  const [rectorCache, setRectorCache] = useState(0);

  useEffect(() => {
    api?.getSettings().then(setSettings).catch(() => {});
    api?.backupUsage().then(setUsage).catch(() => {});
    api?.dockerImageUsage().then(setImages).catch(() => {});
    api?.rectorCacheSize().then(setRectorCache).catch(() => {});
  }, [api]);

  const clearCache = async () => {
    if (!api) return;
    try {
      await api.clearRectorCache();
      toast.push({ tone: "success", title: t.settings.rectorCacheDone });
    } catch (e) {
      toast.push({ tone: "error", title: t.toast.deleteFailed, description: errorMessage(e, t.common.genericError) });
    } finally {
      api.rectorCacheSize().then(setRectorCache).catch(() => {});
    }
  };

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
    { value: "system" as Theme, label: t.settings.system },
    { value: "light" as Theme, label: t.settings.light },
    { value: "dark" as Theme, label: t.settings.dark },
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
  const toolPlace = (path?: string) => {
    if (!path) return t.settings.notFound;
    return toolchain && path.startsWith(toolchain.toolsDir) ? t.settings.inToolsDir : path;
  };

  return (
    <>
      <PageHeader title={t.settings.title} />
      <div className="page-scroll">
        <div className="page-body">
          <div className="settings-grid">
            <div className="settings-column">
              <Card title={t.settings.appearance} bodyClassName="lines-card">
                <div className="lines">
                  <div className="line">
                    <span className="line__main">{t.settings.language}</span>
                    <Segmented<Lang> size="sm" label={t.settings.language} options={LANGS} value={lang} onChange={setLang} />
                  </div>
                  <div className="line">
                    <span className="line__main">{t.settings.theme}</span>
                    <Segmented size="sm" label={t.settings.theme} options={themes} value={theme} onChange={setTheme} />
                  </div>
                  <div className="line">
                    <Label text={t.settings.editor} hint={t.settings.editorText} />
                    <div className="setting-select">
                      <Select
                        label={t.settings.editor}
                        value={settings.editor ?? SYSTEM_EDITOR}
                        options={[
                          { value: SYSTEM_EDITOR, label: t.settings.editorSystem },
                          ...EDITORS.map(([value, label]) => ({ value, label })),
                          { value: "custom", label: t.settings.editorCustom },
                        ]}
                        onChange={(editor) => void save({ editor: editor === SYSTEM_EDITOR ? null : editor })}
                      />
                    </div>
                  </div>
                  {settings.editor === "custom" && <EditorCommand value={settings.editorCommand ?? ""} onSave={(editorCommand) => void save({ editorCommand: editorCommand || null })} />}
                </div>
              </Card>
              {project && <ProjectSettings key={project.root} />}
            </div>

            <div className="settings-column">
              <Card
                title={t.settings.tools}
                actions={
                  <Tooltip text={t.settings.recheck} align="end">
                    <Button size="sm" variant="ghost" iconOnly aria-label={t.settings.recheck} onClick={recheck} loading={checking}>
                      <RefreshCw size={14} />
                    </Button>
                  </Tooltip>
                }
                bodyClassName="lines-card"
              >
                <ul className="lines">
                  {tools.map((tool) => (
                    <li key={tool.name} className="line">
                      <span
                        className={`dot ${tool.info ? (tool.name === "PHP" && phpVersion && isVulnerable(phpVersion) ? "dot--error" : "dot--ok") : tool.required ? "dot--error" : "dot--warn"}`}
                        aria-hidden
                      />
                      <span className="line__main">
                        <span className="line__title">{tool.name}</span>
                        <span className="line__sub selectable" title={tool.info?.path}>
                          {toolPlace(tool.info?.path)}
                        </span>
                      </span>
                      <span className="line__end">
                        {tool.info?.version && <Badge mono>{tool.info.version.match(/\d+(\.\d+)+/)?.[0] ?? tool.info.version}</Badge>}
                        {tool.name === "PHP" && (
                          <>
                            {settings.phpPath && (
                              <Button size="sm" variant="ghost" onClick={resetPhp}>
                                {t.settings.auto}
                              </Button>
                            )}
                            <Button size="sm" onClick={choosePhp}>
                              {t.settings.choose}
                            </Button>
                          </>
                        )}
                        {tool.name === "Rector" && (
                          <Button size="sm" onClick={installRector} loading={installingRector} disabled={!toolchain?.composer} title={toolchain && !toolchain.composer ? t.settings.needsComposer : undefined}>
                            {tool.info ? t.settings.update : t.settings.install}
                          </Button>
                        )}
                      </span>
                    </li>
                  ))}
                </ul>
                {phpVersion && isVulnerable(phpVersion) && (
                  <div className="setting-warning">
                    <Callout tone="danger" icon={<ShieldAlert size={15} aria-hidden />}>
                      {t.settings.vulnerablePhp(phpVersion, PHP2SHELL_CVES, patchedFor(phpVersion))}
                    </Callout>
                  </div>
                )}
              </Card>

              <Card title={t.settings.storage} className="settings-fill" bodyClassName="lines-card">
                <div className="lines">
                  <div className="line">
                    <Label text={t.settings.keep} hint={t.settings.keepText} />
                    <Segmented<LimitValue>
                      size="sm"
                      label={t.settings.keep}
                      options={limits}
                      value={limits.some((l) => l.value === limitValue) ? limitValue : "none"}
                      onChange={(v) => save({ snapshotLimit: v === "none" ? null : Number(v) })}
                    />
                  </div>
                  <div className="line">
                    <span className="line__main">
                      <span>{t.settings.usage}</span>
                      <span className="line__sub">{usage && usage.snapshots > 0 ? t.settings.usageText(usage.snapshots, usage.projects, size) : t.settings.usageEmpty}</span>
                    </span>
                    <Button size="sm" variant="ghost" className="danger-text" onClick={() => setClearing(true)} disabled={!usage?.snapshots}>
                      {t.settings.deleteAll}
                    </Button>
                  </div>
                  <div className="line">
                    <span className="line__main">
                      <span>{t.settings.rectorCache}</span>
                      <span className="line__sub">{rectorCache > 0 ? formatBytes(rectorCache, t.locale) : t.settings.rectorCacheEmpty}</span>
                    </span>
                    <Button size="sm" variant="ghost" className="danger-text" onClick={clearCache} disabled={rectorCache === 0 || previewing}>
                      {t.settings.rectorCacheDelete}
                    </Button>
                  </div>
                  <div className="line">
                    <span className="line__main">
                      <span>{t.settings.images}</span>
                      <span className="line__sub">{!images ? t.settings.imagesUnknown : images.images > 0 ? t.settings.imagesText(images.images, imagesSize) : t.settings.imagesEmpty}</span>
                    </span>
                    <Button size="sm" variant="ghost" className="danger-text" onClick={() => setClearingImages(true)} disabled={!images?.images}>
                      {t.settings.imagesDelete}
                    </Button>
                  </div>
                </div>
              </Card>
            </div>
          </div>

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
        </div>
      </div>
    </>
  );
}
