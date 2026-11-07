import { Bot, Check } from "lucide-react";
import { PageHeader } from "../components/Shell";
import { Badge } from "../components/ui";
import { useI18n } from "../i18n";

export function Roadmap() {
  const { t } = useI18n();
  const content = t.roadmap.ai;
  return (
    <>
      <PageHeader title={content.title} />
      <div className="page-scroll">
        <div className="roadmap">
          <div className="roadmap__icon">
            <Bot size={26} />
          </div>
          <Badge tone="accent">{t.common.soon}</Badge>
          <h2>{content.title}</h2>
          <p>{content.text}</p>
          <ul>
            {content.items.map((i) => (
              <li key={i}>
                <Check size={14} />
                {i}
              </li>
            ))}
          </ul>
        </div>
      </div>
    </>
  );
}
