import { useEffect, useMemo, useState } from "react";
import { CircleAlert, CircleCheck, Container, Play, ShieldAlert, Square, Wand2 } from "lucide-react";
import { DiffViewer } from "../components/DiffViewer";
import { ConfirmDialog } from "../components/Modal";
import { PageHeader } from "../components/Shell";
import { useToast } from "../components/Toast";
import { Badge, Button, Callout, Card, CommandLine, EmptyState, RiskBadge } from "../components/ui";
import { useI18n } from "../i18n";
import { containerPath as shortPath, touches } from "../lib/fixes";
import { errorMessage } from "../lib/api";
import { useStore } from "../lib/store";
import { KNOWN_SOURCES, type DatabaseChoice, type DatabasePlan, type DockerComparison, type DockerDelta, type DockerEvent, type DockerOrigin, type DockerOutcome, type DockerPage, type DockerRun, type Target, type VersionSuggestion } from "../lib/types";

const choice = (source: string, file: string | null, root: string): DatabaseChoice | null => {
  if (source === "live" || source === "seed") return { kind: source };
  if (source === "file") return file ? { kind: "dump", path: file } : null;
  return source.startsWith("dump:") ? { kind: "dump", path: `${root}/${source.slice(5)}` } : null;
};
const parsePaths = (text: string) => text.split(",").map((p) => p.trim()).filter(Boolean).map((p) => (p.startsWith("/") ? p : `/${p}`));

function RunCard({ run }: { run: DockerRun }) {
  const { t } = useI18n();
  const tests = !run.tests
    ? t.docker.noTests
    : run.tests.timedOut
      ? t.docker.testsTimedOut
      : run.tests.passed
        ? t.docker.testsPassed
        : t.docker.testsFailed(run.failedTests.length, run.tests.exitCode);
  return (
    <Card title={`PHP ${run.version}`}>
      <dl className="docker-run">
        <dt>{t.docker.syntax}</dt>
        <dd>{t.docker.syntaxErrors(run.lintErrors)}</dd>
        <dt>{t.docker.tests}</dt>
        <dd>
          {tests}
          {run.tests?.byFile && ` ${t.docker.byFile}`}
        </dd>
        <dt>{t.docker.messages}</dt>
        <dd>{run.events.length}</dd>
        {run.installedExtensions.length > 0 && (
          <>
            <dt>{t.docker.added}</dt>
            <dd>{run.installedExtensions.join(", ")}</dd>
          </>
        )}
      </dl>
      {run.log && (
        <details className="check-log">
          <summary>{t.findings.showOutput}</summary>
          <code className="check-log__command">{run.image}</code>
          <pre>{run.log}</pre>
        </details>
      )}
    </Card>
  );
}

function PagesCard({ pages, version }: { pages: DockerPage[]; version: string }) {
  const { t } = useI18n();
  const changed = pages.filter((p) => p.change !== "same");
  return (
    <Card title={t.docker.pagesTitle(version, changed.length, pages.length)}>
      {changed.length === 0 ? (
        <p className="subtle">{t.docker.pagesSame}</p>
      ) : (
        <ul className="docker-events">
          {changed.map((p) => (
            <li key={p.path}>
              <Badge tone={p.change === "unstable" || p.change === "unverified" ? "neutral" : "danger"}>{t.docker.pageChange[p.change]}</Badge>
              <code>{p.path}</code>
              <span>
                {p.sourceStatus} → {p.targetStatus}
              </span>
              {p.newErrors[0] && <span>{p.newErrors[0]}</span>}
            </li>
          ))}
        </ul>
      )}
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

function TrialResult({ outcome, version, files, onEdit }: { outcome: DockerOutcome; version: string; files: string[]; onEdit: () => void }) {
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
    await applyChanges(files, undefined, true);
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

function FixLoop({ result, onTry }: { result: DockerComparison; onTry: (files: string[]) => void }) {
  const { t } = useI18n();
  const { toolchain, preview, previewMode, previewing, loadPreview, dockerTrial, setView } = useStore();
  const [editing, setEditing] = useState(false);
  const ready = preview && previewMode === "compat" && preview.target === result.target.version ? preview : null;
  const changes = useMemo(() => ready?.changes ?? [], [ready]);
  const known = dockerTrial?.previewId === ready?.id && dockerTrial?.files.every((f) => changes.some((c) => c.file === f)) ? dockerTrial : null;
  const related = (file: string) => result.newOnTarget.filter((e) => changes.some((c) => c.file === file && touches(e, c))).length;
  const [picked, setPicked] = useState<{ previewId: number; files: Set<string> } | null>(null);
  const chosen = useMemo(() => {
    if (ready && picked?.previewId === ready.id) return picked.files;
    if (known) return new Set(known.files);
    return new Set(changes.filter((c) => c.fix === "auto" || result.newOnTarget.some((e) => touches(e, c))).map((c) => c.file));
  }, [ready, picked, known, changes, result]);
  const toggle = (file: string) => {
    if (!ready) return;
    const files = new Set(chosen);
    if (!files.delete(file)) files.add(file);
    setPicked({ previewId: ready.id, files });
  };
  const matched = result.newOnTarget.filter((e) => changes.some((c) => touches(e, c))).length;
  return (
    <Card title={t.docker.fixTitle}>
      {known && !editing ? (
        <TrialResult outcome={known.outcome} version={result.target.version} files={known.files} onEdit={() => setEditing(true)} />
      ) : !ready ? (
        <>
          <p>{t.docker.fixIntro}</p>
          <p className="hint">{t.changes.introTrust}</p>
          {toolchain && !toolchain.rector ? (
            <>
              <p className="subtle">{t.docker.fixNeedsRector}</p>
              <Button variant="secondary" onClick={() => setView("changes")}>
                {t.docker.fixOpenChanges}
              </Button>
            </>
          ) : (
            <Button variant="primary" icon={<Wand2 size={14} />} loading={previewing} onClick={() => void loadPreview("compat")}>
              {t.docker.fixPrepare}
            </Button>
          )}
        </>
      ) : changes.length === 0 ? (
        <p className="subtle">{t.docker.fixNone}</p>
      ) : (
        <>
          <p>{t.docker.fixIntro}</p>
          <p className="hint">{t.changes.introTrust}</p>
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
          <Button variant="primary" icon={<Play size={14} />} disabled={chosen.size === 0} onClick={() => onTry(changes.filter((c) => chosen.has(c.file)).map((c) => c.file))}>
            {t.docker.fixTry(chosen.size)}
          </Button>
        </>
      )}
    </Card>
  );
}

function Result({ result, onTry }: { result: DockerComparison; onTry: (files: string[]) => void }) {
  const { t } = useI18n();
  const { showFindings } = useStore();
  const bad = result.verdict === "regressed";
  const incomplete = result.verdict === "incomplete";
  const tested = !!result.source.tests?.passed && !!result.target.tests;
  const missing = [...new Set([...result.source.missingExtensions, ...result.target.missingExtensions])];
  const fixable = bad || result.newOnTarget.length > 0 || result.pages.some((p) => p.change !== "same");
  return (
    <>
      <Callout tone={bad ? "danger" : tested && !incomplete ? "success" : "warn"} icon={bad || incomplete ? <CircleAlert size={15} /> : <CircleCheck size={15} />}>
        {bad ? t.docker.worse(result.target.version) : incomplete ? t.docker.incomplete : tested ? t.docker.same(result.target.version) : t.docker.notTested}
        {result.gaps.length > 0 && (
          <ul className="docker-gaps">
            {result.gaps.map((gap) => (
              <li key={gap}>{t.docker.gap[gap]}</li>
            ))}
          </ul>
        )}
      </Callout>
      {result.origins.some((o) => o.kind !== "project") && (
        <Card>
          <Origins origins={result.origins} target={result.target.version} />
        </Card>
      )}
      {fixable && <FixLoop result={result} onTry={onTry} />}
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
      <div className="grid-2">
        <RunCard run={result.source} />
        <RunCard run={result.target} />
      </div>
      {result.pages.length > 0 && <PagesCard pages={result.pages} version={result.target.version} />}
      {result.databaseUsed && (
        <Card title={t.docker.dataTitle(result.dataChanges.length)}>
          {result.dataChanges.length === 0 ? (
            <p className="subtle">{t.docker.dataSame}</p>
          ) : (
            <ul className="docker-events">
              {result.dataChanges.map((c) => (
                <li key={c.table}>
                  <code>{c.table}</code>
                  <span>
                    {c.sourceRows ?? "—"} → {c.targetRows ?? "—"}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </Card>
      )}
      {result.newFailures.length > 0 && (
        <Card title={t.docker.failuresTitle(result.target.version, result.newFailures.length)}>
          <ul className="docker-events">
            {result.newFailures.map((name) => (
              <li key={name}>
                <code>{name}</code>
              </li>
            ))}
          </ul>
        </Card>
      )}
      {result.source.tests && !result.source.tests.passed && <Callout tone="warn">{t.docker.sourceFailing(result.source.version)}</Callout>}
      <Card title={t.docker.newTitle(result.target.version, result.newOnTarget.length)}>
        {result.newOnTarget.length === 0 ? (
          <p className="subtle">{t.docker.noneNew}</p>
        ) : (
          <>
            <ul className="docker-events">
              {result.newOnTarget.map((e) => (
                <li key={`${e.path}:${e.line}:${e.message}`}>
                  <Badge tone={e.level === "fatal" ? "danger" : "manual"}>{t.docker.level[e.level]}</Badge>
                  <code>
                    {shortPath(e.path)}:{e.line}
                  </code>
                  <span>{e.message}</span>
                </li>
              ))}
            </ul>
            <Button variant="secondary" size="sm" onClick={() => showFindings()}>
              {t.docker.openFindings}
            </Button>
          </>
        )}
        {result.goneOnTarget.length > 0 && <p className="subtle">{t.docker.gone(result.goneOnTarget.length)}</p>}
      </Card>
    </>
  );
}

export function DockerCompare() {
  const { t } = useI18n();
  const { api, project, target, toolchain, docker, dockerStep, runDocker, cancelDocker, dockerArgs, tryFixes, saveConfig } = useStore();
  const saved = project?.config.docker;
  const [trying, setTrying] = useState<string[] | null>(null);
  const [plan, setPlan] = useState<DatabasePlan | null>(null);
  const [dataSource, setDataSource] = useState(saved?.dump ? `dump:${saved.dump}` : "none");
  const [dumpFile, setDumpFile] = useState<string | null>(null);
  const [hosts, setHosts] = useState<string[]>([]);
  const [remote, setRemote] = useState({ host: "", engine: "mysql" as "mysql" | "postgres", name: "", user: "", sendPassword: false });
  const [confirming, setConfirming] = useState(false);
  const root = project?.root;
  useEffect(() => {
    if (root) api?.databasePlan(root).then(setPlan).catch(() => setPlan(null));
    api?.sshHosts().then(setHosts).catch(() => {});
  }, [api, root]);
  const [picked, setPicked] = useState<string | null>(saved?.source ?? null);
  const [extensions, setExtensions] = useState(saved?.extensions ?? true);
  const [cleanImages, setCleanImages] = useState(false);
  const [comparePages, setComparePages] = useState(saved?.pages ? saved.pages.length > 0 : true);
  const [paths, setPaths] = useState(saved?.pages?.join(", ") || "/");
  const [prepare, setPrepare] = useState(saved?.prepare ?? "");
  const [test, setTest] = useState(saved?.test ?? "");
  if (!project) return null;
  const sources = KNOWN_SOURCES.filter((v) => Number(v) < Number(target));
  const source = picked ?? project.currentVersion ?? "";
  const running = !!dockerStep;
  const lastDatabase = dockerArgs?.database;
  const lastRemote = lastDatabase?.kind === "ssh" ? lastDatabase : null;
  const shown = trying && lastRemote ? lastRemote : remote;
  const attempt = (files: string[]) => (lastRemote ? setTrying(files) : void tryFixes(files));
  const projectCommands = [prepare.trim(), test.trim()].filter((c) => c && (c === project.config.docker.prepare || c === project.config.docker.test));
  const asksServer = dataSource === "ssh" || !!trying;
  const start = () => {
    setConfirming(false);
    if (trying) {
      setTrying(null);
      void tryFixes(trying);
      return;
    }
    const database = dataSource === "ssh" ? ({ kind: "ssh", ...remote } as const) : choice(dataSource, dumpFile, project.root);
    void runDocker(source, extensions, extensions && cleanImages, comparePages ? parsePaths(paths) : [], database, prepare.trim() || null, test.trim() || null);
  };
  const dump = dataSource.startsWith("dump:") ? dataSource.slice(5) : null;
  const saveOptions = () =>
    void saveConfig({
      ...project.config,
      target,
      docker: { source: source || null, extensions, pages: comparePages ? parsePaths(paths) : [], dump, prepare: prepare.trim() || null, test: test.trim() || null },
    });
  const stepText = dockerStep && (dockerStep.step === "install" ? t.docker.installing : dockerStep.step === "extensions" ? t.docker.preparing(dockerStep.version ?? "") : dockerStep.step === "database" ? t.docker.preparingData : dockerStep.step === "prepare" ? t.docker.runningPrepare : dockerStep.step === "pages" ? t.docker.requesting(dockerStep.version ?? "") : dockerStep.step === "lint" ? t.docker.linting(dockerStep.version ?? "") : t.docker.testing(dockerStep.version ?? ""));
  return (
    <>
      <PageHeader title={t.docker.title} subtitle={t.docker.subtitle(source || "?", target as Target)} />
      <div className="page-scroll">
        <div className="stack">
          {toolchain && !toolchain.docker ? (
            <EmptyState icon={<Container size={24} />} title={t.docker.noDocker}>
              {t.docker.noDockerText}
            </EmptyState>
          ) : (
            <Card>
              <p>{t.docker.intro}</p>
              <p className="subtle">{t.docker.safety}</p>
              <div className="docker-start">
                <label>
                  <span>{t.docker.from}</span>
                  <select className="input" value={source} disabled={running} onChange={(e) => setPicked(e.target.value)}>
                    {!source && <option value="">{t.common.unknown}</option>}
                    {[...new Set([...sources, ...(source ? [source] : [])])].map((v) => (
                      <option key={v} value={v}>
                        PHP {v}
                      </option>
                    ))}
                  </select>
                </label>
                {running ? (
                  <Button variant="secondary" icon={<Square size={14} />} onClick={cancelDocker}>
                    {t.docker.stop}
                  </Button>
                ) : (
                  <Button variant="primary" icon={<Play size={14} />} disabled={!source || (dataSource === "file" && !dumpFile) || (dataSource === "ssh" && !(remote.host && remote.name && remote.user))} onClick={() => (dataSource === "ssh" || projectCommands.length > 0 ? setConfirming(true) : start())}>
                    {docker ? t.docker.again : t.docker.start}
                  </Button>
                )}
              </div>
              <label className="docker-option">
                <input type="checkbox" className="checkbox" checked={extensions} disabled={running} onChange={(e) => setExtensions(e.target.checked)} />
                <span>{t.docker.installExtensions}</span>
              </label>
              <label className="docker-option">
                <input type="checkbox" className="checkbox" checked={extensions && cleanImages} disabled={running || !extensions} onChange={(e) => setCleanImages(e.target.checked)} />
                <span>{t.docker.cleanAfter}</span>
              </label>
              <label className="docker-option">
                <input type="checkbox" className="checkbox" checked={comparePages} disabled={running} onChange={(e) => setComparePages(e.target.checked)} />
                <span>{t.docker.comparePages}</span>
              </label>
              {comparePages && (
                <input className="input docker-paths" aria-label={t.docker.pagesLabel} placeholder="/, /login" value={paths} disabled={running} onChange={(e) => setPaths(e.target.value)} />
              )}
              <label className="docker-field">
                <span>{t.docker.dataSource}</span>
                <select className="input" value={dataSource} disabled={running} onChange={(e) => setDataSource(e.target.value)}>
                  <option value="none">{t.docker.dataNone}</option>
                  <option value="file">{t.docker.dataFile}</option>
                  {saved?.dump && !plan?.dumps.includes(saved.dump) && <option value={`dump:${saved.dump}`}>{t.docker.dataDump(saved.dump)}</option>}
                  {plan?.dumps.map((d) => (
                    <option key={d} value={`dump:${d}`}>
                      {t.docker.dataDump(d)}
                    </option>
                  ))}
                  {plan?.live && <option value="live">{t.docker.dataLive(plan.name ?? "", plan.host ?? "")}</option>}
                  {plan?.seed && <option value="seed">{t.docker.dataSeed(plan.seed)}</option>}
                  <option value="ssh">{t.docker.dataSsh}</option>
                </select>
              </label>
              {dataSource === "file" && (
                <Button
                  variant="secondary"
                  size="sm"
                  disabled={running}
                  onClick={() => void api?.pickLogFile(t.docker.dataPick).then((f) => f && setDumpFile(f))}
                >
                  {dumpFile ?? t.docker.dataPick}
                </Button>
              )}
              {dataSource === "live" && <p className="subtle">{t.docker.dataLiveNote}</p>}
              {dataSource === "ssh" && (
                <div className="docker-remote">
                  <input className="input" list="ssh-hosts" aria-label={t.docker.sshHost} placeholder={t.docker.sshHost} value={remote.host} disabled={running} onChange={(e) => setRemote({ ...remote, host: e.target.value })} />
                  <datalist id="ssh-hosts">
                    {hosts.map((h) => (
                      <option key={h} value={h} />
                    ))}
                  </datalist>
                  <select className="input" aria-label={t.docker.sshEngine} value={remote.engine} disabled={running} onChange={(e) => setRemote({ ...remote, engine: e.target.value as "mysql" | "postgres" })}>
                    <option value="mysql">MySQL / MariaDB</option>
                    <option value="postgres">PostgreSQL</option>
                  </select>
                  <input className="input" aria-label={t.docker.sshName} placeholder={plan?.name ?? t.docker.sshName} value={remote.name} disabled={running} onChange={(e) => setRemote({ ...remote, name: e.target.value })} />
                  <input className="input" aria-label={t.docker.sshUser} placeholder={plan?.user ?? t.docker.sshUser} value={remote.user} disabled={running} onChange={(e) => setRemote({ ...remote, user: e.target.value })} />
                  <label className="docker-option">
                    <input type="checkbox" className="checkbox" checked={remote.sendPassword} disabled={running} onChange={(e) => setRemote({ ...remote, sendPassword: e.target.checked })} />
                    <span>{t.docker.sshSendPassword}</span>
                  </label>
                  <p className="subtle">{t.docker.sshNote}</p>
                </div>
              )}
              <label className="docker-field">
                <span>{t.docker.prepare}</span>
                <input className="input docker-paths" placeholder="php bin/console sass:build" value={prepare} disabled={running} onChange={(e) => setPrepare(e.target.value)} />
                {prepare.trim() && <span className="subtle">{t.docker.prepareNote}</span>}
              </label>
              <label className="docker-field">
                <span>{t.docker.testCommand}</span>
                <input className="input docker-paths" placeholder="vendor/bin/pest" value={test} disabled={running} onChange={(e) => setTest(e.target.value)} />
                {test.trim() && <span className="subtle">{t.docker.testCommandNote}</span>}
              </label>
              <div className="docker-save">
                <Button variant="secondary" size="sm" disabled={running} onClick={saveOptions}>
                  {t.docker.saveOptions}
                </Button>
                <span className="subtle">{t.docker.saveOptionsNote}</span>
              </div>
              {stepText && (
                <p className="subtle" role="status">
                  {stepText}
                </p>
              )}
            </Card>
          )}
          {docker && !running && <Result result={docker} onTry={attempt} />}
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
        </div>
      </div>
    </>
  );
}
