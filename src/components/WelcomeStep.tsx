import { ScanSearch } from "lucide-react";

export function WelcomeStep({ onStart }: { onStart: () => void }) {
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
      <button className="primary-button" onClick={onStart}>
        <ScanSearch size={18} />
        开始扫描 C 盘
      </button>
    </div>
  );
}
