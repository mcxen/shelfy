type TauriWindow = Window & {
  __TAURI_INTERNALS__?: {
    invoke?: unknown;
  };
};

export function hasTauriRuntime(): boolean {
  return typeof window !== "undefined"
    && typeof (window as TauriWindow).__TAURI_INTERNALS__?.invoke === "function";
}
