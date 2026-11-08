import type {
  SourceLine,
  CheckSummary,
  DatabaseChoice,
  DatabasePlan,
  DockerFixChoice,
  DockerImageUsage,
  DockerResponse,
  DockerSummary,
  HistoryEntry,
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
  AiConfig,
  AiPrompt,
  AiSuggestion,
  AiView,
  LocalAiServer,
  LocalModelView,
  LocalProgress,
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
  cancelLocalCheck(path: string): Promise<void>;
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
  rectorPreview(path: string, target: string, mode: RectorMode, withPins: boolean): Promise<RectorPreview>;
  cancelPreview(path?: string): Promise<void>;
  cancelScan(path: string): Promise<void>;
  dockerCompare(path: string, source: string, target: string, extensions: boolean, cleanImages: boolean, pages: string[], database: DatabaseChoice | null, prepare: string | null, test: string | null, fix?: DockerFixChoice): Promise<DockerResponse>;
  keepDockerTrial(path: string): Promise<DockerResponse>;
  databasePlan(path: string): Promise<DatabasePlan>;
  sshHosts(): Promise<string[]>;
  dockerImageUsage(): Promise<DockerImageUsage>;
  scanHistory(path: string): Promise<HistoryEntry[]>;
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
  rectorCacheSize(): Promise<number>;
  clearRectorCache(): Promise<void>;
  deleteAllBackups(): Promise<void>;
  exportReport(path: string, destination: string): Promise<void>;
  ignoreFinding(path: string, id: string, reason: string): Promise<ScanResult>;
  unignoreFinding(path: string, id: string): Promise<ScanResult>;
  pruneIgnores(path: string): Promise<ScanResult>;
  openInEditor(path: string, file: string, line: number, column: number): Promise<void>;
  sourceLines(path: string, file: string, line: number): Promise<SourceLine[]>;
  revealPath(path: string): Promise<void>;
  getSettings(): Promise<Settings>;
  aiProvider(): Promise<string | null>;
  aiConfig(): Promise<AiView>;
  /** `key`: a new key, "" to delete the stored one, or null to keep it. */
  saveAiConfig(config: AiConfig, key: string | null): Promise<AiView>;
  checkAiConfig(): Promise<void>;
  /** Ollama or LM Studio, when one is running on this computer. */
  localAiServers(): Promise<LocalAiServer[]>;
  localModel(): Promise<LocalModelView>;
  /** Downloads llama.cpp and the model for this computer, checks both and runs the model once. */
  installLocalModel(): Promise<LocalModelView>;
  cancelLocalModel(): Promise<void>;
  removeLocalModel(): Promise<LocalModelView>;
  onLocalModelProgress(handler: (p: LocalProgress) => void): Promise<() => void>;
  aiPrompt(path: string, id: string): Promise<AiPrompt>;
  aiSuggest(path: string, id: string): Promise<AiSuggestion>;
  cancelAi(path: string): Promise<void>;
  applyAiSuggestion(path: string, id: number, label: string): Promise<ApplyResult>;
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
    cancelLocalCheck: (path) => invoke("cancel_local_check", { path }),
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
    rectorPreview: (path, target, mode, withPins) => invoke("rector_preview", { path, target, mode, withPins }),
    cancelPreview: (path) => invoke("cancel_rector_preview", { path }),
    cancelScan: (path) => invoke("cancel_scan", { path }),
    dockerCompare: (path, source, target, extensions, cleanImages, pages, database, prepare, test, fix) => invoke("docker_compare", { path, source, target, extensions, cleanImages, pages, database, prepare, test, fix: fix ?? null }),
    keepDockerTrial: (path) => invoke("keep_docker_trial", { path }),
    databasePlan: (path) => invoke("database_plan", { path }),
    sshHosts: () => invoke("ssh_hosts"),
    dockerImageUsage: () => invoke("docker_image_usage"),
    scanHistory: (path) => invoke("scan_history", { path }),
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
    rectorCacheSize: () => invoke("rector_cache_size"),
    clearRectorCache: () => invoke("clear_rector_cache"),
    deleteAllBackups: () => invoke("delete_all_snapshots"),
    exportReport: (path, destination) => invoke("export_report", { path, destination }),
    ignoreFinding: (path, id, reason) => invoke("ignore_finding", { path, id, reason }),
    unignoreFinding: (path, id) => invoke("unignore_finding", { path, id }),
    pruneIgnores: (path) => invoke("prune_ignores", { path }),
    sourceLines: (path, file, line) => invoke("source_lines", { path, file, line }),
    openInEditor: (path, file, line, column) => invoke("open_in_editor", { path, file, line, column }),
    revealPath: (path) => opener.revealItemInDir(path),
    getSettings: () => invoke("get_settings"),
    aiProvider: () => invoke("ai_provider"),
    aiConfig: () => invoke("ai_config"),
    saveAiConfig: (config, key) => invoke("save_ai_config", { config, key }),
    checkAiConfig: () => invoke("check_ai_config"),
    localAiServers: () => invoke("local_ai_servers"),
    localModel: () => invoke("local_model"),
    installLocalModel: () => invoke("install_local_model"),
    cancelLocalModel: () => invoke("cancel_local_model"),
    removeLocalModel: () => invoke("remove_local_model"),
    onLocalModelProgress: (handler) => listen<LocalProgress>("local-model://progress", (e) => handler(e.payload)),
    aiPrompt: (path, id) => invoke("ai_prompt", { path, id }),
    aiSuggest: (path, id) => invoke("ai_suggest", { path, id }),
    cancelAi: (path) => invoke("cancel_ai", { path }),
    applyAiSuggestion: (path, id, label) => invoke("apply_ai_suggestion", { path, id, label }),
    saveSettings: (settings) => invoke("save_settings", { settings }),
    setLanguage: (language) => invoke("set_language", { language }),
    setWindowTheme: (theme) => getCurrentWindow().setTheme(theme),
  };
}

let instance: Promise<Api> | null = null;

export function getApi(): Promise<Api> {
  const params = new URLSearchParams(window.location.search);
  const scale = Number(params.get("mockScale")) || 1;
  const localAi = params.has("mockLocalAi") ? [{ name: "Ollama", baseUrl: "http://127.0.0.1:11434/v1", models: ["qwen2.5-coder:7b", "qwen3:8b"] }] : undefined;
  instance ??= isTauri() ? tauriApi() : Promise.resolve(mockApi({ scale, localAi }));
  return instance;
}

export function errorMessage(e: unknown, fallback: string): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  return fallback;
}
