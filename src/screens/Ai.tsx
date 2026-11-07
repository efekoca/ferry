import type { ReactNode } from "react";
import { HardDrive, KeyRound } from "lucide-react";
import { usePageTitle } from "../components/Shell";
import { Badge, Button } from "../components/ui";
import { useI18n } from "../i18n";
import { useStore } from "../lib/store";

function Way({ icon, title, text, action }: { icon: ReactNode; title: string; text: string; action: string }) {
  return (
    <section className="card ai-way">
      <span className="badge-icon badge-icon--accent" aria-hidden>
        {icon}
      </span>
      <div className="ai-way__text">
        <h3>{title}</h3>
        <p>{text}</p>
      </div>
      <Button disabled>{action}</Button>
    </section>
  );
}

export function Ai() {
  const { t } = useI18n();
  const { scan } = useStore();
  usePageTitle(t.ai.title);
  const manual = scan?.summary.manual ?? 0;
  return (
    <div className="page-scroll real-page">
      <h1 className="sr-only">{t.ai.title}</h1>
      <div className="overview-start ai-page" data-tauri-drag-region>
        <div className="ai-page__head">
          <Badge tone="accent">{t.common.soon}</Badge>
          <h2 className="page-title">{manual > 0 ? t.ai.headline(manual) : t.ai.headlineEmpty}</h2>
          <p className="page-lead">{t.ai.lead}</p>
        </div>
        <div className="ai-ways">
          <Way icon={<KeyRound size={15} />} title={t.ai.keyTitle} text={t.ai.keyText} action={t.ai.keyAction} />
          <Way icon={<HardDrive size={15} />} title={t.ai.localTitle} text={t.ai.localText} action={t.ai.localAction} />
        </div>
      </div>
    </div>
  );
}
