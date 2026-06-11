import { Check, Circle, LoaderCircle } from "lucide-react";

const steps = ["扫描", "建议", "确认", "清理", "结果"];

export function StepIndicator({ currentStep }: { currentStep: number }) {
  return (
    <ol className="step-indicator">
      {steps.map((step, index) => {
        const complete = index < currentStep;
        const active = index === currentStep;
        return (
          <li data-active={active} data-complete={complete} key={step}>
            <span className="step-icon">
              {complete && <Check size={16} />}
              {active && !complete && <LoaderCircle size={16} />}
              {!active && !complete && <Circle size={16} />}
            </span>
            <span>{step}</span>
          </li>
        );
      })}
    </ol>
  );
}
