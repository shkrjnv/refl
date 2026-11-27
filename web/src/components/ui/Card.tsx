import type { HTMLAttributes } from "react";

export type CardTone = "plain" | "tinted" | "hero" | "quiet";

export interface CardProps extends HTMLAttributes<HTMLDivElement> {
  tone?: CardTone;
  raised?: boolean;
}

export function Card({
  tone = "plain",
  raised = false,
  className = "",
  ...props
}: CardProps) {
  const classes = [
    "ui-card",
    `ui-card-${tone}`,
    raised ? "ui-card-raised" : "",
    className,
  ]
    .filter(Boolean)
    .join(" ");
  return <div {...props} className={classes} />;
}
