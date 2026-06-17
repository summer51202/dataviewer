import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useParams } from "react-router-dom";

import { Panel } from "../../../components/ui/Panel";
import {
  getDatasetMapPayload,
  saveDatasetMapReviews,
  startEmbeddingJob,
} from "../../../lib/api";
import {
  DatasetMapPoint,
  DatasetMapScope,
  DatasetReviewStatus,
  EmbeddingRuntimePreference,
} from "../../../types/workspace";

const reviewStatuses: DatasetReviewStatus[] = ["needs-review", "keep", "fix", "exclude"];
const reviewStatusLabels: Record<DatasetReviewStatus, string> = {
  unreviewed: "Unreviewed",
  "needs-review": "Needs review",
  keep: "Keep",
  fix: "Fix",
  exclude: "Exclude",
};

function normalizeCoordinate(value: number, min: number, max: number, padding: number) {
  if (max === min) {
    return 500;
  }

  return padding + ((value - min) / (max - min)) * (1000 - padding * 2);
}

function formatAreaRatio(point: DatasetMapPoint) {
  if (!point.bbox?.areaRatio) {
    return "Area unavailable";
  }

  return `${(point.bbox.areaRatio * 100).toFixed(1)}% image area`;
}

function pointDisplayName(point: DatasetMapPoint) {
  if (point.scope === "image") {
    return point.filename;
  }

  return `${point.categoryName ?? "Unknown"} in ${point.filename}`;
}

function deriveImagePoints(points: DatasetMapPoint[]): DatasetMapPoint[] {
  const grouped = new Map<string, DatasetMapPoint[]>();
  points.forEach((point) => {
    const existing = grouped.get(point.imageId) ?? [];
    existing.push(point);
    grouped.set(point.imageId, existing);
  });

  return Array.from(grouped.entries()).map(([imageId, imagePoints]) => {
    const first = imagePoints[0];
    const reviewedPoint = imagePoints.find((point) => point.reviewStatus !== "unreviewed");
    return {
      id: imageId,
      scope: "image",
      imageId,
      annotationId: null,
      filename: first.filename,
      sourceId: first.sourceId,
      sourceName: first.sourceName,
      categoryId: null,
      categoryName: imagePoints.map((point) => point.categoryName).filter(Boolean).join(", "),
      bbox: null,
      x: imagePoints.reduce((sum, point) => sum + point.x, 0) / imagePoints.length,
      y: imagePoints.reduce((sum, point) => sum + point.y, 0) / imagePoints.length,
      reviewStatus: reviewedPoint?.reviewStatus ?? "unreviewed",
    };
  });
}

export function DatasetMapPage() {
  const { workspaceId = "factory-defect-v1" } = useParams();
  const queryClient = useQueryClient();
  const [scope, setScope] = useState<DatasetMapScope>("object");
  const [modelId, setModelId] = useState("clip-vit-b32");
  const [runtimePreference, setRuntimePreference] = useState<EmbeddingRuntimePreference>("auto");
  const [selectedPointIds, setSelectedPointIds] = useState<string[]>([]);
  const [localReviewState, setLocalReviewState] = useState<Record<string, DatasetReviewStatus>>({});

  const { data, isLoading, isError } = useQuery({
    queryKey: ["dataset-map", workspaceId, scope, modelId],
    queryFn: () => getDatasetMapPayload({ workspaceId, scope, modelId }),
  });

  useEffect(() => {
    if (data && !data.models.some((model) => model.id === modelId)) {
      setModelId(data.modelId);
    }
  }, [data, modelId]);

  useEffect(() => {
    setSelectedPointIds([]);
    setLocalReviewState({});
  }, [scope, modelId]);

  const visiblePoints = useMemo(() => {
    const sourcePoints = data?.points ?? [];
    const scopedPoints = sourcePoints.filter((point) => point.scope === scope);
    const points =
      scope === "image" && scopedPoints.length === 0
        ? deriveImagePoints(sourcePoints)
        : scopedPoints;

    return points
      .map((point) => ({
        ...point,
        reviewStatus: localReviewState[point.id] ?? point.reviewStatus,
      }));
  }, [data?.points, localReviewState, scope]);

  const selectedPointSet = useMemo(() => new Set(selectedPointIds), [selectedPointIds]);
  const selectedPoints = useMemo(
    () => visiblePoints.filter((point) => selectedPointSet.has(point.id)),
    [selectedPointSet, visiblePoints],
  );
  const reviewQueue = useMemo(
    () => visiblePoints.filter((point) => point.reviewStatus !== "unreviewed"),
    [visiblePoints],
  );

  const bounds = useMemo(() => {
    const xs = visiblePoints.map((point) => point.x);
    const ys = visiblePoints.map((point) => point.y);
    return {
      minX: xs.length ? Math.min(...xs) : -1,
      maxX: xs.length ? Math.max(...xs) : 1,
      minY: ys.length ? Math.min(...ys) : -1,
      maxY: ys.length ? Math.max(...ys) : 1,
    };
  }, [visiblePoints]);

  const reviewMutation = useMutation({
    mutationFn: (status: DatasetReviewStatus) =>
      saveDatasetMapReviews({
        workspaceId,
        scope,
        updates: selectedPointIds.map((targetId) => ({ targetId, status })),
      }),
    onSuccess: (_updates, status) => {
      setLocalReviewState((current) => {
        const next = { ...current };
        selectedPointIds.forEach((pointId) => {
          next[pointId] = status;
        });
        return next;
      });
      void queryClient.invalidateQueries({ queryKey: ["dataset-map", workspaceId] });
    },
  });

  const jobMutation = useMutation({
    mutationFn: () =>
      startEmbeddingJob({
        workspaceId,
        scope,
        modelId,
        runtimePreference,
      }),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["dataset-map", workspaceId] });
    },
  });

  const handlePointToggle = (pointId: string) => {
    setSelectedPointIds((current) =>
      current.includes(pointId)
        ? current.filter((selectedId) => selectedId !== pointId)
        : [...current, pointId],
    );
  };

  const modelOptions = data?.models ?? [];
  const selectedModel = modelOptions.find((model) => model.id === modelId);
  const selectedRuntime = data?.runtime.selectedBackend ?? "cpu";

  return (
    <div className="dataset-map-page">
      <Panel
        title="Dataset Map"
        subtitle="Explore object-crop and image-level embedding spaces before training."
        actions={
          <button
            className="button button-primary"
            disabled={jobMutation.isPending || isLoading}
            onClick={() => jobMutation.mutate()}
            type="button"
          >
            {jobMutation.isPending ? "Running..." : "Run / Refresh Embeddings"}
          </button>
        }
      >
        <div className="dataset-map-toolbar">
          <div className="dataset-map-tabs" role="tablist" aria-label="Dataset map mode">
            <button
              aria-selected={scope === "object"}
              className={`button ${scope === "object" ? "button-primary" : "button-secondary"}`}
              onClick={() => setScope("object")}
              role="tab"
              type="button"
            >
              Object Map
            </button>
            <button
              aria-selected={scope === "image"}
              className={`button ${scope === "image" ? "button-primary" : "button-secondary"}`}
              onClick={() => setScope("image")}
              role="tab"
              type="button"
            >
              Image Map
            </button>
          </div>

          <label className="field dataset-map-control">
            <span>Encoder</span>
            <select value={modelId} onChange={(event) => setModelId(event.target.value)}>
              {modelOptions.map((model) => (
                <option key={model.id} value={model.id}>
                  {model.displayName}
                </option>
              ))}
            </select>
          </label>

          <label className="field dataset-map-control">
            <span>Runtime</span>
            <select
              value={runtimePreference}
              onChange={(event) =>
                setRuntimePreference(event.target.value as EmbeddingRuntimePreference)
              }
            >
              <option value="auto">Auto</option>
              <option value="cuda">CUDA</option>
              <option value="windows-gpu">Windows GPU</option>
              <option value="cpu">CPU</option>
            </select>
          </label>
        </div>

        {isError ? (
          <div className="notice notice-error">Dataset Map could not load.</div>
        ) : null}

        <div className="dataset-map-layout">
          <aside className="dataset-map-filter-panel" aria-label="Dataset map filters">
            <div className="dataset-map-filter-block">
              <span className="filter-label">Scope</span>
              <strong>{scope === "object" ? "Existing bbox crops" : "Images"}</strong>
              <span className="field-help">
                {scope === "object"
                  ? "One point represents one imported annotation box."
                  : "One point represents one source image."}
              </span>
            </div>
            <div className="dataset-map-filter-block">
              <span className="filter-label">Embedding</span>
              <strong>{selectedModel?.displayName ?? data?.modelId ?? modelId}</strong>
              <span className="field-help">
                {selectedModel
                  ? `${selectedModel.embeddingDim} dimensions, ${selectedModel.inputSize}px input`
                  : "Model metadata unavailable"}
              </span>
            </div>
            <div className="dataset-map-filter-block">
              <span className="filter-label">Runtime</span>
              <strong>{runtimePreference === "auto" ? `Auto: ${selectedRuntime}` : runtimePreference}</strong>
              <span className="field-help">{data?.runtime.fallbackReason ?? "Runtime ready."}</span>
            </div>
          </aside>

          <section className="dataset-map-stage" aria-label="Dataset embedding scatterplot">
            <div className="dataset-map-stage-header">
              <strong>{visiblePoints.length} points</strong>
              <span>
                {selectedPointIds.length} selected · {reviewQueue.length} reviewed
              </span>
            </div>

            {isLoading ? (
              <div className="dataset-map-empty">Loading Dataset Map...</div>
            ) : visiblePoints.length === 0 ? (
              <div className="dataset-map-empty">No points are available for this map mode yet.</div>
            ) : (
              <svg className="dataset-map-canvas" role="img" viewBox="0 0 1000 640">
                <title>Dataset embedding scatterplot</title>
                <rect className="dataset-map-canvas-bg" x="0" y="0" width="1000" height="640" />
                {visiblePoints.map((point) => {
                  const cx = normalizeCoordinate(point.x, bounds.minX, bounds.maxX, 60);
                  const cy = 640 - normalizeCoordinate(point.y, bounds.minY, bounds.maxY, 50);
                  const selected = selectedPointSet.has(point.id);
                  return (
                    <g className="dataset-map-point-hit" key={point.id}>
                      <circle
                        aria-label={pointDisplayName(point)}
                        className={`dataset-map-point dataset-map-point-${point.reviewStatus} ${
                          selected ? "dataset-map-point-selected" : ""
                        }`}
                        cx={cx}
                        cy={cy}
                        onKeyDown={(event) => {
                          if (event.key === "Enter" || event.key === " ") {
                            event.preventDefault();
                            handlePointToggle(point.id);
                          }
                        }}
                        onClick={() => handlePointToggle(point.id)}
                        r={selected ? 10 : 7}
                        role="button"
                        tabIndex={0}
                      >
                        <title>{pointDisplayName(point)}</title>
                      </circle>
                    </g>
                  );
                })}
              </svg>
            )}

            <div className="dataset-map-legend" aria-label="Review status legend">
              {(["unreviewed", ...reviewStatuses] as DatasetReviewStatus[]).map((status) => (
                <span className="legend-item" key={status}>
                  <span className={`legend-swatch dataset-map-legend-${status}`} />
                  {reviewStatusLabels[status]}
                </span>
              ))}
            </div>
          </section>

          <aside className="dataset-map-selection-panel" aria-label="Selection details">
            <div>
              <span className="filter-label">Selection</span>
              <strong>{selectedPoints.length} selected</strong>
            </div>
            {selectedPoints.length > 0 ? (
              <div className="dataset-map-selected-list">
                {selectedPoints.slice(0, 4).map((point) => (
                  <article className="dataset-map-selected-item" key={point.id}>
                    <strong>{pointDisplayName(point)}</strong>
                    <span>{point.sourceName}</span>
                    <span>{formatAreaRatio(point)}</span>
                  </article>
                ))}
              </div>
            ) : (
              <span className="field-help">Select points in the map to review them.</span>
            )}

            <div className="dataset-map-review-actions">
              {reviewStatuses.map((status) => (
                <button
                  className="button button-secondary button-sm"
                  disabled={selectedPointIds.length === 0 || reviewMutation.isPending}
                  key={status}
                  onClick={() => reviewMutation.mutate(status)}
                  type="button"
                >
                  {reviewStatusLabels[status]}
                </button>
              ))}
            </div>
          </aside>
        </div>
      </Panel>

      <Panel
        title="Review Queue"
        subtitle="Items marked from either map mode stay in the shared pre-training review queue."
      >
        {reviewQueue.length > 0 ? (
          <div className="dataset-map-review-queue">
            {reviewQueue.map((point) => (
              <article className="dataset-map-review-row" key={point.id}>
                <div>
                  <strong>{pointDisplayName(point)}</strong>
                  <span>{point.sourceName}</span>
                </div>
                <span className={`badge dataset-map-review-badge dataset-map-review-${point.reviewStatus}`}>
                  {reviewStatusLabels[point.reviewStatus]}
                </span>
              </article>
            ))}
          </div>
        ) : (
          <div className="helper-text">No reviewed items yet.</div>
        )}
      </Panel>
    </div>
  );
}
