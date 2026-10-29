import { useEffect, useId, useRef, type ReactNode } from "react";
import { X } from "lucide-react";
import { Button } from "./ui";
import { useI18n } from "../i18n";

export function ConfirmDialog({
  open,
  icon,
  tone = "accent",
  title,
  children,
  confirmLabel,
  cancelLabel,
  loading = false,
  onConfirm,
  onCancel,
}: {
  open: boolean;
  icon: ReactNode;
  tone?: "accent" | "danger";
  title: string;
  children: ReactNode;
  confirmLabel: string;
  cancelLabel?: string;
  loading?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  const { t } = useI18n();
  const titleId = useId();
  const confirmRef = useRef<HTMLButtonElement>(null);
  const previousFocus = useRef<HTMLElement | null>(null);

  useEffect(() => {
    if (!open) return;
    previousFocus.current = document.activeElement as HTMLElement | null;
    confirmRef.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !loading) onCancel();
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      previousFocus.current?.focus?.();
    };
  }, [open, loading, onCancel]);

  if (!open) return null;
  return (
    <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && !loading && onCancel()}>
      <div className="modal" role="dialog" aria-modal="true" aria-labelledby={titleId}>
        <div className="modal__body">
          <div className={`modal__icon ${tone === "danger" ? "modal__icon--danger" : ""}`} aria-hidden>
            {icon}
          </div>
          <h2 id={titleId}>{title}</h2>
          <div className="muted">{children}</div>
        </div>
        <div className="modal__footer">
          <Button variant="ghost" onClick={onCancel} disabled={loading}>
            {cancelLabel ?? t.common.cancel}
          </Button>
          <Button
            ref={confirmRef}
            variant={tone === "danger" ? "danger" : "primary"}
            loading={loading}
            onClick={onConfirm}
          >
            {confirmLabel}
          </Button>
        </div>
      </div>
    </div>
  );
}

export function Sheet({
  open,
  title,
  meta,
  footer,
  children,
  onClose,
}: {
  open: boolean;
  title: string;
  meta?: ReactNode;
  footer?: ReactNode;
  children: ReactNode;
  onClose: () => void;
}) {
  const { t } = useI18n();
  const titleId = useId();
  const closeRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!open) return;
    const previous = document.activeElement as HTMLElement | null;
    closeRef.current?.focus();
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      previous?.focus?.();
    };
  }, [open, onClose]);

  if (!open) return null;
  return (
    <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="modal sheet" role="dialog" aria-modal="true" aria-labelledby={titleId}>
        <header className="sheet__header">
          <div className="sheet__title">
            <h2 id={titleId}>{title}</h2>
            {meta && <div className="sheet__meta">{meta}</div>}
          </div>
          <Button ref={closeRef} variant="ghost" size="sm" iconOnly aria-label={t.common.close} onClick={onClose}>
            <X size={15} />
          </Button>
        </header>
        <div className="sheet__body">{children}</div>
        {footer && <div className="modal__footer">{footer}</div>}
      </div>
    </div>
  );
}
