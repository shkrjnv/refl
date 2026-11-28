import { StudioIcon } from "./StudioIcons";

export interface BookCoverProps {
  title: string;
  level: string;
  coverUrl: string | null;
  onEdit?: () => void;
  editLabel?: string;
}

export function BookCover({
  title,
  level,
  coverUrl,
  onEdit,
  editLabel,
}: BookCoverProps) {
  return (
    <div className={`book-cover book-cover-${level.toLocaleLowerCase()}`}>
      {coverUrl ? (
        <img src={coverUrl} alt="" loading="lazy" />
      ) : (
        <div className="book-cover-fallback">
          <span className="book-cover-kicker">refl · library</span>
          <strong>{title}</strong>
          <span>{level}</span>
        </div>
      )}
      {onEdit ? (
        <button
          type="button"
          className="book-cover-edit"
          aria-label={editLabel}
          onClick={onEdit}
        >
          <StudioIcon name="camera" size={16} />
        </button>
      ) : null}
    </div>
  );
}
