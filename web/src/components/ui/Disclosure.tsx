import { useId, useState, type ReactNode } from "react";

export interface DisclosureProps {
  summary: string;
  children: ReactNode;
}

export function Disclosure({ summary, children }: DisclosureProps) {
  const [open, setOpen] = useState(false);
  const id = useId();
  return (
    <div className={`ui-disclosure${open ? " open" : ""}`}>
      <button
        type="button"
        className="ui-disclosure-toggle"
        aria-expanded={open}
        aria-controls={id}
        onClick={() => setOpen((value) => !value)}
      >
        <span>{summary}</span>
        <span className="ui-disclosure-mark" aria-hidden="true">
          {open ? "−" : "+"}
        </span>
      </button>
      {open ? (
        <div className="ui-disclosure-body" id={id}>
          {children}
        </div>
      ) : null}
    </div>
  );
}
