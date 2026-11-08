import type { Localized } from "../i18n";

export type Framework = "wordpress" | "laravel" | "symfony" | "drupal" | "magento" | "prestashop" | "joomla" | "moodle" | "mediawiki" | "phpbb" | "codeigniter" | "yii" | "cakephp" | "opencart" | "generic";
export type Severity = "breaking" | "deprecated" | "behavior";
export type FixKind = "auto" | "likely" | "manual";
export type DependencyStatus = "compatible" | "incompatible" | "unknown";
export type RectorMode = "compat" | "modernize" | "pins";

export const TARGETS = ["8.1", "8.2", "8.3", "8.4", "8.5"] as const;
export type Target = (typeof TARGETS)[number];
const PHP_VERSIONS = ["5.6", "7.0", "7.1", "7.2", "7.3", "7.4", "8.0", ...TARGETS];

/** The versions between the two, for moving up one version at a time. */
export const stepsBetween = (current: string | null, target: string) => {
  const from = current ? PHP_VERSIONS.indexOf(current) : -1;
  const to = PHP_VERSIONS.indexOf(target);
  return from < 0 ? [] : PHP_VERSIONS.slice(from + 1, Math.max(to, from + 1));
};

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
  config: ProjectConfig;
  configError: string | null;
}

export interface ProjectConfig {
  target: string | null;
  exclude: string[];
  baseline: string | null;
  docker: {
    source: string | null;
    extensions: boolean | null;
    pages: string[] | null;
    dump: string | null;
    prepare: string | null;
    test: string | null;
  };
  git: {
    requireClean: boolean;
    branchPerBatch: boolean;
  };
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
  inline?: boolean;
}

export interface IgnoreEntry {
  ruleId: string;
  file: string;
  snippet: string;
  reason: string;
}

export interface ScanCounts {
  breaking: number;
  deprecated: number;
  behavior: number;
  files: number;
}

export interface HistoryEntry {
  at: number;
  target: string;
  score: number;
  total: number;
  counts?: ScanCounts | null;
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
  incomplete?: boolean;
  error?: string | null;
}

export interface ScanResult {
  target: string;
  filesFound: number;
  filesScanned: number;
  linesScanned: number;
  findings: Finding[];
  errors: { file: string; message: string }[];
  durationMs: number;
  stamp: string;
  summary: Summary;
  ignored: IgnoredFinding[];
  staleIgnores: IgnoreEntry[];
  progress?: ScanHistory | null;
  runtime?: RuntimeSummary | null;
}

export interface LocalPhp {
  line: string;
  version: string;
  path: string;
}

export interface CheckSummary {
  at: number;
  target: string;
  label: string;
  kind: "lint" | "tests";
  passed: boolean;
  timedOut: boolean;
  exitCode: number | null;
  failed: number;
  durationMs: number;
}

export interface CheckRun {
  kind: "lint" | "tests";
  command: string;
  exitCode: number | null;
  timedOut: boolean;
  passed: boolean;
  durationMs: number;
  files: number;
  failed: number;
  notRun: number;
  log: string;
}

export const KNOWN_SOURCES = ["5.6", "7.0", "7.1", "7.2", "7.3", "7.4", "8.0", "8.1", "8.2", "8.3", "8.4"];

export interface DockerEvent {
  path: string;
  line: number;
  level: "fatal" | "warning" | "deprecated";
  message: string;
  count: number;
}

export interface DockerRun {
  version: string;
  image: string;
  lintErrors: number;
  lintFiles: number;
  lintStopped: boolean;
  pagesStopped: boolean;
  pagesLimited: boolean;
  tests: { exitCode: number | null; timedOut: boolean; passed: boolean; byFile: boolean; ran: boolean; total: number | null } | null;
  failedTests: string[];
  missingExtensions: string[];
  installedExtensions: string[];
  imageFailed: boolean;
  events: DockerEvent[];
  log: string;
}

export interface DockerPage {
  path: string;
  change: "same" | "status" | "redirect" | "errors" | "body" | "unstable" | "unverified" | "missing";
  sourceStatus: number;
  targetStatus: number;
  newErrors: string[];
  sourceText: string;
  targetText: string;
}

export interface DatabasePlan {
  engine: "mysql" | "postgres" | "sqlite" | null;
  host: string | null;
  name: string | null;
  user: string | null;
  live: boolean;
  seed: string | null;
  dumps: string[];
}

export type DatabaseChoice =
  | { kind: "dump"; path: string }
  | { kind: "live" }
  | { kind: "seed" }
  | { kind: "ssh"; host: string; engine: "mysql" | "postgres"; name: string; user: string; sendPassword: boolean };

export interface DockerDataChange {
  table: string;
  sourceRows: number | null;
  targetRows: number | null;
}

export interface DockerOrigin {
  kind: "project" | "core" | "package" | "extension";
  name: string;
  version: string | null;
  messages: number;
  fatal: number;
  advice: Localized | null;
}

export interface DockerComparison {
  source: DockerRun;
  target: DockerRun;
  dependenciesInstalled: boolean;
  installLog: string;
  newFailures: string[];
  pages: DockerPage[];
  dataChanges: DockerDataChange[];
  databaseUsed: boolean;
  newOnTarget: DockerEvent[];
  goneOnTarget: DockerEvent[];
  origins: DockerOrigin[];
  verdict: "clean" | "regressed" | "incomplete";
  gaps: DockerGap[];
  stamp: string;
}

export type DockerGap = "dependencies" | "noFiles" | "lintStopped" | "pagesStopped" | "pagesLimited" | "testsStopped" | "testsDidNotRun" | "fewerTests" | "testsNotCompared" | "extensionImage" | "unreadFiles" | "differences";

export interface DockerSummary {
  at: number;
  source: string;
  target: string;
  regressed: boolean;
  testsCompared: boolean;
  pages: number;
  brokenPages: number;
  differingPages: number;
  unverifiedPages: number;
  newFailures: number;
  newMessages: number;
  dataChanges: number;
  incomplete: boolean;
  stamp: string;
}

export interface DockerImageUsage {
  images: number;
  bytes: number;
}

export interface DockerStep {
  step: "install" | "extensions" | "lint" | "database" | "prepare" | "pages" | "tests";
  version: string | null;
}

export interface DockerDelta<T> {
  fixed: T[];
  remaining: T[];
  introduced: T[];
}

export interface DockerOutcome {
  messages: DockerDelta<DockerEvent>;
  tests: DockerDelta<string>;
  pages: DockerDelta<string>;
  tables: DockerDelta<string>;
  lintBefore: number;
  lintAfter: number;
  regressedBefore: boolean;
  regressedAfter: boolean;
  origins: DockerOrigin[];
}

export interface DockerResponse {
  comparison: DockerComparison;
  scan: ScanResult | null;
  outcome: DockerOutcome | null;
}

export interface DockerFixChoice {
  previewId: number;
  files: string[];
  skipped: Record<string, number[]>;
}

export interface DockerRunArgs {
  source: string;
  extensions: boolean;
  cleanImages: boolean;
  pages: string[];
  database: DatabaseChoice | null;
  prepare: string | null;
  test: string | null;
}

export interface DockerTrial {
  previewId: number;
  files: string[];
  skipped: Record<string, number[]>;
  outcome: DockerOutcome;
}

export interface RuntimeSummary {
  file: string;
  events: number;
  confirmed: number;
  added: number;
  outside: number;
  outsideFatals: number;
  check?: CheckRun | null;
  roots: string[];
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
  requiredBy?: string[];
  dev?: boolean;
}

export interface DependencyReport {
  target: string;
  rootConstraint: string | null;
  rootStatus: DependencyStatus;
  packages: Dependency[];
  hasLockFile: boolean;
  lockUnreadable?: boolean;
  manifestUnreadable?: boolean;
  rootConflict?: string | null;
  extensions: ExtensionUse[];
  wordpress: WpComponent[];
}

export interface WpComponent {
  kind: "plugin" | "theme";
  slug: string;
  name: string;
  version: string | null;
  requiresPhp: string | null;
  dir: string;
}

export interface WpLatest {
  kind: "plugin" | "theme";
  slug: string;
  version: string | null;
  requiresPhp: string | null;
  lastUpdated: string | null;
  closed: boolean;
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

export interface SnapshotConflict {
  file: string;
  hash: string | null;
}

export interface ApplyResult {
  snapshot: Snapshot;
  written: string[];
  lintErrors: string[];
  lintChecked: boolean;
  branch?: string | null;
  commit?: string | null;
  gitError?: string | null;
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
  editorCommand: string | null;
  aiProjects: string[];
}

/** Exactly what an AI suggestion sends, shown before anything leaves the computer. */
export interface AiPrompt {
  system: string;
  user: string;
  file: string;
  firstLine: number;
  lastLine: number;
}

export type AiKind = "anthropic" | "openai" | "local";

/** Which provider AI suggestions use. The key isn't part of it: it stays in the system keychain. */
export interface AiConfig {
  kind: AiKind | null;
  model: string | null;
  baseUrl: string | null;
}

export interface AiView extends AiConfig {
  hasKey: boolean;
  models: string[];
}

export interface LocalModelInfo {
  id: string;
  name: string;
  license: string;
  repo: string;
  size: number;
}

export interface LocalModelView {
  /** The model for this computer, or null when its memory is too small. */
  model: LocalModelInfo | null;
  installed: boolean;
  installing: boolean;
  remaining: number;
  memory: number;
}

export type LocalPhase = "runtime" | "model" | "verify" | "check";

export interface LocalProgress {
  phase: LocalPhase;
  done: number;
  total: number;
}

export interface LocalAiServer {
  name: string;
  baseUrl: string;
  models: string[];
}

export interface AiSuggestion {
  id: number;
  change: FileChange;
  lintErrors: string[];
  lintChecked: boolean;
}

export interface SourceLine {
  number: number;
  text: string;
}
