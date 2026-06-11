export function ErrorPanel({
  message,
  onRetry,
  onDismiss,
}: {
  message: string;
  onRetry: () => void;
  onDismiss: () => void;
}) {
  return (
    <section className="error-panel" role="alert">
      <div>
        <p className="eyebrow">遇到问题</p>
        <h3>本次操作没有完成</h3>
        <p>{message}</p>
      </div>
      <div className="button-row">
        <button className="secondary-button" onClick={onDismiss}>
          先不处理
        </button>
        <button className="primary-button" onClick={onRetry}>
          重试
        </button>
      </div>
    </section>
  );
}
