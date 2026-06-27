import { useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Link, useParams, useSearchParams } from "react-router-dom";

import { Panel } from "../../../components/ui/Panel";
import { getImageDetail } from "../../../lib/api";
import { toAssetUrl } from "../../../lib/tauri";
import { BoundingBoxRecord } from "../../../types/workspace";

function getBoxAreaRatio(
  box: BoundingBoxRecord,
  imageWidth?: number | null,
  imageHeight?: number | null,
) {
  if (box.annotationFormat === "yolo") {
    return Math.max(0, box.bboxWidth) * Math.max(0, box.bboxHeight);
  }

  if (!imageWidth || !imageHeight || imageWidth <= 0 || imageHeight <= 0) {
    return null;
  }

  return (Math.max(0, box.bboxWidth) * Math.max(0, box.bboxHeight)) / (imageWidth * imageHeight);
}

function formatPercent(value: number | null) {
  if (value == null) {
    return "ratio unavailable";
  }

  return `${(value * 100).toFixed(1)}%`;
}

export function ImageDetailPage() {
  const { workspaceId = "factory-defect-v1", imageId = "" } = useParams();
  const [searchParams] = useSearchParams();
  const returnTarget = searchParams.get("from") === "dataset-map" ? "dataset-map" : "browser";
  const highlightedAnnotationId = searchParams.get("annotationId");
  const backPath = `/workspace/${workspaceId}/${returnTarget}`;
  const backLabel = returnTarget === "dataset-map" ? "Back to Dataset Map" : "Back to Browser";
  const { data } = useQuery({
    queryKey: ["image-detail", workspaceId, imageId],
    queryFn: () => getImageDetail(workspaceId, imageId),
  });
  const [assetUrl, setAssetUrl] = useState<string | null>(null);
  const [naturalSize, setNaturalSize] = useState<{ width: number; height: number } | null>(null);

  useEffect(() => {
    if (!data) {
      setAssetUrl(null);
      return;
    }

    setAssetUrl(toAssetUrl(data.originalPath));
  }, [data]);

  const boxMetadata = useMemo(() => {
    if (!data) {
      return [];
    }

    const imageWidth = naturalSize?.width ?? data.width;
    const imageHeight = naturalSize?.height ?? data.height;

    return data.boxes.map((box, index) => ({
      id: box.id,
      label: `${index + 1}. ${box.categoryName}${highlightedAnnotationId === box.id ? " (selected)" : ""}`,
      ratioLabel: formatPercent(getBoxAreaRatio(box, imageWidth, imageHeight)),
    }));
  }, [data, highlightedAnnotationId, naturalSize]);

  if (!data) {
    return (
      <Panel title="Image Not Found" subtitle="The selected image is not available in the current mock payload.">
        <Link className="button button-secondary" to={backPath}>
          {backLabel}
        </Link>
      </Panel>
    );
  }

  const stageAspectRatio = naturalSize
    ? `${naturalSize.width} / ${naturalSize.height}`
    : data.width && data.height
      ? `${data.width} / ${data.height}`
      : "4 / 3";

  const getBoxStyle = (box: BoundingBoxRecord) => {
    if (box.annotationFormat === "yolo") {
      return {
        left: `${(box.bboxX - box.bboxWidth / 2) * 100}%`,
        top: `${(box.bboxY - box.bboxHeight / 2) * 100}%`,
        width: `${box.bboxWidth * 100}%`,
        height: `${box.bboxHeight * 100}%`,
      };
    }

    const width = naturalSize?.width ?? data.width;
    const height = naturalSize?.height ?? data.height;
    if (!width || !height) {
      return null;
    }

    return {
      left: `${(box.bboxX / width) * 100}%`,
      top: `${(box.bboxY / height) * 100}%`,
      width: `${(box.bboxWidth / width) * 100}%`,
      height: `${(box.bboxHeight / height) * 100}%`,
    };
  };

  return (
    <div className="image-detail-layout">
      <Panel
        title={data.filename}
        subtitle="Single-image inspection view for bounding boxes and source metadata."
        actions={
          <Link className="button button-secondary" to={backPath}>
            {backLabel}
          </Link>
        }
      >
        <div className="image-stage">
          <div className="image-stage-canvas" style={{ aspectRatio: stageAspectRatio }}>
            {assetUrl ? (
              <img
                alt={data.filename}
                className="image-stage-image"
                onLoad={(event) =>
                  setNaturalSize({
                    width: event.currentTarget.naturalWidth,
                    height: event.currentTarget.naturalHeight,
                  })
                }
                src={assetUrl}
              />
            ) : (
              <span>Image preview unavailable</span>
            )}
            {data.boxes.map((box) => {
              const style = getBoxStyle(box);
              if (!style) {
                return null;
              }

              const isHighlighted = highlightedAnnotationId === box.id;

              return (
                <div
                  className={`bbox bbox-real${isHighlighted ? " bbox-highlighted" : ""}`}
                  key={box.id}
                  style={style}
                >
                  <span className="bbox-label">{box.categoryName}</span>
                </div>
              );
            })}
          </div>
        </div>
      </Panel>

      <Panel title="Metadata" subtitle="Source path, category list, and per-box size ratio.">
        <dl className="metadata-list">
          <div>
            <dt>Source Folder</dt>
            <dd>{data.sourceName}</dd>
          </div>
          <div>
            <dt>Original Full Path</dt>
            <dd>{data.originalPath}</dd>
          </div>
          <div>
            <dt>Categories</dt>
            <dd>{data.categories.length > 0 ? data.categories.join(", ") : "unlabeled"}</dd>
          </div>
          <div>
            <dt>Boxes</dt>
            <dd>{data.boxes.length}</dd>
          </div>
          <div>
            <dt>Box Details</dt>
            <dd>
              {boxMetadata.length > 0 ? (
                boxMetadata.map((item) => `${item.label} (${item.ratioLabel})`).join(", ")
              ) : (
                "no boxes"
              )}
            </dd>
          </div>
        </dl>
      </Panel>
    </div>
  );
}
