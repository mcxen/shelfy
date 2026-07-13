import type { TFunction } from "i18next";

export function ordenOperationLabel(t: TFunction, value: string): string {
  const normalized = value.trim().toLowerCase();
  if (!normalized) return value;
  if (normalized === "ignore") return t("settings.rules.actionIgnore", { defaultValue: value });
  const sender = t(`settings.orden.workflow.senders.${normalized}`, { defaultValue: value });
  return t(`settings.orden.workflow.steps.${normalized}.label`, { defaultValue: sender });
}

export function ordenLevelLabel(t: TFunction, value: string): string {
  return t(`settings.orden.workflow.levels.${value.trim().toLowerCase()}`, { defaultValue: value });
}
