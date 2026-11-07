import { useEffect, useState } from "react";
import { X } from "lucide-react";
import { Logo } from "../components/Shell";
import { Button, Tooltip } from "../components/ui";
import { useI18n } from "../i18n";
import { formatRelative, modKey } from "../lib/labels";
import { useStore } from "../lib/store";

export function Welcome() {
  const { t } = useI18n();
  const { openProject, opening, recent, forgetRecent } = useStore();
  const [dragging, setDragging] = useState(false);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    if ("__TAURI_INTERNALS__" in window) {
      import("@tauri-apps/api/webview").then(({ getCurrentWebview }) =>
        getCurrentWebview()
          .onDragDropEvent((e) => {
            if (e.payload.type === "over" || e.payload.type === "enter") setDragging(true);
            else if (e.payload.type === "leave") setDragging(false);
            else if (e.payload.type === "drop") {
              setDragging(false);
              const first = e.payload.paths[0];
              if (first) openProject(first);
            }
          })
          .then((fn) => (cancelled ? fn() : (unlisten = fn))),
      );
    }
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [openProject]);

  return (
    <div className="welcome" data-tauri-drag-region data-dragging={dragging || undefined}>
      <div className="welcome__inner">
        <Logo className="welcome__logo" />
        <div className="welcome__head">
          <h1>{t.welcome.title}</h1>
          <p className="welcome__lead">{dragging ? t.welcome.dropping : t.welcome.lead}</p>
        </div>

        <Tooltip text={`${modKey()} O`}>
          <Button variant="primary" size="lg" loading={opening} onClick={() => openProject()}>
            {t.welcome.pick}
          </Button>
        </Tooltip>

        {recent.length > 0 && (
          <div className="recent">
            <h2 className="section-title">{t.welcome.recent}</h2>
            <div className="recent__list">
              {recent.map((r) => (
                <div key={r.path} className="recent__row">
                  <button type="button" className="recent__item" title={r.path} onClick={() => openProject(r.path)} disabled={opening}>
                    <span className="recent__text">
                      <span className="strong">{r.name}</span>
                      <span className="recent__path sr-only">{r.path}</span>
                    </span>
                    <span className="recent__time">{formatRelative(r.openedAt, t)}</span>
                  </button>
                  <Button variant="ghost" size="sm" iconOnly aria-label={t.welcome.forget(r.name)} onClick={() => forgetRecent(r.path)} className="recent__forget">
                    <X size={14} />
                  </Button>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
