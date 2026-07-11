import { lazy, Suspense, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { initI18n, SupportedLang } from "./i18n";
import { useAppStore } from "./store/useAppStore";
import { TooltipProvider } from "./components/ui/tooltip";
import { invoke } from "@tauri-apps/api/core";
import { onAction } from "@tauri-apps/plugin-notification";
import { hasTauriRuntime } from "./lib/runtime";

const Popup = lazy(() => import("./components/Popup"));
const Settings = lazy(() => import("./components/Settings"));

function applyTheme(theme: string) {
  const root = document.documentElement;
  if (theme === "dark") {
    root.classList.add("dark");
  } else if (theme === "light") {
    root.classList.remove("dark");
  } else {
    const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
    if (prefersDark) {
      root.classList.add("dark");
    } else {
      root.classList.remove("dark");
    }
  }
}

function App() {
  const { t } = useTranslation();
  const [ready, setReady] = useState(false);
  const { loadSettings, settings } = useAppStore();

  const [hash, setHash] = useState(() => window.location.hash.replace("#/", "") || "popup");
  const route = hash.split("?", 1)[0];

  useEffect(() => {
    const handleHashChange = () => setHash(window.location.hash.replace("#/", "") || "popup");
    window.addEventListener("hashchange", handleHashChange);
    return () => window.removeEventListener("hashchange", handleHashChange);
  }, []);

  useEffect(() => {
    async function boot() {
      await loadSettings();
    }
    boot();
  }, [loadSettings]);

  useEffect(() => {
    if (!settings) return;
    const lang = (settings.language || "en") as SupportedLang;
    initI18n(lang).then(() => setReady(true));
  }, [settings]);

  useEffect(() => {
    if (!settings) return;
    applyTheme(settings.theme);
  }, [settings?.theme]);

  useEffect(() => {
    if (!hasTauriRuntime()) return;
    let actionListener: { unregister: () => Promise<void> } | null = null;

    // Listen for notification action clicks centrally
    onAction((notification) => {
      console.log("Notification click received:", notification);
      const destFolder = (notification.extra as Record<string, unknown> | undefined)?.destFolder as string | undefined;
      if (destFolder) {
        invoke("open_folder_cmd", { path: destFolder })
          .catch((e) => console.error("open_folder_cmd from onAction failed:", e));
      }
    }).then((listener) => {
      actionListener = listener;
    }).catch(console.error);

    // When the popup window gains focus (e.g. after notification click brings app forward),
    // check if there's a pending folder to open. This covers the case where the app window
    // was already visible and single-instance handler didn't fire.
    const handleFocus = async () => {
      try {
        const folder = await invoke<string | null>("get_pending_open_folder_cmd");
        if (folder) {
          // Open the folder in Explorer
          await invoke("open_folder_cmd", { path: folder }).catch(console.error);
        }
      } catch (e) {
        console.error("get_pending_open_folder_cmd error:", e);
      }
    };

    window.addEventListener("focus", handleFocus);

    return () => {
      window.removeEventListener("focus", handleFocus);
      if (actionListener) {
        actionListener.unregister().catch(console.error);
      }
    };
  }, []);

  if (!ready) {
    return (
      <div className="flex h-full items-center justify-center bg-background text-foreground">
        <div className="animate-pulse text-sm">{t("app.loading")}</div>
      </div>
    );
  }

  return (
    <TooltipProvider delayDuration={250}>
      <div className="h-full w-full isolate overflow-hidden rounded-xl bg-background/72 text-foreground ring-1 ring-border/60 shadow-2xl">
        <Suspense fallback={<div className="flex h-full items-center justify-center text-sm text-muted-foreground">{t("app.loading")}</div>}>
          {route === "settings" ? <Settings /> : <Popup />}
        </Suspense>
      </div>
    </TooltipProvider>
  );
}

export default App;
