import { createContext, useCallback, useContext, useMemo, useRef, useState, type ReactNode } from "react";
import { CircleCheck, CircleX, Info, X } from "lucide-react";
import { useI18n } from "../i18n";

type Tone = "success" | "error" | "info";
interface Toast {
  id: number;
  tone: Tone;
  title: string;
  description?: string;
  action?: { label: string; run: () => void };
}

interface ToastApi {
  push(t: Omit<Toast, "id">): void;
}

const ToastContext = createContext<ToastApi>({ push: () => {} });

const ICON: Record<Tone, ReactNode> = {
  success: <CircleCheck size={16} />,
  error: <CircleX size={16} />,
  info: <Info size={16} />,
};

export function ToastProvider({ children }: { children: ReactNode }) {
  const { t } = useI18n();
  const [toasts, setToasts] = useState<Toast[]>([]);
  const next = useRef(1);

  const dismiss = useCallback((id: number) => setToasts((all) => all.filter((t) => t.id !== id)), []);

  const push = useCallback(
    (t: Omit<Toast, "id">) => {
      const id = next.current++;
      setToasts((all) => [...all.slice(-3), { ...t, id }]);
      if (t.tone !== "error") setTimeout(() => dismiss(id), 4000);
    },
    [dismiss],
  );

  const api = useMemo(() => ({ push }), [push]);

  return (
    <ToastContext.Provider value={api}>
      {children}
      <div className="toasts" aria-live="polite">
        {toasts.map((item) => (
          <div key={item.id} className={`toast toast--${item.tone}`} role={item.tone === "error" ? "alert" : "status"}>
            {ICON[item.tone]}
            <div className="toast__text">
              <div className="toast__title">{item.title}</div>
              {item.description && <div className="toast__desc">{item.description}</div>}
              {item.action && (
                <button
                  className="btn btn--ghost btn--sm toast__action"
                  onClick={() => {
                    item.action?.run();
                    dismiss(item.id);
                  }}
                >
                  {item.action.label}
                </button>
              )}
            </div>
            <button className="btn btn--ghost btn--sm btn--icon" aria-label={t.common.close} onClick={() => dismiss(item.id)}>
              <X size={14} />
            </button>
          </div>
        ))}
      </div>
    </ToastContext.Provider>
  );
}

export const useToast = () => useContext(ToastContext);
