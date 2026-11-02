import { useEffect, useState } from "react";
import { scoreTone } from "../lib/labels";
import { useI18n } from "../i18n";

const TONE_COLOR = {
  auto: "var(--risk-auto-fg)",
  manual: "var(--risk-manual-fg)",
  danger: "var(--danger-fg)",
} as const;

export function ScoreRing({ score, size = 132, tone: forced }: { score: number; size?: number; tone?: ReturnType<typeof scoreTone> }) {
  const { t } = useI18n();
  const stroke = 11;
  const r = (size - stroke) / 2;
  const circumference = 2 * Math.PI * r;
  const [shown, setShown] = useState(0);
  useEffect(() => {
    const id = requestAnimationFrame(() => setShown(score));
    return () => cancelAnimationFrame(id);
  }, [score]);
  const tone = forced ?? scoreTone(score);

  return (
    <div
      className="score-ring"
      style={{ width: size, height: size }}
      role="img"
      aria-label={t.scoreLabel(score)}
      data-tone={tone}
    >
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} aria-hidden>
        <circle className="score-ring__track" cx={size / 2} cy={size / 2} r={r} fill="none" strokeWidth={stroke} />
        <circle
          className="score-ring__value"
          cx={size / 2}
          cy={size / 2}
          r={r}
          fill="none"
          stroke={TONE_COLOR[tone]}
          strokeWidth={stroke}
          strokeLinecap="round"
          strokeDasharray={circumference}
          strokeDashoffset={circumference * (1 - shown / 100)}
        />
      </svg>
      <div className="score-ring__label" aria-hidden>
        <div>
          <div className="score-ring__number">{score}</div>
          <div className="score-ring__unit">/ 100</div>
        </div>
      </div>
    </div>
  );
}
