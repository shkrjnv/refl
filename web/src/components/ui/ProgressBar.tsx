export interface ProgressBarProps {
  value: number;
  tone?: "blue" | "sun" | "green" | "purp" | "coral";
  label?: string;
}

export function ProgressBar({
  value,
  tone = "blue",
  label,
}: ProgressBarProps) {
  const clamped = Math.min(100, Math.max(0, value));
  return (
    <div
      className={`ui-progress ui-progress-${tone}`}
      role="progressbar"
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={Math.round(clamped)}
      aria-label={label}
    >
      <span className="ui-progress-fill" style={{ width: `${clamped}%` }} />
    </div>
  );
}
