import { useId, useState, type ReactNode } from "react";

import { CrossfadeIcon } from "../StudioIcons";

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
          <CrossfadeIcon
            name="plus"
            activeName="minus"
            active={open}
            size={15}
          />
        </span>
      </button>
      <div
        className={`ui-disclosure-body${open ? " open" : ""}`}
        id={id}
        aria-hidden={!open}
      >
        <div className="ui-disclosure-inner">{children}</div>
      </div>
    </div>
  );
}
