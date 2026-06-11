export function ScanStep() {
  return (
    <div className="step-content">
      <p className="eyebrow">扫描中</p>
      <h2>正在分析 C 盘可清理项目</h2>
      <div className="progress-track">
        <div className="progress-bar" />
      </div>
      <ul className="scan-list">
        <li>临时文件和下载缓存</li>
        <li>安装包和旧版本</li>
        <li>常见软件缓存</li>
        <li>配置引用和进程占用</li>
      </ul>
    </div>
  );
}
