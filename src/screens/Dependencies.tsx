import { useState } from "react";
import { CircleAlert, CircleCheck, Package, Search, Server } from "lucide-react";
import { PageHeader } from "../components/Shell";
import { Badge, Button, Callout, Card, CommandLine, EmptyState, Segmented, Switch, TextArea, Tile, Tooltip } from "../components/ui";
import { useToast } from "../components/Toast";
import { useI18n } from "../i18n";
import { errorMessage } from "../lib/api";
import { compareProbe, parseProbe } from "../lib/probe";
import { useStore } from "../lib/store";
import type { Dependency, DependencyReport, DependencyStatus, ExtensionUse, VersionSuggestion, WpLatest } from "../lib/types";

const DEP_ORDER: Record<DependencyStatus, number> = { incompatible: 0, unknown: 1, compatible: 2 };
const DEP_DOT: Record<DependencyStatus, "danger" | "neutral" | "ok"> = { incompatible: "danger", unknown: "neutral", compatible: "ok" };
const COMPATIBLE_SHOWN = 3;

function DepState({ status }: { status: DependencyStatus }) {
  const { t } = useI18n();
  return (
    <span className="dep-state">
      <span className={`dot dot--${DEP_DOT[status]}`} aria-hidden />
      {t.dep[status]}
    </span>
  );
}

function PackageTable({ packages, found }: { packages: Dependency[]; found: VersionSuggestion[] | null }) {
  const { t, loc } = useI18n();
  const [all, setAll] = useState(false);
  const compatible = packages.filter((p) => p.status === "compatible").length;
  const hidden = !all && compatible - COMPATIBLE_SHOWN > 1 ? compatible - COMPATIBLE_SHOWN : 0;
  const suggestionFor = (name: string) => found?.find((s) => s.name === name);
  return (
    <section className="card dep-packages" aria-label={t.deps.packages}>
      <table className="table">
        <thead>
          <tr>
            <th>{t.deps.package}</th>
            <th>{t.deps.version}</th>
            <th>{t.deps.requirement}</th>
            <th className="table__status">{t.deps.status}</th>
          </tr>
        </thead>
        <tbody>
          {packages.slice(0, packages.length - hidden).map((p) => (
            <tr key={p.name}>
              <td>
                <span className="mono selectable">{p.name}</span> {p.dev && <Badge>dev</Badge>}
              </td>
              <td className="mono subtle">{p.version}</td>
              <td>
                <span className="mono">{p.phpConstraint ?? <span className="subtle">{t.deps.unspecified}</span>}</span>
                {p.note && <div className="dep-note">{loc(p.note)}</div>}
                {found && p.status === "incompatible" && (
                  <div className="dep-note">
                    {suggestionFor(p.name) ? (
                      <>
                        {t.deps.compatibleFrom(suggestionFor(p.name)!.version)}
                        {suggestionFor(p.name)!.majorChange && <Badge tone="manual">{t.deps.majorChange}</Badge>}
                      </>
                    ) : (
                      t.deps.noneFound
                    )}
                  </div>
                )}
              </td>
              <td>
                <DepState status={p.status} />
              </td>
            </tr>
          ))}
          {hidden > 0 && (
            <tr>
              <td colSpan={4} className="dep-more">
                <Button variant="ghost" size="sm" onClick={() => setAll(true)}>
                  {t.deps.moreCompatible(hidden)}
                </Button>
              </td>
            </tr>
          )}
        </tbody>
      </table>
    </section>
  );
}

function WordPressCard({ deps }: { deps: DependencyReport }) {
  const { t } = useI18n();
  const { scan, api } = useStore();
  const toast = useToast();
  const [latest, setLatest] = useState<{ for: DependencyReport; list: WpLatest[] } | null>(null);
  const [checking, setChecking] = useState(false);
  const wpLatest = latest?.for === deps ? latest.list : null;
  const breakingIn = (dir: string) => scan?.findings.filter((f) => f.severity === "breaking" && f.file.startsWith(`${dir}/`)).length ?? 0;
  const check = async () => {
    if (!api) return;
    setChecking(true);
    try {
      setLatest({ for: deps, list: await api.wordpressLatest(deps.wordpress.map((c) => ({ kind: c.kind, slug: c.slug }))) });
    } catch (e) {
      toast.push({ tone: "error", title: t.deps.wpFailed, description: errorMessage(e, t.common.genericError) });
    } finally {
      setChecking(false);
    }
  };
  return (
    <Card
      title={t.deps.wpTitle}
      bodyClassName=""
      actions={
        !wpLatest &&
        api && (
          <Tooltip text={t.deps.wpCheckNote} align="end">
            <Button size="sm" icon={<Search size={13} />} loading={checking} onClick={check}>
              {t.deps.wpCheck}
            </Button>
          </Tooltip>
        )
      }
    >
      <table className="table dep-table" aria-label={t.deps.wpTitle}>
        <thead>
          <tr>
            <th>{t.deps.wpName}</th>
            <th>{t.deps.wpInstalled}</th>
            <th className="table__num">{t.deps.wpBreaking}</th>
            {wpLatest && <th>{t.deps.wpLatest}</th>}
          </tr>
        </thead>
        <tbody>
          {deps.wordpress.map((c) => {
            const l = wpLatest?.find((x) => x.kind === c.kind && x.slug === c.slug);
            const breaking = breakingIn(c.dir);
            return (
              <tr key={`${c.kind}:${c.slug}`}>
                <td>
                  {c.name} <Badge>{c.kind === "plugin" ? t.deps.wpPlugin : t.deps.wpTheme}</Badge>
                  <div className="dep-note mono">{c.dir}</div>
                </td>
                <td>
                  <span className="mono">{c.version ?? "—"}</span>
                  {c.requiresPhp && <div className="dep-note">{t.deps.wpRequires(c.requiresPhp)}</div>}
                </td>
                <td className="table__num tabular">{breaking > 0 ? <Badge tone="danger">{breaking}</Badge> : <span className="subtle">0</span>}</td>
                {wpLatest && (
                  <td>
                    {!l ? (
                      <span className="subtle">{t.deps.wpNotListed}</span>
                    ) : l.closed ? (
                      <Badge tone="danger">{t.deps.wpClosed}</Badge>
                    ) : (
                      <>
                        <span className="mono">{l.version}</span>
                        {l.requiresPhp && <div className="dep-note">{t.deps.wpRequires(l.requiresPhp)}</div>}
                      </>
                    )}
                  </td>
                )}
              </tr>
            );
          })}
        </tbody>
      </table>
    </Card>
  );
}

function PackagesView({ deps, prodOnly }: { deps: DependencyReport; prodOnly: boolean }) {
  const { t } = useI18n();
  const { api } = useStore();
  const toast = useToast();
  const [suggestions, setSuggestions] = useState<{ for: DependencyReport; list: VersionSuggestion[] } | null>(null);
  const [looking, setLooking] = useState(false);

  const packages = deps.packages
    .filter((p) => !prodOnly || !p.dev)
    .sort((a, b) => DEP_ORDER[a.status] - DEP_ORDER[b.status] || Number(a.dev) - Number(b.dev) || a.name.localeCompare(b.name));
  const incompatible = packages.filter((p) => p.status === "incompatible");
  const bad = incompatible.length;
  const unknown = packages.filter((p) => p.status === "unknown").length;
  const good = packages.length - bad - unknown;
  const command = `composer update ${incompatible.map((p) => p.name).join(" ")} --with-all-dependencies`;
  const found = suggestions?.for === deps ? suggestions.list : null;
  const findVersions = async () => {
    if (!api) return;
    setLooking(true);
    try {
      const list = await api.findCompatibleVersions(deps.target, incompatible.map((p) => ({ name: p.name, version: p.version })));
      setSuggestions({ for: deps, list });
    } catch (e) {
      toast.push({ tone: "error", title: t.deps.findFailed, description: errorMessage(e, t.common.genericError) });
    } finally {
      setLooking(false);
    }
  };
  const rootBlocked = deps.rootStatus === "incompatible";
  const rootHint = [deps.rootConstraint && rootBlocked && t.deps.rootBlockedText(deps.rootConstraint, deps.target), deps.rootConflict && t.deps.rootConflictText(deps.rootConflict, deps.target)].filter(Boolean).join(" ");

  return (
    <>
      {rootBlocked && !deps.rootConstraint && (
        <Callout tone="danger" icon={<CircleAlert size={16} />}>
          <strong>{t.deps.rootBlockedTitle(deps.target)}</strong> {rootHint}
        </Callout>
      )}
      {deps.manifestUnreadable && (
        <Callout tone="warn" icon={<CircleAlert size={16} />}>
          {t.deps.manifestUnreadable}
        </Callout>
      )}
      {deps.lockUnreadable ? (
        <Callout tone="warn" icon={<CircleAlert size={16} />}>
          {t.deps.lockUnreadable}
        </Callout>
      ) : !deps.hasLockFile ? (
        <Callout tone="warn" icon={<CircleAlert size={16} />}>
          {t.deps.noLock}
        </Callout>
      ) : bad === 0 && !rootBlocked && unknown > 0 ? (
        <Callout tone="warn" icon={<CircleAlert size={16} />}>
          {t.deps.someUnknown(unknown, deps.target)}
        </Callout>
      ) : bad === 0 && !rootBlocked && !deps.manifestUnreadable ? (
        <Callout tone="info" icon={<CircleCheck size={16} />}>
          {t.deps.allGood(deps.target)}
        </Callout>
      ) : null}

      {(packages.length > 0 || deps.rootConstraint) && (
        <dl className="card tiles">
          {packages.length > 0 && (
            <>
              <Tile label={t.deps.tileBad} tone={bad > 0 ? "danger" : "neutral"}>
                {bad}
              </Tile>
              <Tile label={t.deps.tileUnknown} tone="neutral">
                {unknown}
              </Tile>
              <Tile label={t.deps.tileGood} tone={good > 0 ? "ok" : "neutral"}>
                {good}
              </Tile>
            </>
          )}
          {deps.rootConstraint && (
            <Tile label={t.deps.rootTile} tone={DEP_DOT[deps.rootStatus]}>
              {rootHint ? (
                <Tooltip text={rootHint} align="end">
                  <code className="tile__code" tabIndex={0}>
                    {deps.rootConstraint}
                  </code>
                </Tooltip>
              ) : (
                <code className="tile__code">{deps.rootConstraint}</code>
              )}
            </Tile>
          )}
        </dl>
      )}

      {bad > 0 && (
        <Card
          title={t.deps.oneCommand}
          actions={
            !found &&
            api && (
              <Tooltip text={t.deps.findVersionsNote} align="end">
                <Button size="sm" icon={<Search size={13} />} loading={looking} onClick={findVersions}>
                  {t.deps.findVersions}
                </Button>
              </Tooltip>
            )
          }
        >
          <CommandLine command={command} />
        </Card>
      )}

      {packages.length > 0 && <PackageTable packages={packages} found={found} />}
      {deps.wordpress.length > 0 && <WordPressCard deps={deps} />}
    </>
  );
}

function ExtensionsView({ deps }: { deps: DependencyReport }) {
  const { t } = useI18n();
  const undeclared = deps.extensions.filter((e) => !e.declared && !e.removed).map((e) => `"ext-${e.name}": "*"`);
  return (
    <>
      {deps.extensions.length > 0 && (
        <section className="card dep-packages" aria-label={t.deps.extensions}>
          <table className="table" aria-label={t.deps.extensions}>
            <thead>
              <tr>
                <th>{t.deps.extension}</th>
                <th>{t.deps.usedAt}</th>
                <th className="table__num">{t.deps.uses}</th>
                <th>{t.deps.declared}</th>
                <th>{t.deps.local}</th>
              </tr>
            </thead>
            <tbody>
              {deps.extensions.map((e) => (
                <tr key={e.name}>
                  <td>
                    <span className="mono selectable">{e.name}</span> {e.removed && <Badge tone="danger">{t.deps.removedExt}</Badge>}
                  </td>
                  <td>
                    {e.uses > 0 ? (
                      <>
                        <span className="mono">{e.symbol}</span>
                        <div className="dep-note mono">
                          {e.file}:{e.line}
                        </div>
                      </>
                    ) : (
                      <span className="subtle">{t.deps.requiredBy((e.requiredBy ?? []).join(", "))}</span>
                    )}
                    {e.dev && <Badge tone="neutral">{t.deps.devOnly}</Badge>}
                  </td>
                  <td className="table__num tabular">{e.uses > 0 ? e.uses : "—"}</td>
                  <td>{e.declared ? t.deps.yes : <span className="subtle">{t.deps.no}</span>}</td>
                  <td>{e.available === null ? "—" : e.available ? t.deps.yes : <Badge tone="danger">{t.deps.no}</Badge>}</td>
                </tr>
              ))}
            </tbody>
          </table>
          {undeclared.length > 0 && <p className="dep-hint">{t.deps.addExt(undeclared.join(", "))}</p>}
        </section>
      )}
      <ServerProbe target={deps.target} extensions={deps.extensions} />
    </>
  );
}

export function Dependencies() {
  const { t } = useI18n();
  const { deps, scan, runScan, scanning, target } = useStore();
  const [view, setView] = useState<"packages" | "extensions">("packages");
  const [prodOnly, setProdOnly] = useState(false);

  if (!deps) {
    return (
      <>
        <PageHeader title={t.deps.title} />
        <EmptyState
          icon={<Package size={24} />}
          title={t.deps.notChecked}
          action={
            !scan && (
              <Button variant="primary" onClick={() => runScan()} loading={scanning}>
                {t.overview.startScan}
              </Button>
            )
          }
        >
          {t.deps.notCheckedText(target)}
        </EmptyState>
      </>
    );
  }

  return (
    <>
      <PageHeader
        title={t.deps.title}
        subtitle={`PHP ${deps.target}`}
        actions={
          <>
            {view === "packages" && deps.packages.some((p) => p.dev) && (
              <label className="dep-only">
                <Switch checked={prodOnly} onChange={(e) => setProdOnly(e.target.checked)} />
                {t.deps.productionOnly}
              </label>
            )}
            <Segmented
              size="sm"
              label={t.deps.view}
              value={view}
              onChange={setView}
              options={[
                {
                  value: "packages",
                  label: (
                    <>
                      {t.deps.packages} <span className="segmented__count">{deps.packages.length}</span>
                    </>
                  ),
                },
                {
                  value: "extensions",
                  label: (
                    <>
                      {t.deps.extensions} <span className="segmented__count">{deps.extensions.length}</span>
                    </>
                  ),
                },
              ]}
            />
          </>
        }
      />
      <div className="page-scroll">
        <div className="page-body stack">{view === "packages" ? <PackagesView deps={deps} prodOnly={prodOnly} /> : <ExtensionsView deps={deps} />}</div>
      </div>
    </>
  );
}

function ServerProbe({ target, extensions }: { target: string; extensions: ExtensionUse[] }) {
  const { t } = useI18n();
  const { api, project } = useStore();
  const toast = useToast();
  const key = project ? `ferry.probe:${project.root}` : null;
  const [token, setToken] = useState<string | null>(null);
  const [text, setText] = useState(() => {
    try {
      return (key && localStorage.getItem(key)) || "";
    } catch {
      return "";
    }
  });
  const probe = text.trim() ? parseProbe(text) : null;
  const report = probe && compareProbe(probe, target, extensions);
  const remember = (value: string) => {
    setText(value);
    try {
      if (key) localStorage.setItem(key, value);
    } catch {
      /* storage can be unavailable */
    }
  };
  const create = async () => {
    if (!api) return;
    try {
      const dest = await api.pickProbeFile("ferry-probe.php");
      if (!dest) return;
      setToken(await api.writeProbe(dest));
    } catch (e) {
      toast.push({ tone: "error", title: t.deps.probeFailed, description: errorMessage(e, t.common.genericError) });
    }
  };
  return (
    <Card title={t.deps.probeTitle} bodyClassName="card__body stack">
      <p className="muted">{t.deps.probeText}</p>
      <div>
        <Button size="sm" variant="secondary" icon={<Server size={13} />} onClick={create}>
          {t.deps.probeCreate}
        </Button>
      </div>
      {token && (
        <div className="stack stack--tight">
          <span className="subtle">{t.deps.probeOpen}</span>
          <CommandLine command={`https://example.com/ferry-probe.php?token=${token}`} />
        </div>
      )}
      <TextArea className="field" label={t.deps.probePaste} mono rows={3} value={text} placeholder={t.deps.probePlaceholder} onChange={(e) => remember(e.target.value)} />
      {text.trim() && !probe && <Callout tone="warn" icon={<CircleAlert size={15} />}>{t.deps.probeInvalid}</Callout>}
      {report && (
        <>
          <table className="table" aria-label={t.deps.probeTitle}>
            <tbody>
              <tr>
                <th scope="row">{t.deps.probePhp}</th>
                <td>
                  <span className="mono">{report.php}</span>{" "}
                  {report.belowTarget && <Badge tone="danger">{t.deps.probeBelow(target)}</Badge>}{" "}
                  {report.vulnerable && <Badge tone="danger">{t.deps.probeVulnerable}</Badge>}
                </td>
              </tr>
              <tr>
                <th scope="row">{t.deps.probeSapi}</th>
                <td>
                  <span className="mono">{report.sapi}</span>
                  {report.server && <span className="subtle"> · {report.server}</span>}
                  {report.fpm && <div className="dep-note">{t.deps.probeFpm}</div>}
                </td>
              </tr>
              <tr>
                <th scope="row">{t.deps.probeMissing}</th>
                <td>
                  {report.missing.length ? (
                    <span className="row row--wrap">
                      {report.missing.map((m) => (
                        <Badge key={m} tone="danger">
                          {m}
                        </Badge>
                      ))}
                    </span>
                  ) : (
                    <span className="subtle">{t.deps.probeNone}</span>
                  )}
                </td>
              </tr>
              <tr>
                <th scope="row">{t.deps.probeDisabled}</th>
                <td className="mono">{report.disabled.length ? report.disabled.join(", ") : <span className="subtle">{t.deps.probeNone}</span>}</td>
              </tr>
              <tr>
                <th scope="row">{t.deps.probeMemory}</th>
                <td className="mono">{report.memoryLimit ?? "—"}</td>
              </tr>
            </tbody>
          </table>
          <div>
            <Button size="sm" variant="ghost" onClick={() => remember("")}>
              {t.deps.probeClear}
            </Button>
          </div>
        </>
      )}
    </Card>
  );
}
