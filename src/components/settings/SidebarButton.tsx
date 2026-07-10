import { ReactNode } from "react";
import { Button } from "../ui/button";

interface SidebarButtonProps {
  active: boolean;
  onClick: () => void;
  icon: ReactNode;
  label: string;
}

export function SidebarButton({ active, onClick, icon, label }: SidebarButtonProps) {
  return (
    <Button
      onClick={onClick}
      variant="ghost"
      className={`w-full justify-start transition-colors ${
        active
          ? "bg-primary/10 text-primary shadow-sm"
          : "text-muted-foreground hover:bg-accent hover:text-accent-foreground"
      }`}
    >
      {icon}
      {label}
    </Button>
  );
}
