import { useEffect, type ReactNode } from "react";
import {
  Bot,
  ClipboardList,
  Container,
  GitCompareArrows,
  History,
  LayoutDashboard,
  Package,
  Settings,
} from "lucide-react";
import { useI18n } from "../i18n";
import { dependencyBlockers, FRAMEWORK_LABEL, modKey } from "../lib/labels";
import { isVulnerable } from "../lib/security";
import { useStore, type View } from "../lib/store";

export function Logo({ className = "brand__mark" }: { className?: string }) {
  return (
    <svg className={className} viewBox="230 160 560 670" aria-hidden>
      <path d="M466,318 C387,372 301,500 289,628 Q287,648 307,648 L452,648 Q466,648 466,634 Z" fill="var(--sail)" />
      <path d="M500,176 C603,250 719,430 735,628 Q737,648 717,648 L514,648 Q500,648 500,634 Z" fill="var(--hull)" />
      <path d="M250,700 Q512,744 774,700 C736,782 628,814 512,814 C396,814 288,782 250,700 Z" fill="var(--hull)" />
    </svg>
  );
}

interface NavItem {
  view: View;
  shortcut?: number;
  label: string;
  icon: ReactNode;
  count?: number;
  danger?: boolean;
}

export function Sidebar() {
  const { t } = useI18n();
  const { project, view, setView, scan, preview, deps, toolchain, openProject, opening } = useStore();
  const blockers = dependencyBlockers(deps).total;

  const main: NavItem[] = [
    { view: "overview", shortcut: 1, label: t.nav.overview, icon: <LayoutDashboard size={16} /> },
    { view: "findings", shortcut: 2, label: t.nav.findings, icon: <ClipboardList size={16} />, count: scan?.summary.total, danger: (scan?.summary.breaking ?? 0) > 0 },
    { view: "changes", shortcut: 3, label: t.nav.fixes, icon: <GitCompareArrows size={16} />, count: preview?.changes.length },
    { view: "dependencies", shortcut: 4, label: t.nav.dependencies, icon: <Package size={16} />, count: blockers || undefined, danger: blockers > 0 },
    { view: "history", shortcut: 5, label: t.nav.history, icon: <History size={16} /> },
    { view: "runtime", label: t.nav.runtime, icon: <Container size={16} /> },
  ];
  const upcoming: NavItem[] = [
    { view: "ai", label: t.nav.ai, icon: <Bot size={16} /> },
  ];

  const renderItem = (item: NavItem) => (
    <button
      key={item.view}
      className="nav__item"
      aria-current={(project || item.view === "overview") && view === item.view ? "page" : undefined}
      disabled={!project && item.view !== "overview"}
      onClick={() => setView(item.view)}
      title={!project && item.view !== "overview" ? t.nav.needsProject : item.shortcut ? `${item.label} (${modKey()} ${item.shortcut})` : undefined}
    >
      {item.icon}
      <span>{item.label}</span>
      {item.count !== undefined && item.count > 0 && (
        <span className={`nav__count ${item.danger ? "nav__count--danger" : ""}`}>{item.count}</span>
      )}
    </button>
  );

  const toolsOk = !!toolchain?.php && !!toolchain?.rector;
  const phpVersion = toolchain?.php?.version;
  const vulnerablePhp = phpVersion && isVulnerable(phpVersion) ? phpVersion : null;
  const toolsTone = !toolchain ? "" : vulnerablePhp ? "dot--error" : toolsOk ? "dot--ok" : toolchain.php ? "dot--warn" : "dot--error";
  const toolsText = !toolchain
    ? t.nav.toolsChecking
    : vulnerablePhp
      ? t.nav.phpVulnerable(vulnerablePhp)
      : toolsOk
      ? `PHP ${toolchain.php?.version ?? ""} · Rector ${toolchain.rector?.version ?? ""}`
      : toolchain.php
        ? t.nav.rectorMissing
        : t.nav.phpMissing;

  return (
    <aside className="sidebar" aria-label={t.nav.sidebar}>
      <div className="sidebar__drag" data-tauri-drag-region />
      <div className="brand" data-tauri-drag-region>
        <Logo />
        <span className="brand__name">Ferry</span>
      </div>

      {project && (
        <button className="project-chip" onClick={() => openProject()} disabled={opening} title={`${project.root}\n${t.nav.openAnother}`}>
          <span className="project-chip__name">{project.name}</span>
          <span className="project-chip__meta">
            {FRAMEWORK_LABEL[project.framework]}
            {project.currentVersion ? ` · PHP ${project.currentVersion}` : ""}
          </span>
        </button>
      )}

      <nav className="nav" aria-label={t.nav.sections}>
        {main.map(renderItem)}
        {upcoming.map(renderItem)}
      </nav>

      <div className="sidebar__footer">
        <div role="status">
          {vulnerablePhp ? (
            <button type="button" className="status-line__link" onClick={() => setView("settings")} title={t.nav.phpVulnerableHint}>
              <span className={`dot ${toolsTone}`} aria-hidden />
              <span>{toolsText}</span>
            </button>
          ) : (
            <span className="sr-only">{toolsText}</span>
          )}
        </div>
        <button
          className="nav__item"
          aria-current={view === "settings" ? "page" : undefined}
          onClick={() => setView("settings")}
          title={`${t.nav.settings} (${modKey()} ,)`}
        >
          <Settings size={16} />
          <span>{t.nav.settings}</span>
          {!vulnerablePhp && <span className={`dot nav__status ${toolsTone}`} title={toolsText} aria-hidden />}
        </button>
      </div>
    </aside>
  );
}

export function usePageTitle(title: string) {
  useEffect(() => {
    document.title = `${title} · Ferry`;
  }, [title]);
}

export function PageHeader({ title, subtitle, actions }: { title: string; subtitle?: string; actions?: ReactNode }) {
  usePageTitle(title);
  return (
    <header className="page-header" data-tauri-drag-region>
      <div className="page-header__title">
        <h1>{title}</h1>
        {subtitle && <span>{subtitle}</span>}
      </div>
      {actions && <div className="page-header__actions">{actions}</div>}
    </header>
  );
}
