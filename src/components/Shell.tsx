import { useEffect, type ReactNode } from "react";
import {
  Bot,
  ClipboardList,
  Container,
  FolderOpen,
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
    <svg className={className} viewBox="100 100 824 824" aria-hidden>
      <defs>
        <linearGradient id="ferry-logo-bg" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#3D7BF0" />
          <stop offset="0.55" stopColor="#1F4BB8" />
          <stop offset="1" stopColor="#13286E" />
        </linearGradient>
      </defs>
      <rect x="100" y="100" width="824" height="824" rx="186" fill="url(#ferry-logo-bg)" />
      <g fill="#fff" stroke="#fff" strokeWidth="22" strokeLinejoin="round">
        <path d="M528 262 L684 512 H528 Z" />
        <path d="M496 342 L404 512 H496 Z" fill="#CFE0FF" stroke="#CFE0FF" />
        <path d="M296 566 H728 Q714 618 668 664 H356 Q310 618 296 566 Z" />
      </g>
      <path
        d="M262 760 q41.6 -34 83.2 0 t83.2 0 t83.2 0 t83.2 0 t83.2 0 t83.2 0"
        fill="none"
        stroke="#9CC3FF"
        strokeWidth="38"
        strokeLinecap="round"
      />
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
  ];
  const upcoming: NavItem[] = [
    { view: "runtime", label: t.nav.runtime, icon: <Container size={16} /> },
    { view: "ai", label: t.nav.ai, icon: <Bot size={16} /> },
  ];

  const renderItem = (item: NavItem) => (
    <button
      key={item.view}
      className="nav__item"
      aria-current={(project || item.view === "overview") && view === item.view ? "page" : undefined}
      disabled={!project && item.view !== "overview"}
      onClick={() => setView(item.view)}
      title={item.shortcut ? `${item.label} (${modKey()} ${item.shortcut})` : undefined}
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
        <button className="project-chip" onClick={() => openProject()} disabled={opening} title={t.nav.openAnother}>
          <span className="project-chip__icon">
            <FolderOpen size={15} />
          </span>
          <span className="project-chip__text">
            <span className="project-chip__name">
              {project.name}
            </span>
            <span className="project-chip__meta">
              {FRAMEWORK_LABEL[project.framework]}
              {project.currentVersion ? ` · PHP ${project.currentVersion}` : ""}
            </span>
          </span>
        </button>
      )}

      <nav className="nav" aria-label={t.nav.sections}>
        {main.map(renderItem)}
        <div className="nav__label">{t.nav.comingSoon}</div>
        {upcoming.map(renderItem)}
      </nav>

      <div className="sidebar__footer">
        <button
          className="nav__item"
          aria-current={view === "settings" ? "page" : undefined}
          onClick={() => setView("settings")}
          title={`${t.nav.settings} (${modKey()} ,)`}
        >
          <Settings size={16} />
          <span>{t.nav.settings}</span>
        </button>
        <div className="status-line" role="status">
          {vulnerablePhp ? (
            <button type="button" className="status-line__link" onClick={() => setView("settings")} title={t.nav.phpVulnerableHint}>
              <span className={`dot ${toolsTone}`} aria-hidden />
              <span>{toolsText}</span>
            </button>
          ) : (
            <>
              <span className={`dot ${toolsTone}`} aria-hidden />
              <span>{toolsText}</span>
            </>
          )}
        </div>
      </div>
    </aside>
  );
}

export function PageHeader({ title, subtitle, actions }: { title: string; subtitle?: string; actions?: ReactNode }) {
  useEffect(() => {
    document.title = `${title} · Ferry`;
  }, [title]);
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
