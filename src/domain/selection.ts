import type { ScanItem } from "./models";

function isSelectable(item: ScanItem): boolean {
  return item.riskLevel !== "notCleanable" && item.cleanupAction === "directDelete";
}

export function buildDefaultSelection(items: ScanItem[]): string[] {
  return items.filter((item) => item.defaultSelected && isSelectable(item)).map((item) => item.id);
}

export function toggleSelection(current: string[], item: ScanItem, checked: boolean): string[] {
  if (!isSelectable(item)) {
    return current;
  }

  const currentSet = new Set(current);
  if (checked) {
    currentSet.add(item.id);
  } else {
    currentSet.delete(item.id);
  }
  return Array.from(currentSet);
}

export function requiresHighRiskConfirmation(selectedIds: string[], items: ScanItem[]): boolean {
  const selected = new Set(selectedIds);
  return items.some((item) => selected.has(item.id) && item.riskLevel === "highRisk");
}

export function estimateSelectedBytes(selectedIds: string[], items: ScanItem[]): number {
  const selected = new Set(selectedIds);
  return items.reduce((total, item) => (selected.has(item.id) ? total + item.estimatedBytes : total), 0);
}
