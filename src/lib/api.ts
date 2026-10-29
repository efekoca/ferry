import type {
  ApplyResult,
  DependencyReport,
  FileChange,
  ProjectInfo,
  RectorMode,
  RectorPreview,
  ScanProgress,
  ScanResult,
  Settings,
  Snapshot,
  ToolchainStatus,
} from "./types";
import { mockApi } from "./mock";
import type { Lang } from "../i18n";

export interface Api {
  pickProjectFolder(title: string): Promise<string | null>;
  pickSaveFile(defaultName: string): Promise<string | null>;
  pickPhpBinary(title: string): Promise<string | null>;
  inspectProject(path: string): Promise<ProjectInfo>;
  scanProject(path: string, target: string): Promise<ScanResult>;
  onScanProgress(handler: (p: ScanProgress) => void): Promise<() => void>;
  checkDependencies(path: string, target: string): Promise<DependencyReport>;
  toolchainStatus(): Promise<ToolchainStatus>;
  installRector(): Promise<ToolchainStatus>;
  rectorPreview(path: string, target: string, mode: RectorMode): Promise<RectorPreview>;
  cancelPreview(): Promise<void>;
  applyChanges(path: string, files: string[], label: string): Promise<ApplyResult>;
  listSnapshots(path: string): Promise<Snapshot[]>;
  restoreSnapshot(path: string, id: string): Promise<string[]>;
  deleteSnapshot(path: string, id: string): Promise<void>;
  snapshotChanges(path: string, id: string): Promise<FileChange[]>;
  snapshotConflicts(path: string, id: string): Promise<string[]>;
  exportReport(path: string, destination: string): Promise<void>;
  revealPath(path: string): Promise<void>;
  getSettings(): Promise<Settings>;
  setLanguage(lang: Lang): Promise<void>;
  saveSettings(settings: Settings): Promise<void>;
}

export const isTauri = (): boolean =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function tauriApi(): Promise<Api> {
  const { invoke } = await import("@tauri-apps/api/core");
  const { listen } = await import("@tauri-apps/api/event");
  const dialog = await import("@tauri-apps/plugin-dialog");
  const opener = await import("@tauri-apps/plugin-opener");

  const single = (v: string | string[] | null) => (Array.isArray(v) ? v[0] ?? null : v);

  return {
    pickProjectFolder: async (title) => single(await dialog.open({ directory: true, multiple: false, title })),
    pickSaveFile: (defaultName) =>
      dialog.save({ defaultPath: defaultName, filters: [{ name: "Markdown", extensions: ["md"] }] }),
    pickPhpBinary: async (title) => single(await dialog.open({ multiple: false, title })),
    inspectProject: (path) => invoke("inspect_project", { path }),
    scanProject: (path, target) => invoke("scan_project", { path, target }),
    onScanProgress: (handler) => listen<ScanProgress>("scan://progress", (e) => handler(e.payload)),
    checkDependencies: (path, target) => invoke("check_dependencies", { path, target }),
    toolchainStatus: () => invoke("toolchain_status"),
    installRector: () => invoke("install_rector"),
    rectorPreview: (path, target, mode) => invoke("rector_preview", { path, target, mode }),
    cancelPreview: () => invoke("cancel_rector_preview"),
    applyChanges: (path, files, label) => invoke("apply_changes", { path, files, label }),
    listSnapshots: (path) => invoke("list_snapshots", { path }),
    restoreSnapshot: (path, id) => invoke("restore_snapshot", { path, id }),
    deleteSnapshot: (path, id) => invoke("delete_snapshot", { path, id }),
    snapshotChanges: (path, id) => invoke("snapshot_changes", { path, id }),
    snapshotConflicts: (path, id) => invoke("snapshot_conflicts", { path, id }),
    exportReport: (path, destination) => invoke("export_report", { path, destination }),
    revealPath: (path) => opener.revealItemInDir(path),
    getSettings: () => invoke("get_settings"),
    saveSettings: (settings) => invoke("save_settings", { settings }),
    setLanguage: (language) => invoke("set_language", { language }),
  };
}

let instance: Promise<Api> | null = null;

export function getApi(): Promise<Api> {
  instance ??= isTauri() ? tauriApi() : Promise.resolve(mockApi());
  return instance;
}

export function errorMessage(e: unknown, fallback: string): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  return fallback;
}
