import { Menu as MenuPrimitive } from "@base-ui/react/menu";
import { ChevronRight } from "lucide-react";
import type * as React from "react";
import { cn } from "../../lib/utils";

const Menu = MenuPrimitive.Root;
const MenuPortal = MenuPrimitive.Portal;

function MenuTrigger(props: MenuPrimitive.Trigger.Props) {
  return <MenuPrimitive.Trigger data-slot="menu-trigger" {...props} />;
}

function MenuPopup({
  children,
  className,
  sideOffset = 5,
  align = "end",
  alignOffset,
  side = "bottom",
  collisionBoundary,
  collisionPadding = 8,
  collisionAvoidance = { side: "flip", align: "shift", fallbackAxisSide: "none" },
  positionMethod = "fixed",
  portalProps,
  ...props
}: MenuPrimitive.Popup.Props & {
  align?: MenuPrimitive.Positioner.Props["align"];
  sideOffset?: MenuPrimitive.Positioner.Props["sideOffset"];
  alignOffset?: MenuPrimitive.Positioner.Props["alignOffset"];
  side?: MenuPrimitive.Positioner.Props["side"];
  collisionBoundary?: MenuPrimitive.Positioner.Props["collisionBoundary"];
  collisionPadding?: MenuPrimitive.Positioner.Props["collisionPadding"];
  collisionAvoidance?: MenuPrimitive.Positioner.Props["collisionAvoidance"];
  positionMethod?: MenuPrimitive.Positioner.Props["positionMethod"];
  portalProps?: MenuPrimitive.Portal.Props;
}) {
  const defaultBoundary = typeof document === "undefined" ? undefined : document.documentElement;

  return (
    <MenuPortal {...portalProps}>
      <MenuPrimitive.Positioner
        align={align}
        alignOffset={alignOffset}
        collisionAvoidance={collisionAvoidance}
        collisionBoundary={collisionBoundary ?? defaultBoundary}
        collisionPadding={collisionPadding}
        className="z-50"
        positionMethod={positionMethod}
        side={side}
        sideOffset={sideOffset}
      >
        <MenuPrimitive.Popup
          className={cn(
            "relative flex min-w-44 origin-[var(--transform-origin)] rounded-lg border border-border bg-popover text-popover-foreground shadow-xl outline-none transition-[transform,opacity] data-[ending-style]:scale-95 data-[ending-style]:opacity-0 data-[starting-style]:scale-95 data-[starting-style]:opacity-0",
            className,
          )}
          data-slot="menu-popup"
          {...props}
        >
          <div className="max-h-[var(--available-height)] w-full overflow-y-auto bg-popover p-1">{children}</div>
        </MenuPrimitive.Popup>
      </MenuPrimitive.Positioner>
    </MenuPortal>
  );
}

function MenuGroup(props: MenuPrimitive.Group.Props) {
  return <MenuPrimitive.Group data-slot="menu-group" {...props} />;
}

function MenuGroupLabel({ className, ...props }: MenuPrimitive.GroupLabel.Props) {
  return (
    <MenuPrimitive.GroupLabel
      className={cn("px-2 py-1.5 text-[11px] font-medium text-muted-foreground", className)}
      data-slot="menu-group-label"
      {...props}
    />
  );
}

function MenuItem({
  className,
  variant = "default",
  ...props
}: MenuPrimitive.Item.Props & { variant?: "default" | "destructive" }) {
  return (
    <MenuPrimitive.Item
      className={cn(
        "flex min-h-7 cursor-default select-none items-center gap-2 rounded-md px-2 py-1 text-sm outline-none data-[disabled]:pointer-events-none data-[disabled]:opacity-50 data-[highlighted]:bg-accent data-[highlighted]:text-accent-foreground [&>svg]:size-4 [&>svg]:shrink-0 [&>svg]:opacity-80",
        variant === "destructive" && "text-destructive data-[highlighted]:bg-destructive/12 data-[highlighted]:text-destructive",
        className,
      )}
      data-slot="menu-item"
      {...props}
    />
  );
}

function MenuSeparator({ className, ...props }: MenuPrimitive.Separator.Props) {
  return <MenuPrimitive.Separator className={cn("mx-2 my-1 h-px bg-border", className)} {...props} />;
}

function MenuShortcut({ className, ...props }: React.ComponentProps<"kbd">) {
  return <kbd className={cn("ml-auto text-xs tracking-widest text-muted-foreground", className)} {...props} />;
}

const MenuCheckboxItem = MenuPrimitive.CheckboxItem;
const MenuRadioGroup = MenuPrimitive.RadioGroup;
const MenuRadioItem = MenuPrimitive.RadioItem;
const MenuSub = MenuPrimitive.SubmenuRoot;

function MenuSubTrigger({ className, children, ...props }: MenuPrimitive.SubmenuTrigger.Props) {
  return (
    <MenuPrimitive.SubmenuTrigger
      className={cn("flex min-h-7 items-center gap-2 rounded-md px-2 py-1 text-sm outline-none data-[highlighted]:bg-accent data-[popup-open]:bg-accent", className)}
      {...props}
    >
      {children}<ChevronRight className="ml-auto size-4 opacity-70" />
    </MenuPrimitive.SubmenuTrigger>
  );
}

function MenuSubPopup(props: React.ComponentProps<typeof MenuPopup>) {
  return <MenuPopup align="start" side="inline-end" sideOffset={2} {...props} />;
}

export {
  Menu,
  MenuTrigger,
  MenuPopup,
  MenuGroup,
  MenuGroupLabel,
  MenuItem,
  MenuSeparator,
  MenuShortcut,
  MenuCheckboxItem,
  MenuRadioGroup,
  MenuRadioItem,
  MenuSub,
  MenuSubTrigger,
  MenuSubPopup,
};
