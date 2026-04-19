import type { PropsWithChildren } from "react";
import { cva, type VariantProps } from "class-variance-authority";

import { cn } from "@/lib/utils";

const statusBadgeVariants = cva(
  "inline-flex items-center rounded-full border px-2.5 py-1 text-xs font-semibold tracking-[0.08em] uppercase",
  {
    variants: {
      variant: {
        neutral: "border-border/70 bg-muted/80 text-muted-foreground",
        success:
          "border-emerald-400/25 bg-emerald-400/8 text-[color:oklch(0.86_0.08_160)]",
        warning:
          "border-amber-400/25 bg-amber-400/10 text-[color:oklch(0.86_0.09_85)]",
        danger: "border-rose-400/25 bg-rose-400/10 text-[color:oklch(0.82_0.09_18)]",
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
