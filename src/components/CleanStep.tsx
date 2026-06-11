export function CleanStep() {
  return (
    <div className="step-content">
      <p className="eyebrow">清理中</p>
      <h2>正在逐项清理并复查安全条件</h2>
      <div className="progress-track">
        <div className="progress-bar" />
      </div>
      <p>如果某一项失败，软件会继续处理其他项目，并在结果页说明原因。</p>
    </div>
  );
}
