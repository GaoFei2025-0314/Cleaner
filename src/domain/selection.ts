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

export function highRiskSelectionChanged(previousIds: string[], nextIds: string[], items: ScanItem[]): boolean {
  const previous = highRiskSelectionKey(previousIds, items);
  const next = highRiskSelectionKey(nextIds, items);
  return previous !== next;
}

export function estimateSelectedBytes(selectedIds: string[], items: ScanItem[]): number {
  const selected = new Set(selectedIds);
  return items.reduce((total, item) => (selected.has(item.id) ? total + item.estimatedBytes : total), 0);
}

function highRiskSelectionKey(selectedIds: string[], items: ScanItem[]): string {
  const selected = new Set(selectedIds);
  return items
    .filter((item) => selected.has(item.id) && item.riskLevel === "highRisk")
    .map((item) => item.id)
    .sort()
    .join("|");
}
