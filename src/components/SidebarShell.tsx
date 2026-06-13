import {
  Archive,
  FolderSearch,
  HardDrive,
  History,
  type LucideIcon,
  LockKeyhole,
  Settings,
} from "lucide-react";
import type { ReactNode } from "react";

const cleanerLogoUrl = new URL("../assets/cleaner-logo.png", import.meta.url).href;

export type CleanerModule = "cDrive" | "duplicate" | "largeFiles" | "privacy" | "history" | "settings";

const primaryNav: Array<{
  id: CleanerModule;
  label: string;
  icon: LucideIcon;
  badge?: string;
}> = [
  { id: "cDrive", label: "C 盘清理", icon: HardDrive },
  { id: "duplicate", label: "重复文件清理", icon: FolderSearch },
  { id: "largeFiles", label: "大文件迁移", icon: Archive },
  { id: "privacy", label: "隐私清理", icon: LockKeyhole, badge: "V0.3" },
];

const utilityNav: Array<{
  id: CleanerModule;
  label: string;
  icon: LucideIcon;
}> = [
  { id: "history", label: "清理历史", icon: History },
  { id: "settings", label: "设置", icon: Settings },
];

export function SidebarShell({
  activeModule,
  hasBlockingWork = false,
  onModuleChange,
  children,
}: {
  activeModule: CleanerModule;
  hasBlockingWork?: boolean;
  onModuleChange: (module: CleanerModule) => void;
  children: ReactNode;
}) {
  function switchModule(module: CleanerModule) {
    if (module === activeModule) {
      return;
    }
    if (hasBlockingWork && !window.confirm("当前清理任务或选择尚未完成，确定要切换页面吗？")) {
      return;
    }
    onModuleChange(module);
  }

  return (
    <main className="app-frame sidebar-shell">
      <aside className="app-sidebar shell-sidebar">
        <div className="shell-brand" aria-label="Cleaner">
          <div className="brand-mark">
            <img alt="Cleaner logo" className="cleaner-logo" src={cleanerLogoUrl} />
          </div>
          <div>
            <p className="eyebrow">Cleaner</p>
            <h1>Cleaner</h1>
          </div>
        </div>

        <nav className="shell-nav" aria-label="工具导航">
          {primaryNav.map((item) => (
            <NavButton
              key={item.id}
              active={activeModule === item.id}
              badge={item.badge}
              icon={item.icon}
              label={item.label}
              onClick={() => switchModule(item.id)}
            />
          ))}
        </nav>

        {activeModule === "privacy" && <p className="inline-status">隐私清理将在 V0.3 提供</p>}

        <nav className="shell-nav shell-nav-bottom" aria-label="账户和记录">
          {utilityNav.map((item) => (
            <NavButton
              key={item.id}
              active={activeModule === item.id}
              icon={item.icon}
              label={item.label}
              onClick={() => switchModule(item.id)}
            />
          ))}
        </nav>
      </aside>

      <section className="shell-workspace">{children}</section>
    </main>
  );
}

function NavButton({
  active,
  badge,
  icon: Icon,
  label,
  onClick,
}: {
  active: boolean;
  badge?: string;
  icon: LucideIcon;
  label: string;
  onClick: () => void;
}) {
  return (
    <button className="shell-nav-button" data-active={active} type="button" onClick={onClick}>
      <Icon size={18} />
      <span>{label}</span>
      {badge && <strong>{badge}</strong>}
    </button>
  );
}
