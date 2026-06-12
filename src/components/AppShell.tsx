import { HardDrive, ShieldCheck } from "lucide-react";
import type { ScanReport } from "../domain/models";
import { StepIndicator } from "./StepIndicator";

export function AppShell({
  currentStep,
  report,
  children,
}: {
  currentStep: number;
  report: ScanReport | null;
  children: React.ReactNode;
}) {
  return (
    <main className="app-frame">
      <aside className="app-sidebar">
        <div className="brand-mark">
          <HardDrive size={22} />
        </div>
        <div className="brand-copy">
          <p className="eyebrow">Cleaner</p>
          <h1>磁盘清理控制台</h1>
        </div>
        <StepIndicator currentStep={currentStep} />
        <section className="drive-panel">
          <div>
            <span>当前磁盘</span>
            <strong>{report?.driveSummary.drive ?? "C:"}</strong>
          </div>
          <div>
            <span>剩余空间</span>
            <strong>{report ? formatBytes(report.driveSummary.freeBytes) : "待扫描"}</strong>
          </div>
        </section>
        <div className="safety-note">
          <ShieldCheck size={18} />
          <span>普通模式运行，清理前逐项复查。</span>
        </div>
      </aside>
      <section className="app-workspace">{children}</section>
    </main>
  );
}

function formatBytes(bytes: number): string {
  if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
  if (bytes >= 1024 ** 2) return `${(bytes / 1024 ** 2).toFixed(0)} MB`;
  return `${bytes} B`;
}
