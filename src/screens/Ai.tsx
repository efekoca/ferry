import { useCallback, useEffect, useState, type ReactNode } from "react";
import { HardDrive, KeyRound } from "lucide-react";
import { usePageTitle } from "../components/Shell";
import { useToast } from "../components/Toast";
import { Badge, Button, Card, Segmented, Select, Switch, TextField } from "../components/ui";
import { useI18n } from "../i18n";
import { errorMessage } from "../lib/api";
import { useStore } from "../lib/store";
import type { AiKind, AiView, LocalAiServer, Settings } from "../lib/types";

const MODEL_NAMES: Record<string, string> = {
  "claude-opus-5-5": "Claude Opus 5.5",
  "claude-sonnet-5-5": "Claude Sonnet 5.5",
  "claude-haiku-5-5": "Claude Haiku 5.5",
};

function origin(url: string | null | undefined) {
  try {
    return new URL((url ?? "").trim()).origin;
  } catch {
    return (url ?? "").trim();
  }
}

function Way({ icon, title, text, children }: { icon: ReactNode; title: string; text: string; children: ReactNode }) {
  return (
    <section className="card ai-way">
      <span className="badge-icon badge-icon--accent" aria-hidden>
        {icon}
      </span>
      <div className="ai-way__text">
        <h3>{title}</h3>
        <p>{text}</p>
      </div>
      {children}
    </section>
  );
}

function ProviderForm({ view, onSaved }: { view: AiView; onSaved: (view: AiView) => void }) {
  const { t } = useI18n();
  const { api } = useStore();
  const toast = useToast();
  const [kind, setKind] = useState<AiKind>(view.kind ?? "anthropic");
  const [model, setModel] = useState(view.model ?? view.models[0]);
  const [baseUrl, setBaseUrl] = useState(view.baseUrl ?? "");
  const [key, setKey] = useState("");
  const [busy, setBusy] = useState<"save" | "check" | null>(null);
  const keyStored = view.hasKey && view.kind === kind && (kind === "anthropic" || origin(baseUrl) === origin(view.baseUrl));
  if (!api) return null;

  const pick = (next: AiKind) => {
    setKind(next);
    setModel(next === "anthropic" ? view.models[0] : "");
  };
  const save = async (newKey: string | null) => {
    setBusy("save");
    try {
      const saved = await api.saveAiConfig({ kind, model: model || null, baseUrl: kind === "openai" ? baseUrl : null }, newKey);
      setKey("");
      onSaved(saved);
      toast.push({ tone: "success", title: t.ai.saved });
    } catch (e) {
      toast.push({ tone: "error", title: t.ai.saveFailed, description: errorMessage(e, t.common.genericError) });
    } finally {
      setBusy(null);
    }
  };
  const check = async () => {
    setBusy("check");
    try {
      await api.checkAiConfig();
      toast.push({ tone: "success", title: t.ai.checkPassed });
    } catch (e) {
      toast.push({ tone: "error", title: t.ai.checkFailed, description: errorMessage(e, t.common.genericError) });
    } finally {
      setBusy(null);
    }
  };

  return (
    <div className="ai-form">
      <Segmented<AiKind>
        size="sm"
        label={t.ai.provider}
        options={[
          { value: "anthropic", label: "Anthropic" },
          { value: "openai", label: t.ai.openai },
        ]}
        value={kind}
        onChange={pick}
      />
      {kind === "anthropic" ? (
        <Select label={t.ai.model} value={model} options={view.models.map((m) => ({ value: m, label: MODEL_NAMES[m] ?? m }))} onChange={setModel} block />
      ) : (
        <>
          <TextField label={t.ai.address} value={baseUrl} placeholder="https://api.openai.com/v1" mono onChange={(e) => setBaseUrl(e.target.value)} />
          <p className="hint">{t.ai.addressHint}</p>
          <TextField label={t.ai.model} value={model} placeholder="gpt-5" mono onChange={(e) => setModel(e.target.value)} />
        </>
      )}
      {keyStored ? (
        <div className="ai-form__key">
          <span>{t.ai.keyStored}</span>
          <Button size="sm" variant="ghost" className="danger-text" disabled={!!busy} onClick={() => void save("")}>
            {t.ai.keyDelete}
          </Button>
        </div>
      ) : (
        <TextField label={kind === "openai" ? t.ai.keyOptional : t.ai.key} type="password" autoComplete="off" value={key} onChange={(e) => setKey(e.target.value)} />
      )}
      <div className="ai-form__actions">
        <Button variant="primary" loading={busy === "save"} disabled={!!busy || !model} onClick={() => void save(key ? key : null)}>
          {t.ai.save}
        </Button>
        <Button loading={busy === "check"} disabled={!!busy || !view.kind} onClick={() => void check()}>
          {t.ai.check}
        </Button>
      </div>
    </div>
  );
}

function LocalWay({ view, onSaved }: { view: AiView; onSaved: (view: AiView) => void }) {
  const { t } = useI18n();
  const { api } = useStore();
  const toast = useToast();
  const [servers, setServers] = useState<LocalAiServer[] | null>(null);
  const [choice, setChoice] = useState("");
  const [busy, setBusy] = useState(false);
  const search = useCallback(async () => {
    if (!api) return;
    setServers(null);
    setServers(await api.localAiServers().catch(() => []));
  }, [api]);
  useEffect(() => {
    void search();
  }, [search]);

  const running = servers?.filter((s) => s.models.length > 0) ?? [];
  const idle = servers?.filter((s) => s.models.length === 0) ?? [];
  const options = running.flatMap((s) => s.models.map((m) => ({ value: `${s.baseUrl}\n${m}`, label: running.length > 1 ? `${s.name} · ${m}` : m })));
  const inUse = (value: string) => {
    const [baseUrl, model] = value.split("\n");
    return view.kind === "openai" && origin(view.baseUrl) === origin(baseUrl) && view.model === model;
  };
  const selected = choice || options.find((o) => inUse(o.value))?.value || options[0]?.value || "";
  const text = running.length > 0 ? t.ai.localFound(running.map((s) => s.name)) : idle.length > 0 ? t.ai.localNoModels(idle.map((s) => s.name)) : t.ai.localText;

  const use = async () => {
    if (!api || !selected) return;
    const [baseUrl, model] = selected.split("\n");
    setBusy(true);
    try {
      onSaved(await api.saveAiConfig({ kind: "openai", model, baseUrl }, null));
      toast.push({ tone: "success", title: t.ai.saved });
    } catch (e) {
      toast.push({ tone: "error", title: t.ai.saveFailed, description: errorMessage(e, t.common.genericError) });
    } finally {
      setBusy(false);
    }
  };

  return (
    <Way icon={<HardDrive size={15} />} title={t.ai.localTitle} text={text}>
      {running.length > 0 ? (
        <div className="ai-form">
          <Select label={t.ai.model} value={selected} options={options} onChange={setChoice} block />
          <div className="ai-form__actions">
            <Button loading={busy} disabled={busy || inUse(selected)} onClick={() => void use()}>
              {inUse(selected) ? t.ai.localInUse : t.ai.localUse}
            </Button>
          </div>
        </div>
      ) : (
        <Button loading={servers === null} onClick={() => void search()}>
          {t.ai.localSearch}
        </Button>
      )}
    </Way>
  );
}

export function Ai() {
  const { t } = useI18n();
  const { api, scan, project, aiProvider, refreshAiProvider } = useStore();
  const toast = useToast();
  usePageTitle(t.ai.title);
  const [view, setView] = useState<AiView | null>(null);
  const [settings, setSettings] = useState<Settings | null>(null);
  const manual = scan?.summary.manual ?? 0;

  useEffect(() => {
    if (!api) return;
    void api.aiConfig().then(setView).catch(() => setView(null));
    void api.getSettings().then(setSettings).catch(() => setSettings(null));
  }, [api]);

  const saved = (next: AiView) => {
    setView(next);
    void refreshAiProvider();
  };
  const allowed = !!project && !!settings?.aiProjects.includes(project.root);
  const allow = async (on: boolean) => {
    if (!api || !project || !settings) return;
    const aiProjects = on ? [...settings.aiProjects, project.root] : settings.aiProjects.filter((p) => p !== project.root);
    try {
      await api.saveSettings({ ...settings, aiProjects });
      setSettings({ ...settings, aiProjects });
    } catch (e) {
      toast.push({ tone: "error", title: t.toast.saveFailed, description: errorMessage(e, t.common.genericError) });
    }
  };

  return (
    <div className="page-scroll real-page">
      <h1 className="sr-only">{t.ai.title}</h1>
      <div className="overview-start ai-page" data-tauri-drag-region>
        <div className="ai-page__head">
          {aiProvider && <Badge tone="accent">{t.ai.active(aiProvider)}</Badge>}
          <h2 className="page-title">{manual > 0 ? t.ai.headline(manual) : t.ai.headlineEmpty}</h2>
          <p className="page-lead">{t.ai.lead}</p>
        </div>
        <div className="ai-ways">
          <Way icon={<KeyRound size={15} />} title={t.ai.keyTitle} text={t.ai.keyText}>
            {view && <ProviderForm key={[view.kind, view.model, view.baseUrl, view.hasKey].join("\n")} view={view} onSaved={saved} />}
          </Way>
          {view && <LocalWay view={view} onSaved={saved} />}
        </div>
        {project && settings && (
          <Card className="ai-project" bodyClassName="lines-card">
            <div className="lines">
              <label className="line setting-toggle">
                <span className="line__main">
                  <span>{t.ai.projectToggle(project.name)}</span>
                  <span className="line__sub">{t.ai.projectText}</span>
                </span>
                <Switch checked={allowed} onChange={(e) => void allow(e.target.checked)} />
              </label>
            </div>
          </Card>
        )}
      </div>
    </div>
  );
}
