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
      className={`h-8 shrink-0 rounded-md px-2.5 transition-colors ${
        active
          ? "bg-primary text-primary-foreground shadow-sm"
          : "text-muted-foreground hover:bg-accent hover:text-accent-foreground"
      }`}
    >
      {icon}
      {label}
    </Button>
  );
}
