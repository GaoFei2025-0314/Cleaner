import { Plus, Save, Trash2 } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import type { CleanerSettings, DuplicateDefaultStrategy } from "../../domain/v2";
import { getDefaultCleanerSettings, saveCleanerSettings } from "../../services/v2Api";

const thresholdOptions = [
  { label: "100 MB", value: 100 * 1024 * 1024 },
  { label: "500 MB", value: 500 * 1024 * 1024 },
  { label: "1 GB", value: 1024 * 1024 * 1024 },
  { label: "Custom", value: "custom" },
] as const;

const duplicateStrategies: Array<{ label: string; value: DuplicateDefaultStrategy }> = [
  { label: "C 盘优先保留较新", value: "cDriveFirstKeepNewest" },
  { label: "保留最新", value: "keepNewest" },
  { label: "保留最旧", value: "keepOldest" },
];

export function SettingsPage() {
  const [settings, setSettings] = useState<CleanerSettings | null>(null);
  const [protectedPathInput, setProtectedPathInput] = useState("");
  const [customThresholdMb, setCustomThresholdMb] = useState(500);
  const [status, setStatus] = useState("正在加载设置...");

  useEffect(() => {
    let cancelled = false;
    void getDefaultCleanerSettings()
      .then((nextSettings) => {
        if (cancelled) return;
        const safeSettings = sanitizeSettings(nextSettings, defaultFallbackSettings());
        setSettings(safeSettings);
        setCustomThresholdMb(bytesToMb(safeSettings.largeFileDefaultThresholdBytes));
        setStatus("设置已载入");
      })
      .catch(() => {
        if (cancelled) return;
        setStatus("设置加载失败，请稍后重试");
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const selectedThreshold = useMemo(() => {
    if (!settings) return 500 * 1024 * 1024;
    return thresholdOptions.some(
      (option) => typeof option.value === "number" && option.value === settings.largeFileDefaultThresholdBytes,
    )
      ? settings.largeFileDefaultThresholdBytes
      : "custom";
  }, [settings]);

  if (!settings) {
    return <p className="tool-status">{status}</p>;
  }

  function updateSettings(nextSettings: CleanerSettings) {
    setSettings(nextSettings);
  }

  function toggleDrive(drive: string, checked: boolean) {
    if (!settings) return;
    const nextDrives = checked
      ? Array.from(new Set([...settings.defaultScanDrives, drive]))
      : settings.defaultScanDrives.filter((item) => item !== drive);
    updateSettings({ ...settings, defaultScanDrives: nextDrives.length ? nextDrives : ["C:"] });
  }

  function addProtectedPath() {
    if (!settings) return;
    const nextPath = protectedPathInput.trim();
    if (!nextPath) return;
    updateSettings({
      ...settings,
      protectedPaths: Array.from(new Set([...settings.protectedPaths, nextPath])),
    });
    setProtectedPathInput("");
  }

  async function save() {
    if (!settings) return;
    setStatus("正在保存...");
    const safeSettings = sanitizeSettings(settings, settings);
    setSettings(safeSettings);
    setCustomThresholdMb(bytesToMb(safeSettings.largeFileDefaultThresholdBytes));
    try {
      const saved = await saveCleanerSettings(safeSettings);
      const safeSaved = sanitizeSettings(saved, safeSettings);
      setSettings(safeSaved);
      setCustomThresholdMb(bytesToMb(safeSaved.largeFileDefaultThresholdBytes));
      setStatus("设置已保存");
    } catch {
      setStatus("设置保存失败，本次未修改设置");
    }
  }

  return (
    <div className="tool-page settings-page">
      <header className="tool-header">
        <div>
          <p className="eyebrow">Settings</p>
          <h2>设置</h2>
        </div>
        <button className="primary-button" type="button" onClick={() => void save()}>
          <Save size={17} />
          保存设置
        </button>
      </header>

      <section className="settings-section">
        <h3>默认扫描磁盘</h3>
        <div className="check-grid">
          {["C:", "D:", "E:"].map((drive) => (
            <label key={drive} className="check-line">
              <input
                checked={settings.defaultScanDrives.includes(drive)}
                type="checkbox"
                onChange={(event) => toggleDrive(drive, event.currentTarget.checked)}
              />
              <span>{drive}</span>
            </label>
          ))}
        </div>
      </section>

      <section className="settings-section">
        <h3>重复文件默认策略</h3>
        <div className="segmented settings-segmented">
          {duplicateStrategies.map((strategy) => (
            <button
              key={strategy.value}
              data-active={settings.duplicateDefaultStrategy === strategy.value}
              type="button"
              onClick={() => updateSettings({ ...settings, duplicateDefaultStrategy: strategy.value })}
            >
              {strategy.label}
            </button>
          ))}
        </div>
      </section>

      <section className="settings-section">
        <h3>大文件阈值</h3>
        <div className="radio-row">
          {thresholdOptions.map((option) => (
            <label key={option.label} className="check-line">
              <input
                checked={selectedThreshold === option.value}
                name="large-file-threshold"
                type="radio"
                onChange={() => {
                  if (option.value === "custom") {
                    const nextThreshold = toPositiveInteger(customThresholdMb, 500);
                    setCustomThresholdMb(nextThreshold);
                    updateSettings({ ...settings, largeFileDefaultThresholdBytes: nextThreshold * 1024 * 1024 });
                  } else {
                    updateSettings({ ...settings, largeFileDefaultThresholdBytes: option.value });
                    setCustomThresholdMb(bytesToMb(option.value));
                  }
                }}
              />
              <span>{option.label}</span>
            </label>
          ))}
          <label className="inline-field">
            <span>自定义 MB</span>
            <input
              min={1}
              type="number"
              value={customThresholdMb}
              onChange={(event) => {
                const nextValue = toPositiveInteger(event.currentTarget.value, customThresholdMb || 500);
                setCustomThresholdMb(nextValue);
                updateSettings({ ...settings, largeFileDefaultThresholdBytes: nextValue * 1024 * 1024 });
              }}
            />
          </label>
        </div>
      </section>

      <section className="settings-section">
        <h3>历史记录</h3>
        <label className="inline-field">
          <span>历史保留天数</span>
          <input
            aria-label="历史保留天数"
            min={1}
            type="number"
            value={settings.historyRetentionDays}
            onChange={(event) =>
              updateSettings({
                ...settings,
                historyRetentionDays: toPositiveInteger(event.currentTarget.value, settings.historyRetentionDays || 30),
              })
            }
          />
        </label>
      </section>

      <section className="settings-section">
        <h3>受保护路径</h3>
        <div className="path-editor">
          <input
            aria-label="新增受保护路径"
            placeholder="输入要保护的位置"
            value={protectedPathInput}
            onChange={(event) => setProtectedPathInput(event.currentTarget.value)}
          />
          <button className="secondary-button icon-button" type="button" onClick={addProtectedPath}>
            <Plus size={17} />
            添加
          </button>
        </div>
        <div className="protected-list">
          {settings.protectedPaths.length === 0 && <span>尚未添加受保护路径</span>}
          {settings.protectedPaths.map((path) => (
            <div key={path} className="protected-row">
              <span>{path}</span>
              <button
                aria-label={`移除 ${path}`}
                className="icon-only-button"
                type="button"
                onClick={() =>
                  updateSettings({
                    ...settings,
                    protectedPaths: settings.protectedPaths.filter((item) => item !== path),
                  })
                }
              >
                <Trash2 size={16} />
              </button>
            </div>
          ))}
        </div>
      </section>

      <section className="settings-section">
        <h3>系统集成</h3>
        <p className="tool-status">V0.2 仅保留入口，不修改系统设置</p>
        <div className="switch-list">
          <DisabledSwitch label="桌面快捷方式" />
          <DisabledSwitch label="C 盘右键菜单" />
          <DisabledSwitch label="定时扫描提醒" />
        </div>
      </section>

      <p className="tool-status" aria-live="polite">
        {status}
      </p>
    </div>
  );
}

function bytesToMb(bytes: number): number {
  return toPositiveInteger(Math.round(bytes / 1024 / 1024), 500);
}

function toPositiveInteger(value: string | number, fallback: number): number {
  const parsed = typeof value === "number" ? value : Number(value);
  if (!Number.isFinite(parsed) || parsed < 1) {
    return Math.max(1, Math.trunc(fallback) || 1);
  }
  return Math.max(1, Math.trunc(parsed));
}

function sanitizeSettings(settings: CleanerSettings, fallback: CleanerSettings): CleanerSettings {
  const thresholdMb = toPositiveInteger(
    bytesToMb(settings.largeFileDefaultThresholdBytes),
    bytesToMb(fallback.largeFileDefaultThresholdBytes),
  );

  return {
    ...settings,
    defaultScanDrives: settings.defaultScanDrives.length ? settings.defaultScanDrives : ["C:"],
    largeFileDefaultThresholdBytes: thresholdMb * 1024 * 1024,
    historyRetentionDays: toPositiveInteger(settings.historyRetentionDays, fallback.historyRetentionDays),
  };
}

function defaultFallbackSettings(): CleanerSettings {
  return {
    protectedPaths: [],
    defaultScanDrives: ["C:"],
    duplicateDefaultStrategy: "cDriveFirstKeepNewest",
    largeFileDefaultThresholdBytes: 500 * 1024 * 1024,
    historyRetentionDays: 30,
    desktopShortcutEnabled: false,
    cDriveContextMenuEnabled: false,
    scheduledScanReminderEnabled: false,
  };
}

function DisabledSwitch({ label }: { label: string }) {
  return (
    <label className="switch-line">
      <span>{label}</span>
      <input aria-label={label} disabled role="switch" type="checkbox" />
    </label>
  );
}
