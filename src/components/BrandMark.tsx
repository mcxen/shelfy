import { Squirrel } from "lucide-react";
import { AnimatedIcon } from "./ui/animated-icon";
import { cn } from "../lib/utils";

interface BrandMarkProps {
  className?: string;
  iconClassName?: string;
  showLabel?: boolean;
}

export function BrandMark({ className, iconClassName, showLabel = false }: BrandMarkProps) {
  return (
    <div className={cn("inline-flex items-center gap-2", className)}>
      <span
        className={cn(
          "inline-flex h-8 w-8 items-center justify-center rounded-lg border border-primary/20 bg-primary/10 text-primary shadow-sm",
          iconClassName
        )}
      >
        <AnimatedIcon icon={Squirrel} size={19} strokeWidth={2.2} motion="bounce" />
      </span>
      {showLabel && <span className="text-sm font-semibold tracking-normal">Shelfy</span>}
    </div>
  );
}
