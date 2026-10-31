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
import { useStore, type View } from "../lib/store";

export function Logo({ className = "brand__mark" }: { className?: string }) {
  return (
    <svg className={className} viewBox="100 100 824 824" aria-hidden>
      <defs>
        <linearGradient id="ferry-logo-bg" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#1B1A3A" />
          <stop offset="1" stopColor="#121128" />
        </linearGradient>
      </defs>
      <path
        d="M381.78,100 L642.22,100 C723.36,100 763.93,100 807.6,113.81 C855.28,131.16 892.84,168.72 910.19,216.4 C924,260.07 924,300.64 924,381.78 L924,642.22 C924,723.36 924,763.93 910.19,807.6 C892.84,855.28 855.28,892.84 807.6,910.19 C763.93,924 723.36,924 642.22,924 L381.78,924 C300.64,924 260.07,924 216.4,910.19 C168.72,892.84 131.16,855.28 113.81,807.6 C100,763.93 100,723.36 100,642.22 L100,381.78 C100,300.64 100,260.07 113.81,216.4 C131.16,168.72 168.72,131.16 216.4,113.81 C260.07,100 300.64,100 381.78,100 Z"
        fill="url(#ferry-logo-bg)"
        stroke="#fff"
        strokeOpacity="0.12"
        strokeWidth="8"
      />
      <g transform="translate(100 100) scale(0.80469)">
        <path d="M466,318 C387,372 301,500 289,628 Q287,648 307,648 L452,648 Q466,648 466,634 Z" fill="#8C90E6" />
        <path d="M500,176 C603,250 719,430 735,628 Q737,648 717,648 L514,648 Q500,648 500,634 Z" fill="#F4EFE6" />
        <path d="M250,700 Q512,744 774,700 C736,782 628,814 512,814 C396,814 288,782 250,700 Z" fill="#F4EFE6" />
      </g>
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
  const toolsTone = !toolchain ? "" : toolsOk ? "dot--ok" : toolchain.php ? "dot--warn" : "dot--error";
  const toolsText = !toolchain
    ? t.nav.toolsChecking
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
          <span className={`dot ${toolsTone}`} aria-hidden />
          <span>{toolsText}</span>
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
