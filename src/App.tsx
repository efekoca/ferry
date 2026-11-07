import { useEffect } from "react";
import { CircleAlert } from "lucide-react";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { Sidebar } from "./components/Shell";
import { Button, EmptyState } from "./components/ui";
import { ToastProvider } from "./components/Toast";
import { I18nProvider, useI18n } from "./i18n";
import type { Api } from "./lib/api";
import { isTauri } from "./lib/api";
import { StoreProvider, useStore } from "./lib/store";
import { Changes } from "./screens/Changes";
import { Findings } from "./screens/Findings";
import { DockerCompare } from "./screens/DockerCompare";
import { History } from "./screens/History";
import { Dependencies, Roadmap, SettingsScreen } from "./screens/Other";
import { Overview } from "./screens/Overview";
import { Welcome } from "./screens/Welcome";

function detectPlatform(): "macos" | "windows" | "linux" {
  const p = (navigator.platform || navigator.userAgent).toLowerCase();
  if (p.includes("mac")) return "macos";
  if (p.includes("win")) return "windows";
  return "linux";
}

function SafeScreen() {
  const { t } = useI18n();
  const { view, project } = useStore();
  return (
    <ErrorBoundary
      key={`${view}:${project?.root ?? ""}`}
      fallback={(error, reset) => (
        <EmptyState
          icon={<CircleAlert size={24} />}
          title={t.common.screenFailed}
          action={
            <Button variant="secondary" onClick={reset}>
              {t.common.tryAgain}
            </Button>
          }
        >
          <span className="selectable">{error.message}</span>
        </EmptyState>
      )}
    >
      <Screen />
    </ErrorBoundary>
  );
}

function Screen() {
  const { project, view } = useStore();
  if (view === "settings") return <SettingsScreen />;
  if (!project) return <Welcome />;
  switch (view) {
    case "findings":
      return <Findings />;
    case "changes":
      return <Changes />;
    case "dependencies":
      return <Dependencies />;
    case "history":
      return <History />;
    case "runtime":
      return <DockerCompare />;
    case "ai":
      return <Roadmap />;
    default:
      return <Overview />;
  }
}

const VIEW_KEYS = ["overview", "findings", "changes", "dependencies", "history"] as const;

function Shortcuts() {
  const { openProject, project, setView } = useStore();
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey;
      if (!mod || e.altKey || document.querySelector('[role="dialog"][aria-modal="true"]')) return;
      if (e.key.toLowerCase() === "o") {
        e.preventDefault();
        openProject();
      } else if (e.key === ",") {
        e.preventDefault();
        setView("settings");
      } else if (project && /^[1-5]$/.test(e.key)) {
        e.preventDefault();
        setView(VIEW_KEYS[Number(e.key) - 1]);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [openProject, project, setView]);
  return null;
}

export default function App({ api }: { api?: Api }) {
  useEffect(() => {
    const platform = detectPlatform();
    document.documentElement.dataset.platform = platform;
    document.documentElement.dataset.vibrancy = String(platform === "macos" && isTauri());
  }, []);

  return (
    <I18nProvider>
      <ToastProvider>
        <StoreProvider api={api}>
        <Shortcuts />
        <div className="app">
          <Sidebar />
          <main className="main">
            <SafeScreen />
          </main>
        </div>
        </StoreProvider>
      </ToastProvider>
    </I18nProvider>
  );
}
