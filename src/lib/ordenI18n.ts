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

export function ordenJobModeLabel(t: TFunction, jobMode: string): string {
  const normalizedJobMode = jobMode.trim().toLowerCase();
  if (!normalizedJobMode) return jobMode;
  return t(`settings.orden.jobModes.${normalizedJobMode}`, { defaultValue: jobMode });
}

export function ordenRunTriggerLabel(t: TFunction, runTrigger: string): string {
  const normalizedRunTrigger = runTrigger.trim().toLowerCase();
  if (!normalizedRunTrigger) return runTrigger;
  return t(`settings.orden.runTriggers.${normalizedRunTrigger}`, { defaultValue: runTrigger });
}
