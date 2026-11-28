import type { ButtonHTMLAttributes } from "react";

export type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";
export type ButtonSize = "sm" | "md" | "lg";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: ButtonSize;
  block?: boolean;
  busy?: boolean;
}

export function Button({
  variant = "secondary",
  size = "md",
  block = false,
  busy = false,
  className = "",
  disabled,
  children,
  ...props
}: ButtonProps) {
  const classes = [
    "ui-button",
    `ui-button-${variant}`,
    `ui-button-${size}`,
    block ? "ui-button-block" : "",
    className,
  ]
    .filter(Boolean)
    .join(" ");
  return (
    <button
      {...props}
      className={classes}
      disabled={disabled || busy}
      aria-busy={busy || undefined}
    >
      {children}
    </button>
  );
}
