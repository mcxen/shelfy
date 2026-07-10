import { cn } from "../lib/utils";

interface BrandMarkProps {
  className?: string;
  iconClassName?: string;
  showLabel?: boolean;
}

export function BrandMark({ className, iconClassName, showLabel = false }: BrandMarkProps) {
  return (
    <div className={cn("inline-flex items-center gap-2", className)}>
      <img
        src="/shelfy-app-icon.png"
        alt=""
        aria-hidden="true"
        className={cn("size-8 shrink-0 object-contain", iconClassName)}
      />
      {showLabel && <span className="text-sm font-semibold tracking-[0.01em]">Shelfy</span>}
    </div>
  );
}
