import type { PropsWithChildren } from "react";
import { cva, type VariantProps } from "class-variance-authority";

import { cn } from "@/lib/utils";

const statusBadgeVariants = cva(
  "inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-medium",
  {
    variants: {
      variant: {
        neutral: "border-border bg-muted text-foreground",
        success: "border-emerald-400/40 bg-emerald-500/15 text-emerald-700",
        warning: "border-amber-400/50 bg-amber-500/20 text-amber-700",
        danger: "border-rose-400/45 bg-rose-500/20 text-rose-700",
      },
    },
    defaultVariants: {
      variant: "neutral",
    },
  },
);

export function StatusBadge({
  className,
  variant,
  children,
}: PropsWithChildren<
  {
    className?: string;
  } & VariantProps<typeof statusBadgeVariants>
>) {
  return (
    <span className={cn(statusBadgeVariants({ variant }), className)}>{children}</span>
  );
}
