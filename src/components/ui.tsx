import { useEffect, useState, type ButtonHTMLAttributes, type ReactNode, type Ref } from "react";
import { AlertOctagon, Check, CircleHelp, Copy, ShieldCheck, Sparkles, TriangleAlert, Wrench } from "lucide-react";
import { useI18n } from "../i18n";
import type { AppliedRule, FixKind, Severity } from "../lib/types";

type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: "primary" | "secondary" | "ghost" | "danger";
  size?: "sm" | "md" | "lg";
  icon?: ReactNode;
  loading?: boolean;
  iconOnly?: boolean;
  ref?: Ref<HTMLButtonElement>;
};

export function Button({
  variant = "secondary",
  size = "md",
  icon,
  loading = false,
  iconOnly = false,
  className = "",
  children,
  disabled,
  ...rest
}: ButtonProps) {
  const classes = ["btn", `btn--${variant}`, size !== "md" && `btn--${size}`, iconOnly && "btn--icon", className]
    .filter(Boolean)
    .join(" ");
  return (
    <button type="button" className={classes} disabled={disabled || loading} aria-busy={loading || undefined} {...rest}>
      {loading ? <span className="spinner" aria-hidden /> : icon}
      {children}
    </button>
  );
}

export function Badge({
  tone = "neutral",
  mono = false,
  title,
  children,
}: {
  tone?: "neutral" | "auto" | "likely" | "manual" | "danger" | "accent";
  mono?: boolean;
  title?: string;
  children: ReactNode;
}) {
  const cls = ["badge", tone !== "neutral" && `badge--${tone}`, mono && "badge--mono"].filter(Boolean).join(" ");
  return (
    <span className={cls} title={title}>
      {children}
    </span>
  );
}

const FIX_ICON: Record<FixKind, ReactNode> = {
  auto: <Sparkles size={11} strokeWidth={2.4} aria-hidden />,
  likely: <ShieldCheck size={11} strokeWidth={2.4} aria-hidden />,
  manual: <Wrench size={11} strokeWidth={2.4} aria-hidden />,
};

export function RiskBadge({ fix }: { fix: FixKind }) {
  const { t } = useI18n();
  return (
    <Badge tone={fix} title={t.fixDesc[fix]}>
      {FIX_ICON[fix]}
      {t.fix[fix]}
    </Badge>
  );
}

const SEVERITY_ICON: Record<Severity, ReactNode> = {
  breaking: <AlertOctagon size={11} strokeWidth={2.4} aria-hidden />,
  deprecated: <TriangleAlert size={11} strokeWidth={2.4} aria-hidden />,
  behavior: <CircleHelp size={11} strokeWidth={2.4} aria-hidden />,
};

export function SeverityBadge({ severity }: { severity: Severity }) {
  const { t } = useI18n();
  const tone = severity === "breaking" ? "danger" : severity === "deprecated" ? "manual" : "neutral";
  return (
    <Badge tone={tone} title={t.severityDesc[severity]}>
      {SEVERITY_ICON[severity]}
      {t.severity[severity]}
    </Badge>
  );
}

export function Card({
  title,
  actions,
  children,
  className = "",
  bodyClassName = "card__body",
}: {
  title?: ReactNode;
  actions?: ReactNode;
  children: ReactNode;
  className?: string;
  bodyClassName?: string;
}) {
  return (
    <section className={`card ${className}`}>
      {(title || actions) && (
        <header className="card__header">
          {title && <h2 className="card__title">{title}</h2>}
          {actions && <div className="card__actions">{actions}</div>}
        </header>
      )}
      <div className={bodyClassName}>{children}</div>
    </section>
  );
}

export function Segmented<T extends string>({
  options,
  value,
  onChange,
  label,
  size = "md",
  disabled = false,
}: {
  options: readonly { value: T; label: ReactNode; disabled?: boolean; describedBy?: string }[];
  value: T;
  onChange: (v: T) => void;
  label: string;
  size?: "md" | "lg";
  disabled?: boolean;
}) {
  const move = (group: HTMLElement, dir: 1 | -1) => {
    if (disabled) return;
    let j = options.findIndex((o) => o.value === value);
    for (let step = 0; step < options.length; step++) {
      j = (j + dir + options.length) % options.length;
      if (!options[j].disabled) break;
    }
    if (options[j].value === value) return;
    onChange(options[j].value);
    group.querySelectorAll<HTMLButtonElement>('[role="radio"]')[j]?.focus();
  };
  return (
    <div
      role="radiogroup"
      aria-label={label}
      className={`segmented ${size === "lg" ? "segmented--lg" : ""}`}
      onKeyDown={(e) => {
        if (e.key === "ArrowRight") move(e.currentTarget, 1);
        if (e.key === "ArrowLeft") move(e.currentTarget, -1);
      }}
    >
      {options.map((o) => (
        <button
          key={o.value}
          type="button"
          role="radio"
          aria-checked={o.value === value}
          tabIndex={o.value === value ? 0 : -1}
          disabled={disabled || o.disabled}
          aria-describedby={o.describedBy}
          onClick={() => onChange(o.value)}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}

export function Progress({ value, label }: { value: number | null; label: string }) {
  const pct = value === null ? null : Math.max(0, Math.min(100, value));
  return (
    <div
      className={`progress ${pct === null ? "progress--indeterminate" : ""}`}
      role="progressbar"
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={pct ?? undefined}
    >
      <div className="progress__bar" style={pct === null ? undefined : { width: `${pct}%` }} />
    </div>
  );
}

export function EmptyState({
  icon,
  title,
  children,
  action,
}: {
  icon: ReactNode;
  title: string;
  children?: ReactNode;
  action?: ReactNode;
}) {
  return (
    <div className="empty">
      <div className="empty__icon" aria-hidden>
        {icon}
      </div>
      <h2>{title}</h2>
      {children && <p>{children}</p>}
      {action}
    </div>
  );
}

export function RuleChips({ rules }: { rules: AppliedRule[] }) {
  const { loc } = useI18n();
  if (rules.length === 0) return null;
  return (
    <div className="rule-chips">
      {rules.map((r) => (
        <Badge key={r.class} tone={r.fix === "likely" ? "likely" : "neutral"} title={r.class}>
          {r.fix === "likely" ? <ShieldCheck size={11} /> : <Sparkles size={11} />}
          {loc(r.title)}
          {r.since && <span className="subtle">· {r.since}</span>}
        </Badge>
      ))}
    </div>
  );
}

export function Callout({
  tone = "info",
  icon,
  children,
}: {
  tone?: "success" | "info" | "warn" | "danger" | "neutral";
  icon?: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className={`callout ${tone !== "neutral" ? `callout--${tone}` : ""}`} role={tone === "danger" ? "alert" : undefined}>
      {icon}
      <div>{children}</div>
    </div>
  );
}

const SEVERITY_COLOR: Record<Severity, string> = {
  breaking: "var(--danger-fg)",
  deprecated: "var(--risk-manual-fg)",
  behavior: "var(--text-3)",
};

export function SeverityIcon({ severity }: { severity: Severity }) {
  const { t } = useI18n();
  const Icon = severity === "breaking" ? AlertOctagon : severity === "deprecated" ? TriangleAlert : CircleHelp;
  return (
    <span className="severity-icon" title={`${t.severity[severity]}: ${t.severityDesc[severity]}`}>
      <Icon size={14} strokeWidth={2.2} color={SEVERITY_COLOR[severity]} aria-hidden />
      <span className="sr-only">{t.severity[severity]}</span>
    </span>
  );
}

export function CommandLine({ command }: { command: string }) {
  const { t } = useI18n();
  const [copied, setCopied] = useState(false);
  useEffect(() => {
    if (!copied) return;
    const id = setTimeout(() => setCopied(false), 1600);
    return () => clearTimeout(id);
  }, [copied]);
  return (
    <div className="command">
      <code className="command__text selectable">{command}</code>
      <Button
        variant="ghost"
        size="sm"
        iconOnly
        aria-label={copied ? t.common.copied : t.common.copy}
        title={copied ? t.common.copied : t.common.copy}
        onClick={() =>
          navigator.clipboard
            ?.writeText(command)
            .then(() => setCopied(true))
            .catch(() => {})
        }
      >
        {copied ? <Check size={14} /> : <Copy size={14} />}
      </Button>
    </div>
  );
}
