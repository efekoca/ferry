import type { Localized } from "../i18n";

export type Framework = "wordpress" | "laravel" | "symfony" | "drupal" | "magento" | "prestashop" | "generic";
export type Severity = "breaking" | "deprecated" | "behavior";
export type FixKind = "auto" | "likely" | "manual";
export type DependencyStatus = "compatible" | "incompatible" | "unknown";
export type RectorMode = "compat" | "modernize";

export const TARGETS = ["8.1", "8.2", "8.3", "8.4", "8.5"] as const;
export type Target = (typeof TARGETS)[number];

export interface ProjectInfo {
  root: string;
  name: string;
  framework: Framework;
  phpFiles: number;
  phpConstraint: string | null;
  currentVersion: string | null;
  hasComposerLock: boolean;
  hasVendor: boolean;
  isGitRepo: boolean;
  excludedDirs: string[];
}

export interface Finding {
  id: string;
  file: string;
  line: number;
  column: number;
  ruleId: string;
  title: Localized;
  message: Localized;
  since: string;
  severity: Severity;
  fix: FixKind;
  replacement: Localized | null;
  snippet: string;
  runtimeHits: number;
}

export interface Summary {
  total: number;
  breaking: number;
  deprecated: number;
  behavior: number;
  auto: number;
  likely: number;
  manual: number;
  filesAffected: number;
  score: number;
}

export interface IgnoredFinding {
  finding: Finding;
  reason: string;
}

export interface IgnoreEntry {
  ruleId: string;
  file: string;
  snippet: string;
  reason: string;
}

export interface HistoryEntry {
  at: number;
  target: string;
  score: number;
  total: number;
}

export interface ScanHistory {
  history: HistoryEntry[];
  newSinceLast: number | null;
  fixedSinceLast: number | null;
}

export interface TargetSummary {
  target: string;
  summary: Summary;
  blockingPackages: number;
  rootBlocks: boolean;
}

export interface ScanResult {
  target: string;
  filesScanned: number;
  linesScanned: number;
  findings: Finding[];
  errors: { file: string; message: string }[];
  durationMs: number;
  summary: Summary;
  ignored: IgnoredFinding[];
  staleIgnores: IgnoreEntry[];
  progress?: ScanHistory | null;
  runtime?: RuntimeSummary | null;
}

export interface RuntimeSummary {
  file: string;
  events: number;
  confirmed: number;
  added: number;
  outside: number;
}

export interface ScanProgress {
  run: number;
  done: number;
  total: number;
  file: string;
}

export interface Dependency {
  name: string;
  version: string;
  phpConstraint: string | null;
  status: DependencyStatus;
  dev: boolean;
  note?: Localized | null;
}

export interface ExtensionUse {
  name: string;
  uses: number;
  file: string;
  line: number;
  symbol: string;
  declared: boolean;
  removed: boolean;
  available: boolean | null;
}

export interface DependencyReport {
  target: string;
  rootConstraint: string | null;
  rootStatus: DependencyStatus;
  packages: Dependency[];
  hasLockFile: boolean;
  extensions: ExtensionUse[];
}

export interface VersionSuggestion {
  name: string;
  version: string;
  majorChange: boolean;
}

export interface ToolInfo {
  path: string;
  version: string | null;
}

export interface ToolchainStatus {
  php: ToolInfo | null;
  composer: ToolInfo | null;
  rector: ToolInfo | null;
  docker: ToolInfo | null;
  toolsDir: string;
  platform: "macos" | "windows" | "linux";
}

export interface DiffLine {
  kind: "context" | "add" | "del";
  oldNo: number | null;
  newNo: number | null;
  text: string;
}

export interface Hunk {
  oldStart: number;
  newStart: number;
  lines: DiffLine[];
}

export interface FileDiff {
  hunks: Hunk[];
  additions: number;
  deletions: number;
}

export interface AppliedRule {
  class: string;
  name: string;
  title: Localized;
  since: string | null;
  fix: FixKind;
}

export interface FileChange {
  file: string;
  rules: AppliedRule[];
  fix: FixKind;
  diff: FileDiff;
}

export interface RectorPreview {
  id: number;
  target: string;
  mode: RectorMode;
  changes: FileChange[];
  errors: string[];
  durationMs: number;
}

export interface Snapshot {
  id: string;
  createdAt: number;
  projectRoot: string;
  label: string;
  files: { path: string; existed: boolean }[];
}

export interface ApplyResult {
  snapshot: Snapshot;
  written: string[];
  lintErrors: string[];
}

export interface BackupUsage {
  snapshots: number;
  projects: number;
  bytes: number;
}

export interface Settings {
  phpPath: string | null;
  snapshotLimit: number | null;
  editor: string | null;
}
