import { ShieldCheck } from "lucide-react";
import type { ReactNode } from "react";
import type { ScanReport } from "../domain/models";
import { StepIndicator } from "./StepIndicator";

const cleanerLogoUrl = new URL("../assets/cleaner-logo.png", import.meta.url).href;

export function AppShell({
  currentStep,
  report,
  children,
}: {
  currentStep: number;
  report: ScanReport | null;
  children: ReactNode;
}) {
  return (
    <div className="cdrive-workflow">
      <header className="workflow-header">
        <div className="brand-mark">
          <img alt="Cleaner logo" className="cleaner-logo" src={cleanerLogoUrl} />
        </div>
        <div className="brand-copy">
          <p className="eyebrow">C Drive</p>
          <h1>C 盘清理</h1>
        </div>
      </header>

      <section className="workflow-status">
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
      </section>

      <section className="app-workspace">{children}</section>
    </div>
  );
}

function formatBytes(bytes: number): string {
  if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
  if (bytes >= 1024 ** 2) return `${(bytes / 1024 ** 2).toFixed(0)} MB`;
  return `${bytes} B`;
}
