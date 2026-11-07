import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { ArrowRight, Check, CircleAlert, CircleCheck, Code2, Container, Database, FlaskConical, Globe, Play, RotateCw, ShieldAlert, ShieldCheck, Wand2 } from "lucide-react";
import { DiffViewer } from "../components/DiffViewer";
import { ConfirmDialog, Sheet } from "../components/Modal";
import { PageHeader, usePageTitle } from "../components/Shell";
import { useToast } from "../components/Toast";
import { Badge, Button, Callout, Card, ComboField, CommandLine, EmptyState, RiskBadge, Select, Switch, TagInput, TextField, Tile, Tooltip } from "../components/ui";
import { useI18n } from "../i18n";
import { containerPath as shortPath, touches } from "../lib/fixes";
import { errorMessage } from "../lib/api";
import { useStore } from "../lib/store";
import { KNOWN_SOURCES, type DatabaseChoice, type DatabasePlan, type DockerComparison, type DockerDelta, type DockerEvent, type DockerOrigin, type DockerRun, type DockerStep, type DockerTrial, type RectorPreview, type VersionSuggestion } from "../lib/types";
import { CodeContext } from "./FindingDetail";

const choice = (source: string, file: string | null, root: string): DatabaseChoice | null => {
  if (source === "live" || source === "seed") return { kind: source };
  if (source === "file") return file ? { kind: "dump", path: file } : null;
  return source.startsWith("dump:") ? { kind: "dump", path: `${root}/${source.slice(5)}` } : null;
};
const pagePaths = (list: string[]) => [...new Set(list.map((p) => (p.startsWith("/") ? p : `/${p}`)))];

function RunsCard({ source, target }: { source: DockerRun; target: DockerRun }) {
  const { t } = useI18n();
  const [log, setLog] = useState<DockerRun | null>(null);
  const tests = (run: DockerRun) => {
    const state = !run.tests
      ? t.docker.noTests
      : run.tests.timedOut
        ? t.docker.testsTimedOut
        : run.tests.passed
          ? t.docker.testsPassed
          : t.docker.testsFailed(run.failedTests.length, run.tests.exitCode);
    return run.tests?.byFile ? `${state} ${t.docker.byFile}` : state;
  };
  const rows: [string, (run: DockerRun) => ReactNode][] = [
    [t.docker.syntax, (run) => t.docker.syntaxErrors(run.lintErrors)],
    [t.docker.tests, tests],
    [t.docker.messages, (run) => run.events.length],
  ];
  if (source.installedExtensions.length + target.installedExtensions.length > 0) {
    rows.push([t.docker.added, (run) => run.installedExtensions.join(", ") || "—"]);
  }
  const runs = [source, target];
  return (
    <Card title={t.docker.sideBySide} bodyClassName="docker-runs">
      <table className="docker-compare">
        <thead>
          <tr>
            <td />
            {runs.map((run) => (
              <th key={run.version} scope="col">
                PHP {run.version}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map(([name, value]) => (
            <tr key={name}>
              <th scope="row">{name}</th>
              {runs.map((run) => (
                <td key={run.version}>{value(run)}</td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
      <div className="docker-logs">
        {runs.map(
          (run) =>
            run.log && (
              <Button key={run.version} variant="ghost" size="sm" onClick={() => setLog(run)}>
                {t.docker.outputOf(run.version)}
              </Button>
            ),
        )}
      </div>
      <Sheet open={!!log} title={t.docker.outputOf(log?.version ?? "")} meta={log?.image} onClose={() => setLog(null)}>
        <pre className="docker-log selectable">{log?.log}</pre>
      </Sheet>
    </Card>
  );
}

function Origins({ origins, target }: { origins: DockerOrigin[]; target: string }) {
  const { t, loc } = useI18n();
  const { api } = useStore();
  const toast = useToast();
  const [found, setFound] = useState<VersionSuggestion[] | null>(null);
  const [looking, setLooking] = useState(false);
  if (origins.every((o) => o.kind === "project")) return null;
  const packages = origins.filter((o) => o.kind === "package" && o.version);
  const look = async () => {
    if (!api) return;
    setLooking(true);
    try {
      setFound(await api.findCompatibleVersions(target, packages.map((o) => ({ name: o.name, version: o.version ?? "" }))));
    } catch (e) {
      toast.push({ tone: "error", title: t.deps.findFailed, description: errorMessage(e, t.common.genericError) });
    } finally {
      setLooking(false);
    }
  };
  const step = (o: DockerOrigin) => {
    if (o.advice) return loc(o.advice);
    if (o.kind === "project") return t.docker.originProject;
    if (o.kind === "core") return t.docker.originCore(o.name);
    return o.kind === "package" ? t.docker.originPackage : t.docker.originExtension;
  };
  return (
    <div className="docker-origins">
      <h3>{t.docker.originTitle}</h3>
      <ul className="docker-events">
        {origins.map((o) => {
          const suggestion = found?.find((s) => s.name === o.name);
          return (
            <li key={`${o.kind}:${o.name}`}>
              <Badge tone={o.fatal > 0 ? "danger" : "neutral"}>{t.docker.originKind[o.kind]}</Badge>
              {o.name && (
                <code>
                  {o.name}
                  {o.version && ` ${o.version}`}
                </code>
              )}
              <span>{t.docker.originCount(o.messages, o.fatal)}</span>
              <span className="subtle docker-origins__step">{step(o)}</span>
              {o.kind === "package" && suggestion && <code className="docker-origins__step">{t.docker.originSuggested(o.name, suggestion.version)}</code>}
              {o.kind === "package" && found && !suggestion && <span className="subtle docker-origins__step">{t.docker.originNoSuggestion}</span>}
            </li>
          );
        })}
      </ul>
      {packages.length > 0 && !found && (
        <>
          <Button variant="secondary" size="sm" loading={looking} onClick={() => void look()}>
            {t.deps.findVersions}
          </Button>
          <p className="subtle">{t.deps.findVersionsNote}</p>
        </>
      )}
    </div>
  );
}

function EventList({ events, label, tone }: { events: DockerEvent[]; label: string; tone: "danger" | "manual" }) {
  return (
    <ul className="docker-events">
      {events.map((e) => (
        <li key={`${e.path}:${e.line}:${e.message}`}>
          <Badge tone={tone}>{label}</Badge>
          <code>
            {shortPath(e.path)}:{e.line}
          </code>
          <span>{e.message}</span>
        </li>
      ))}
    </ul>
  );
}

function TrialResult({ trial, version, onEdit }: { trial: DockerTrial; version: string; onEdit: () => void }) {
  const { outcome, files, skipped } = trial;
  const { t } = useI18n();
  const { applyChanges, applying } = useStore();
  const [confirming, setConfirming] = useState(false);
  const rows: [string, DockerDelta<unknown>][] = [
    [t.docker.fixMessages, outcome.messages],
    [t.docker.fixTests, outcome.tests],
    [t.docker.fixPages, outcome.pages],
    [t.docker.fixTables, outcome.tables],
  ];
  const introduced = rows.some(([, d]) => d.introduced.length > 0) || outcome.lintAfter > outcome.lintBefore;
  const left = rows.some(([, d]) => d.remaining.length > 0) || outcome.lintAfter > 0;
  const named = [...outcome.tests.introduced, ...outcome.pages.introduced, ...outcome.tables.introduced];
  const open = [...outcome.tests.remaining, ...outcome.pages.remaining, ...outcome.tables.remaining];
  const write = async () => {
    await applyChanges(files, skipped, true);
    setConfirming(false);
  };
  return (
    <>
      <Callout tone={introduced ? "danger" : outcome.regressedAfter || left ? "warn" : "success"} icon={introduced || outcome.regressedAfter ? <CircleAlert size={15} /> : <CircleCheck size={15} />}>
        {introduced ? t.docker.fixWorse : outcome.regressedAfter ? t.docker.fixStillBroken(version) : left ? t.docker.fixPartly(version) : t.docker.fixClean(version)}
      </Callout>
      <table className="docker-delta">
        <thead>
          <tr>
            <th scope="col">{t.docker.fixKind}</th>
            <th scope="col">{t.docker.fixFixed}</th>
            <th scope="col">{t.docker.fixRemaining}</th>
            <th scope="col">{t.docker.fixIntroduced}</th>
          </tr>
        </thead>
        <tbody>
          {rows.map(([name, d]) => (
            <tr key={name}>
              <th scope="row">{name}</th>
              <td>{d.fixed.length}</td>
              <td>{d.remaining.length}</td>
              <td>{d.introduced.length}</td>
            </tr>
          ))}
        </tbody>
      </table>
      {outcome.lintBefore + outcome.lintAfter > 0 && <p className="subtle">{t.docker.fixLint(outcome.lintBefore, outcome.lintAfter)}</p>}
      <EventList events={outcome.messages.introduced} label={t.docker.fixIntroduced} tone="danger" />
      {named.length > 0 && (
        <ul className="docker-events">
          {named.map((name) => (
            <li key={name}>
              <Badge tone="danger">{t.docker.fixIntroduced}</Badge>
              <code>{name}</code>
            </li>
          ))}
        </ul>
      )}
      <EventList events={outcome.messages.remaining} label={t.docker.fixRemaining} tone="manual" />
      {open.length > 0 && (
        <ul className="docker-events">
          {open.map((name) => (
            <li key={name}>
              <Badge tone="manual">{t.docker.fixRemaining}</Badge>
              <code>{name}</code>
            </li>
          ))}
        </ul>
      )}
      {left && <p className="subtle">{t.docker.fixManual}</p>}
      <Origins origins={outcome.origins} target={version} />
      <div className="docker-actions">
        <Button variant={introduced ? "secondary" : "primary"} onClick={() => setConfirming(true)}>
          {t.docker.fixWrite(files.length)}
        </Button>
        <Button variant="secondary" onClick={onEdit}>
          {t.docker.fixEdit}
        </Button>
      </div>
      <ConfirmDialog open={confirming} icon={<Wand2 size={20} />} title={t.docker.fixConfirmTitle} confirmLabel={t.docker.fixConfirm} loading={applying} onCancel={() => setConfirming(false)} onConfirm={() => void write()}>
        {t.docker.fixConfirmText(files.length)}
      </ConfirmDialog>
    </>
  );
}

function FixLoop({ result, preview, onTry }: { result: DockerComparison; preview: RectorPreview; onTry: (files: string[]) => void }) {
  const { t } = useI18n();
  const { dockerTrial } = useStore();
  const [editing, setEditing] = useState(false);
  const changes = preview.changes;
  const known = dockerTrial?.previewId === preview.id && dockerTrial.files.every((f) => changes.some((c) => c.file === f)) ? dockerTrial : null;
  const related = (file: string) => result.newOnTarget.filter((e) => changes.some((c) => c.file === file && touches(e, c))).length;
  const [picked, setPicked] = useState<{ previewId: number; files: Set<string> } | null>(null);
  const chosen = useMemo(() => {
    if (picked?.previewId === preview.id) return picked.files;
    if (known) return new Set(known.files);
    return new Set(changes.filter((c) => c.fix === "auto" || result.newOnTarget.some((e) => touches(e, c))).map((c) => c.file));
  }, [preview, picked, known, changes, result]);
  const toggle = (file: string) => {
    const files = new Set(chosen);
    if (!files.delete(file)) files.add(file);
    setPicked({ previewId: preview.id, files });
  };
  const matched = result.newOnTarget.filter((e) => changes.some((c) => touches(e, c))).length;
  return (
    <Card title={t.docker.fixTitle}>
      {known && !editing ? (
        <TrialResult trial={known} version={result.target.version} onEdit={() => setEditing(true)} />
      ) : changes.length === 0 ? (
        <p className="subtle">{t.docker.fixNone}</p>
      ) : (
        <>
          <p>{t.docker.fixIntro}</p>
          {result.newOnTarget.length > 0 && <p className="subtle">{t.docker.fixMatched(matched, result.newOnTarget.length)}</p>}
          <div className="docker-fixes">
            {changes.map((c) => (
              <div className="docker-fix" key={c.file}>
                <label>
                  <input type="checkbox" className="checkbox" checked={chosen.has(c.file)} onChange={() => toggle(c.file)} />
                  <code>{c.file}</code>
                  <RiskBadge fix={c.fix} />
                  {related(c.file) > 0 && <Badge tone="danger">{t.docker.fixRelated(related(c.file))}</Badge>}
                </label>
                <details>
                  <summary>{t.docker.fixShow}</summary>
                  <DiffViewer diff={c.diff} file={c.file} />
                </details>
              </div>
            ))}
          </div>
          <Button variant="primary" icon={<Play size={14} />} disabled={chosen.size === 0} title={chosen.size === 0 ? t.changes.pickFileFirst : undefined} onClick={() => onTry(changes.filter((c) => chosen.has(c.file)).map((c) => c.file))}>
            {t.docker.fixTry(chosen.size)}
          </Button>
        </>
      )}
    </Card>
  );
}

function Verdict({ result, action, note }: { result: DockerComparison; action?: ReactNode; note?: string }) {
  const { t } = useI18n();
  const bad = result.verdict === "regressed";
  const incomplete = result.verdict === "incomplete";
  const tested = !!result.source.tests?.passed && !!result.target.tests;
  const tone = bad ? "bad" : tested && !incomplete ? "good" : "warn";
  return (
    <section className="card real-hero" role={bad ? "alert" : undefined}>
      <span className={`badge-icon badge-icon--${tone} real-hero__icon`} aria-hidden>
        {tone === "good" ? <CircleCheck size={20} /> : <CircleAlert size={20} />}
      </span>
      <div className="real-hero__text">
        <h2>{bad ? t.docker.worse(result.target.version) : incomplete ? t.docker.incomplete : tested ? t.docker.same(result.target.version) : t.docker.notTested}</h2>
        {result.gaps.length > 0 && (
          <ul className="docker-gaps">
            {result.gaps.map((gap) => (
              <li key={gap}>{t.docker.gap[gap]}</li>
            ))}
          </ul>
        )}
        {note && <p className="hint">{note}</p>}
      </div>
      {action}
    </section>
  );
}

function Metrics({ result }: { result: DockerComparison }) {
  const { t } = useI18n();
  const pages = result.pages.filter((p) => p.change !== "same").length;
  const metrics: [string, number, ReactNode][] = [
    [t.docker.metricSyntax, Math.max(0, result.target.lintErrors - result.source.lintErrors), null],
    [t.docker.metricMessages, result.newOnTarget.length, null],
    [t.docker.metricTests, result.newFailures.length, null],
  ];
  if (result.pages.length > 0) metrics.push([t.docker.metricPages, pages, `${pages} / ${result.pages.length}`]);
  if (result.databaseUsed) metrics.push([t.docker.metricTables, result.dataChanges.length, null]);
  return (
    <dl className="card tiles">
      {metrics.map(([name, value, text]) => (
        <Tile key={name} label={name} tone={value > 0 ? "danger" : "ok"}>
          {text ?? value}
        </Tile>
      ))}
    </dl>
  );
}

function NewOnTarget({ result }: { result: DockerComparison }) {
  const { t } = useI18n();
  const { api, project, showFindings } = useStore();
  const [open, setOpen] = useState<string | null>(null);
  const pages = result.pages.filter((p) => p.change !== "same");
  const count = result.newOnTarget.length + result.newFailures.length + pages.length + result.dataChanges.length;
  const plain = (key: string, title: string, sub: string | null, kind: string, tone: "danger" | "neutral" = "danger") => (
    <li key={key} className="line real-row">
      <span className={`dot dot--${tone}`} aria-hidden />
      <span className="line__main">
        <span className="line__title mono">{title}</span>
        {sub && <span className="line__sub">{sub}</span>}
      </span>
      <Badge>{kind}</Badge>
    </li>
  );
  return (
    <Card
      title={t.docker.newTitle(result.target.version, count)}
      actions={
        result.newOnTarget.length > 0 && (
          <Button variant="ghost" size="sm" onClick={() => showFindings()}>
            {t.docker.openFindings}
          </Button>
        )
      }
      bodyClassName="lines-card"
    >
      {count === 0 ? (
        <p className="subtle real-none">{t.docker.noneNew}</p>
      ) : (
        <ul className="lines">
          {result.newOnTarget.map((e) => {
            const key = `${e.path}:${e.line}:${e.message}`;
            const file = shortPath(e.path);
            const shown = open === key;
            return (
              <li key={key} className="real-event">
                <button className="line line--soft real-row" aria-expanded={shown} onClick={() => setOpen(shown ? null : key)}>
                  <span className={`dot dot--${e.level === "fatal" ? "danger" : "warn"}`} aria-hidden />
                  <span className="line__main">
                    <span className="line__title">{e.message}</span>
                    <span className="line__sub mono">
                      {file}:{e.line}
                    </span>
                  </span>
                  <Badge>{t.docker.level[e.level]}</Badge>
                </button>
                {shown && <CodeContext finding={{ id: key, file, line: e.line, snippet: "" }} root={project?.root} api={api} />}
              </li>
            );
          })}
          {result.newFailures.map((name) => plain(`test:${name}`, name, null, t.docker.kindTest))}
          {pages.map((p) => plain(`page:${p.path}`, p.path, [`${p.sourceStatus} → ${p.targetStatus}`, p.newErrors[0]].filter(Boolean).join(" · "), t.docker.pageChange[p.change], p.change === "unstable" || p.change === "unverified" ? "neutral" : "danger"))}
          {result.dataChanges.map((c) => plain(`table:${c.table}`, c.table, `${c.sourceRows ?? "—"} → ${c.targetRows ?? "—"}`, t.docker.kindTable))}
        </ul>
      )}
      {result.goneOnTarget.length > 0 && <p className="subtle real-none">{t.docker.gone(result.goneOnTarget.length)}</p>}
    </Card>
  );
}

function Result({ result, onTry }: { result: DockerComparison; onTry: (files: string[]) => void }) {
  const { t } = useI18n();
  const { toolchain, preview, previewMode, previewing, loadPreview, setView } = useStore();
  const bad = result.verdict === "regressed";
  const missing = [...new Set([...result.source.missingExtensions, ...result.target.missingExtensions])];
  const fixable = bad || result.newOnTarget.length > 0 || result.pages.some((p) => p.change !== "same");
  const ready = fixable && preview && previewMode === "compat" && preview.target === result.target.version ? preview : null;
  const prepares = fixable && !ready && !(toolchain && !toolchain.rector);
  const action =
    !fixable || ready ? undefined : toolchain && !toolchain.rector ? (
      <Button size="lg" onClick={() => setView("changes")}>
        {t.docker.fixOpenChanges}
      </Button>
    ) : (
      <Button variant="primary" size="lg" loading={previewing} onClick={() => void loadPreview("compat")}>
        {t.docker.fixPrepare}
      </Button>
    );
  return (
    <>
      <Verdict result={result} action={action} note={prepares ? t.changes.introTrust : undefined} />
      {ready && <FixLoop result={result} preview={ready} onTry={onTry} />}
      <Metrics result={result} />
      {!result.dependenciesInstalled && (
        <Callout tone="warn">
          {t.docker.notInstalled}
          {result.installLog && (
            <details className="check-log">
              <summary>{t.findings.showOutput}</summary>
              <pre>{result.installLog}</pre>
            </details>
          )}
        </Callout>
      )}
      {(result.source.imageFailed || result.target.imageFailed) && <Callout tone="warn">{t.docker.imageFailed}</Callout>}
      {missing.length > 0 && <Callout tone="warn">{t.docker.missing(missing.join(", "))}</Callout>}
      {result.source.tests && !result.source.tests.passed && <Callout tone="warn">{t.docker.sourceFailing(result.source.version)}</Callout>}
      <div className="grid-rail">
        <NewOnTarget result={result} />
        <RunsCard source={result.source} target={result.target} />
      </div>
      {result.origins.some((o) => o.kind !== "project") && (
        <Card>
          <Origins origins={result.origins} target={result.target.version} />
        </Card>
      )}
    </>
  );
}

const useElapsed = (running: boolean) => {
  const [seconds, setSeconds] = useState(0);
  useEffect(() => {
    if (!running) return;
    const started = Date.now();
    const id = setInterval(() => setSeconds(Math.floor((Date.now() - started) / 1000)), 1000);
    return () => {
      clearInterval(id);
      setSeconds(0);
    };
  }, [running]);
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
};

type Stage = DockerStep["step"];

function Progress({ step, label, source, target, stages, onStop }: { step: DockerStep; label: string; source: string; target: string; stages: [Stage, string][]; onStop: () => void }) {
  const { t } = useI18n();
  const elapsed = useElapsed(true);
  const lane = useRef(0);
  if (step.version === target) lane.current = 1;
  const preparing = step.step === "install" || step.step === "prepare";
  const at = stages.findIndex(([stage]) => stage === step.step);
  const state = (line: number, index: number) => {
    if (preparing || line > lane.current) return "waiting";
    if (line < lane.current || index < at) return "done";
    return index === at ? "active" : "waiting";
  };
  return (
    <section className="card real-card" aria-label={t.docker.progressLabel}>
      <div className="real-run">
        <span className="spinner" aria-hidden />
        <p className="real-run__status" role="status">
          {label}
        </p>
        <span className="real-run__time">{elapsed}</span>
        <Button onClick={onStop}>{t.docker.stop}</Button>
      </div>
      <div className="docker-lanes">
        {[source, target].map((version, line) => (
          <div className="docker-lane" key={version}>
            <span className="docker-lane__name">PHP {version}</span>
            <ol aria-label={`PHP ${version}`}>
              {stages.map(([stage, name], index) => (
                <li key={stage} data-state={state(line, index)}>
                  <span className="docker-lane__dot" aria-hidden>
                    {state(line, index) === "done" && <Check size={10} strokeWidth={3} />}
                  </span>
                  {name}
                </li>
              ))}
            </ol>
          </div>
        ))}
      </div>
    </section>
  );
}

function ScopeItem({ icon, title, value, checked, onToggle, disabled }: { icon: ReactNode; title: string; value: string; checked?: boolean; onToggle?: (on: boolean) => void; disabled?: boolean }) {
  const body = (
    <>
      <span className="badge-icon" aria-hidden>
        {icon}
      </span>
      <span className="real-scope__text">
        <span className="real-scope__title">{title}</span>
        <span className="real-scope__value" title={value}>
          {value}
        </span>
      </span>
    </>
  );
  if (!onToggle) return <div className="real-scope__item">{body}</div>;
  return (
    <label className="real-scope__item real-scope__item--toggle">
      {body}
      <Switch checked={checked} disabled={disabled} label={title} onChange={(e) => onToggle(e.target.checked)} />
    </label>
  );
}

export function DockerCompare() {
  const { t } = useI18n();
  const { api, project, target, toolchain, docker, dockerStep, runDocker, cancelDocker, dockerArgs, tryFixes, saveConfig } = useStore();
  usePageTitle(t.docker.title);
  const saved = project?.config.docker;
  const [trying, setTrying] = useState<string[] | null>(null);
  const [plan, setPlan] = useState<DatabasePlan | null>(null);
  const [dataSource, setDataSource] = useState(saved?.dump ? `dump:${saved.dump}` : "none");
  const [dumpFile, setDumpFile] = useState<string | null>(null);
  const [hosts, setHosts] = useState<string[]>([]);
  const [remote, setRemote] = useState({ host: "", engine: "mysql" as "mysql" | "postgres", name: "", user: "", sendPassword: false });
  const [confirming, setConfirming] = useState(false);
  const [showOptions, setShowOptions] = useState(false);
  const [editing, setEditing] = useState(false);
  const root = project?.root;
  useEffect(() => {
    if (root) api?.databasePlan(root).then(setPlan).catch(() => setPlan(null));
    api?.sshHosts().then(setHosts).catch(() => {});
  }, [api, root]);
  const [picked, setPicked] = useState<string | null>(saved?.source ?? null);
  const [extensions, setExtensions] = useState(saved?.extensions ?? true);
  const [cleanImages, setCleanImages] = useState(false);
  const [comparePages, setComparePages] = useState(saved?.pages ? saved.pages.length > 0 : true);
  const [paths, setPaths] = useState<string[]>(saved?.pages?.length ? saved.pages : ["/"]);
  const [prepare, setPrepare] = useState(saved?.prepare ?? "");
  const [test, setTest] = useState(saved?.test ?? "");
  if (!project) return null;
  const sources = KNOWN_SOURCES.filter((v) => Number(v) < Number(target));
  const source = picked ?? project.currentVersion ?? "";
  const lastDatabase = dockerArgs?.database;
  const lastRemote = lastDatabase?.kind === "ssh" ? lastDatabase : null;
  const shown = trying && lastRemote ? lastRemote : remote;
  const attempt = (files: string[]) => (lastRemote ? setTrying(files) : void tryFixes(files));
  const projectCommands = [prepare.trim(), test.trim()].filter((c) => c && (c === project.config.docker.prepare?.trim() || c === project.config.docker.test?.trim()));
  const asksServer = dataSource === "ssh" || !!trying;
  const start = () => {
    setConfirming(false);
    if (trying) {
      setTrying(null);
      void tryFixes(trying);
      return;
    }
    const database = dataSource === "ssh" ? ({ kind: "ssh", ...remote } as const) : choice(dataSource, dumpFile, project.root);
    setEditing(false);
    setShowOptions(false);
    void runDocker(source, extensions, extensions && cleanImages, comparePages ? pagePaths(paths) : [], database, prepare.trim() || null, test.trim() || null);
  };
  const dump = dataSource.startsWith("dump:") ? dataSource.slice(5) : null;
  const saveOptions = () =>
    void saveConfig({
      ...project.config,
      docker: { source: source || null, extensions, pages: comparePages ? pagePaths(paths) : [], dump, prepare: prepare.trim() || null, test: test.trim() || null },
    });
  const stepText = dockerStep && (dockerStep.step === "install" ? t.docker.installing : dockerStep.step === "extensions" ? t.docker.preparing(dockerStep.version ?? "") : dockerStep.step === "database" ? t.docker.preparingData : dockerStep.step === "prepare" ? t.docker.runningPrepare : dockerStep.step === "pages" ? t.docker.requesting(dockerStep.version ?? "") : dockerStep.step === "lint" ? t.docker.linting(dockerStep.version ?? "") : t.docker.testing(dockerStep.version ?? ""));
  const usesData = dataSource !== "none";
  const lastPages = dockerArgs ? dockerArgs.pages.length > 0 : comparePages;
  const lastData = dockerArgs ? !!dockerArgs.database : usesData;
  const stages: [Stage, string][] = [
    ["extensions", t.docker.stageImage],
    ["lint", t.docker.stageSyntax],
    ...(lastData ? ([["database", t.docker.stageData]] as [Stage, string][]) : []),
    ...(lastPages ? ([["pages", t.docker.stagePages]] as [Stage, string][]) : []),
    ["tests", t.docker.stageTests],
  ];
  const incomplete = !source || (dataSource === "file" && !dumpFile) || (dataSource === "ssh" && !(remote.host && remote.name && remote.user));
  const toggleData = (on: boolean) => {
    if (!on) return setDataSource("none");
    const best = plan?.dumps[0] ? `dump:${plan.dumps[0]}` : plan?.seed ? "seed" : plan?.live ? "live" : null;
    setDataSource(best ?? "file");
    if (!best) setShowOptions(true);
  };
  const dataValue = !usesData
    ? t.docker.scopeOff
    : dump
      ? dump
      : dataSource === "live"
        ? (plan?.name ?? t.docker.dataSource)
        : dataSource === "seed"
          ? (plan?.seed ?? t.docker.dataSource)
          : dataSource === "ssh"
            ? remote.host || t.docker.dataSsh
            : (dumpFile ?? t.docker.dataPick);
  const page = (body: ReactNode) => (
    <div className="page-scroll real-page">
      <h1 className="sr-only">{t.docker.title}</h1>
      <div className="overview-start real-start" data-tauri-drag-region>
        {body}
      </div>
    </div>
  );
  const dialogs = (
    <ConfirmDialog
      open={confirming || !!trying}
      tone="danger"
      icon={<ShieldAlert size={20} />}
      title={asksServer ? t.docker.sshConfirmTitle : t.docker.commandsConfirmTitle}
      confirmLabel={asksServer ? t.docker.sshConfirm : t.docker.commandsConfirm}
      onCancel={() => {
        setConfirming(false);
        setTrying(null);
      }}
      onConfirm={start}
    >
      {asksServer && t.docker.sshConfirmText(shown.host, shown.name, shown.user)}
      {asksServer && shown.sendPassword && <p>{t.docker.sshPasswordWarning(shown.host)}</p>}
      {projectCommands.length > 0 && !trying && (
        <>
          <p>{t.docker.commandsConfirmText}</p>
          {projectCommands.map((command) => (
            <CommandLine key={command} command={command} />
          ))}
        </>
      )}
    </ConfirmDialog>
  );

  if (toolchain && !toolchain.docker) {
    return page(
      <EmptyState icon={<Container size={24} />} title={t.docker.noDocker}>
        {t.docker.noDockerText}
      </EmptyState>,
    );
  }

  if (dockerStep && stepText) {
    return page(<Progress step={dockerStep} label={stepText} source={dockerArgs?.source ?? source} target={target} stages={stages} onStop={cancelDocker} />);
  }

  if (docker && !editing) {
    return (
      <>
        <PageHeader
          title={t.docker.title}
          subtitle={t.docker.subtitle(docker.source.version, docker.target.version)}
          actions={
            <Button icon={<RotateCw size={14} />} onClick={() => setEditing(true)}>
              {t.docker.again}
            </Button>
          }
        />
        <div className="page-scroll">
          <div className="page-body stack">
            <Result result={docker} onTry={attempt} />
            {dialogs}
          </div>
        </div>
      </>
    );
  }

  return page(
    <>
      <h2 className="page-title">{t.docker.startTitle}</h2>
      <p className="page-lead">{t.docker.startLead}</p>
      <section className="card real-card">
        <div className="real-versions">
          <div className="real-version">
            <span className="real-version__label">{t.docker.from}</span>
            <Select
              label={t.docker.from}
              value={source}
              placeholder={t.common.unknown}
              options={[...new Set([...sources, ...(source ? [source] : [])])].map((v) => ({ value: v, label: `PHP ${v}` }))}
              onChange={setPicked}
            />
          </div>
          <span className="real-versions__arrow" aria-hidden>
            <ArrowRight size={15} />
          </span>
          <div className="real-version real-version--target">
            <span className="real-version__label">{t.docker.to}</span>
            <span className="real-version__value">PHP {target}</span>
          </div>
        </div>

        <div className="real-scope">
          <ScopeItem icon={<Code2 size={15} />} title={t.docker.syntax} value={t.docker.scopeFiles} />
          <ScopeItem icon={<Globe size={15} />} title={t.docker.stagePages} value={comparePages ? paths.join(", ") : t.docker.scopeOff} checked={comparePages} onToggle={setComparePages} />
          <ScopeItem icon={<FlaskConical size={15} />} title={t.docker.tests} value={test.trim() || t.docker.scopeTests} />
          <ScopeItem icon={<Database size={15} />} title={t.docker.dataSource} value={dataValue} checked={usesData} onToggle={toggleData} />
        </div>

        {showOptions && (
          <div className="real-options">
            {comparePages && (
              <div className="real-options__wide">
                <TagInput label={t.docker.pagesLabel} values={paths} placeholder={t.docker.pagesAdd} removeLabel={t.docker.pagesRemove} onChange={(list) => setPaths(pagePaths(list))} />
                <p className="hint real-note">{t.docker.pagesHint}</p>
              </div>
            )}
            <div>
              <TextField label={t.docker.testCommand} prefix="$" mono placeholder="vendor/bin/pest" value={test} onChange={(e) => setTest(e.target.value)} />
              {test.trim() && <p className="hint real-note">{t.docker.testCommandNote}</p>}
            </div>
            <div>
              <TextField label={t.docker.prepare} prefix="$" mono placeholder={t.docker.optional} value={prepare} onChange={(e) => setPrepare(e.target.value)} />
              {prepare.trim() && <p className="hint real-note">{t.docker.prepareNote}</p>}
            </div>
            <div className="real-options__wide">
              <Select
                block
                label={t.docker.dataSource}
                value={dataSource}
                icon={<Database size={14} aria-hidden />}
                options={[
                  { value: "none", label: t.docker.dataNone },
                  { value: "file", label: t.docker.dataFile },
                  ...(saved?.dump && !plan?.dumps.includes(saved.dump) ? [{ value: `dump:${saved.dump}`, label: t.docker.dataDump(saved.dump) }] : []),
                  ...(plan?.dumps ?? []).map((d) => ({ value: `dump:${d}`, label: t.docker.dataDump(d) })),
                  ...(plan?.live ? [{ value: "live", label: t.docker.dataLive(plan.name ?? "", plan.host ?? "") }] : []),
                  ...(plan?.seed ? [{ value: "seed", label: t.docker.dataSeed(plan.seed) }] : []),
                  { value: "ssh", label: t.docker.dataSsh },
                ]}
                onChange={setDataSource}
              />
              {dataSource === "live" && <p className="hint real-note">{t.docker.dataLiveNote}</p>}
            </div>
            {dataSource === "file" && (
              <div className="real-options__wide">
                <Button onClick={() => void api?.pickLogFile(t.docker.dataPick).then((f) => f && setDumpFile(f))}>{dumpFile ?? t.docker.dataPick}</Button>
              </div>
            )}
            {dataSource === "ssh" && (
              <>
                <ComboField label={t.docker.sshHost} value={remote.host} options={hosts} onChange={(host) => setRemote({ ...remote, host })} />
                <Select
                  block
                  label={t.docker.sshEngine}
                  value={remote.engine}
                  options={[
                    { value: "mysql", label: "MySQL / MariaDB" },
                    { value: "postgres", label: "PostgreSQL" },
                  ]}
                  onChange={(engine) => setRemote({ ...remote, engine })}
                />
                <TextField label={t.docker.sshName} placeholder={plan?.name ?? undefined} value={remote.name} onChange={(e) => setRemote({ ...remote, name: e.target.value })} />
                <TextField label={t.docker.sshUser} placeholder={plan?.user ?? undefined} value={remote.user} onChange={(e) => setRemote({ ...remote, user: e.target.value })} />
                <label className="real-check real-options__wide">
                  <input type="checkbox" className="checkbox" checked={remote.sendPassword} onChange={(e) => setRemote({ ...remote, sendPassword: e.target.checked })} />
                  {t.docker.sshSendPassword}
                </label>
                <p className="hint real-options__wide">{t.docker.sshNote}</p>
              </>
            )}
            <label className="real-check">
              <input type="checkbox" className="checkbox" checked={extensions} onChange={(e) => setExtensions(e.target.checked)} />
              {t.docker.installExtensions}
            </label>
            <label className="real-check">
              <input type="checkbox" className="checkbox" checked={extensions && cleanImages} disabled={!extensions} onChange={(e) => setCleanImages(e.target.checked)} />
              {t.docker.cleanAfter}
            </label>
            <div className="real-options__wide real-options__save">
              <Tooltip text={t.docker.saveOptionsNote} align="start">
                <Button variant="ghost" size="sm" onClick={saveOptions}>
                  {t.docker.saveOptions}
                </Button>
              </Tooltip>
            </div>
          </div>
        )}

        <div className="real-foot">
          <Tooltip text={t.docker.safety} align="start">
            <span className="real-safety" tabIndex={0}>
              <ShieldCheck size={14} aria-hidden />
              {t.docker.safetyShort}
            </span>
          </Tooltip>
          <span className="real-foot__actions">
            {docker && (
              <Button variant="ghost" onClick={() => setEditing(false)}>
                {t.docker.backToResult}
              </Button>
            )}
            <Button variant="ghost" aria-expanded={showOptions} onClick={() => setShowOptions(!showOptions)}>
              {showOptions ? t.docker.closeOptions : t.docker.options}
            </Button>
            <Button variant="primary" disabled={incomplete} title={incomplete ? (!source ? t.docker.needSource : dataSource === "file" ? t.docker.needDump : t.docker.needServer) : undefined} onClick={() => (dataSource === "ssh" || projectCommands.length > 0 ? setConfirming(true) : start())}>
              {t.docker.start}
            </Button>
          </span>
        </div>
      </section>
      {dialogs}
    </>,
  );
}
