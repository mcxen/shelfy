import { motion, useReducedMotion } from "motion/react";
import type { LucideIcon } from "lucide-react";
import { cn } from "../../lib/utils";

type IconMotion = "draw" | "bounce" | "pulse" | "tilt" | "float";

const hoverMotion: Record<IconMotion, Record<string, number | string | number[]>> = {
  draw: { scale: 1.06, rotate: -3 },
  bounce: { y: [0, -3, 0], scale: 1.05 },
  pulse: { scale: [1, 1.12, 1] },
  tilt: { rotate: [-4, 4, 0], scale: 1.04 },
  float: { y: -2, scale: 1.04 },
};

interface AnimatedIconProps {
  icon: LucideIcon;
  className?: string;
  size?: number;
  strokeWidth?: number;
  motion?: IconMotion;
  title?: string;
}

export function AnimatedIcon({
  icon: Icon,
  className,
  size = 16,
  strokeWidth = 2,
  motion: motionName = "draw",
  title,
}: AnimatedIconProps) {
  const reduceMotion = useReducedMotion();

  return (
    <motion.span
      aria-hidden={title ? undefined : true}
      aria-label={title}
      className={cn("inline-flex items-center justify-center", className)}
      whileHover={reduceMotion ? undefined : hoverMotion[motionName]}
      whileTap={reduceMotion ? undefined : { scale: 0.94 }}
      transition={{ duration: 0.28, ease: [0.16, 1, 0.3, 1] }}
    >
      <Icon size={size} strokeWidth={strokeWidth} />
    </motion.span>
  );
}
