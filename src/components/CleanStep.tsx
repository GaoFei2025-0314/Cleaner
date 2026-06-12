import { CheckCircle2, RotateCw, ShieldCheck, Trash2 } from "lucide-react";

export function CleanStep({ progress }: { progress: number }) {
  const percent = normalizeProgress(progress);

  return (
    <div className="step-content progress-step">
      <div className="progress-copy">
        <p className="eyebrow">清理中</p>
        <h2>正在逐项清理已选项目</h2>
        <p>推荐项会直接处理；高风险项只有在你二次确认后才会进入清理队列。</p>
      </div>
      <div className="progress-console" aria-live="polite">
        <strong className="progress-percent">{percent}%</strong>
        <div
          aria-label="清理进度"
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
        <ScanProbe icon={<ShieldCheck size={22} />} label="复查安全条件" />
        <ScanProbe icon={<Trash2 size={22} />} label="删除已选缓存" />
        <ScanProbe icon={<RotateCw size={22} />} label="继续处理失败以外项目" />
        <ScanProbe icon={<CheckCircle2 size={22} />} label="生成清理结果" />
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
