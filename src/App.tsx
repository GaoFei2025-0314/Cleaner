import { useState } from "react";
import type { CleanupResult, ScanReport } from "./domain/models";
import { buildDefaultSelection } from "./domain/selection";
import { executeCleanup, scanCDrive } from "./services/tauriApi";
import { AppShell } from "./components/AppShell";
import { WelcomeStep } from "./components/WelcomeStep";
import { ScanStep } from "./components/ScanStep";
import { SuggestionsStep } from "./components/SuggestionsStep";
import { ConfirmStep } from "./components/ConfirmStep";
import { CleanStep } from "./components/CleanStep";
import { ResultStep } from "./components/ResultStep";
import { ErrorPanel } from "./components/ErrorPanel";

type Step = "welcome" | "scan" | "suggestions" | "confirm" | "clean" | "result";
type FailedAction = "scan" | "clean";

const stepIndex: Record<Step, number> = {
  welcome: 0,
  scan: 0,
  suggestions: 1,
  confirm: 2,
  clean: 3,
  result: 4,
};

export default function App() {
  const [step, setStep] = useState<Step>("welcome");
  const [report, setReport] = useState<ScanReport | null>(null);
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const [view, setView] = useState<"risk" | "source">("risk");
  const [highRiskConfirmed, setHighRiskConfirmed] = useState(false);
  const [result, setResult] = useState<CleanupResult | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [failedAction, setFailedAction] = useState<FailedAction | null>(null);
  const [analyticsEnabled, setAnalyticsEnabled] = useState(
    () => localStorage.getItem("analyticsEnabled") !== "false",
  );

  async function startScan() {
    setErrorMessage(null);
    setFailedAction(null);
    setStep("scan");
    try {
      const nextReport = await scanCDrive();
      setReport(nextReport);
      setSelectedIds(buildDefaultSelection(nextReport.items));
      setHighRiskConfirmed(false);
      setStep("suggestions");
    } catch (error) {
      setErrorMessage(toUserMessage(error));
      setFailedAction("scan");
      setStep("welcome");
    }
  }

  async function confirmCleanup() {
    if (!report) return;
    setErrorMessage(null);
    setFailedAction(null);
    setStep("clean");
    try {
      const nextResult = await executeCleanup({
        selectedItemIds: selectedIds,
        highRiskConfirmed,
        requestAdminMode: false,
      });
      setResult(nextResult);
      setStep("result");
    } catch (error) {
      setErrorMessage(toUserMessage(error));
      setFailedAction("clean");
      setStep("confirm");
    }
  }

  function restart() {
    setReport(null);
    setSelectedIds([]);
    setResult(null);
    setHighRiskConfirmed(false);
    setErrorMessage(null);
    setFailedAction(null);
    setStep("welcome");
  }

  function retryFailedAction() {
    if (failedAction === "scan") {
      void startScan();
    }
    if (failedAction === "clean") {
      void confirmCleanup();
    }
  }

  function updateAnalyticsEnabled(enabled: boolean) {
    setAnalyticsEnabled(enabled);
    localStorage.setItem("analyticsEnabled", String(enabled));
  }

  return (
    <AppShell currentStep={stepIndex[step]} report={report}>
      {errorMessage && (
        <ErrorPanel
          message={errorMessage}
          onDismiss={() => {
            setErrorMessage(null);
            setFailedAction(null);
          }}
          onRetry={retryFailedAction}
        />
      )}
      {step === "welcome" && (
        <WelcomeStep
          analyticsEnabled={analyticsEnabled}
          onAnalyticsEnabledChange={updateAnalyticsEnabled}
          onStart={() => void startScan()}
        />
      )}
      {step === "scan" && <ScanStep />}
      {step === "suggestions" && report && (
        <SuggestionsStep
          items={report.items}
          selectedIds={selectedIds}
          view={view}
          onViewChange={setView}
          onSelectionChange={setSelectedIds}
          onNext={() => setStep("confirm")}
        />
      )}
      {step === "confirm" && report && (
        <ConfirmStep
          items={report.items}
          selectedIds={selectedIds}
          highRiskConfirmed={highRiskConfirmed}
          onHighRiskConfirmed={setHighRiskConfirmed}
          onBack={() => setStep("suggestions")}
          onConfirm={() => void confirmCleanup()}
        />
      )}
      {step === "clean" && <CleanStep />}
      {step === "result" && result && <ResultStep result={result} onRestart={restart} />}
    </AppShell>
  );
}

function toUserMessage(error: unknown): string {
  if (error instanceof Error && error.message.trim()) {
    return error.message;
  }
  if (typeof error === "string" && error.trim()) {
    return error;
  }
  return "系统返回了未知错误，本次没有删除任何新项目。";
}
