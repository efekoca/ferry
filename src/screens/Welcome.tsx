import { useEffect, useState } from "react";
import { FolderOpen, GitCompareArrows, History, ScanSearch, X } from "lucide-react";
import { Logo } from "../components/Shell";
import { Button } from "../components/ui";
import { useI18n } from "../i18n";
import { FRAMEWORK_LABEL, formatRelative, modKey } from "../lib/labels";
import { useStore } from "../lib/store";

const FEATURE_ICONS = [ScanSearch, GitCompareArrows, History];

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
    <div className="welcome" data-tauri-drag-region>
      <div className="welcome__inner">
        <Logo className="welcome__logo" />
        <h1>{t.welcome.title}</h1>
        <p className="welcome__lead">{t.welcome.lead}</p>

        <button
          className="dropzone"
          data-active={dragging}
          onClick={() => openProject()}
          disabled={opening}
          aria-label={t.welcome.pick}
        >
          <span className="dropzone__icon">
            <FolderOpen size={22} />
          </span>
          <strong>{dragging ? t.welcome.dropping : t.welcome.drop}</strong>
          <span className="muted">
            {t.welcome.supports} · <span className="kbd">{modKey()}</span> <span className="kbd">O</span>
          </span>
        </button>

        {recent.length > 0 && (
          <div className="recent">
            <div className="section-title">{t.welcome.recent}</div>
            <div className="recent__list">
              {recent.map((r) => (
                <div key={r.path} className="row" style={{ gap: 0 }}>
                  <button className="recent__item" style={{ flex: 1, minWidth: 0 }} onClick={() => openProject(r.path)}>
                    <FolderOpen size={16} className="subtle" />
                    <span style={{ minWidth: 0, flex: 1 }}>
                      <span style={{ fontWeight: 500 }}>{r.name}</span>{" "}
                      <span className="subtle">· {FRAMEWORK_LABEL[r.framework]}</span>
                      <div className="recent__path">{r.path}</div>
                    </span>
                    <span className="subtle" style={{ fontSize: "var(--text-sm)", whiteSpace: "nowrap" }}>
                      {formatRelative(r.openedAt, t)}
                    </span>
                  </button>
                  <Button
                    variant="ghost"
                    size="sm"
                    iconOnly
                    aria-label={t.welcome.forget(r.name)}
                    onClick={() => forgetRecent(r.path)}
                    style={{ marginRight: 8 }}
                  >
                    <X size={14} />
                  </Button>
                </div>
              ))}
            </div>
          </div>
        )}

        <div className="features">
          {t.welcome.features.map((f, i) => {
            const Icon = FEATURE_ICONS[i];
            return (
              <div key={f.title} className="card feature">
                <Icon size={18} className="feature__icon" />
                <h3>{f.title}</h3>
                <p>{f.text}</p>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
