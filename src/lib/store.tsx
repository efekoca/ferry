import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { errorMessage, getApi, type Api } from "./api";
import type {
  DependencyReport,
  Framework,
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
} from "./types";
import { TARGETS } from "./types";
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
const FRAMEWORKS: readonly Framework[] = ["wordpress", "laravel", "symfony", "drupal", "generic"];

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

  deps: DependencyReport | null;

  toolchain: ToolchainStatus | null;
  refreshToolchain(): Promise<void>;
  installingRector: boolean;
  installRector(): Promise<void>;

  preview: RectorPreview | null;
  previewMode: RectorMode;
  previewing: boolean;
  loadPreview(mode: RectorMode): Promise<boolean>;
  cancelPreview(): void;
  applying: boolean;
  applyChanges(files: string[]): Promise<boolean>;

  snapshots: Snapshot[];
  refreshSnapshots(): Promise<void>;
  restoreSnapshot(id: string, acknowledged: string[]): Promise<void>;
  deleteSnapshot(id: string): Promise<void>;
  deleteAllSnapshots(): Promise<void>;

  exportReport(): Promise<void>;
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
  const [target, setTargetState] = useState<Target>(() => readStorage<Target>(TARGET_KEY, "8.4", (v) => TARGETS.includes(v as Target)));
  const [scan, setScan] = useState<ScanResult | null>(null);
  const [scanning, setScanning] = useState(false);
  const [progress, setProgress] = useState<ScanProgress | null>(null);
  const [deps, setDeps] = useState<DependencyReport | null>(null);
  const [toolchain, setToolchain] = useState<ToolchainStatus | null>(null);
  const [installingRector, setInstallingRector] = useState(false);
  const [preview, setPreview] = useState<RectorPreview | null>(null);
  const [previewMode, setPreviewMode] = useState<RectorMode>("compat");
  const [previewing, setPreviewing] = useState(false);
  const [applying, setApplying] = useState(false);
  const [snapshots, setSnapshots] = useState<Snapshot[]>([]);
  const generation = useRef(0);
  const previewSeq = useRef(0);
  const cancelledSeq = useRef(0);
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
    apply();
    media?.addEventListener?.("change", apply);
    return () => media?.removeEventListener?.("change", apply);
  }, [theme]);

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
        rootRef.current = info.root;
        resetResults();
        setFindingsFilter(NO_FILTER);
        setProject(info);
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
        fail(t.toast.openFailed, e);
      } finally {
        setOpening(false);
      }
    },
    [api, resetResults, fail, t],
  );

  const forgetRecent = useCallback((path: string) => {
    setRecent((list) => {
      const next = list.filter((r) => r.path !== path);
      writeStorage(RECENT_KEY, next);
      return next;
    });
  }, []);

  const closeProject = useCallback(() => {
    rootRef.current = null;
    resetResults();
    setProject(null);
    setView("overview");
  }, [resetResults]);

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
        api.checkDependencies(project.root, scanTarget).catch(() => null),
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
    async (mode: RectorMode) => {
      if (!api || !project) return false;
      const root = project.root;
      const seq = ++previewSeq.current;
      const previous = previewMode;
      setPreviewMode(mode);
      setPreviewing(true);
      try {
        const result = await api.rectorPreview(root, target, mode);
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

  const applyChanges = useCallback(
    async (files: string[]) => {
      if (!api || !project || files.length === 0) return false;
      setApplying(true);
      const root = project.root;
      try {
        const label = t.changes.snapshotLabel(previewMode === "compat", target);
        if (!preview) return false;
        const result = await api.applyChanges(root, preview.id, files, label);
        if (rootRef.current !== root) return true;
        setPreview((p) => (p ? { ...p, changes: p.changes.filter((c) => !result.written.includes(c.file)) } : p));
        toast.push({ tone: "success", title: t.toast.applied(result.written.length), description: t.toast.appliedText });
        await refreshSnapshots();
        if (scan && rootRef.current === root) await runScan();
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
    async (id: string, acknowledged: string[]) => {
      if (!api || !project) return;
      try {
        const root = project.root;
        const files = await api.restoreSnapshot(root, id, acknowledged);
        if (rootRef.current !== root) return;
        setPreview(null);
        toast.push({ tone: "success", title: t.toast.restored, description: t.toast.restoredText(files.length) });
        if (scan) await runScan();
      } catch (e) {
        fail(t.toast.restoreFailed, e);
      }
    },
    [api, project, toast, scan, runScan, fail, t],
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

  const value = useMemo<AppStore>(
    () => ({
      api, view, setView, findingsFilter, setFindingsFilter, showFindings, theme, setTheme, recent, forgetRecent, project, opening, openProject, closeProject,
      target, setTarget, scan, scanning, progress, runScan, deps, toolchain, refreshToolchain, installingRector,
      installRector, preview, previewMode, previewing, loadPreview, cancelPreview, applying, applyChanges, snapshots,
      refreshSnapshots, restoreSnapshot, deleteSnapshot, deleteAllSnapshots, exportReport,
    }),
    [
      api, view, findingsFilter, showFindings, theme, setTheme, recent, forgetRecent, project, opening, openProject, closeProject, target,
      setTarget, scan, scanning, progress, runScan, deps, toolchain, refreshToolchain, installingRector,
      installRector, preview, previewMode, previewing, loadPreview, cancelPreview, applying, applyChanges, snapshots,
      refreshSnapshots, restoreSnapshot, deleteSnapshot, deleteAllSnapshots, exportReport,
    ],
  );

  return <StoreContext.Provider value={value}>{children}</StoreContext.Provider>;
}

export function useStore(): AppStore {
  const ctx = useContext(StoreContext);
  if (!ctx) throw new Error("useStore must be used inside StoreProvider");
  return ctx;
}
