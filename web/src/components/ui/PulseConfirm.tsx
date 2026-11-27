export interface PulseConfirmProps {
  active?: boolean;
  className?: string;
}

export function PulseConfirm({
  active = false,
  className = "",
}: PulseConfirmProps) {
  return (
    <span
      className={`pulse-confirm${active ? " active" : ""} ${className}`.trim()}
      aria-hidden="true"
    />
  );
}
