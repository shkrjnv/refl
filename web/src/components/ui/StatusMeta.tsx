import type { ReactNode } from "react";

export type StatusTone =
  | "neutral"
  | "accent"
  | "success"
  | "warning"
  | "danger";

export interface StatusMetaProps {
  tone?: StatusTone;
  dot?: boolean;
  className?: string;
  children: ReactNode;
}

export function StatusMeta({
  tone = "neutral",
  dot = false,
  className = "",
  children,
}: StatusMetaProps) {
  const classes = [
    "status-meta",
    `status-meta-${tone}`,
    dot ? "status-meta-dot" : "",
    className,
  ]
    .filter(Boolean)
    .join(" ");
  return <span className={classes}>{children}</span>;
}
