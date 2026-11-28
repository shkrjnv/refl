import type { CompanionId } from "../companions/registry";
import { IellaSprite, type IellaMood } from "./IellaSprite";
import { SendoSprite, type SendoMood } from "./SendoSprite";

export interface CompanionSpriteProps {
  companion: CompanionId;
  size?: number;
  className?: string;
  decorative?: boolean;
  label?: string;
  mood?: "idle" | "perched" | "curious" | "proud" | SendoMood;
  onActivate?: () => void;
}

/** One sprite contract for every companion; the registry chooses the art. */
export function CompanionSprite({
  companion,
  size = 60,
  className = "",
  decorative = true,
  label,
  mood = "idle",
  onActivate,
}: CompanionSpriteProps) {
  if (companion === "sendo") {
    return onActivate ? (
      <SendoSprite
        size={size}
        className={className}
        label={label ?? "Sendo"}
        mood={mood as SendoMood}
        onActivate={onActivate}
      />
    ) : (
      <SendoSprite
        size={size}
        className={className}
        decorative={decorative}
        label={label}
        mood={mood as SendoMood}
      />
    );
  }
  return onActivate ? (
    <IellaSprite
      size={size}
      className={className}
      label={label ?? "Iella"}
      mood={mood as IellaMood}
      onActivate={onActivate}
    />
  ) : (
    <IellaSprite
      size={size}
      className={className}
      decorative={decorative}
      label={label}
      mood={mood as IellaMood}
    />
  );
}
