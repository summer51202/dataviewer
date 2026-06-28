import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useParams } from "react-router-dom";

import { Panel } from "../../../components/ui/Panel";
import {
  deleteSampleSet,
  getDatasetMapPayload,
  getSampleSetMembers,
  listSampleSets,
  runSampleSelection,
  saveDatasetMapReviews,
  startEmbeddingJob,
} from "../../../lib/api";
import {
  DatasetMapPoint,
  DatasetMapScope,
  DatasetReviewStatus,
  EmbeddingRuntimePreference,
  SamplingMode,
} from "../../../types/workspace";

const reviewStatuses: DatasetReviewStatus[] = ["needs-review", "keep", "fix", "exclude"];
const reviewStatusLabels: Record<DatasetReviewStatus, string> = {
  unreviewed: "Unreviewed",
  "needs-review": "Needs review",
  keep: "Keep",
  fix: "Fix",
  exclude: "Exclude",
};

type FilterOption = {
  id: string;
  label: string;
};

function normalizeCoordinate(
  value: number,
  min: number,
  max: number,
  padding: number,
  size = 1000,
) {
  if (max === min) {
    return size / 2;
  }

  return padding + ((value - min) / (max - min)) * (size - padding * 2);
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

function pointCategoryLabel(point: DatasetMapPoint) {
  return point.categoryName || (point.scope === "image" ? "Multiple categories" : "Unknown category");
}

function formatJobError(error: unknown) {
  if (error instanceof Error) {
    return error.message;
  }
  if (typeof error === "string") {
    return error;
  }
  return "Embedding job could not run.";
}

function uniqueOptions(
  points: DatasetMapPoint[],
  getId: (point: DatasetMapPoint) => string | null | undefined,
  getLabel: (point: DatasetMapPoint) => string | null | undefined,
): FilterOption[] {
  const options = new Map<string, string>();
  points.forEach((point) => {
    const id = getId(point);
    const label = getLabel(point);
    if (id && label && !options.has(id)) {
      options.set(id, label);
    }
  });

  return Array.from(options.entries())
    .map(([id, label]) => ({ id, label }))
    .sort((a, b) => a.label.localeCompare(b.label));
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
  const [modelId, setModelId] = useState("fast-preview");
  const [runtimePreference, setRuntimePreference] = useState<EmbeddingRuntimePreference>("auto");
  const [selectedPointIds, setSelectedPointIds] = useState<string[]>([]);
  const [localReviewState, setLocalReviewState] = useState<Record<string, DatasetReviewStatus>>({});
  const [sourceFilter, setSourceFilter] = useState("all");
  const [categoryFilter, setCategoryFilter] = useState("all");
  const [reviewStatusFilter, setReviewStatusFilter] = useState<DatasetReviewStatus | "all">("all");
  const [isEmbeddingJobActive, setIsEmbeddingJobActive] = useState(false);
  const [sampleName, setSampleName] = useState("coverage");
  const [sampleTargetMode, setSampleTargetMode] = useState<"ratio" | "count">("ratio");
  const [sampleRatio, setSampleRatio] = useState(0.3);
  const [sampleCount, setSampleCount] = useState(100);
  const [mode, setMode] = useState<SamplingMode>("balanced");
  const [removeOutliers, setRemoveOutliers] = useState(false);
  const [previewSampleSet, setPreviewSampleSet] = useState("");

  const { data, isLoading, isError } = useQuery({
    queryKey: ["dataset-map", workspaceId, scope, modelId],
    queryFn: () => getDatasetMapPayload({ workspaceId, scope, modelId }),
    refetchInterval: (query) => {
      const jobs = query.state.data?.jobs ?? [];
      return isEmbeddingJobActive || jobs.some((job) => job.status === "running") ? 1000 : false;
    },
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

  useEffect(() => {
    setSelectedPointIds([]);
  }, [sourceFilter, categoryFilter, reviewStatusFilter]);

  useEffect(() => {
    if (scope !== "object") {
      setCategoryFilter("all");
    }
  }, [scope]);

  const scopedPoints = useMemo(() => {
    const sourcePoints = data?.points ?? [];
    const scopedSourcePoints = sourcePoints.filter((point) => point.scope === scope);
    const points =
      scope === "image" && scopedSourcePoints.length === 0
        ? deriveImagePoints(sourcePoints)
        : scopedSourcePoints;

    return points
      .map((point) => ({
        ...point,
        reviewStatus: localReviewState[point.id] ?? point.reviewStatus,
      }));
  }, [data?.points, localReviewState, scope]);

  const sourceOptions = useMemo(
    () => uniqueOptions(scopedPoints, (point) => point.sourceId, (point) => point.sourceName),
    [scopedPoints],
  );

  const categoryOptions = useMemo(
    () => uniqueOptions(scopedPoints, (point) => point.categoryId, (point) => point.categoryName),
    [scopedPoints],
  );

  const visiblePoints = useMemo(() => (
    scopedPoints.filter((point) => {
      const matchesSource = sourceFilter === "all" || point.sourceId === sourceFilter;
      const matchesCategory =
        categoryFilter === "all" || (scope === "object" && point.categoryId === categoryFilter);
      const matchesReview =
        reviewStatusFilter === "all" || point.reviewStatus === reviewStatusFilter;
      return matchesSource && matchesCategory && matchesReview;
    })
  ), [categoryFilter, reviewStatusFilter, scope, scopedPoints, sourceFilter]);

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
    if (visiblePoints.length === 0) {
      return { minX: -1, maxX: 1, minY: -1, maxY: 1 };
    }

    return visiblePoints.reduce(
      (current, point) => ({
        minX: Math.min(current.minX, point.x),
        maxX: Math.max(current.maxX, point.x),
        minY: Math.min(current.minY, point.y),
        maxY: Math.max(current.maxY, point.y),
      }),
      {
        minX: visiblePoints[0].x,
        maxX: visiblePoints[0].x,
        minY: visiblePoints[0].y,
        maxY: visiblePoints[0].y,
      },
    );
  }, [visiblePoints]);

  const reviewMutation = useMutation({
    mutationFn: ({ status, pointIds }: { status: DatasetReviewStatus; pointIds: string[] }) =>
      saveDatasetMapReviews({
        workspaceId,
        scope,
        updates: pointIds.map((targetId) => ({ targetId, status })),
      }),
    onSuccess: (_updates, { status, pointIds }) => {
      setLocalReviewState((current) => {
        const next = { ...current };
        pointIds.forEach((pointId) => {
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
    onMutate: () => {
      setIsEmbeddingJobActive(true);
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["dataset-map", workspaceId] });
    },
    onSettled: () => {
      setIsEmbeddingJobActive(false);
    },
  });

  const sampleSetsQuery = useQuery({
    queryKey: ["sample-sets", workspaceId],
    queryFn: () => listSampleSets(workspaceId),
  });

  const sampleMutation = useMutation({
    mutationFn: () =>
      runSampleSelection({
        workspaceId,
        scope,
        modelId,
        name: sampleName.trim() || "coverage",
        targetImages: sampleTargetMode === "count" ? sampleCount : null,
        targetRatio: sampleTargetMode === "ratio" ? sampleRatio : null,
        mode,
        removeOutliers,
      }),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["sample-sets", workspaceId] });
    },
  });

  const deleteSampleMutation = useMutation({
    mutationFn: (name: string) => deleteSampleSet(workspaceId, name),
    onSuccess: (_result, name) => {
      if (name === previewSampleSet) {
        setPreviewSampleSet("");
      }
      void queryClient.invalidateQueries({ queryKey: ["sample-sets", workspaceId] });
    },
  });

  const previewMembersQuery = useQuery({
    queryKey: ["sample-set-members", workspaceId, previewSampleSet],
    queryFn: () => getSampleSetMembers(workspaceId, previewSampleSet),
    enabled: previewSampleSet !== "",
  });

  const sampledImageIds = useMemo(
    () => new Set(previewMembersQuery.data?.imageIds ?? []),
    [previewMembersQuery.data],
  );

  const handlePointToggle = (pointId: string) => {
    setSelectedPointIds((current) =>
      current.includes(pointId)
        ? current.filter((selectedId) => selectedId !== pointId)
        : [...current, pointId],
    );
  };

  const modelOptions = data?.models ?? [];
  const selectedModel = modelOptions.find((model) => model.id === modelId);
  const isPreviewModel = modelId === "fast-preview";
  const selectedRuntime = data?.runtime.selectedBackend ?? "cpu";
  const activeJob = data?.jobs.find(
    (job) => job.workspaceId === workspaceId && job.scope === scope && job.modelId === modelId,
  );
  const isJobRunning = jobMutation.isPending || activeJob?.status === "running";
  const jobProcessed = activeJob?.processedItems ?? 0;
  const jobTotal = activeJob?.totalItems ?? 0;
  const jobPercent = jobTotal > 0 ? Math.min(100, Math.round((jobProcessed / jobTotal) * 100)) : 0;
  const jobStage = jobMutation.isPending
    ? activeJob?.message ?? (isPreviewModel ? "Generating preview layout" : "Running embeddings")
    : activeJob?.message ?? null;
  const filtersActive =
    sourceFilter !== "all" || categoryFilter !== "all" || reviewStatusFilter !== "all";

  return (
    <div className="dataset-map-page">
      <Panel
        title="Dataset Map"
        subtitle="Explore object-crop and image-level embedding spaces before training."
        actions={
          <button
            className="button button-primary"
            disabled={isJobRunning || isLoading}
            onClick={() => jobMutation.mutate()}
            type="button"
          >
            {isJobRunning
              ? "Running..."
              : isPreviewModel
                ? "Generate Preview Layout"
                : "Run / Refresh Embeddings"}
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
              disabled={isPreviewModel}
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
        {jobMutation.isError ? (
          <div className="notice notice-error">
            {formatJobError(jobMutation.error)}
          </div>
        ) : null}
        {isPreviewModel ? (
          <div className="notice notice-info">
            Fast Preview uses stable dataset metadata for layout. No CLIP or DINO model inference is run.
          </div>
        ) : null}
        {isJobRunning ? (
          <div className="dataset-map-progress" role="status" aria-live="polite">
            <div className="dataset-map-progress-head">
              <strong>{jobStage ?? "Running embeddings"}</strong>
              <span>{jobTotal > 0 ? `${jobProcessed} / ${jobTotal}` : "Preparing"}</span>
            </div>
            <div className="scan-progress-bar">
              <div
                className={`scan-progress-bar-fill${jobTotal === 0 ? " scan-progress-bar-fill-indeterminate" : ""}`}
                style={jobTotal > 0 ? { width: `${jobPercent}%` } : undefined}
              />
            </div>
          </div>
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
                {isPreviewModel
                  ? "Stable preview layout, no model inference"
                  : selectedModel
                  ? `${selectedModel.embeddingDim} dimensions, ${selectedModel.inputSize}px input`
                  : "Model metadata unavailable"}
              </span>
            </div>
            <div className="dataset-map-filter-block">
              <span className="filter-label">Runtime</span>
              <strong>{isPreviewModel ? "Not used" : runtimePreference === "auto" ? `Auto: ${selectedRuntime}` : runtimePreference}</strong>
              <span className="field-help">
                {isPreviewModel ? "Preview layout does not use ONNX Runtime." : data?.runtime.fallbackReason ?? "Runtime ready."}
              </span>
            </div>
            <label className="field dataset-map-filter-field">
              <span>Source</span>
              <select value={sourceFilter} onChange={(event) => setSourceFilter(event.target.value)}>
                <option value="all">All sources</option>
                {sourceOptions.map((option) => (
                  <option key={option.id} value={option.id}>
                    {option.label}
                  </option>
                ))}
              </select>
            </label>
            <label className="field dataset-map-filter-field">
              <span>Category</span>
              <select
                disabled={scope !== "object" || categoryOptions.length === 0}
                value={categoryFilter}
                onChange={(event) => setCategoryFilter(event.target.value)}
              >
                <option value="all">All categories</option>
                {categoryOptions.map((option) => (
                  <option key={option.id} value={option.id}>
                    {option.label}
                  </option>
                ))}
              </select>
            </label>
            <label className="field dataset-map-filter-field">
              <span>Review Status</span>
              <select
                value={reviewStatusFilter}
                onChange={(event) =>
                  setReviewStatusFilter(event.target.value as DatasetReviewStatus | "all")
                }
              >
                <option value="all">All statuses</option>
                {(["unreviewed", ...reviewStatuses] as DatasetReviewStatus[]).map((status) => (
                  <option key={status} value={status}>
                    {reviewStatusLabels[status]}
                  </option>
                ))}
              </select>
            </label>
            {filtersActive ? (
              <button
                className="button button-secondary button-sm"
                onClick={() => {
                  setSourceFilter("all");
                  setCategoryFilter("all");
                  setReviewStatusFilter("all");
                }}
                type="button"
              >
                Clear filters
              </button>
            ) : null}
          </aside>

          <section className="dataset-map-stage" aria-label="Dataset embedding scatterplot">
            <div className="dataset-map-stage-header">
              <strong>{visiblePoints.length} points</strong>
              <span>
                {selectedPointIds.length} selected / {reviewQueue.length} reviewed
                {previewSampleSet
                  ? ` · sample "${previewSampleSet}" highlighted (${sampledImageIds.size} images)`
                  : ""}
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
                  const cy = 640 - normalizeCoordinate(point.y, bounds.minY, bounds.maxY, 50, 640);
                  const selected = selectedPointSet.has(point.id);
                  const sampled = sampledImageIds.has(point.imageId);
                  return (
                    <g className="dataset-map-point-hit" key={point.id}>
                      <circle
                        aria-label={pointDisplayName(point)}
                        className={`dataset-map-point dataset-map-point-${point.reviewStatus} ${
                          selected ? "dataset-map-point-selected" : ""
                        }`}
                        cx={cx}
                        cy={cy}
                        opacity={previewSampleSet && !sampled ? 0.2 : 1}
                        stroke={sampled ? "#22c55e" : undefined}
                        strokeWidth={sampled ? 2.5 : undefined}
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
            <div className="dataset-map-selection-header">
              <div>
                <span className="filter-label">Selection</span>
                <strong>{selectedPoints.length} selected</strong>
              </div>
              {selectedPoints.length > 0 ? (
                <button
                  className="button button-secondary button-sm"
                  onClick={() => setSelectedPointIds([])}
                  type="button"
                >
                  Clear
                </button>
              ) : null}
            </div>
            {selectedPoints.length > 0 ? (
              <div className="dataset-map-selected-list">
                {selectedPoints.slice(0, 4).map((point) => (
                  <article className="dataset-map-selected-item" key={point.id}>
                    <Link
                      className="dataset-map-selected-filename"
                      title={point.filename}
                      to={`/workspace/${workspaceId}/image/${point.imageId}?from=dataset-map${
                        point.annotationId ? `&annotationId=${encodeURIComponent(point.annotationId)}` : ""
                      }`}
                    >
                      {point.filename}
                    </Link>
                    <span title={pointCategoryLabel(point)}>{pointCategoryLabel(point)}</span>
                    <span title={point.sourceName}>{point.sourceName}</span>
                    <span>{formatAreaRatio(point)}</span>
                  </article>
                ))}
                {selectedPoints.length > 4 ? (
                  <span className="field-help">Showing 4 of {selectedPoints.length} selected items.</span>
                ) : null}
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
                  onClick={() => reviewMutation.mutate({ status, pointIds: selectedPointIds })}
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
        title="Auto Sample Selection"
        subtitle="Coverage-maximising subset computed on high-dimensional embeddings (independent from the 2D map projection)."
      >
        {isPreviewModel ? (
          <div className="notice notice-info">
            Sampling needs real embeddings. Pick a non-preview encoder and run embeddings first.
          </div>
        ) : (
          <>
            <div className="dataset-map-toolbar">
              <label className="field dataset-map-control">
                <span>Sample set name</span>
                <input
                  value={sampleName}
                  onChange={(event) => setSampleName(event.target.value)}
                />
              </label>
              <label className="field dataset-map-control">
                <span>Target</span>
                <select
                  value={sampleTargetMode}
                  onChange={(event) =>
                    setSampleTargetMode(event.target.value as "ratio" | "count")
                  }
                >
                  <option value="ratio">Ratio</option>
                  <option value="count">Image count</option>
                </select>
              </label>
              {sampleTargetMode === "ratio" ? (
                <label className="field dataset-map-control">
                  <span>Ratio (0-1)</span>
                  <input
                    type="number"
                    min={0.01}
                    max={1}
                    step={0.05}
                    value={sampleRatio}
                    onChange={(event) => setSampleRatio(Number(event.target.value))}
                  />
                </label>
              ) : (
                <label className="field dataset-map-control">
                  <span>Images</span>
                  <input
                    type="number"
                    min={1}
                    step={1}
                    value={sampleCount}
                    onChange={(event) => setSampleCount(Number(event.target.value))}
                  />
                </label>
              )}
              <label className="field dataset-map-control">
                <span>Mode</span>
                <select
                  value={mode}
                  onChange={(event) => setMode(event.target.value as SamplingMode)}
                >
                  <option value="balanced">Balanced (representative)</option>
                  <option value="diverse">Diverse (max spread)</option>
                </select>
              </label>
              <label className="field dataset-map-control">
                <span>Outliers</span>
                <select
                  value={removeOutliers ? "remove" : "keep"}
                  onChange={(event) => setRemoveOutliers(event.target.value === "remove")}
                >
                  <option value="keep">Keep all</option>
                  <option value="remove">Exclude outliers</option>
                </select>
              </label>
              <button
                className="button button-primary"
                type="button"
                disabled={sampleMutation.isPending}
                onClick={() => sampleMutation.mutate()}
              >
                {sampleMutation.isPending ? "Sampling..." : "Run sampling"}
              </button>
            </div>

            {sampleMutation.isPending ? (
              <div className="dataset-map-progress" role="status" aria-live="polite">
                <div className="dataset-map-progress-head">
                  <strong>Sampling on high-dimensional embeddings</strong>
                  <span>{mode}</span>
                </div>
                <div className="scan-progress-bar">
                  <div className="scan-progress-bar-fill scan-progress-bar-fill-indeterminate" />
                </div>
              </div>
            ) : null}
            {sampleMutation.isError ? (
              <div className="notice notice-error">{formatJobError(sampleMutation.error)}</div>
            ) : null}
            {sampleMutation.isSuccess && sampleMutation.data ? (
              <div className="notice notice-info">
                Selected {sampleMutation.data.selectedImages} images (
                {sampleMutation.data.selectedObjects} objects)
                {sampleMutation.data.excludedOutliers > 0
                  ? `, excluded ${sampleMutation.data.excludedOutliers} outliers`
                  : ""}
                {sampleMutation.data.saturated ? " — saturated (all images selected)" : ""}.
              </div>
            ) : null}

            <div className="dataset-map-review-queue">
              {(sampleSetsQuery.data ?? []).length === 0 ? (
                <div className="helper-text">No sample sets yet.</div>
              ) : (
                (sampleSetsQuery.data ?? []).map((set) => (
                  <article className="dataset-map-review-row" key={set.id}>
                    <div>
                      <strong>{set.name}</strong>
                      <span>
                        {set.selectedImages} images · {set.mode} · {set.modelId}
                        {set.saturated ? " · saturated" : ""}
                      </span>
                    </div>
                    <div className="button-row">
                      <button
                        className={`button button-sm ${
                          previewSampleSet === set.name ? "button-primary" : "button-secondary"
                        }`}
                        type="button"
                        onClick={() =>
                          setPreviewSampleSet((current) => (current === set.name ? "" : set.name))
                        }
                      >
                        {previewSampleSet === set.name ? "Previewing" : "Preview on map"}
                      </button>
                      <button
                        className="button button-secondary button-sm"
                        type="button"
                        disabled={deleteSampleMutation.isPending}
                        onClick={() => deleteSampleMutation.mutate(set.name)}
                      >
                        Delete
                      </button>
                    </div>
                  </article>
                ))
              )}
            </div>
          </>
        )}
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
