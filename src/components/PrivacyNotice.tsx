export function PrivacyNotice({
  enabled,
  onEnabledChange,
}: {
  enabled: boolean;
  onEnabledChange: (enabled: boolean) => void;
}) {
  return (
    <section className="privacy-notice">
      <h3>匿名统计</h3>
      <p>默认开启匿名统计，只上传规则类别、空间区间和错误类型，不上传完整路径、文件名、用户名或文件内容。</p>
      <label>
        <input checked={enabled} onChange={(event) => onEnabledChange(event.target.checked)} type="checkbox" />
        允许匿名统计帮助改进软件
      </label>
    </section>
  );
}
