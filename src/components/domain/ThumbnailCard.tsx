import { memo } from "react";
import { Link } from "react-router-dom";

import { cx } from "../../lib/cx";
import { toAssetUrl } from "../../lib/tauri";
import { BoxSummary, ImageCard } from "../../types/workspace";

type ThumbnailCardProps = {
  image: ImageCard;
  selected: boolean;
  onToggle: (imageId: string, modifiers?: { shiftKey?: boolean }) => void;
  workspaceId: string;
};

function formatAreaRatio(value?: number | null) {
  if (value == null) {
    return null;
  }

  return `${(value * 100).toFixed(1)}%`;
}

function formatBoxSummary(summary: BoxSummary) {
  const ratioLabel = formatAreaRatio(summary.areaRatio);
  return ratioLabel ? `${summary.categoryName} (${ratioLabel})` : summary.categoryName;
}

function ThumbnailCardComponent({
  image,
  selected,
  onToggle,
  workspaceId,
}: ThumbnailCardProps) {
  const assetUrl = toAssetUrl(image.originalPath);
  const maxRatioLabel = formatAreaRatio(image.maxBoxAreaRatio);
  const boxSummaryLabel = image.boxSummaries.map(formatBoxSummary).join(", ");

  return (
    <article
      className={cx(
        "thumbnail-card",
        image.annotationStatus === "annotated"
          ? "thumbnail-card-annotated"
          : "thumbnail-card-unannotated",
        selected && "thumbnail-card-selected",
      )}
      onClick={(event) => onToggle(image.id, { shiftKey: event.shiftKey })}
      onKeyDown={(event) => {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          onToggle(image.id, { shiftKey: event.shiftKey });
        }
      }}
      role="button"
      tabIndex={0}
    >
      <div className="thumbnail-card-art">
        <button
          aria-label={`toggle ${image.filename}`}
          className={cx("thumbnail-card-checkbox", selected && "thumbnail-card-checkbox-selected")}
          onClick={(event) => {
            event.stopPropagation();
            onToggle(image.id, { shiftKey: event.shiftKey });
          }}
          type="button"
        >
          {selected ? (
            <svg aria-hidden="true" className="thumbnail-card-check-icon" viewBox="0 0 16 16">
              <path
                d="M3.5 8.5 6.5 11.5 12.5 4.5"
                fill="none"
                stroke="currentColor"
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth="2.2"
              />
            </svg>
          ) : (
            ""
          )}
        </button>
        {selected ? <span className="thumbnail-card-selected-badge">Selected</span> : null}
        {assetUrl ? (
          <img
            alt={image.filename}
            className="thumbnail-card-image"
            decoding="async"
            draggable={false}
            loading="lazy"
            src={assetUrl}
          />
        ) : (
          <div className="thumbnail-card-placeholder">
            <span>{image.filename}</span>
          </div>
        )}
      </div>
      <div className={cx("thumbnail-card-body", selected && "thumbnail-card-body-selected")}>
        <Link
          className="thumbnail-card-title"
          onClick={(event) => event.stopPropagation()}
          to={`/workspace/${workspaceId}/image/${image.id}`}
        >
          {image.filename}
        </Link>
        <div className="thumbnail-card-meta">{image.sourceName}</div>
        <div className="thumbnail-card-metrics">
          {image.annotationCount} boxes
          {maxRatioLabel ? ` | max ${maxRatioLabel}` : ""}
        </div>
        {boxSummaryLabel ? <div className="thumbnail-card-box-list">{boxSummaryLabel}</div> : null}
        <div className="thumbnail-card-tags">
          {image.categories.length > 0 ? image.categories.join(", ") : "unlabeled"}
        </div>
      </div>
    </article>
  );
}

export const ThumbnailCard = memo(ThumbnailCardComponent);
