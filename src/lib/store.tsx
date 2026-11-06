import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { errorMessage, getApi, type Api } from "./api";
import type {
  CheckSummary,
  SnapshotConflict,
  DependencyReport,
  Framework,
  ProjectConfig,
  ProjectInfo,
  RectorMode,
  RectorPreview,
  ScanProgress,
  ScanResult,
  FixKind,
  Severity,
  Snapshot,
  Target,
  ToolchainStatus,
  DatabaseChoice,
  DockerComparison,
  DockerStep,
  DockerRunArgs,
  DockerSummary,
  DockerTrial,
} from "./types";
import { TARGETS } from "./types";
import { isBlockedTarget } from "./security";
import { FRAMEWORK_LABEL } from "./labels";
import { useToast } from "../components/Toast";
import { useI18n } from "../i18n";

export type View = "overview" | "findings" | "changes" | "dependencies" | "history" | "runtime" | "ai" | "settings";
export type Theme = "system" | "light" | "dark";

export interface FindingsFilter {
  fix: FixKind | null;
  severity: Severity | null;
  rule: string | null;
  query: string;
}

export const NO_FILTER: FindingsFilter = { fix: null, severity: null, rule: null, query: "" };

export interface RecentProject {
  path: string;
  name: string;
  framework: Framework;
  openedAt: number;
}

const RECENT_KEY = "ferry.recent";
const THEME_KEY = "ferry.theme";
const TARGET_KEY = "ferry.target";

const THEMES: readonly Theme[] = ["system", "light", "dark"];
const FRAMEWORKS = Object.keys(FRAMEWORK_LABEL) as Framework[];

function readStorage<T>(key: string, fallback: T, valid: (v: unknown) => boolean): T {
  try {
    const raw = localStorage.getItem(key);
    if (!raw) return fallback;
    const value: unknown = JSON.parse(raw);
    return valid(value) ? (value as T) : fallback;
  } catch {
    return fallback;
  }
}

function isRecent(v: unknown): v is RecentProject {
  if (!v || typeof v !== "object") return false;
  const r = v as Record<string, unknown>;
  return (
    typeof r.path === "string" &&
    typeof r.name === "string" &&
    typeof r.openedAt === "number" &&
    FRAMEWORKS.includes(r.framework as Framework)
  );
}

function readRecent(): RecentProject[] {
  return readStorage<unknown[]>(RECENT_KEY, [], Array.isArray).filter(isRecent).slice(0, 6);
}

function writeStorage(key: string, value: unknown) {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {}
}

interface AppStore {
  api: Api | null;
  view: View;
  setView(v: View): void;
  findingsFilter: FindingsFilter;
  setFindingsFilter(f: FindingsFilter): void;
  showFindings(f?: Partial<FindingsFilter>): void;
  theme: Theme;
  setTheme(t: Theme): void;

  recent: RecentProject[];
  forgetRecent(path: string): void;
  project: ProjectInfo | null;
  opening: boolean;
  openProject(path?: string): Promise<void>;
  closeProject(): void;

  target: Target;
  setTarget(t: Target): void;

  scan: ScanResult | null;
  scanning: boolean;
  progress: ScanProgress | null;
  runScan(target?: Target): Promise<void>;
  cancelScan(): void;

  deps: DependencyReport | null;

  toolchain: ToolchainStatus | null;
  refreshToolchain(): Promise<void>;
  installingRector: boolean;
  installRector(): Promise<void>;

  preview: RectorPreview | null;
  previewMode: RectorMode;
  previewing: boolean;
  loadPreview(mode: RectorMode, step?: string): Promise<boolean>;
  cancelPreview(): void;
  docker: DockerComparison | null;
  dockerHistory: DockerSummary[];
  checkHistory: CheckSummary[];
  dockerStep: DockerStep | null;
  runDocker(source: string, extensions: boolean, cleanImages: boolean, pages: string[], database: DatabaseChoice | null, prepare: string | null, test: string | null): Promise<boolean>;
  cancelDocker(): void;
  saveConfig(config: ProjectConfig): Promise<boolean>;
  dockerArgs: DockerRunArgs | null;
  dockerTrial: DockerTrial | null;
  tryFixes(files: string[]): Promise<boolean>;
  skippedHunks: Record<string, number[]>;
  skipHunks(file: string, hunks: number[]): void;
  applying: boolean;
  applyChanges(files: string[], skipped?: Record<string, number[]>, keepTrial?: boolean, commitMessage?: string): Promise<boolean>;

  snapshots: Snapshot[];
  refreshSnapshots(): Promise<void>;
  restoreSnapshot(id: string, acknowledged: SnapshotConflict[]): Promise<void>;
  deleteSnapshot(id: string): Promise<void>;
  deleteAllSnapshots(): Promise<void>;

  exportReport(): Promise<void>;
  ignoreFinding(id: string, reason: string): Promise<boolean>;
  unignoreFinding(id: string): Promise<boolean>;
  pruneIgnores(): Promise<boolean>;
  importRuntimeLog(): Promise<boolean>;
  setRuntimeRoots(roots: string): Promise<boolean>;
  clearRuntimeLog(): Promise<boolean>;
  runLocalCheck(kind: "lint" | "tests"): Promise<boolean>;
}

const StoreContext = createContext<AppStore | null>(null);

export function StoreProvider({ children, api: injected }: { children: ReactNode; api?: Api }) {
  const toast = useToast();
  const { t, lang } = useI18n();
  const [api, setApi] = useState<Api | null>(injected ?? null);
  const [view, setView] = useState<View>("overview");
  const [findingsFilter, setFindingsFilter] = useState<FindingsFilter>(NO_FILTER);
  const [theme, setThemeState] = useState<Theme>(() => readStorage<Theme>(THEME_KEY, "system", (v) => THEMES.includes(v as Theme)));
  const [recent, setRecent] = useState<RecentProject[]>(readRecent);
  const [project, setProject] = useState<ProjectInfo | null>(null);
  const [opening, setOpening] = useState(false);
  const [target, setTargetState] = useState<Target>(() => readStorage<Target>(TARGET_KEY, "8.4", (v) => TARGETS.includes(v as Target) && !isBlockedTarget(v as string)));
  const [scan, setScan] = useState<ScanResult | null>(null);
  const [scanning, setScanning] = useState(false);
  const [progress, setProgress] = useState<ScanProgress | null>(null);
  const [deps, setDeps] = useState<DependencyReport | null>(null);
  const [toolchain, setToolchain] = useState<ToolchainStatus | null>(null);
  const [docker, setDocker] = useState<DockerComparison | null>(null);
  const [dockerStep, setDockerStep] = useState<DockerStep | null>(null);
  const [dockerArgs, setDockerArgs] = useState<DockerRunArgs | null>(null);
  const [dockerTrial, setDockerTrial] = useState<DockerTrial | null>(null);
  const [dockerHistory, setDockerHistory] = useState<DockerSummary[]>([]);
  const [checkHistory, setCheckHistory] = useState<CheckSummary[]>([]);
  const [installingRector, setInstallingRector] = useState(false);
  const [preview, setPreview] = useState<RectorPreview | null>(null);
  const [hunkSkips, setHunkSkips] = useState<{ previewId: number | null; files: Record<string, number[]> }>({ previewId: null, files: {} });
  const skippedHunks = useMemo(() => (preview && hunkSkips.previewId === preview.id ? hunkSkips.files : {}), [preview, hunkSkips]);
  const skipHunks = useCallback(
    (file: string, hunks: number[]) => setHunkSkips({ previewId: preview?.id ?? null, files: { ...skippedHunks, [file]: hunks } }),
    [preview, skippedHunks],
  );
  const [previewMode, setPreviewMode] = useState<RectorMode>("compat");
  const [previewing, setPreviewing] = useState(false);
  const [applying, setApplying] = useState(false);
  const [snapshots, setSnapshots] = useState<Snapshot[]>([]);
  const generation = useRef(0);
  const previewSeq = useRef(0);
  const openSeq = useRef(0);
  const cancelledSeq = useRef(0);
  const dockerStopped = useRef(false);
  const rootRef = useRef<string | null>(null);

  useEffect(() => {
    if (!injected) getApi().then(setApi);
  }, [injected]);

  useEffect(() => {
    api?.setLanguage(lang).catch(() => {});
  }, [api, lang]);

  useEffect(() => {
    const media = window.matchMedia?.("(prefers-color-scheme: dark)");
    const apply = () => {
      const dark = theme === "dark" || (theme === "system" && !!media?.matches);
      document.documentElement.dataset.theme = dark ? "dark" : "light";
    };
    api?.setWindowTheme(theme === "system" ? null : theme).catch(() => {});
    apply();
    media?.addEventListener?.("change", apply);
    return () => media?.removeEventListener?.("change", apply);
  }, [theme, api]);

  const setTheme = useCallback((next: Theme) => {
    setThemeState(next);
    writeStorage(THEME_KEY, next);
  }, []);

  const fail = useCallback(
    (title: string, e: unknown) => toast.push({ tone: "error", title, description: errorMessage(e, t.common.genericError) }),
    [toast, t],
  );

  const refreshToolchain = useCallback(async () => {
    if (!api) return;
    try {
      setToolchain(await api.toolchainStatus());
    } catch (e) {
      fail(t.toast.toolsFailed, e);
    }
  }, [api, fail, t]);

  useEffect(() => {
    refreshToolchain();
  }, [refreshToolchain]);

  const resetResults = useCallback(() => {
    api?.cancelPreview().catch(() => {});
    generation.current++;
    previewSeq.current++;
    setDocker(null);
    setDockerArgs(null);
    setDockerTrial(null);
    setScan(null);
    setDeps(null);
    setPreview(null);
    setScanning(false);
    setPreviewing(false);
    setProgress(null);
  }, [api]);

  const refreshSnapshots = useCallback(async () => {
    if (!api || !project) return;
    const root = project.root;
    try {
      const list = await api.listSnapshots(root);
      if (rootRef.current === root) setSnapshots(list);
    } catch (e) {
      if (rootRef.current === root) fail(t.toast.historyFailed, e);
    }
  }, [api, project, fail, t]);

  useEffect(() => {
    if (project) refreshSnapshots();
    else setSnapshots([]);
  }, [project, refreshSnapshots]);

  const openProject = useCallback(
    async (path?: string) => {
      if (!api) return;
      const seq = ++openSeq.current;
      let chosen: string | null | undefined = path;
      try {
        chosen ??= await api.pickProjectFolder(t.dialogs.pickProject);
      } catch (e) {
        fail(t.toast.openFailed, e);
        return;
      }
      if (!chosen) return;
      setOpening(true);
      try {
        const info = await api.inspectProject(chosen);
        if (seq !== openSeq.current) return;
        if (rootRef.current) void api.closeProject(rootRef.current).catch(() => {});
        rootRef.current = info.root;
        resetResults();
        setFindingsFilter(NO_FILTER);
        setProject(info);
        const pinned = info.config.target as Target | null;
        if (pinned && TARGETS.includes(pinned) && !isBlockedTarget(pinned)) setTargetState(pinned);
        if (info.configError) toast.push({ tone: "error", title: t.toast.configFailed, description: info.configError });
        void api.interruptedWrites(info.root).then((list) => {
          for (const s of list)
            toast.push({
              tone: "error",
              title: t.toast.interruptedWrite,
              description: t.toast.interruptedWriteText(s.label),
              action: { label: t.toast.interruptedChecked, run: () => void api.dismissInterrupted(info.root, s.id).catch(() => {}) },
            });
        }).catch(() => {});
        setDockerHistory([]);
        setCheckHistory([]);
        void api.dockerHistory(info.root).then(setDockerHistory).catch(() => {});
        void api.checkHistory(info.root).then(setCheckHistory).catch(() => {});
        setView("overview");
        setRecent((list) => {
          const next = [
            { path: info.root, name: info.name, framework: info.framework, openedAt: Date.now() },
            ...list.filter((r) => r.path !== info.root),
          ].slice(0, 6);
          writeStorage(RECENT_KEY, next);
          return next;
        });
      } catch (e) {
        if (seq === openSeq.current) fail(t.toast.openFailed, e);
      } finally {
        if (seq === openSeq.current) setOpening(false);
      }
    },
    [api, resetResults, fail, t, toast],
  );

  const forgetRecent = useCallback((path: string) => {
    setRecent((list) => {
      const next = list.filter((r) => r.path !== path);
      writeStorage(RECENT_KEY, next);
      return next;
    });
  }, []);

  const closeProject = useCallback(() => {
    if (rootRef.current) void api?.closeProject(rootRef.current).catch(() => {});
    rootRef.current = null;
    resetResults();
    setProject(null);
    setView("overview");
  }, [api, resetResults]);

  const showFindings = useCallback((f: Partial<FindingsFilter> = {}) => {
    setFindingsFilter({ ...NO_FILTER, ...f });
    setView("findings");
  }, []);

  const runScan = useCallback(async (scanTarget: Target = target) => {
    if (!api || !project) return;
    const gen = ++generation.current;
    setScanning(true);
    setProgress({ run: gen, done: 0, total: project.phpFiles, file: "" });
    let unlisten: (() => void) | undefined;
    try {
      unlisten = await api.onScanProgress((p) => {
        if (gen === generation.current && p.run === gen) setProgress(p);
      });
      const [result, depReport] = await Promise.all([
        api.scanProject(project.root, scanTarget, gen),
        api.checkDependencies(project.root, scanTarget).catch((e) => {
          fail(t.toast.depsFailed, e);
          return null;
        }),
      ]);
      if (gen !== generation.current) return;
      setScan(result);
      setDeps(depReport);
    } catch (e) {
      if (gen === generation.current) fail(t.toast.scanFailed, e);
    } finally {
      unlisten?.();
      if (gen === generation.current) {
        setScanning(false);
        setProgress(null);
      }
    }
  }, [api, project, target, fail, t]);

  const cancelScan = useCallback(() => {
    generation.current += 1;
    setScanning(false);
    setProgress(null);
    if (project) api?.cancelScan(project.root).catch(() => {});
  }, [api, project]);

  const saveConfig = useCallback(
    async (config: ProjectConfig) => {
      if (!api || !project) return false;
      try {
        const info = await api.saveProjectConfig(project.root, config);
        const moved = info.excludedDirs.join() !== project.excludedDirs.join();
        setProject(info);
        toast.push({ tone: "success", title: t.toast.configSaved, description: t.toast.configSavedText });
        if (moved) {
          const rescan = !!scan;
          resetResults();
          if (rescan) void runScan();
        }
        return true;
      } catch (e) {
        fail(t.toast.saveFailed, e);
        return false;
      }
    },
    [api, project, scan, resetResults, runScan, toast, fail, t],
  );

  const setTarget = useCallback(
    (next: Target) => {
      const rescan = !!scan;
      setTargetState(next);
      writeStorage(TARGET_KEY, next);
      resetResults();
      if (rescan) runScan(next);
    },
    [scan, resetResults, runScan],
  );

  const installRector = useCallback(async () => {
    if (!api) return;
    setInstallingRector(true);
    try {
      setToolchain(await api.installRector());
      toast.push({ tone: "success", title: t.toast.rectorInstalled, description: t.toast.rectorInstalledText });
    } catch (e) {
      fail(t.toast.rectorFailed, e);
    } finally {
      setInstallingRector(false);
    }
  }, [api, toast, fail, t]);

  const loadPreview = useCallback(
    async (mode: RectorMode, step?: string) => {
      if (!api || !project) return false;
      const root = project.root;
      const seq = ++previewSeq.current;
      const previous = previewMode;
      setPreviewMode(mode);
      setPreviewing(true);
      try {
        const result = await api.rectorPreview(root, step ?? target, mode);
        if (seq === previewSeq.current) setPreview(result);
        return true;
      } catch (e) {
        if (seq !== previewSeq.current || rootRef.current !== root) return true;
        setPreviewMode(previous);
        if (cancelledSeq.current !== seq) fail(t.toast.previewFailed, e);
        return false;
      } finally {
        if (seq === previewSeq.current) setPreviewing(false);
      }
    },
    [api, project, target, previewMode, fail, t],
  );

  const cancelPreview = useCallback(() => {
    cancelledSeq.current = previewSeq.current;
    api?.cancelPreview(project?.root).catch(() => {});
  }, [api, project]);

  const withDockerSteps = useCallback(
    async (source: string, work: () => Promise<boolean>) => {
      if (!api) return false;
      const stop = await api.onDockerStep(setDockerStep);
      setDockerStep({ step: "lint", version: source });
      try {
        return await work();
      } catch (e) {
        if (!dockerStopped.current) fail(t.docker.failed, e);
        return false;
      } finally {
        dockerStopped.current = false;
        stop();
        setDockerStep(null);
      }
    },
    [api, fail, t],
  );

  const runDocker = useCallback(
    async (source: string, extensions: boolean, cleanImages: boolean, pages: string[], database: DatabaseChoice | null, prepare: string | null, test: string | null) => {
      if (!api || !project) return false;
      const root = project.root;
      return withDockerSteps(source, async () => {
        const result = await api.dockerCompare(root, source, target, extensions, cleanImages, pages, database, prepare, test);
        if (rootRef.current !== root) return false;
        setDocker(result.comparison);
        setDockerArgs({ source, extensions, cleanImages, pages, database, prepare, test });
        setDockerTrial(null);
        void api.dockerHistory(root).then(setDockerHistory).catch(() => {});
        if (result.scan) setScan(result.scan);
        return true;
      });
    },
    [api, project, target, withDockerSteps],
  );

  const tryFixes = useCallback(
    async (files: string[]) => {
      if (!api || !project || !preview || !dockerArgs) return false;
      const root = project.root;
      const previewId = preview.id;
      const { source, extensions, cleanImages, pages, database, prepare, test } = dockerArgs;
      const skipped = Object.fromEntries(Object.entries(skippedHunks).filter(([file]) => files.includes(file)));
      return withDockerSteps(source, async () => {
        const choice = { previewId, files, skipped };
        const result = await api.dockerCompare(root, source, target, extensions, cleanImages, pages, database, prepare, test, choice);
        if (rootRef.current !== root || !result.outcome) return false;
        setDockerTrial({ previewId, files, skipped, outcome: result.outcome });
        return true;
      });
    },
    [api, project, preview, dockerArgs, target, withDockerSteps, skippedHunks],
  );

  const cancelDocker = useCallback(() => {
    dockerStopped.current = true;
    if (project) api?.cancelDockerCompare(project.root).catch(() => {});
  }, [api, project]);

  const applyChanges = useCallback(
    async (files: string[], skipped?: Record<string, number[]>, keepTrial = false, commitMessage?: string) => {
      if (!api || !project || files.length === 0) return false;
      setApplying(true);
      const root = project.root;
      try {
        const label = t.changes.snapshotLabel(previewMode, preview?.target ?? target);
        if (!preview) return false;
        const result = await api.applyChanges(root, preview.id, files, label, skipped, commitMessage);
        if (rootRef.current !== root) return true;
        setPreview((p) => (p ? { ...p, changes: p.changes.filter((c) => !result.written.includes(c.file)) } : p));
        if (result.lintErrors.length > 0) {
          toast.push({
            tone: "error",
            title: t.toast.appliedWithErrors(result.written.length, result.lintErrors.length),
            description: `${result.lintErrors.slice(0, 3).join("\n")}\n${t.toast.lintFailedText}`,
          });
        } else {
          toast.push({
            tone: result.lintChecked ? "success" : "info",
            title: t.toast.applied(result.written.length),
            description: result.lintChecked ? t.toast.appliedText : `${t.toast.lintSkippedText}\n${t.toast.appliedText}`,
          });
        }
        if (result.commit && result.branch) toast.push({ tone: "success", title: t.git.committed(result.branch, result.commit) });
        if (result.gitError) toast.push({ tone: "error", title: t.git.commitFailed, description: result.gitError });
        void refreshSnapshots();
        if (keepTrial) {
          const kept = await api.keepDockerTrial(root).catch((e) => {
            fail(t.toast.trialNotKept, e);
            return null;
          });
          if (kept) setDocker(kept.comparison);
          setDockerTrial(null);
          void api.dockerHistory(root).then(setDockerHistory).catch(() => {});
        }
        if (scan && rootRef.current === root) void runScan();
        return true;
      } catch (e) {
        if (rootRef.current === root) fail(t.toast.applyFailed, e);
        return false;
      } finally {
        setApplying(false);
      }
    },
    [api, project, preview, previewMode, target, toast, refreshSnapshots, scan, runScan, fail, t],
  );

  const restoreSnapshot = useCallback(
    async (id: string, acknowledged: SnapshotConflict[]) => {
      if (!api || !project) return;
      try {
        const root = project.root;
        const files = await api.restoreSnapshot(root, id, acknowledged);
        if (rootRef.current !== root) return;
        setPreview(null);
        toast.push({ tone: "success", title: t.toast.restored, description: t.toast.restoredText(files.length) });
        void refreshSnapshots();
        if (scan) void runScan();
      } catch (e) {
        fail(t.toast.restoreFailed, e);
      }
    },
    [api, project, toast, refreshSnapshots, scan, runScan, fail, t],
  );

  const deleteSnapshot = useCallback(
    async (id: string) => {
      if (!api || !project) return;
      try {
        await api.deleteSnapshot(project.root, id);
        await refreshSnapshots();
      } catch (e) {
        fail(t.toast.deleteFailed, e);
      }
    },
    [api, project, refreshSnapshots, fail, t],
  );

  const deleteAllSnapshots = useCallback(async () => {
    if (!api || !project) return;
    let error: unknown = null;
    for (const s of snapshots) {
      try {
        await api.deleteSnapshot(project.root, s.id);
      } catch (e) {
        error ??= e;
      }
    }
    if (error) fail(t.toast.deleteFailed, error);
    await refreshSnapshots();
  }, [api, project, snapshots, refreshSnapshots, fail, t]);

  const exportReport = useCallback(async () => {
    if (!api || !project || !scan) return;
    try {
      const dest = await api.pickSaveFile(t.dialogs.reportName(project.name, target));
      if (!dest) return;
      await api.exportReport(project.root, dest);
      toast.push({ tone: "success", title: t.toast.reportSaved, description: dest });
    } catch (e) {
      fail(t.toast.reportFailed, e);
    }
  }, [api, project, scan, target, toast, fail, t]);

  const updateIgnores = useCallback(
    async (change: (root: string) => Promise<ScanResult>, failure?: string) => {
      if (!project) return false;
      const gen = generation.current;
      try {
        const result = await change(project.root);
        if (gen === generation.current) setScan((prev) => ({ ...result, progress: result.progress ?? prev?.progress ?? null }));
        return true;
      } catch (e) {
        fail(failure ?? t.toast.ignoreFailed, e);
        return false;
      }
    },
    [project, fail, t],
  );
  const ignoreFinding = useCallback(
    (id: string, reason: string) => (api ? updateIgnores((root) => api.ignoreFinding(root, id, reason)) : Promise.resolve(false)),
    [api, updateIgnores],
  );
  const unignoreFinding = useCallback(
    (id: string) => (api ? updateIgnores((root) => api.unignoreFinding(root, id)) : Promise.resolve(false)),
    [api, updateIgnores],
  );

  const pruneIgnores = useCallback(
    () => (api ? updateIgnores((root) => api.pruneIgnores(root)) : Promise.resolve(false)),
    [api, updateIgnores],
  );

  const importRuntimeLog = useCallback(async () => {
    if (!api) return false;
    let log: string | null;
    try {
      log = await api.pickLogFile(t.dialogs.pickLog);
    } catch (e) {
      fail(t.toast.runtimeFailed, e);
      return false;
    }
    if (!log) return false;
    const picked = log;
    return updateIgnores((root) => api.importRuntimeLog(root, picked), t.toast.runtimeFailed);
  }, [api, updateIgnores, fail, t]);
  const setRuntimeRoots = useCallback(
    (roots: string) => (api ? updateIgnores((root) => api.setRuntimeRoots(root, roots), t.toast.runtimeFailed) : Promise.resolve(false)),
    [api, updateIgnores, t],
  );
  const clearRuntimeLog = useCallback(
    () => (api ? updateIgnores((root) => api.clearRuntimeLog(root), t.toast.runtimeFailed) : Promise.resolve(false)),
    [api, updateIgnores, t],
  );

  const runLocalCheck = useCallback(
    (kind: "lint" | "tests") =>
      api
        ? updateIgnores((root) => api.runLocalCheck(root, target, kind), t.toast.localCheckFailed).finally(() => {
            if (rootRef.current) void api.checkHistory(rootRef.current).then(setCheckHistory).catch(() => {});
          })
        : Promise.resolve(false),
    [api, updateIgnores, target, t],
  );

  const value = useMemo<AppStore>(
    () => ({
      api, view, setView, findingsFilter, setFindingsFilter, showFindings, theme, setTheme, recent, forgetRecent, project, opening, openProject, closeProject,
      target, setTarget, scan, scanning, progress, runScan, cancelScan, deps, toolchain, refreshToolchain, installingRector,
      installRector, preview, previewMode, previewing, loadPreview, cancelPreview, skippedHunks, skipHunks, docker, dockerHistory, checkHistory, dockerStep, runDocker, cancelDocker, saveConfig, dockerArgs, dockerTrial, tryFixes, applying, applyChanges, snapshots,
      refreshSnapshots, restoreSnapshot, deleteSnapshot, deleteAllSnapshots, exportReport, ignoreFinding, unignoreFinding, pruneIgnores, importRuntimeLog, setRuntimeRoots, clearRuntimeLog, runLocalCheck,
    }),
    [
      api, view, findingsFilter, showFindings, theme, setTheme, recent, forgetRecent, project, opening, openProject, closeProject, target,
      setTarget, scan, scanning, progress, runScan, cancelScan, deps, toolchain, refreshToolchain, installingRector,
      installRector, preview, previewMode, previewing, loadPreview, cancelPreview, skippedHunks, skipHunks, docker, dockerHistory, checkHistory, dockerStep, runDocker, cancelDocker, saveConfig, dockerArgs, dockerTrial, tryFixes, applying, applyChanges, snapshots,
      refreshSnapshots, restoreSnapshot, deleteSnapshot, deleteAllSnapshots, exportReport, ignoreFinding, unignoreFinding, pruneIgnores, importRuntimeLog, setRuntimeRoots, clearRuntimeLog, runLocalCheck,
    ],
  );

  return <StoreContext.Provider value={value}>{children}</StoreContext.Provider>;
}

export function useStore(): AppStore {
  const ctx = useContext(StoreContext);
  if (!ctx) throw new Error("useStore must be used inside StoreProvider");
  return ctx;
}
