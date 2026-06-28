import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { useNavigate, useParams, useSearchParams } from "react-router-dom";

import { Panel } from "../../../components/ui/Panel";
import { StatCard } from "../../../components/ui/StatCard";
import {
  getBrowserPayload,
  getExportPreview,
  getSampleSetMembers,
  listSampleSets,
  openExportFolder,
  startExport,
} from "../../../lib/api";
import { describeError, pickFolder } from "../../../lib/tauri";
import { useWorkspaceStore } from "../../../state/useWorkspaceStore";
import { ExportScopeMode, resolveExportScopeImages } from "../browserScope";

type CategoryRatioRow = {
  name: string;
  imageCount: number;
  ratio: number;
};

type SplitCounts = {
  train: number;
  valid: number;
  test: number;
};

function buildSuggestedOutputPath(baseOutputPath: string, outputFormat: string) {
  const nextSuffix = outputFormat.toLowerCase() === "yolo" ? "-yolo" : "-coco";

  if (baseOutputPath.endsWith("-coco")) {
    return `${baseOutputPath.slice(0, -5)}${nextSuffix}`;
  }

  if (baseOutputPath.endsWith("-yolo")) {
    return `${baseOutputPath.slice(0, -5)}${nextSuffix}`;
  }

  return `${baseOutputPath}${nextSuffix}`;
}

function computeSplitCounts(total: number, trainRatio: number, validRatio: number, testRatio: number): SplitCounts {
  const ratioSum = trainRatio + validRatio + testRatio;
  if (total === 0 || ratioSum === 0) {
    return { train: 0, valid: 0, test: 0 };
  }

  const train = Math.floor(total * (trainRatio / ratioSum));
  const valid = Math.floor(total * (validRatio / ratioSum));
  const test = Math.max(0, total - train - valid);

  return { train, valid, test };
}

function parseRatio(value: string) {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : 0;
}

function formatPercent(value: number) {
  return `${(value * 100).toFixed(1)}%`;
}

export function ExportPage() {
  const { workspaceId = "factory-defect-v1" } = useParams();
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const scopeMode = searchParams.get("scope") === "filtered" ? "filtered" : "selected";

  const search = useWorkspaceStore((state) => state.search);
  const annotationFilter = useWorkspaceStore((state) => state.annotationFilter);
  const selectedSourceIds = useWorkspaceStore((state) => state.selectedSourceIds);
  const selectedCategoryIds = useWorkspaceStore((state) => state.selectedCategoryIds);
  const annotationCountFilter = useWorkspaceStore((state) => state.annotationCountFilter);
  const minBoxAreaRatioPercent = useWorkspaceStore((state) => state.minBoxAreaRatioPercent);
  const selectedImageIds = useWorkspaceStore((state) => state.selectedImageIds);

  const { data: browserPayload } = useQuery({
    queryKey: ["browser", workspaceId],
    queryFn: () => getBrowserPayload(workspaceId),
  });

  const scopedImages = useMemo(
    () =>
      resolveExportScopeImages(
        browserPayload,
        {
          search,
          annotationFilter,
          selectedSourceIds,
          selectedCategoryIds,
          annotationCountFilter,
          minBoxAreaRatioPercent,
        },
        selectedImageIds,
        scopeMode as ExportScopeMode,
      ),
    [
      annotationCountFilter,
      annotationFilter,
      browserPayload,
      minBoxAreaRatioPercent,
      scopeMode,
      search,
      selectedCategoryIds,
      selectedImageIds,
      selectedSourceIds,
    ],
  );
  const browserImageIds = useMemo(() => scopedImages.map((image) => image.id), [scopedImages]);

  const [sampleSetName, setSampleSetName] = useState("");

  const sampleSetsQuery = useQuery({
    queryKey: ["sample-sets", workspaceId],
    queryFn: () => listSampleSets(workspaceId),
  });

  const sampleMembersQuery = useQuery({
    queryKey: ["sample-set-members", workspaceId, sampleSetName],
    queryFn: () => getSampleSetMembers(workspaceId, sampleSetName),
    enabled: sampleSetName !== "",
  });

  // When a sample set is chosen it overrides the Browser scope.
  const exportImageIds = useMemo(
    () => (sampleSetName ? sampleMembersQuery.data?.imageIds ?? [] : browserImageIds),
    [sampleSetName, sampleMembersQuery.data, browserImageIds],
  );

  const { data } = useQuery({
    queryKey: ["export-preview", workspaceId, scopeMode, sampleSetName, exportImageIds],
    queryFn: () =>
      getExportPreview({
        workspaceId,
        imageIds: exportImageIds,
      }),
    enabled: !!browserPayload && (sampleSetName === "" || sampleMembersQuery.isSuccess),
  });

  const [outputFormat, setOutputFormat] = useState("COCO");
  const [trainRatio, setTrainRatio] = useState("80");
  const [validRatio, setValidRatio] = useState("15");
  const [testRatio, setTestRatio] = useState("5");
  const [randomSeed, setRandomSeed] = useState("42");
  const [outputPath, setOutputPath] = useState("");
  const [lastSuggestedPath, setLastSuggestedPath] = useState("");
  const [allowAutoRenameConflicts, setAllowAutoRenameConflicts] = useState(true);
  const [excludeDatasetMapItems, setExcludeDatasetMapItems] = useState(false);
  const [errorDetail, setErrorDetail] = useState("");
  const [folderOpenError, setFolderOpenError] = useState("");

  const exportMutation = useMutation({
    mutationFn: () =>
      startExport({
        workspaceId,
        outputFormat,
        trainRatio: Number(trainRatio),
        validRatio: Number(validRatio),
        testRatio: Number(testRatio),
        randomSeed: Number(randomSeed),
        outputPath,
        allowAutoRenameConflicts,
        excludeDatasetMapItems,
        imageIds: exportImageIds,
      }),
    onMutate: () => {
      setErrorDetail("");
    },
    onError: (error: unknown) => {
      setErrorDetail(describeError(error));
    },
  });

  const openFolderMutation = useMutation({
    mutationFn: (path: string) => openExportFolder(path),
    onMutate: () => {
      setFolderOpenError("");
    },
    onError: (error: unknown) => {
      setFolderOpenError(describeError(error));
    },
  });

  const scopeSummary = useMemo(() => {
    const annotatedCount = scopedImages.filter((image) => image.annotationStatus === "annotated").length;
    const categoryNames = Array.from(
      scopedImages.reduce((accumulator, image) => {
        image.categories.forEach((category) => accumulator.add(category));
        return accumulator;
      }, new Set<string>()),
    ).sort();
    const sourceNames = Array.from(
      scopedImages.reduce((accumulator, image) => {
        accumulator.add(image.sourceName);
        return accumulator;
      }, new Set<string>()),
    ).sort();

    return {
      annotatedCount,
      categoryNames,
      sourceNames,
      totalCount: scopedImages.length,
      unannotatedCount: scopedImages.length - annotatedCount,
    };
  }, [scopedImages]);

  const exportableCategoryRatios = useMemo<CategoryRatioRow[]>(() => {
    const exportableImages = scopedImages.filter((image) => image.annotationStatus === "annotated");
    if (exportableImages.length === 0) {
      return [];
    }

    const counts = new Map<string, number>();
    exportableImages.forEach((image) => {
      const uniqueCategories = new Set(image.categories);
      uniqueCategories.forEach((category) => {
        counts.set(category, (counts.get(category) ?? 0) + 1);
      });
    });

    return Array.from(counts.entries())
      .map(([name, imageCount]) => ({
        name,
        imageCount,
        ratio: imageCount / exportableImages.length,
      }))
      .sort((left, right) => right.imageCount - left.imageCount || left.name.localeCompare(right.name));
  }, [scopedImages]);

  const computedSplitCounts = useMemo(
    () => computeSplitCounts(data?.includedImages ?? 0, parseRatio(trainRatio), parseRatio(validRatio), parseRatio(testRatio)),
    [data?.includedImages, testRatio, trainRatio, validRatio],
  );

  useEffect(() => {
    if (data?.outputPath) {
      const suggestedPath = buildSuggestedOutputPath(data.outputPath, outputFormat);
      setOutputPath((current) => current || suggestedPath);
      setLastSuggestedPath(suggestedPath);
    }
  }, [data?.outputPath, outputFormat]);

  useEffect(() => {
    if (!data?.outputPath) {
      return;
    }

    const nextSuggestedPath = buildSuggestedOutputPath(data.outputPath, outputFormat);
    setOutputPath((current) => (current === "" || current === lastSuggestedPath ? nextSuggestedPath : current));
    setLastSuggestedPath(nextSuggestedPath);
  }, [data?.outputPath, lastSuggestedPath, outputFormat]);

  const handleBrowseOutputFolder = async () => {
    setFolderOpenError("");
    const selectedPath = await pickFolder();
    if (!selectedPath) {
      return;
    }

    setOutputPath(selectedPath);
  };

  if (!data) {
    return null;
  }

  return (
    <div className="export-layout">
      {exportMutation.isError ? (
        <div className="status-banner status-banner-error">
          <strong>Export failed.</strong>
          <span>{errorDetail || "Please check the selected format, conflict settings, and output path."}</span>
        </div>
      ) : null}

      {exportMutation.isSuccess ? (
        <div className="status-banner status-banner-success">
          <strong>{exportMutation.data.outputFormat} export completed.</strong>
          <span>{exportMutation.data.outputPath}</span>
          <div className="status-banner-actions">
            <button
              className="button button-secondary button-sm"
              disabled={openFolderMutation.isPending}
              onClick={() => openFolderMutation.mutate(exportMutation.data.outputPath)}
              type="button"
            >
              {openFolderMutation.isPending ? "Opening..." : "Open Output Folder"}
            </button>
          </div>
        </div>
      ) : null}

      {folderOpenError ? (
        <div className="status-banner status-banner-error">
          <strong>Open folder failed.</strong>
          <span>{folderOpenError}</span>
        </div>
      ) : null}

      {exportMutation.isPending ? (
        <div className="task-status" role="status">
          <strong>Exporting</strong>
          <span>
            Preparing {outputFormat} dataset with seed {randomSeed} and split {trainRatio}/{validRatio}/{testRatio}.
          </span>
        </div>
      ) : null}

      {data.filenameConflicts > 0 ? (
        <div className="status-banner status-banner-warning">
          <strong>Filename conflicts detected.</strong>
          <span>
            {allowAutoRenameConflicts
              ? `${data.filenameConflicts} conflict groups will be auto-renamed during export unless you turn this option off.`
              : `${data.filenameConflicts} conflict groups are inside the current export scope. Review them below and enable auto rename before exporting.`}
          </span>
        </div>
      ) : null}

      {data.datasetMapExcludedImages > 0 || data.datasetMapExcludedBoxes > 0 ? (
        <div className="status-banner status-banner-warning">
          <strong>Dataset Map exclusions detected.</strong>
          <span>
            {data.datasetMapExcludedImages} images and {data.datasetMapExcludedBoxes} boxes are marked exclude. Export output is unchanged unless the Dataset Map exclusion option is enabled.
          </span>
        </div>
      ) : null}

      {scopeSummary.totalCount === 0 && !sampleSetName ? (
        <div className="status-banner status-banner-warning">
          <strong>No images in the current export scope.</strong>
          <span>Go back to Browser to adjust filters or select images first.</span>
        </div>
      ) : null}

      {sampleSetName ? (
        <div className="status-banner status-banner-info">
          <strong>Sample set scope active: {sampleSetName}</strong>
          <span>
            Exporting {exportImageIds.length} images from this sample set. Browser scope is ignored.
            Counts in Export Summary below are authoritative.
          </span>
        </div>
      ) : null}

      <Panel
        title="Export Scope"
        subtitle={scopeMode === "selected" ? "Using the images currently selected in Browser." : "Using the images currently visible under Browser filters."}
        actions={
          <div className="button-row">
            <button className="button button-secondary" onClick={() => navigate(`/workspace/${workspaceId}/browser`)} type="button">
              Back to Browser
            </button>
          </div>
        }
      >
        <div className="stats-grid">
          <StatCard hint={scopeMode === "selected" ? "currently selected" : "matched by current Browser filters"} label="Selected Images" value={scopeSummary.totalCount} />
          <StatCard hint="can be exported immediately" label="Annotated" value={scopeSummary.annotatedCount} />
          <StatCard hint="excluded from export" label="Unannotated" value={scopeSummary.unannotatedCount} />
          <StatCard hint="present in scoped images" label="Categories" value={scopeSummary.categoryNames.length} />
          <StatCard hint="present in scoped images" label="Sources" value={scopeSummary.sourceNames.length} />
        </div>

        <div className="export-scope-notes">
          <div className="export-scope-chip-row">
            <span className="chip">Scope: {scopeMode === "selected" ? "Selected Images" : "Filtered Browser Results"}</span>
            <span className="chip">Annotation Filter: {annotationFilter}</span>
            {search.trim() ? <span className="chip">Search: {search.trim()}</span> : null}
          </div>

          <div className="export-scope-list">
            <strong>Sources</strong>
            <span>{scopeSummary.sourceNames.length > 0 ? scopeSummary.sourceNames.join(", ") : "All visible sources"}</span>
          </div>

          <div className="export-scope-list">
            <strong>Categories</strong>
            <span>{scopeSummary.categoryNames.length > 0 ? scopeSummary.categoryNames.join(", ") : "No categories in current scope"}</span>
          </div>

          <div className="export-scope-list">
            <strong>Category Image Ratio</strong>
            {exportableCategoryRatios.length > 0 ? (
              <div className="ratio-list">
                {exportableCategoryRatios.map((row) => (
                  <div className="ratio-row" key={row.name}>
                    <span>{row.name}</span>
                    <strong>
                      {row.imageCount} images ({formatPercent(row.ratio)})
                    </strong>
                  </div>
                ))}
              </div>
            ) : (
              <span>No exportable annotated images in the current scope yet.</span>
            )}
          </div>
        </div>
      </Panel>

      <Panel title="Export Settings" subtitle="Configure output format, split ratios, conflict handling, and target folder.">
        <div className="export-settings-grid">
          <label className="field">
            <span>Sample Set</span>
            <select value={sampleSetName} onChange={(event) => setSampleSetName(event.target.value)}>
              <option value="">None (use Browser scope)</option>
              {(sampleSetsQuery.data ?? []).map((set) => (
                <option key={set.id} value={set.name}>
                  {set.name} ({set.selectedImages} images)
                </option>
              ))}
            </select>
            <span className="field-help">
              Selecting a sample set exports exactly its images and overrides the Browser scope.
            </span>
          </label>

          <label className="field">
            <span>Output Format</span>
            <div className="field-inline">
              <label className="radio-row">
                <input checked={outputFormat === "COCO"} onChange={() => setOutputFormat("COCO")} type="radio" />
                <span>COCO</span>
              </label>
              <label className="radio-row">
                <input checked={outputFormat === "YOLO"} onChange={() => setOutputFormat("YOLO")} type="radio" />
                <span>YOLO</span>
              </label>
            </div>
          </label>

          <label className="field">
            <span>Split Ratios</span>
            <div className="field-inline field-inline-split">
              <input onChange={(event) => setTrainRatio(event.target.value)} type="number" value={trainRatio} />
              <input onChange={(event) => setValidRatio(event.target.value)} type="number" value={validRatio} />
              <input onChange={(event) => setTestRatio(event.target.value)} type="number" value={testRatio} />
            </div>
          </label>

          <label className="field">
            <span>Random Seed</span>
            <input onChange={(event) => setRandomSeed(event.target.value)} type="number" value={randomSeed} />
          </label>

          <label className="field export-output-field">
            <span>Output Folder</span>
            <div className="field-inline">
              <input onChange={(event) => setOutputPath(event.target.value)} type="text" value={outputPath} />
              <button className="button button-secondary button-sm" onClick={() => void handleBrowseOutputFolder()} type="button">
                Browse Folder
              </button>
            </div>
            <span className="field-help">Suggested path updates with the selected export format, but you can also choose a custom export folder.</span>
          </label>
        </div>

        <div className="export-conflict-setting">
          <div className="export-conflict-copy">
            <strong>Conflict Handling</strong>
            <span>
              Auto rename is enabled by default so export can keep moving when multiple files resolve to the same output name.
            </span>
          </div>
          <label className="checkbox-row export-conflict-checkbox">
            <input
              checked={allowAutoRenameConflicts}
              onChange={(event) => setAllowAutoRenameConflicts(event.target.checked)}
              type="checkbox"
            />
            <span>Allow auto rename for filename conflicts during export</span>
          </label>
        </div>

        <div className="export-conflict-setting">
          <div className="export-conflict-copy">
            <strong>Dataset Map</strong>
            <span>
              Apply image-level and object-level exclude marks from Dataset Map to this export.
            </span>
          </div>
          <label className="checkbox-row export-conflict-checkbox">
            <input
              checked={excludeDatasetMapItems}
              onChange={(event) => setExcludeDatasetMapItems(event.target.checked)}
              type="checkbox"
            />
            <span>Exclude items marked exclude in Dataset Map</span>
          </label>
        </div>

        <div className="button-row">
          <button className="button button-secondary" onClick={() => navigate(`/workspace/${workspaceId}/browser`)} type="button">
            Adjust Browser Scope
          </button>
          <button
            className="button button-primary"
            disabled={
              exportMutation.isPending ||
              (!sampleSetName && scopeSummary.totalCount === 0) ||
              data.includedImages === 0 ||
              (data.filenameConflicts > 0 && !allowAutoRenameConflicts)
            }
            onClick={() => exportMutation.mutate()}
            type="button"
          >
            {exportMutation.isPending ? "Exporting..." : "Start Export"}
          </button>
        </div>
      </Panel>

      {data.conflictDetails.length > 0 ? (
        <Panel title="Filename Conflicts" subtitle="These files resolve to the same output filename inside the current scope.">
          <div className="conflict-list">
            {data.conflictDetails.map((conflict) => (
              <div className="conflict-card" key={conflict.fileName}>
                <div className="conflict-title">{conflict.fileName}</div>
                <div className="conflict-items">
                  {conflict.items.map((item) => (
                    <div className="conflict-item" key={item.imageId}>
                      <div className="conflict-source">{item.sourceId}</div>
                      <div className="conflict-path" title={item.originalPath}>{item.originalPath}</div>
                    </div>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </Panel>
      ) : null}

      <Panel title="Export Summary" subtitle="Preview the final candidate pool before generating a standalone dataset.">
        <div className="stats-grid">
          <StatCard hint="categories present in exported annotations" label="Categories" value={data.categoryCount} />
          <StatCard hint="annotated images after scope filtering" label="Images Included" value={data.includedImages} />
          <StatCard hint="in scope but not exportable" label="Images Excluded" value={data.excludedImages} />
          <StatCard hint="opt-in Dataset Map filter" label="Map Excluded" value={data.datasetMapExcludedImages + data.datasetMapExcludedBoxes} />
          <StatCard hint="total annotations" label="Boxes Included" value={data.includedBoxes} />
          <StatCard hint="pending manual review" label="Filename Conflicts" value={data.filenameConflicts} />
        </div>

        <div className="split-preview">
          <div>train: {computedSplitCounts.train}</div>
          <div>valid: {computedSplitCounts.valid}</div>
          <div>test: {computedSplitCounts.test}</div>
        </div>
      </Panel>
    </div>
  );
}
