import * as React from "react";
import { Check, ChevronRight } from "lucide-react";
import { cn } from "../../lib/utils";

type MenuContextValue = { open: boolean; setOpen: (open: boolean) => void };
const MenuContext = React.createContext<MenuContextValue | null>(null);

function Menu({ children }: { children: React.ReactNode }) {
  const [open, setOpen] = React.useState(false);
  const ref = React.useRef<HTMLDivElement>(null);
  React.useEffect(() => {
    const close = (event: MouseEvent) => {
      if (!ref.current?.contains(event.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", close);
    return () => document.removeEventListener("mousedown", close);
  }, []);
  return <MenuContext.Provider value={{ open, setOpen }}><div ref={ref} className="relative inline-flex">{children}</div></MenuContext.Provider>;
}

function MenuTrigger({ render, children }: { render?: React.ReactElement<{ onClick?: React.MouseEventHandler }>; children?: React.ReactNode }) {
  const context = React.useContext(MenuContext);
  if (!context) throw new Error("MenuTrigger must be used inside Menu");
  const onClick: React.MouseEventHandler = (event) => {
    event.stopPropagation();
    context.setOpen(!context.open);
  };
  if (render) return React.cloneElement(render, { onClick }, children);
  return <button type="button" onClick={onClick}>{children}</button>;
}

function MenuPopup({ className, children }: React.HTMLAttributes<HTMLDivElement>) {
  const context = React.useContext(MenuContext);
  if (!context?.open) return null;
  return (
    <div className={cn("absolute right-0 top-full z-50 mt-1 min-w-52 rounded-lg border border-border bg-popover p-1 text-popover-foreground shadow-lg", className)}>
      {children}
    </div>
  );
}

function MenuGroup({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return <div className={className} {...props} />;
}
function MenuGroupLabel({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("px-2 py-1.5 text-xs font-medium text-muted-foreground", className)} {...props} />;
}

interface MenuItemProps extends React.ButtonHTMLAttributes<HTMLButtonElement> { variant?: "default" | "destructive" }
function MenuItem({ className, variant = "default", onClick, ...props }: MenuItemProps) {
  const context = React.useContext(MenuContext);
  return (
    <button
      type="button"
      className={cn("flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm outline-none hover:bg-accent disabled:pointer-events-none disabled:opacity-50 [&>svg]:size-4", variant === "destructive" && "text-destructive hover:bg-destructive/10", className)}
      onClick={(event) => { onClick?.(event); if (!event.defaultPrevented) context?.setOpen(false); }}
      {...props}
    />
  );
}
function MenuSeparator({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("-mx-1 my-1 h-px bg-border", className)} {...props} />;
}
function MenuShortcut({ className, ...props }: React.HTMLAttributes<HTMLSpanElement>) {
  return <span className={cn("ml-auto text-xs tracking-widest text-muted-foreground", className)} {...props} />;
}

interface CheckItemProps extends Omit<MenuItemProps, "onChange" | "variant"> { checked?: boolean; variant?: "default" | "destructive" | "switch"; onCheckedChange?: (checked: boolean) => void }
function MenuCheckboxItem({ checked = false, onCheckedChange, variant: _variant, children, ...props }: CheckItemProps) {
  return <MenuItem {...props} onClick={(event) => { event.preventDefault(); onCheckedChange?.(!checked); }}><span className="flex size-4 items-center justify-center">{checked && <Check size={13} />}</span>{children}</MenuItem>;
}
function MenuRadioGroup({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) { return <div className={className} {...props} />; }
function MenuRadioItem({ value: _value, children, ...props }: MenuItemProps & { value: string }) { return <MenuItem {...props}>{children}</MenuItem>; }

function MenuSub({ children }: { children: React.ReactNode }) { return <div className="group/sub relative">{children}</div>; }
function MenuSubTrigger({ children, className, ...props }: MenuItemProps) {
  return <MenuItem className={className} {...props} onClick={(event) => event.preventDefault()}>{children}<ChevronRight className="ml-auto" /></MenuItem>;
}
function MenuSubPopup({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("invisible absolute left-full top-0 z-50 ml-1 min-w-48 rounded-lg border border-border bg-popover p-1 opacity-0 shadow-lg group-hover/sub:visible group-hover/sub:opacity-100", className)} {...props} />;
}

export { Menu, MenuTrigger, MenuPopup, MenuGroup, MenuGroupLabel, MenuItem, MenuSeparator, MenuShortcut, MenuCheckboxItem, MenuRadioGroup, MenuRadioItem, MenuSub, MenuSubTrigger, MenuSubPopup };
