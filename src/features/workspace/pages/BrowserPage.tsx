import { useDeferredValue, useEffect, useMemo, useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useNavigate, useParams } from "react-router-dom";

import { ThumbnailCard } from "../../../components/domain/ThumbnailCard";
import { Panel } from "../../../components/ui/Panel";
import { getBrowserPayload } from "../../../lib/api";
import { useWorkspaceStore } from "../../../state/useWorkspaceStore";
import { AnnotationCountFilter } from "../../../types/workspace";
import { filterBrowserImages } from "../browserScope";

const annotationCountOptions: Array<{ value: AnnotationCountFilter; label: string }> = [
  { value: "", label: "Any" },
  { value: "0", label: "0" },
  { value: "1", label: "1" },
  { value: "2", label: "2" },
  { value: "3", label: "3" },
  { value: "4", label: "4" },
  { value: "5+", label: "5+" },
];

type SelectionRect = {
  left: number;
  top: number;
  width: number;
  height: number;
};

function sanitizePercentInput(value: string) {
  const sanitized = value.replace(/[^\d.]/g, "");
  if (sanitized === "") {
    return "";
  }

  const parsed = Number.parseFloat(sanitized);
  if (!Number.isFinite(parsed)) {
    return "";
  }

  return String(Math.min(100, Math.max(0, parsed)));
}

function buildSelectionRect(originX: number, originY: number, currentX: number, currentY: number): SelectionRect {
  return {
    left: Math.min(originX, currentX),
    top: Math.min(originY, currentY),
    width: Math.abs(currentX - originX),
    height: Math.abs(currentY - originY),
  };
}

export function BrowserPage() {
  const [visibleCount, setVisibleCount] = useState(180);
  const [boxSelectMode, setBoxSelectMode] = useState(false);
  const [selectionRect, setSelectionRect] = useState<SelectionRect | null>(null);
  const [dragPreviewIds, setDragPreviewIds] = useState<string[]>([]);
  const [lastSelectedImageId, setLastSelectedImageId] = useState<string | null>(null);
  const navigate = useNavigate();
  const { workspaceId = "factory-defect-v1" } = useParams();
  const gridRef = useRef<HTMLDivElement | null>(null);
  const dragStateRef = useRef<{
    pointerId: number;
    originX: number;
    originY: number;
    dragged: boolean;
  } | null>(null);
  const suppressNextToggleRef = useRef(false);

  const { data } = useQuery({
    queryKey: ["browser", workspaceId],
    queryFn: () => getBrowserPayload(workspaceId),
  });

  const search = useWorkspaceStore((state) => state.search);
  const annotationFilter = useWorkspaceStore((state) => state.annotationFilter);
  const selectedSourceIds = useWorkspaceStore((state) => state.selectedSourceIds);
  const selectedCategoryIds = useWorkspaceStore((state) => state.selectedCategoryIds);
  const annotationCountFilter = useWorkspaceStore((state) => state.annotationCountFilter);
  const minBoxAreaRatioPercent = useWorkspaceStore((state) => state.minBoxAreaRatioPercent);
  const selectedImageIds = useWorkspaceStore((state) => state.selectedImageIds);
  const setSearch = useWorkspaceStore((state) => state.setSearch);
  const setAnnotationFilter = useWorkspaceStore((state) => state.setAnnotationFilter);
  const setAnnotationCountFilter = useWorkspaceStore((state) => state.setAnnotationCountFilter);
  const setMinBoxAreaRatioPercent = useWorkspaceStore((state) => state.setMinBoxAreaRatioPercent);
  const toggleSource = useWorkspaceStore((state) => state.toggleSource);
  const toggleCategory = useWorkspaceStore((state) => state.toggleCategory);
  const toggleImageSelection = useWorkspaceStore((state) => state.toggleImageSelection);
  const setSelectedImageIds = useWorkspaceStore((state) => state.setSelectedImageIds);
  const deferredSearch = useDeferredValue(search);
  const selectedImageIdSet = useMemo(() => new Set(selectedImageIds), [selectedImageIds]);
  const dragPreviewSet = useMemo(() => new Set(dragPreviewIds), [dragPreviewIds]);

  useEffect(() => {
    setVisibleCount(180);
  }, [
    deferredSearch,
    annotationFilter,
    selectedSourceIds,
    selectedCategoryIds,
    annotationCountFilter,
    minBoxAreaRatioPercent,
  ]);

  const filteredImages = useMemo(
    () =>
      filterBrowserImages(data?.images ?? [], {
        search: deferredSearch,
        annotationFilter,
        selectedSourceIds,
        selectedCategoryIds,
        annotationCountFilter,
        minBoxAreaRatioPercent,
      }),
    [
      annotationFilter,
      annotationCountFilter,
      data?.images,
      deferredSearch,
      minBoxAreaRatioPercent,
      selectedCategoryIds,
      selectedSourceIds,
    ],
  );
  const visibleImages = useMemo(() => filteredImages.slice(0, visibleCount), [filteredImages, visibleCount]);
  const visibleImageIds = useMemo(() => visibleImages.map((image) => image.id), [visibleImages]);
  const filteredImageIds = useMemo(() => filteredImages.map((image) => image.id), [filteredImages]);
  const selectedFilteredCount = useMemo(
    () => filteredImageIds.filter((imageId) => selectedImageIdSet.has(imageId)).length,
    [filteredImageIds, selectedImageIdSet],
  );
  const allFilteredSelected = filteredImages.length > 0 && selectedFilteredCount === filteredImages.length;

  const toggleFilteredSelection = () => {
    if (allFilteredSelected) {
      const filteredIdSet = new Set(filteredImageIds);
      setSelectedImageIds(selectedImageIds.filter((imageId) => !filteredIdSet.has(imageId)));
      return;
    }

    const nextSelection = new Set(selectedImageIds);
    filteredImageIds.forEach((imageId) => nextSelection.add(imageId));
    setSelectedImageIds(Array.from(nextSelection));
  };

  const clearSelection = () => {
    setSelectedImageIds([]);
    setLastSelectedImageId(null);
  };

  const handleImageToggle = (imageId: string, modifiers?: { shiftKey?: boolean }) => {
    if (suppressNextToggleRef.current) {
      suppressNextToggleRef.current = false;
      return;
    }

    if (modifiers?.shiftKey && lastSelectedImageId) {
      const currentIndex = visibleImageIds.indexOf(imageId);
      const lastIndex = visibleImageIds.indexOf(lastSelectedImageId);

      if (currentIndex >= 0 && lastIndex >= 0) {
        const [start, end] = currentIndex < lastIndex ? [currentIndex, lastIndex] : [lastIndex, currentIndex];
        const nextSelection = new Set(selectedImageIds);
        visibleImageIds.slice(start, end + 1).forEach((id) => nextSelection.add(id));
        setSelectedImageIds(Array.from(nextSelection));
        setLastSelectedImageId(imageId);
        return;
      }
    }

    toggleImageSelection(imageId);
    setLastSelectedImageId(imageId);
  };

  const updateDragPreview = (nextRect: SelectionRect) => {
    const grid = gridRef.current;
    if (!grid) {
      return;
    }

    const gridRect = grid.getBoundingClientRect();
    const previewIds = Array.from(grid.querySelectorAll<HTMLElement>("[data-thumbnail-id]"))
      .filter((element) => {
        const rect = element.getBoundingClientRect();
        const left = rect.left - gridRect.left;
        const top = rect.top - gridRect.top;
        const right = left + rect.width;
        const bottom = top + rect.height;
        const selectionRight = nextRect.left + nextRect.width;
        const selectionBottom = nextRect.top + nextRect.height;

        return !(right < nextRect.left || left > selectionRight || bottom < nextRect.top || top > selectionBottom);
      })
      .map((element) => element.dataset.thumbnailId)
      .filter((value): value is string => Boolean(value));

    setDragPreviewIds(previewIds);
  };

  const handleGridPointerDown = (event: React.PointerEvent<HTMLDivElement>) => {
    if (!boxSelectMode || event.button !== 0) {
      return;
    }

    if ((event.target as HTMLElement).closest("a, button, input, label")) {
      return;
    }

    const grid = gridRef.current;
    if (!grid) {
      return;
    }

    const bounds = grid.getBoundingClientRect();
    const originX = event.clientX - bounds.left;
    const originY = event.clientY - bounds.top;
    dragStateRef.current = {
      pointerId: event.pointerId,
      originX,
      originY,
      dragged: false,
    };
    setSelectionRect({ left: originX, top: originY, width: 0, height: 0 });
    setDragPreviewIds([]);
    grid.setPointerCapture(event.pointerId);
  };

  const handleGridPointerMove = (event: React.PointerEvent<HTMLDivElement>) => {
    const dragState = dragStateRef.current;
    const grid = gridRef.current;
    if (!dragState || dragState.pointerId !== event.pointerId || !grid) {
      return;
    }

    const bounds = grid.getBoundingClientRect();
    const currentX = event.clientX - bounds.left;
    const currentY = event.clientY - bounds.top;
    const nextRect = buildSelectionRect(dragState.originX, dragState.originY, currentX, currentY);
    dragState.dragged = dragState.dragged || nextRect.width > 4 || nextRect.height > 4;
    dragStateRef.current = dragState;
    setSelectionRect(nextRect);
    updateDragPreview(nextRect);
  };

  const finishDragSelection = (pointerId: number) => {
    const dragState = dragStateRef.current;
    const grid = gridRef.current;
    if (!dragState || dragState.pointerId !== pointerId || !grid) {
      return;
    }

    if (dragState.dragged && dragPreviewIds.length > 0) {
      const nextSelection = new Set(selectedImageIds);
      dragPreviewIds.forEach((imageId) => nextSelection.add(imageId));
      setSelectedImageIds(Array.from(nextSelection));
      setLastSelectedImageId(dragPreviewIds[dragPreviewIds.length - 1] ?? lastSelectedImageId);
      suppressNextToggleRef.current = true;
    }

    if (grid.hasPointerCapture(pointerId)) {
      grid.releasePointerCapture(pointerId);
    }
    dragStateRef.current = null;
    setSelectionRect(null);
    setDragPreviewIds([]);
  };

  if (!data) {
    return null;
  }

  return (
    <div className="browser-layout">
      <Panel title="Filters" subtitle="Filter by source, category, annotation state, filename, and annotation size cues.">
        <div className="filter-group">
          <label className="filter-label">Source Folder</label>
          {data.sources.map((source) => (
            <label className="checkbox-row" key={source.id}>
              <input checked={selectedSourceIds.includes(source.id)} onChange={() => toggleSource(source.id)} type="checkbox" />
              <span>{source.name}</span>
            </label>
          ))}
        </div>

        <div className="filter-group">
          <label className="filter-label">Category</label>
          {data.categories.map((category) => (
            <label className="checkbox-row" key={category.id}>
              <input checked={selectedCategoryIds.includes(category.id)} onChange={() => toggleCategory(category.id)} type="checkbox" />
              <span>{category.name}</span>
            </label>
          ))}
        </div>

        <div className="filter-group">
          <label className="filter-label">Annotation Status</label>
          <label className="radio-row">
            <input checked={annotationFilter === "all"} onChange={() => setAnnotationFilter("all")} type="radio" />
            <span>All</span>
          </label>
          <label className="radio-row">
            <input checked={annotationFilter === "annotated"} onChange={() => setAnnotationFilter("annotated")} type="radio" />
            <span>Annotated</span>
          </label>
          <label className="radio-row">
            <input checked={annotationFilter === "unannotated"} onChange={() => setAnnotationFilter("unannotated")} type="radio" />
            <span>Unannotated</span>
          </label>
        </div>

        <div className="filter-group">
          <label className="filter-label" htmlFor="annotation-count-filter">Annotation Count</label>
          <select
            className="search-input"
            id="annotation-count-filter"
            onChange={(event) => setAnnotationCountFilter(event.target.value as AnnotationCountFilter)}
            value={annotationCountFilter}
          >
            {annotationCountOptions.map((option) => (
              <option key={option.value || "any"} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
          <div className="helper-text">Keep images whose box count matches this bucket.</div>
        </div>

        <div className="filter-group">
          <label className="filter-label" htmlFor="min-largest-box-ratio">Box Size Ratio</label>
          <input
            className="search-input"
            id="min-largest-box-ratio"
            inputMode="decimal"
            max="100"
            min="0"
            onChange={(event) => setMinBoxAreaRatioPercent(sanitizePercentInput(event.target.value))}
            placeholder="Minimum Largest Box Ratio (%)"
            type="number"
            value={minBoxAreaRatioPercent}
          />
          <div className="helper-text">Keep images where any single box covers at least this percentage of the image area.</div>
        </div>

        <div className="legend">
          <span className="legend-item">
            <span className="legend-swatch legend-swatch-green" />
            Annotated
          </span>
          <span className="legend-item">
            <span className="legend-swatch legend-swatch-amber" />
            Unannotated
          </span>
        </div>
      </Panel>

      <Panel
        title="Browser"
        subtitle={`${filteredImages.length} images match the current filters. Showing ${visibleImages.length}.`}
        actions={
          <div className="toolbar-actions">
            <input className="search-input" onChange={(event) => setSearch(event.target.value)} placeholder="Search filename..." value={search} />
            <button className="button button-secondary" onClick={() => setBoxSelectMode((current) => !current)} type="button">
              {boxSelectMode ? "Box Select On" : "Box Select Off"}
            </button>
            <button className="button button-secondary" onClick={toggleFilteredSelection} type="button">
              {allFilteredSelected ? "Deselect Filtered" : "Select All Filtered"}
            </button>
            <button className="button button-secondary" disabled={selectedImageIds.length === 0} onClick={clearSelection} type="button">
              Clear Selection
            </button>
            <button className="button button-secondary" onClick={() => navigate(`/workspace/${workspaceId}/export?scope=selected`)} type="button">
              Export Selected
            </button>
            <button className="button button-primary" onClick={() => navigate(`/workspace/${workspaceId}/cvat`)} type="button">
              Send to CVAT
            </button>
          </div>
        }
      >
        <div className="browser-toolbar browser-toolbar-strong">
          <span>Selected: {selectedImageIds.length}</span>
          <span>{selectedFilteredCount} of {filteredImages.length} filtered images selected</span>
          <span>
            Tip: click to toggle, hold Shift to select a range, and {boxSelectMode ? "drag on the loaded grid to box-select." : "turn on Box Select to drag over loaded thumbnails."}
          </span>
        </div>

        <div
          className={`thumbnail-grid thumbnail-grid-selectable${boxSelectMode ? " thumbnail-grid-box-mode" : ""}`}
          onPointerDown={handleGridPointerDown}
          onPointerMove={handleGridPointerMove}
          onPointerUp={(event) => finishDragSelection(event.pointerId)}
          onPointerCancel={(event) => finishDragSelection(event.pointerId)}
          ref={gridRef}
        >
          {selectionRect ? (
            <div
              className="thumbnail-selection-rect"
              style={{
                left: `${selectionRect.left}px`,
                top: `${selectionRect.top}px`,
                width: `${selectionRect.width}px`,
                height: `${selectionRect.height}px`,
              }}
            />
          ) : null}

          {visibleImages.map((image) => (
            <div className="thumbnail-grid-item" data-thumbnail-id={image.id} key={image.id}>
              <ThumbnailCard
                image={image}
                onToggle={handleImageToggle}
                selected={selectedImageIdSet.has(image.id) || dragPreviewSet.has(image.id)}
                workspaceId={workspaceId}
              />
            </div>
          ))}
        </div>

        {visibleImages.length < filteredImages.length ? (
          <div className="browser-load-more">
            <button className="button button-secondary" onClick={() => setVisibleCount((current) => current + 180)} type="button">
              Load 180 More
            </button>
          </div>
        ) : null}
      </Panel>
    </div>
  );
}
