import { useEffect, useId, useLayoutEffect, useRef, type ReactNode, type RefObject } from "react";
import { X } from "lucide-react";
import { Button } from "./ui";
import { useI18n } from "../i18n";

const FOCUSABLE = 'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]';
/** Toasts stay reachable while a dialog is open. */
const TOAST_FOCUSABLE = FOCUSABLE.split(", ")
  .map((selector) => `.toasts ${selector}`)
  .join(", ");

function hideBackground(dialog: Element) {
  const hidden: Element[] = [];
  for (let node = dialog; node.parentElement; node = node.parentElement) {
    for (const sibling of node.parentElement.children) {
      const skip = sibling === node || sibling.hasAttribute("inert") || sibling.matches("script, style, [aria-live]");
      if (skip) continue;
      sibling.setAttribute("inert", "");
      hidden.push(sibling);
    }
  }
  return hidden;
}

function useModal(open: boolean, dialogRef: RefObject<HTMLElement | null>, initialRef: RefObject<HTMLElement | null>, onEscape: () => void) {
  const escapeRef = useRef(onEscape);
  useLayoutEffect(() => {
    escapeRef.current = onEscape;
  });

  useEffect(() => {
    if (!open) return;
    const previous = document.activeElement as HTMLElement | null;
    const dialog = dialogRef.current;
    const field = dialog?.querySelector<HTMLElement>("input:not([disabled]), textarea:not([disabled]), select:not([disabled])");
    (field ?? initialRef.current)?.focus();
    if (dialog && !dialog.contains(document.activeElement)) dialog.focus();
    const hidden = dialog ? hideBackground(dialog) : [];
    const onKey = (e: KeyboardEvent) => {
      const dialog = dialogRef.current;
      const top = [...document.querySelectorAll('[role="dialog"][aria-modal="true"]')].pop();
      if (!dialog || top !== dialog) return;
      if (e.key === "Escape") {
        escapeRef.current();
        return;
      }
      if (e.key !== "Tab") return;
      const usable = (el: HTMLElement) => el.tabIndex >= 0 && !el.matches(":disabled");
      const toasts = [...document.querySelectorAll<HTMLElement>(TOAST_FOCUSABLE)].filter(usable);
      const items = [...[...dialog.querySelectorAll<HTMLElement>(FOCUSABLE)].filter(usable), ...toasts];
      if (!items.length) {
        e.preventDefault();
        return;
      }
      const first = items[0];
      const last = items[items.length - 1];
      const active = document.activeElement;
      const outside = !dialog.contains(active) && !toasts.includes(active as HTMLElement);
      if (e.shiftKey && (active === first || outside)) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && (active === last || outside)) {
        e.preventDefault();
        first.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      hidden.forEach((el) => el.removeAttribute("inert"));
      previous?.focus?.();
    };
  }, [open, dialogRef, initialRef]);
}

export function ConfirmDialog({
  open,
  icon,
  tone = "accent",
  title,
  children,
  confirmLabel,
  cancelLabel,
  loading = false,
  confirmDisabled = false,
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
  confirmDisabled?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  const { t } = useI18n();
  const titleId = useId();
  const bodyId = useId();
  const confirmRef = useRef<HTMLButtonElement>(null);
  const cancelRef = useRef<HTMLButtonElement>(null);
  const dialogRef = useRef<HTMLDivElement>(null);
  useModal(open, dialogRef, tone === "danger" ? cancelRef : confirmRef, () => {
    if (!loading) onCancel();
  });

  if (!open) return null;
  return (
    <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && !loading && onCancel()}>
      <div ref={dialogRef} className="modal" role="dialog" aria-modal="true" tabIndex={-1} aria-labelledby={titleId} aria-describedby={bodyId}>
        <div className="modal__body">
          <div className={`modal__icon ${tone === "danger" ? "modal__icon--danger" : ""}`} aria-hidden>
            {icon}
          </div>
          <h2 id={titleId}>{title}</h2>
          <div className="muted" id={bodyId}>
            {children}
          </div>
        </div>
        <div className="modal__footer">
          <Button ref={cancelRef} variant="ghost" onClick={onCancel} disabled={loading}>
            {cancelLabel ?? t.common.cancel}
          </Button>
          <Button
            ref={confirmRef}
            variant={tone === "danger" ? "danger" : "primary"}
            loading={loading}
            disabled={confirmDisabled}
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
  const dialogRef = useRef<HTMLDivElement>(null);
  useModal(open, dialogRef, closeRef, onClose);

  if (!open) return null;
  return (
    <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div ref={dialogRef} className="modal sheet" role="dialog" aria-modal="true" aria-labelledby={titleId}>
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
