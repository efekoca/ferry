import type {
  CheckSummary,
  DatabaseChoice,
  DatabasePlan,
  DockerFixChoice,
  DockerImageUsage,
  DockerResponse,
  DockerSummary,
  DockerStep,
  SnapshotConflict,
  ApplyResult,
  BackupUsage,
  DependencyReport,
  FileChange,
  LocalPhp,
  ProjectConfig,
  ProjectInfo,
  RectorMode,
  RectorPreview,
  ScanProgress,
  ScanResult,
  Settings,
  Snapshot,
  TargetSummary,
  ToolchainStatus,
  VersionSuggestion,
  WpLatest,
} from "./types";
import { mockApi } from "./mock";
import type { Lang } from "../i18n";

export interface Api {
  pickProjectFolder(title: string): Promise<string | null>;
  pickSaveFile(defaultName: string): Promise<string | null>;
  pickPhpBinary(title: string): Promise<string | null>;
  pickProbeFile(defaultName: string): Promise<string | null>;
  writeProbe(destination: string): Promise<string>;
  pickLogFile(title: string): Promise<string | null>;
  importRuntimeLog(path: string, log: string): Promise<ScanResult>;
  setRuntimeRoots(path: string, roots: string): Promise<ScanResult>;
  clearRuntimeLog(path: string): Promise<ScanResult>;
  closeProject(path: string): Promise<void>;
  localPhpVersions(): Promise<LocalPhp[]>;
  runLocalCheck(path: string, target: string, kind: "lint" | "tests"): Promise<ScanResult>;
  inspectProject(path: string): Promise<ProjectInfo>;
  saveProjectConfig(path: string, config: ProjectConfig): Promise<ProjectInfo>;
  scanProject(path: string, target: string, run: number): Promise<ScanResult>;
  onScanProgress(handler: (p: ScanProgress) => void): Promise<() => void>;
  checkDependencies(path: string, target: string): Promise<DependencyReport>;
  interruptedWrites(path: string): Promise<Snapshot[]>;
  dismissInterrupted(path: string, id: string): Promise<void>;
  gitStatus(path: string, files: string[]): Promise<{ repo: boolean; dirty: string[] }>;
  compareTargets(path: string): Promise<TargetSummary[]>;
  findCompatibleVersions(target: string, packages: { name: string; version: string }[]): Promise<VersionSuggestion[]>;
  wordpressLatest(items: { kind: string; slug: string }[]): Promise<WpLatest[]>;
  toolchainStatus(): Promise<ToolchainStatus>;
  installRector(): Promise<ToolchainStatus>;
  rectorPreview(path: string, target: string, mode: RectorMode): Promise<RectorPreview>;
  cancelPreview(path?: string): Promise<void>;
  dockerCompare(path: string, source: string, target: string, extensions: boolean, cleanImages: boolean, pages: string[], database: DatabaseChoice | null, prepare: string | null, test: string | null, fix?: DockerFixChoice): Promise<DockerResponse>;
  keepDockerTrial(path: string): Promise<DockerResponse>;
  databasePlan(path: string): Promise<DatabasePlan>;
  sshHosts(): Promise<string[]>;
  dockerImageUsage(): Promise<DockerImageUsage>;
  dockerHistory(path: string): Promise<DockerSummary[]>;
  checkHistory(path: string): Promise<CheckSummary[]>;
  removeDockerImages(): Promise<number>;
  cancelDockerCompare(path: string): Promise<void>;
  onDockerStep(handler: (step: DockerStep) => void): Promise<() => void>;
  applyChanges(path: string, previewId: number, files: string[], label: string, skipped?: Record<string, number[]>, commitMessage?: string): Promise<ApplyResult>;
  listSnapshots(path: string): Promise<Snapshot[]>;
  restoreSnapshot(path: string, id: string, acknowledged: SnapshotConflict[]): Promise<string[]>;
  deleteSnapshot(path: string, id: string): Promise<void>;
  snapshotChanges(path: string, id: string): Promise<FileChange[]>;
  snapshotConflicts(path: string, id: string): Promise<SnapshotConflict[]>;
  backupUsage(): Promise<BackupUsage>;
  deleteAllBackups(): Promise<void>;
  exportReport(path: string, destination: string): Promise<void>;
  ignoreFinding(path: string, id: string, reason: string): Promise<ScanResult>;
  unignoreFinding(path: string, id: string): Promise<ScanResult>;
  pruneIgnores(path: string): Promise<ScanResult>;
  openInEditor(path: string, file: string, line: number, column: number): Promise<void>;
  revealPath(path: string): Promise<void>;
  getSettings(): Promise<Settings>;
  setLanguage(lang: Lang): Promise<void>;
  setWindowTheme(theme: "light" | "dark" | null): Promise<void>;
  saveSettings(settings: Settings): Promise<void>;
}

export const isTauri = (): boolean =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function tauriApi(): Promise<Api> {
  const { invoke } = await import("@tauri-apps/api/core");
  const { listen } = await import("@tauri-apps/api/event");
  const dialog = await import("@tauri-apps/plugin-dialog");
  const opener = await import("@tauri-apps/plugin-opener");
  const { getCurrentWindow } = await import("@tauri-apps/api/window");

  const single = (v: string | string[] | null) => (Array.isArray(v) ? v[0] ?? null : v);

  return {
    pickProjectFolder: async (title) => single(await dialog.open({ directory: true, multiple: false, title })),
    pickSaveFile: (defaultName) =>
      dialog.save({ defaultPath: defaultName, filters: [{ name: "Markdown", extensions: ["md"] }] }),
    pickPhpBinary: async (title) => single(await dialog.open({ multiple: false, title })),
    pickProbeFile: (defaultName) => dialog.save({ defaultPath: defaultName, filters: [{ name: "PHP", extensions: ["php"] }] }),
    writeProbe: (destination) => invoke("write_probe", { destination }),
    pickLogFile: async (title) => single(await dialog.open({ multiple: false, title })),
    importRuntimeLog: (path, log) => invoke("import_runtime_log", { path, log }),
    setRuntimeRoots: (path, roots) => invoke("set_runtime_roots", { path, roots }),
    clearRuntimeLog: (path) => invoke("clear_runtime_log", { path }),
    closeProject: (path) => invoke("close_project", { path }),
    localPhpVersions: () => invoke("local_php_versions"),
    runLocalCheck: (path, target, kind) => invoke("run_local_check", { path, target, kind }),
    inspectProject: (path) => invoke("inspect_project", { path }),
    saveProjectConfig: (path, config) => invoke("save_project_config", { path, config }),
    scanProject: (path, target, run) => invoke("scan_project", { path, target, run }),
    onScanProgress: (handler) => listen<ScanProgress>("scan://progress", (e) => handler(e.payload)),
    checkDependencies: (path, target) => invoke("check_dependencies", { path, target }),
    gitStatus: (path, files) => invoke("git_status", { path, files }),
    compareTargets: (path) => invoke("compare_targets", { path }),
    findCompatibleVersions: (target, packages) => invoke("find_compatible_versions", { target, packages }),
    wordpressLatest: (items) => invoke("wordpress_latest", { items }),
    toolchainStatus: () => invoke("toolchain_status"),
    installRector: () => invoke("install_rector"),
    rectorPreview: (path, target, mode) => invoke("rector_preview", { path, target, mode }),
    cancelPreview: (path) => invoke("cancel_rector_preview", { path }),
    dockerCompare: (path, source, target, extensions, cleanImages, pages, database, prepare, test, fix) => invoke("docker_compare", { path, source, target, extensions, cleanImages, pages, database, prepare, test, fix: fix ?? null }),
    keepDockerTrial: (path) => invoke("keep_docker_trial", { path }),
    databasePlan: (path) => invoke("database_plan", { path }),
    sshHosts: () => invoke("ssh_hosts"),
    dockerImageUsage: () => invoke("docker_image_usage"),
    dockerHistory: (path) => invoke("docker_history", { path }),
    checkHistory: (path) => invoke("check_history", { path }),
    removeDockerImages: () => invoke("remove_docker_images"),
    cancelDockerCompare: (path) => invoke("cancel_docker_compare", { path }),
    onDockerStep: (handler) => listen<DockerStep>("docker://step", (e) => handler(e.payload)),
    interruptedWrites: (path) => invoke("interrupted_writes", { path }),
    dismissInterrupted: (path, id) => invoke("dismiss_interrupted", { path, id }),
    applyChanges: (path, previewId, files, label, skipped, commitMessage) =>
      invoke("apply_changes", { path, previewId, files, skipped: skipped ?? null, label, commitMessage: commitMessage ?? null }),
    listSnapshots: (path) => invoke("list_snapshots", { path }),
    restoreSnapshot: (path, id, acknowledged) => invoke("restore_snapshot", { path, id, acknowledged }),
    deleteSnapshot: (path, id) => invoke("delete_snapshot", { path, id }),
    snapshotChanges: (path, id) => invoke("snapshot_changes", { path, id }),
    snapshotConflicts: (path, id) => invoke("snapshot_conflicts", { path, id }),
    backupUsage: () => invoke("snapshot_usage"),
    deleteAllBackups: () => invoke("delete_all_snapshots"),
    exportReport: (path, destination) => invoke("export_report", { path, destination }),
    ignoreFinding: (path, id, reason) => invoke("ignore_finding", { path, id, reason }),
    unignoreFinding: (path, id) => invoke("unignore_finding", { path, id }),
    pruneIgnores: (path) => invoke("prune_ignores", { path }),
    openInEditor: (path, file, line, column) => invoke("open_in_editor", { path, file, line, column }),
    revealPath: (path) => opener.revealItemInDir(path),
    getSettings: () => invoke("get_settings"),
    saveSettings: (settings) => invoke("save_settings", { settings }),
    setLanguage: (language) => invoke("set_language", { language }),
    setWindowTheme: (theme) => getCurrentWindow().setTheme(theme),
  };
}

let instance: Promise<Api> | null = null;

export function getApi(): Promise<Api> {
  const scale = Number(new URLSearchParams(window.location.search).get("mockScale")) || 1;
  instance ??= isTauri() ? tauriApi() : Promise.resolve(mockApi({ scale }));
  return instance;
}

export function errorMessage(e: unknown, fallback: string): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  return fallback;
}
