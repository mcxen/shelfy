import { Dialog as DialogPrimitive } from "@base-ui/react/dialog";
import type * as React from "react";
import { cn } from "../../lib/utils";

const Dialog = DialogPrimitive.Root;
const DialogTrigger = DialogPrimitive.Trigger;
const DialogClose = DialogPrimitive.Close;

function DialogPopup({ className, ...props }: DialogPrimitive.Popup.Props) {
  return (
    <DialogPrimitive.Portal>
      <DialogPrimitive.Backdrop className="fixed inset-0 z-50 bg-black/45 backdrop-blur-[2px] transition-opacity data-[ending-style]:opacity-0 data-[starting-style]:opacity-0" />
      <DialogPrimitive.Viewport className="fixed inset-0 z-50 grid place-items-center p-4">
        <DialogPrimitive.Popup className={cn("flex max-h-[min(36rem,calc(100vh-2rem))] w-full max-w-md flex-col rounded-lg border border-border bg-popover text-popover-foreground shadow-2xl outline-none transition-[transform,opacity] data-[ending-style]:scale-95 data-[ending-style]:opacity-0 data-[starting-style]:scale-95 data-[starting-style]:opacity-0", className)} {...props} />
      </DialogPrimitive.Viewport>
    </DialogPrimitive.Portal>
  );
}

function DialogHeader({ className, ...props }: React.ComponentProps<"div">) { return <div className={cn("space-y-1.5 p-4", className)} {...props} />; }
function DialogPanel({ className, ...props }: React.ComponentProps<"div">) { return <div className={cn("min-h-0 overflow-y-auto px-4 pb-4", className)} {...props} />; }
function DialogFooter({ className, ...props }: React.ComponentProps<"div">) { return <div className={cn("flex justify-end gap-2 border-t border-border bg-muted/45 px-4 py-3", className)} {...props} />; }
function DialogTitle({ className, ...props }: DialogPrimitive.Title.Props) { return <DialogPrimitive.Title className={cn("text-base font-semibold", className)} {...props} />; }
function DialogDescription({ className, ...props }: DialogPrimitive.Description.Props) { return <DialogPrimitive.Description className={cn("text-sm text-muted-foreground", className)} {...props} />; }

export { Dialog, DialogTrigger, DialogClose, DialogPopup, DialogHeader, DialogPanel, DialogFooter, DialogTitle, DialogDescription };
