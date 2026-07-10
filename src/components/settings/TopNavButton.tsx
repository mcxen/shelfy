import { ReactNode } from "react";
import { Button } from "../ui/button";

interface TopNavButtonProps {
  active: boolean;
  onClick: () => void;
  icon: ReactNode;
  label: string;
}

export function TopNavButton({ active, onClick, icon, label }: TopNavButtonProps) {
  return (
    <Button
      type="button"
      onClick={onClick}
      variant="ghost"
      size="sm"
      aria-current={active ? "page" : undefined}
      className={`h-9 shrink-0 rounded-xl px-3 transition-colors ${
        active
          ? "bg-background/85 text-foreground shadow-sm ring-1 ring-border/70"
          : "text-muted-foreground hover:bg-background/55 hover:text-foreground"
      }`}
    >
      {icon}
      {label}
    </Button>
  );
}
