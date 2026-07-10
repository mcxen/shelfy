import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { Download, ExternalLink, Heart } from "lucide-react";
import { BrandMark } from "./BrandMark";
import { Button } from "./ui/button";
import { Card } from "./ui/card";
import { Separator } from "./ui/separator";
import { AnimatedIcon } from "./ui/animated-icon";

export default function About() {
  const { t } = useTranslation();

  return (
    <div className="space-y-6 max-w-md">
      {/* Logo + tagline */}
      <div className="flex items-center gap-3">
        <BrandMark iconClassName="h-12 w-12 rounded-xl [&_svg]:size-7" />
        <div>
          <h2 className="text-xl font-semibold">Shelfy</h2>
          <p className="text-sm text-muted-foreground">{t("settings.about.tagline")}</p>
        </div>
      </div>

      <p className="text-sm text-muted-foreground leading-relaxed">
        {t("settings.about.description")}
      </p>

      {/* Check for Updates */}
      <Button
        onClick={() =>
          invoke("open_folder_cmd", {
            path: "https://shelfy.cc/#download",
          })
        }
        variant="outline"
        className="w-full justify-start"
      >
        <AnimatedIcon icon={Download} size={16} className="text-primary" motion="float" />
        {t("settings.about.checkUpdates")}
        <ExternalLink size={14} className="ml-auto text-muted-foreground" />
      </Button>

      {/* Author */}
      <div className="space-y-2">
        <Separator />
        <Button
          onClick={() =>
            invoke("open_folder_cmd", { path: "https://github.com/hsr88" })
          }
          variant="link"
          className="h-auto px-0"
        >
          <ExternalLink size={16} />
          github.com/hsr88
        </Button>
        <p className="text-xs text-muted-foreground">{t("settings.about.builtBy")}</p>
      </div>

      {/* Ko-fi - big & bold */}
      <Card className="p-4 text-center">
        <Button
          onClick={() =>
            invoke("open_folder_cmd", { path: "https://ko-fi.com/hsr" })
          }
          size="lg"
          className="w-full"
        >
          <AnimatedIcon icon={Heart} size={18} motion="pulse" />
          {t("settings.about.support")}
        </Button>
        <p className="text-xs text-muted-foreground mt-3">
          {t("settings.about.supportDesc")}
        </p>
      </Card>
    </div>
  );
}
