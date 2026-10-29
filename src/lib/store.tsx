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
  Snapshot,
  Target,
  ToolchainStatus,
} from "./types";
import { useToast } from "../components/Toast";
import { useI18n } from "../i18n";

export type View = "overview" | "findings" | "changes" | "dependencies" | "history" | "runtime" | "ai" | "settings";
export type Theme = "system" | "light" | "dark";

export interface RecentProject {
  path: string;
  name: string;
  framework: Framework;
  openedAt: number;
}

const RECENT_KEY = "ferry.recent";
const THEME_KEY = "ferry.theme";
const TARGET_KEY = "ferry.target";

function readStorage<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key);
    return raw ? (JSON.parse(raw) as T) : fallback;
  } catch {
    return fallback;
  }
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
  runScan(): Promise<void>;

  deps: DependencyReport | null;

  toolchain: ToolchainStatus | null;
  refreshToolchain(): Promise<void>;
  installingRector: boolean;
  installRector(): Promise<void>;

  preview: RectorPreview | null;
  previewMode: RectorMode;
  previewing: boolean;
  loadPreview(mode: RectorMode): Promise<void>;
  applying: boolean;
  applyChanges(files: string[]): Promise<boolean>;

  snapshots: Snapshot[];
  refreshSnapshots(): Promise<void>;
  restoreSnapshot(id: string): Promise<void>;
  deleteSnapshot(id: string): Promise<void>;

  exportReport(): Promise<void>;
}

const StoreContext = createContext<AppStore | null>(null);

export function StoreProvider({ children, api: injected }: { children: ReactNode; api?: Api }) {
  const toast = useToast();
  const { t, lang } = useI18n();
  const [api, setApi] = useState<Api | null>(injected ?? null);
  const [view, setView] = useState<View>("overview");
  const [theme, setThemeState] = useState<Theme>(() => readStorage<Theme>(THEME_KEY, "system"));
  const [recent, setRecent] = useState<RecentProject[]>(() => readStorage<RecentProject[]>(RECENT_KEY, []));
  const [project, setProject] = useState<ProjectInfo | null>(null);
  const [opening, setOpening] = useState(false);
  const [target, setTargetState] = useState<Target>(() => readStorage<Target>(TARGET_KEY, "8.4"));
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
    generation.current++;
    setScan(null);
    setDeps(null);
    setPreview(null);
    setProgress(null);
  }, []);

  const refreshSnapshots = useCallback(async () => {
    if (!api || !project) return;
    try {
      setSnapshots(await api.listSnapshots(project.root));
    } catch (e) {
      fail(t.toast.historyFailed, e);
    }
  }, [api, project, fail, t]);

  useEffect(() => {
    if (project) refreshSnapshots();
    else setSnapshots([]);
  }, [project, refreshSnapshots]);

  const openProject = useCallback(
    async (path?: string) => {
      if (!api) return;
      const chosen = path ?? (await api.pickProjectFolder(t.dialogs.pickProject));
      if (!chosen) return;
      setOpening(true);
      try {
        const info = await api.inspectProject(chosen);
        resetResults();
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
    resetResults();
    setProject(null);
    setView("overview");
  }, [resetResults]);

  const setTarget = useCallback(
    (next: Target) => {
      setTargetState(next);
      writeStorage(TARGET_KEY, next);
      resetResults();
    },
    [resetResults],
  );

  const runScan = useCallback(async () => {
    if (!api || !project) return;
    const gen = ++generation.current;
    setScanning(true);
    setProgress({ done: 0, total: project.phpFiles, file: "" });
    const unlisten = await api.onScanProgress((p) => gen === generation.current && setProgress(p));
    try {
      const [result, depReport] = await Promise.all([
        api.scanProject(project.root, target),
        api.checkDependencies(project.root, target).catch(() => null),
      ]);
      if (gen !== generation.current) return;
      setScan(result);
      setDeps(depReport);
    } catch (e) {
      if (gen === generation.current) fail(t.toast.scanFailed, e);
    } finally {
      unlisten();
      if (gen === generation.current) {
        setScanning(false);
        setProgress(null);
      }
    }
  }, [api, project, target, fail, t]);

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
      if (!api || !project) return;
      const gen = generation.current;
      setPreviewMode(mode);
      setPreviewing(true);
      try {
        const result = await api.rectorPreview(project.root, target, mode);
        if (gen === generation.current) setPreview(result);
      } catch (e) {
        fail(t.toast.previewFailed, e);
      } finally {
        setPreviewing(false);
      }
    },
    [api, project, target, fail, t],
  );

  const applyChanges = useCallback(
    async (files: string[]) => {
      if (!api || !project || files.length === 0) return false;
      setApplying(true);
      try {
        const label = t.changes.snapshotLabel(previewMode === "compat", target);
        const result = await api.applyChanges(project.root, files, label);
        setPreview((p) => (p ? { ...p, changes: p.changes.filter((c) => !result.written.includes(c.file)) } : p));
        toast.push({ tone: "success", title: t.toast.applied(result.written.length), description: t.toast.appliedText });
        await refreshSnapshots();
        if (scan) await runScan();
        return true;
      } catch (e) {
        fail(t.toast.applyFailed, e);
        return false;
      } finally {
        setApplying(false);
      }
    },
    [api, project, previewMode, target, toast, refreshSnapshots, scan, runScan, fail, t],
  );

  const restoreSnapshot = useCallback(
    async (id: string) => {
      if (!api || !project) return;
      try {
        const files = await api.restoreSnapshot(project.root, id);
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

  const exportReport = useCallback(async () => {
    if (!api || !project || !scan) return;
    const dest = await api.pickSaveFile(t.dialogs.reportName(project.name, target));
    if (!dest) return;
    try {
      await api.exportReport(project.root, dest);
      toast.push({ tone: "success", title: t.toast.reportSaved, description: dest });
    } catch (e) {
      fail(t.toast.reportFailed, e);
    }
  }, [api, project, scan, target, toast, fail, t]);

  const value = useMemo<AppStore>(
    () => ({
      api, view, setView, theme, setTheme, recent, forgetRecent, project, opening, openProject, closeProject,
      target, setTarget, scan, scanning, progress, runScan, deps, toolchain, refreshToolchain, installingRector,
      installRector, preview, previewMode, previewing, loadPreview, applying, applyChanges, snapshots,
      refreshSnapshots, restoreSnapshot, deleteSnapshot, exportReport,
    }),
    [
      api, view, theme, setTheme, recent, forgetRecent, project, opening, openProject, closeProject, target,
      setTarget, scan, scanning, progress, runScan, deps, toolchain, refreshToolchain, installingRector,
      installRector, preview, previewMode, previewing, loadPreview, applying, applyChanges, snapshots,
      refreshSnapshots, restoreSnapshot, deleteSnapshot, exportReport,
    ],
  );

  return <StoreContext.Provider value={value}>{children}</StoreContext.Provider>;
}

export function useStore(): AppStore {
  const ctx = useContext(StoreContext);
  if (!ctx) throw new Error("useStore must be used inside StoreProvider");
  return ctx;
}
