import { useState } from "react";
import { CircleAlert, CircleCheck, Container, Play, Square } from "lucide-react";
import { PageHeader } from "../components/Shell";
import { Badge, Button, Callout, Card, EmptyState } from "../components/ui";
import { useI18n } from "../i18n";
import { useStore } from "../lib/store";
import { KNOWN_SOURCES, type DockerComparison, type DockerRun, type Target } from "../lib/types";

const shortPath = (path: string) => path.replace(/^\/app\//, "");

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

function Result({ result }: { result: DockerComparison }) {
  const { t } = useI18n();
  const { showFindings } = useStore();
  const fatal = result.newOnTarget.filter((e) => e.level === "fatal").length;
  const regressed = result.newFailures.length > 0 || (!!result.target.tests && !result.target.tests.passed && !!result.source.tests?.passed);
  const tested = !!result.source.tests?.passed && !!result.target.tests;
  const missing = [...new Set([...result.source.missingExtensions, ...result.target.missingExtensions])];
  const bad = fatal > 0 || regressed || result.target.lintErrors > result.source.lintErrors;
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
  const { project, target, toolchain, docker, dockerStep, runDocker, cancelDocker } = useStore();
  const [picked, setPicked] = useState<string | null>(null);
  const [extensions, setExtensions] = useState(true);
  const [cleanImages, setCleanImages] = useState(false);
  if (!project) return null;
  const sources = KNOWN_SOURCES.filter((v) => Number(v) < Number(target));
  const source = picked ?? project.currentVersion ?? "";
  const running = !!dockerStep;
  const stepText = dockerStep && (dockerStep.step === "install" ? t.docker.installing : dockerStep.step === "extensions" ? t.docker.preparing(dockerStep.version ?? "") : dockerStep.step === "lint" ? t.docker.linting(dockerStep.version ?? "") : t.docker.testing(dockerStep.version ?? ""));
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
                  <Button variant="primary" icon={<Play size={14} />} disabled={!source} onClick={() => void runDocker(source, extensions, extensions && cleanImages)}>
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
              {stepText && (
                <p className="subtle" role="status">
                  {stepText}
                </p>
              )}
            </Card>
          )}
          {docker && !running && <Result result={docker} />}
        </div>
      </div>
    </>
  );
}
