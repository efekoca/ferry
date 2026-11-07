import {
  useEffect,
  useId,
  useLayoutEffect,
  useRef,
  useState,
  type ButtonHTMLAttributes,
  type CSSProperties,
  type InputHTMLAttributes,
  type KeyboardEvent,
  type ReactNode,
  type Ref,
  type TextareaHTMLAttributes,
} from "react";
import { Check, ChevronDown, Copy, X } from "lucide-react";
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
  tone?: "neutral" | "auto" | "likely" | "manual" | "danger" | "accent" | "info";
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

export function RiskBadge({ fix }: { fix: FixKind }) {
  const { t } = useI18n();
  return (
    <Badge tone={fix} title={t.fixDesc[fix]}>
      {t.fix[fix]}
    </Badge>
  );
}

export function SeverityBadge({ severity }: { severity: Severity }) {
  const { t } = useI18n();
  const tone = severity === "breaking" ? "danger" : severity === "deprecated" ? "manual" : "info";
  return (
    <Badge tone={tone} title={t.severityDesc[severity]}>
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

export function SeverityIcon({ severity }: { severity: Severity }) {
  const { t } = useI18n();
  return (
    <span className="severity-icon" title={`${t.severity[severity]}: ${t.severityDesc[severity]}`}>
      <span className={`dot dot--${severity}`} aria-hidden />
      <span className="sr-only">{t.severity[severity]}</span>
    </span>
  );
}

export function Tooltip({
  text,
  place = "up",
  align = "center",
  className = "",
  children,
}: {
  text: ReactNode;
  place?: "up" | "down";
  align?: "center" | "start" | "end";
  className?: string;
  children: ReactNode;
}) {
  const classes = ["tip__box", place === "down" && "tip__box--down", align !== "center" && `tip__box--${align}`].filter(Boolean).join(" ");
  return (
    <span className={`tip ${className}`}>
      {children}
      <span className={classes} role="tooltip">
        {text}
      </span>
    </span>
  );
}

export interface SelectOption<T extends string> {
  value: T;
  label: string;
  hint?: string;
  icon?: ReactNode;
  disabled?: boolean;
}

export function Select<T extends string>({
  value,
  onChange,
  options,
  label,
  placeholder,
  icon,
  disabled = false,
  block = false,
  title,
  className = "",
}: {
  value: T | "";
  onChange: (value: T) => void;
  options: readonly SelectOption<T>[];
  label: string;
  placeholder?: string;
  icon?: ReactNode;
  disabled?: boolean;
  block?: boolean;
  title?: string;
  className?: string;
}) {
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(-1);
  const [place, setPlace] = useState<CSSProperties>({ visibility: "hidden" });
  const root = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const list = useRef<HTMLDivElement>(null);
  const typed = useRef({ text: "", at: 0 });
  const id = useId();
  const current = options.find((o) => o.value === value);

  const show = () => {
    const at = options.findIndex((o) => o.value === value && !o.disabled);
    setActive(at >= 0 ? at : options.findIndex((o) => !o.disabled));
    setOpen(true);
  };
  const close = (refocus = true) => {
    setOpen(false);
    if (refocus) trigger.current?.focus();
  };
  const choose = (option: SelectOption<T> | undefined) => {
    if (!option || option.disabled) return;
    close();
    if (option.value !== value) onChange(option.value);
  };
  const step = (dir: 1 | -1) => {
    let at = active;
    for (let n = 0; n < options.length; n++) {
      at = (at + dir + options.length) % options.length;
      if (!options[at].disabled) break;
    }
    setActive(at);
  };
  const seek = (key: string) => {
    const now = Date.now();
    const text = now - typed.current.at < 600 ? typed.current.text + key.toLocaleLowerCase() : key.toLocaleLowerCase();
    typed.current = { text, at: now };
    const at = options.findIndex((o) => !o.disabled && o.label.toLocaleLowerCase().startsWith(text));
    if (at >= 0) setActive(at);
  };
  const onKeyDown = (e: KeyboardEvent<HTMLButtonElement>) => {
    if (!open) {
      if (["ArrowDown", "ArrowUp", "Enter", " "].includes(e.key)) {
        e.preventDefault();
        show();
      }
      return;
    }
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      step(e.key === "ArrowDown" ? 1 : -1);
    } else if (e.key === "Home" || e.key === "End") {
      e.preventDefault();
      setActive(e.key === "Home" ? options.findIndex((o) => !o.disabled) : options.findLastIndex((o) => !o.disabled));
    } else if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      choose(options[active]);
    } else if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      close();
    } else if (e.key === "Tab") {
      close(false);
    } else if (e.key.length === 1 && !e.metaKey && !e.ctrlKey) {
      seek(e.key);
    }
  };

  useLayoutEffect(() => {
    if (!open || !trigger.current || !list.current) return;
    const box = trigger.current.getBoundingClientRect();
    const height = list.current.offsetHeight;
    const below = window.innerHeight - box.bottom - 12;
    const up = height > below && box.top > below;
    const width = Math.max(box.width, list.current.offsetWidth);
    setPlace({
      left: Math.max(8, Math.min(box.left, window.innerWidth - width - 8)),
      minWidth: box.width,
      maxHeight: Math.max(160, (up ? box.top : below) - 12),
      ...(up ? { bottom: window.innerHeight - box.top + 6 } : { top: box.bottom + 6 }),
    });
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const away = (e: PointerEvent) => {
      if (!root.current?.contains(e.target as Node)) close(false);
    };
    const moved = (e: Event) => {
      if (!list.current?.contains(e.target as Node)) close(false);
    };
    document.addEventListener("pointerdown", away);
    window.addEventListener("scroll", moved, true);
    window.addEventListener("resize", moved);
    return () => {
      document.removeEventListener("pointerdown", away);
      window.removeEventListener("scroll", moved, true);
      window.removeEventListener("resize", moved);
    };
  }, [open]);

  useEffect(() => {
    const menu = list.current;
    const item = menu?.children[active] as HTMLElement | undefined;
    if (!menu || !item) return;
    if (item.offsetTop < menu.scrollTop) menu.scrollTop = item.offsetTop;
    else if (item.offsetTop + item.offsetHeight > menu.scrollTop + menu.clientHeight) menu.scrollTop = item.offsetTop + item.offsetHeight - menu.clientHeight;
  }, [open, active]);

  return (
    <div ref={root} className={`select ${block ? "select--block" : ""} ${className}`}>
      <button
        ref={trigger}
        type="button"
        role="combobox"
        className="select__trigger"
        aria-label={label}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={open ? `${id}-list` : undefined}
        aria-activedescendant={open && active >= 0 ? `${id}-${active}` : undefined}
        disabled={disabled}
        title={title}
        onClick={() => (open ? close() : show())}
        onKeyDown={onKeyDown}
      >
        {icon}
        <span className={`select__value ${current ? "" : "select__value--empty"}`}>{current?.label ?? placeholder ?? label}</span>
        <ChevronDown className="select__chevron" size={14} aria-hidden />
      </button>
      {open && (
        <div ref={list} id={`${id}-list`} role="listbox" aria-label={label} className="menu select__menu" style={place} onMouseDown={(e) => e.preventDefault()}>
          {options.map((o, i) => (
            <div
              key={o.value}
              id={`${id}-${i}`}
              role="option"
              aria-selected={o.value === value}
              aria-disabled={o.disabled || undefined}
              className={`menu__item ${i === active ? "is-active" : ""}`}
              onPointerEnter={() => !o.disabled && setActive(i)}
              onClick={() => choose(o)}
            >
              {o.icon}
              <span className="menu__main">
                <span className="menu__title">{o.label}</span>
                {o.hint && <span className="menu__sub">{o.hint}</span>}
              </span>
              {o.value === value && <Check className="menu__check" size={14} strokeWidth={2.6} aria-hidden />}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

type TextFieldProps = InputHTMLAttributes<HTMLInputElement> & {
  label: string;
  prefix?: ReactNode;
  icon?: ReactNode;
  end?: ReactNode;
  mono?: boolean;
  ref?: Ref<HTMLInputElement>;
};

export function TextField({ label, prefix, icon, end, mono = false, className = "", disabled, ...rest }: TextFieldProps) {
  return (
    <label className={`fbox ${disabled ? "fbox--disabled" : ""} ${className}`}>
      <span className="fbox__label">{label}</span>
      <span className="fbox__row">
        {icon}
        {prefix && <span className="fbox__prefix">{prefix}</span>}
        <input className={`fbox__input ${mono ? "fbox__input--mono" : ""}`} disabled={disabled} {...rest} />
        {end && <span className="fbox__end">{end}</span>}
      </span>
    </label>
  );
}

export function TextArea({
  label,
  mono = false,
  className = "",
  ...rest
}: TextareaHTMLAttributes<HTMLTextAreaElement> & { label: string; mono?: boolean }) {
  const area = useRef<HTMLTextAreaElement>(null);
  useLayoutEffect(() => {
    const el = area.current;
    if (!el || CSS?.supports?.("field-sizing", "content")) return;
    el.style.height = "auto";
    el.style.height = `${el.scrollHeight}px`;
  }, [rest.value]);
  return (
    <label className={`fbox ${className}`}>
      <span className="fbox__label">{label}</span>
      <textarea ref={area} className={`fbox__input ${mono ? "fbox__input--mono" : ""}`} {...rest} />
    </label>
  );
}

export function TagInput({
  label,
  values,
  onChange,
  placeholder,
  removeLabel,
  disabled = false,
  end,
  className = "",
}: {
  label: string;
  values: readonly string[];
  onChange: (values: string[]) => void;
  placeholder?: string;
  removeLabel: (value: string) => string;
  disabled?: boolean;
  end?: ReactNode;
  className?: string;
}) {
  const [draft, setDraft] = useState("");
  const id = useId();
  const commit = (text: string) => {
    const added = text.split(/[\n,]/).map((v) => v.trim()).filter(Boolean);
    const next = [...values];
    for (const value of added) if (!next.includes(value)) next.push(value);
    if (next.length !== values.length) onChange(next);
    setDraft("");
  };
  return (
    <div className={`fbox fbox--tags ${disabled ? "fbox--disabled" : ""} ${className}`}>
      <label className="fbox__label" htmlFor={id}>
        {label}
      </label>
      <span className="fbox__row">
        {values.map((value) => (
          <span className="tag" key={value}>
            <span className="tag__text">{value}</span>
            <button type="button" className="tag__remove" aria-label={removeLabel(value)} disabled={disabled} onClick={() => onChange(values.filter((v) => v !== value))}>
              <X size={11} strokeWidth={2.4} aria-hidden />
            </button>
          </span>
        ))}
        <input
          id={id}
          className="fbox__input fbox__input--mono"
          value={draft}
          placeholder={placeholder}
          disabled={disabled}
          onChange={(e) => (/[\n,]/.test(e.target.value) ? commit(e.target.value) : setDraft(e.target.value))}
          onBlur={() => commit(draft)}
          onPaste={(e) => {
            const text = e.clipboardData.getData("text");
            if (!/[\n,]/.test(text)) return;
            e.preventDefault();
            commit(text);
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              commit(draft);
            } else if (e.key === "Backspace" && !draft && values.length > 0) {
              onChange(values.slice(0, -1));
            }
          }}
        />
        {end && <span className="fbox__end">{end}</span>}
      </span>
    </div>
  );
}

export function Switch({ label, className = "", ...rest }: InputHTMLAttributes<HTMLInputElement> & { label?: string }) {
  return <input type="checkbox" role="switch" className={`switch ${className}`} aria-label={label} {...rest} />;
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
      {!/^https?:/.test(command) && (
        <span className="command__prompt" aria-hidden>
          $
        </span>
      )}
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
