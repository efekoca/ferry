import { useEffect, useState, type ReactNode } from "react";
import { HardDrive, KeyRound } from "lucide-react";
import { usePageTitle } from "../components/Shell";
import { useToast } from "../components/Toast";
import { Badge, Button, Card, Segmented, Select, Switch, TextField } from "../components/ui";
import { useI18n } from "../i18n";
import { errorMessage } from "../lib/api";
import { useStore } from "../lib/store";
import type { AiKind, AiView, Settings } from "../lib/types";

const MODEL_NAMES: Record<string, string> = {
  "claude-opus-5-5": "Claude Opus 5.5",
  "claude-sonnet-5-5": "Claude Sonnet 5.5",
  "claude-haiku-5-5": "Claude Haiku 5.5",
};

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
  const keyStored = view.hasKey && view.kind === kind;
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
            {view && (
              <ProviderForm
                view={view}
                onSaved={(saved) => {
                  setView(saved);
                  void refreshAiProvider();
                }}
              />
            )}
          </Way>
          <Way icon={<HardDrive size={15} />} title={t.ai.localTitle} text={t.ai.localText}>
            <Button disabled>{t.common.soon}</Button>
          </Way>
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
