import { useEffect, useState } from "react";
import { CircleAlert, Info } from "lucide-react";
import { DiffViewer } from "./DiffViewer";
import { Sheet } from "./Modal";
import { Button, Callout } from "./ui";
import { useI18n } from "../i18n";
import { errorMessage } from "../lib/api";
import { useStore } from "../lib/store";
import type { AiPrompt, AiSuggestion, Finding, Settings } from "../lib/types";

type Step =
  | { kind: "loading" }
  | { kind: "consent"; prompt: AiPrompt; settings: Settings }
  | { kind: "review"; prompt: AiPrompt }
  | { kind: "asking"; prompt: AiPrompt }
  | { kind: "result"; prompt: AiPrompt; suggestion: AiSuggestion }
  | { kind: "failed"; prompt: AiPrompt | null; message: string };

/** Asks the provider only after the user has seen exactly what goes out, and writes only what they apply. */
export function AiSuggest({ finding, onClose }: { finding: Finding | null; onClose: () => void }) {
  const { t, loc } = useI18n();
  const { api, project, aiProvider, applyAiSuggestion, applying } = useStore();
  const [step, setStep] = useState<Step>({ kind: "loading" });
  const root = project?.root;
  const provider = aiProvider ?? "";

  useEffect(() => {
    if (!finding || !api || !root) return;
    let live = true;
    setStep({ kind: "loading" });
    Promise.all([api.aiPrompt(root, finding.id), api.getSettings()])
      .then(([prompt, settings]) => {
        if (live) setStep(settings.aiProjects.includes(root) ? { kind: "review", prompt } : { kind: "consent", prompt, settings });
      })
      .catch((e) => live && setStep({ kind: "failed", prompt: null, message: errorMessage(e, t.common.genericError) }));
    return () => {
      live = false;
    };
  }, [finding, api, root, t]);

  if (!finding || !api || !root) return null;
  const failed = (prompt: AiPrompt | null, e: unknown) => setStep({ kind: "failed", prompt, message: errorMessage(e, t.common.genericError) });

  const enable = async (prompt: AiPrompt, settings: Settings) => {
    try {
      await api.saveSettings({ ...settings, aiProjects: [...settings.aiProjects, root] });
      setStep({ kind: "review", prompt });
    } catch (e) {
      failed(prompt, e);
    }
  };
  const send = async (prompt: AiPrompt) => {
    setStep({ kind: "asking", prompt });
    try {
      setStep({ kind: "result", prompt, suggestion: await api.aiSuggest(root, finding.id) });
    } catch (e) {
      failed(prompt, e);
    }
  };
  const apply = async (suggestion: AiSuggestion) => {
    if (await applyAiSuggestion(suggestion.id, t.ai.label(loc(finding.title)))) onClose();
  };

  const prompt = step.kind === "loading" ? null : step.prompt;
  const meta = prompt && t.ai.sendsTo(prompt.file, prompt.firstLine, prompt.lastLine, provider);
  const footer =
    step.kind === "consent" ? (
      <Button variant="primary" onClick={() => void enable(step.prompt, step.settings)}>
        {t.ai.enable}
      </Button>
    ) : step.kind === "review" ? (
      <Button variant="primary" onClick={() => void send(step.prompt)}>
        {t.ai.send}
      </Button>
    ) : step.kind === "asking" ? (
      <Button onClick={() => void api.cancelAi(root)}>{t.ai.stop}</Button>
    ) : step.kind === "result" ? (
      <>
        <Button variant="primary" loading={applying} disabled={step.suggestion.lintErrors.length > 0} onClick={() => void apply(step.suggestion)}>
          {t.common.apply}
        </Button>
        <Button onClick={() => void send(step.prompt)}>{t.ai.again}</Button>
      </>
    ) : step.kind === "failed" && step.prompt ? (
      <Button onClick={() => void send(step.prompt as AiPrompt)}>{t.common.tryAgain}</Button>
    ) : null;

  return (
    <Sheet open title={t.ai.sheetTitle} meta={meta} footer={footer} onClose={onClose}>
      <div className="ai-suggest">
        {step.kind === "loading" && <p className="subtle">{t.ai.preparing}</p>}
        {step.kind === "consent" && (
          <Callout tone="info" icon={<Info size={15} />}>
            <strong>{t.ai.consentTitle}</strong> {t.ai.consentText(provider)}
          </Callout>
        )}
        {(step.kind === "consent" || step.kind === "review") && (
          <>
            <p className="hint">{t.ai.sendsNote}</p>
            <pre className="code ai-prompt selectable">{step.prompt.user}</pre>
          </>
        )}
        {step.kind === "asking" && <p className="subtle">{t.ai.asking(provider)}</p>}
        {step.kind === "result" && (
          <>
            {step.suggestion.lintErrors.length > 0 ? (
              <Callout tone="danger" icon={<CircleAlert size={15} />}>
                {t.ai.lintErrors}
                <pre className="code selectable">{step.suggestion.lintErrors.join("\n")}</pre>
              </Callout>
            ) : (
              !step.suggestion.lintChecked && <p className="hint">{t.ai.lintSkipped}</p>
            )}
            <DiffViewer diff={step.suggestion.change.diff} file={step.suggestion.change.file} />
            <p className="hint">{t.ai.review}</p>
          </>
        )}
        {step.kind === "failed" && (
          <Callout tone="danger" icon={<CircleAlert size={15} />}>
            <strong>{t.ai.failed}</strong> <span className="selectable">{step.message}</span>
          </Callout>
        )}
      </div>
    </Sheet>
  );
}
