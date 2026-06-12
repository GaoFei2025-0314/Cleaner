import { AppWindow, Download, PackageCheck, ShieldCheck } from "lucide-react";

export function ScanStep({ progress }: { progress: number }) {
  const percent = normalizeProgress(progress);

  return (
    <div className="step-content progress-step">
      <div className="progress-copy">
        <p className="eyebrow">扫描中</p>
        <h2>正在分析 C 盘可清理项目</h2>
        <p>正在识别系统缓存、安装包、常用软件缓存和高风险数据，完成后会自动勾选推荐清理项。</p>
      </div>
      <div className="progress-console" aria-live="polite">
        <strong className="progress-percent">{percent}%</strong>
        <div
          aria-label="扫描进度"
          aria-valuemax={100}
          aria-valuemin={0}
          aria-valuenow={percent}
          className="progress-track"
          role="progressbar"
        >
          <div className="progress-bar" style={{ width: `${percent}%` }} />
        </div>
      </div>
      <div className="scan-grid">
        <ScanProbe icon={<Download size={22} />} label="临时文件和下载缓存" />
        <ScanProbe icon={<PackageCheck size={22} />} label="安装包和旧版本" />
        <ScanProbe icon={<AppWindow size={22} />} label="常见软件缓存" />
        <ScanProbe icon={<ShieldCheck size={22} />} label="配置引用和进程占用" />
      </div>
    </div>
  );
}

function ScanProbe({ icon, label }: { icon: React.ReactNode; label: string }) {
  return (
    <div className="scan-probe">
      <span>{icon}</span>
      <strong>{label}</strong>
    </div>
  );
}

function normalizeProgress(progress: number): number {
  return Math.max(0, Math.min(100, Math.round(progress)));
}
