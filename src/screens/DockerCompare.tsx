import { useEffect, useState } from "react";
import { CircleAlert, CircleCheck, Container, Play, ShieldAlert, Square } from "lucide-react";
import { ConfirmDialog } from "../components/Modal";
import { PageHeader } from "../components/Shell";
import { Badge, Button, Callout, Card, EmptyState } from "../components/ui";
import { useI18n } from "../i18n";
import { useStore } from "../lib/store";
import { KNOWN_SOURCES, type DatabaseChoice, type DatabasePlan, type DockerComparison, type DockerPage, type DockerRun, type Target } from "../lib/types";

const shortPath = (path: string) => path.replace(/^\/app\//, "");
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

function Result({ result }: { result: DockerComparison }) {
  const { t } = useI18n();
  const { showFindings } = useStore();
  const fatal = result.newOnTarget.filter((e) => e.level === "fatal").length;
  const regressed = result.newFailures.length > 0 || (!!result.target.tests && !result.target.tests.passed && !!result.source.tests?.passed);
  const tested = !!result.source.tests?.passed && !!result.target.tests;
  const missing = [...new Set([...result.source.missingExtensions, ...result.target.missingExtensions])];
  const brokenPages = result.pages.some((p) => p.change === "status" || p.change === "errors" || p.change === "missing");
  const bad = fatal > 0 || regressed || brokenPages || result.target.lintErrors > result.source.lintErrors;
  return (
    <>
      <Callout tone={bad ? "danger" : tested ? "success" : "warn"} icon={bad ? <CircleAlert size={15} /> : <CircleCheck size={15} />}>
        {bad ? t.docker.worse(result.target.version) : tested ? t.docker.same(result.target.version) : t.docker.notTested}
      </Callout>
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
  const { api, project, target, toolchain, docker, dockerStep, runDocker, cancelDocker } = useStore();
  const [plan, setPlan] = useState<DatabasePlan | null>(null);
  const [dataSource, setDataSource] = useState("none");
  const [dumpFile, setDumpFile] = useState<string | null>(null);
  const [hosts, setHosts] = useState<string[]>([]);
  const [remote, setRemote] = useState({ host: "", engine: "mysql" as "mysql" | "postgres", name: "", user: "" });
  const [confirming, setConfirming] = useState(false);
  const root = project?.root;
  useEffect(() => {
    if (root) api?.databasePlan(root).then(setPlan).catch(() => setPlan(null));
    api?.sshHosts().then(setHosts).catch(() => {});
  }, [api, root]);
  const [picked, setPicked] = useState<string | null>(null);
  const [extensions, setExtensions] = useState(true);
  const [cleanImages, setCleanImages] = useState(false);
  const [comparePages, setComparePages] = useState(true);
  const [paths, setPaths] = useState("/");
  const [prepare, setPrepare] = useState("");
  if (!project) return null;
  const sources = KNOWN_SOURCES.filter((v) => Number(v) < Number(target));
  const source = picked ?? project.currentVersion ?? "";
  const running = !!dockerStep;
  const start = () => {
    setConfirming(false);
    const database = dataSource === "ssh" ? ({ kind: "ssh", ...remote } as const) : choice(dataSource, dumpFile, project.root);
    void runDocker(source, extensions, extensions && cleanImages, comparePages ? parsePaths(paths) : [], database, prepare.trim() || null);
  };
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
                  <Button variant="primary" icon={<Play size={14} />} disabled={!source || (dataSource === "file" && !dumpFile) || (dataSource === "ssh" && !(remote.host && remote.name && remote.user))} onClick={() => (dataSource === "ssh" ? setConfirming(true) : start())}>
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
                  <p className="subtle">{t.docker.sshNote}</p>
                </div>
              )}
              <label className="docker-field">
                <span>{t.docker.prepare}</span>
                <input className="input docker-paths" placeholder="php bin/console sass:build" value={prepare} disabled={running} onChange={(e) => setPrepare(e.target.value)} />
                {prepare.trim() && <span className="subtle">{t.docker.prepareNote}</span>}
              </label>
              {stepText && (
                <p className="subtle" role="status">
                  {stepText}
                </p>
              )}
            </Card>
          )}
          {docker && !running && <Result result={docker} />}
          <ConfirmDialog
            open={confirming}
            tone="danger"
            icon={<ShieldAlert size={20} />}
            title={t.docker.sshConfirmTitle}
            confirmLabel={t.docker.sshConfirm}
            onCancel={() => setConfirming(false)}
            onConfirm={start}
          >
            {t.docker.sshConfirmText(remote.host, remote.name, remote.user)}
          </ConfirmDialog>
        </div>
      </div>
    </>
  );
}
