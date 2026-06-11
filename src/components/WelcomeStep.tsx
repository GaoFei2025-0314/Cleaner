import { ScanSearch } from "lucide-react";
import { PrivacyNotice } from "./PrivacyNotice";

export function WelcomeStep({
  onStart,
  analyticsEnabled,
  onAnalyticsEnabledChange,
}: {
  onStart: () => void;
  analyticsEnabled: boolean;
  onAnalyticsEnabledChange: (enabled: boolean) => void;
}) {
  return (
    <div className="step-content welcome-panel">
      <p className="eyebrow">开始前</p>
      <h2>先扫描，不会直接删除任何文件</h2>
      <p>软件会检查 C 盘里的临时文件、缓存、安装包、常见软件数据和配置引用风险。</p>
      <div className="assurance-grid">
        <div>默认只勾选推荐清理项</div>
        <div>高风险项目需要二次确认</div>
        <div>被配置引用的目录不会清理</div>
      </div>
      <PrivacyNotice enabled={analyticsEnabled} onEnabledChange={onAnalyticsEnabledChange} />
      <section className="admin-note">
        <h3>系统轻量清理</h3>
        <p>普通模式不需要管理员权限。V0.1 只展示管理员清理能力说明，不执行提权清理；后续版本会在你主动选择后再请求管理员权限。</p>
      </section>
      <button className="primary-button" onClick={onStart}>
        <ScanSearch size={18} />
        开始扫描 C 盘
      </button>
    </div>
  );
}
