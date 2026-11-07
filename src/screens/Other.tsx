import { useEffect, useRef, useState, type ReactElement } from "react";
import {
  Bot,
  Check,
  FolderOpen,
  Monitor,
  Moon,
  RefreshCw,
  Sun,
  Trash2,
  ShieldAlert,
} from "lucide-react";
import { ConfirmDialog } from "../components/Modal";
import { PageHeader } from "../components/Shell";
import { Badge, Button, Callout, Card, Segmented, Select, TagInput } from "../components/ui";
import { PHP2SHELL_CVES, isVulnerable, patchedFor } from "../lib/security";
import { useToast } from "../components/Toast";
import { LANGS, useI18n, type Lang } from "../i18n";
import { errorMessage } from "../lib/api";
import { formatBytes } from "../lib/labels";
import { useStore, type Theme } from "../lib/store";
import type { BackupUsage, DockerImageUsage, Settings } from "../lib/types";

const withIcon = (icon: ReactElement, label: string) => (
  <span className="row row--tight">
    {icon}
    {label}
  </span>
);

const SYSTEM_EDITOR = "system";

const EDITORS = [
  ["vscode", "Visual Studio Code"],
  ["cursor", "Cursor"],
  ["phpstorm", "PhpStorm"],
  ["sublime", "Sublime Text"],
  ["zed", "Zed"],
] as const;

function EditorCommand({ value, onSave }: { value: string; onSave: (command: string) => void }) {
  const { t } = useI18n();
  const [command, setCommand] = useState(value);
  useEffect(() => setCommand(value), [value]);
  const missing = command.trim() !== "" && !command.includes("{file}");
  return (
    <div className="setting setting--stacked">
      <label className="docker-field">
        <span>{t.settings.editorCommand}</span>
        <input
          className="input mono"
          placeholder="emacsclient -n +{line}:{column} {file}"
          spellCheck={false}
          aria-invalid={missing}
          value={command}
          onChange={(e) => setCommand(e.target.value)}
          onBlur={() => onSave(command.trim())}
        />
      </label>
      <p className={missing ? "field-error" : "hint"}>{missing ? t.settings.editorCommandMissing : t.settings.editorCommandHint}</p>
    </div>
  );
}

function ProjectSettings() {
  const { t } = useI18n();
  const { project, saveConfig } = useStore();
  const [exclude, setExclude] = useState(project?.config.exclude ?? []);
  const [requireClean, setRequireClean] = useState(project?.config.git.requireClean ?? false);
  const [branchPerBatch, setBranchPerBatch] = useState(project?.config.git.branchPerBatch ?? false);
  if (!project) return null;
  const pinned = project.config.target;
  return (
    <Card title={t.settings.projectTitle(project.name)}>
      <p className="subtle">{t.settings.projectNote}</p>
      {pinned && <p>{t.settings.projectTarget(pinned)}</p>}
      <TagInput className="field" label={t.settings.projectExclude} values={exclude} placeholder={t.settings.projectExcludeAdd} removeLabel={t.settings.projectExcludeRemove} onChange={setExclude} />
      {project.isGitRepo && (
        <>
          <label className="git-batch">
            <input type="checkbox" className="checkbox" checked={requireClean} onChange={(e) => setRequireClean(e.target.checked)} />
            <span>{t.settings.projectRequireClean}</span>
          </label>
          <label className="git-batch">
            <input type="checkbox" className="checkbox" checked={branchPerBatch} onChange={(e) => setBranchPerBatch(e.target.checked)} />
            <span>{t.settings.projectBranchPerBatch}</span>
          </label>
        </>
      )}
      <div className="row row--end">
        <Button variant="secondary" size="sm" onClick={() => void saveConfig({ ...project.config, exclude, git: { requireClean, branchPerBatch } })}>
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
  const [settings, setSettings] = useState<Settings>({ phpPath: null, snapshotLimit: null, editor: null, editorCommand: null });
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
              {settings.editor === "custom" && <EditorCommand value={settings.editorCommand ?? ""} onSave={(editorCommand) => void save({ editorCommand: editorCommand || null })} />}
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
