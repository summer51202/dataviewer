import {
  sampleAnnotationVersions,
  sampleBrowserPayload,
  sampleDatasetMapPayload,
  sampleCvatTasks,
  sampleExportHistory,
  sampleExportPreview,
  sampleImageDetailPayload,
  sampleImportReviewRows,
  sampleRecentWorkspaces,
  sampleWorkspaceOverview,
} from "../lib/mock-data";
import { hasTauriRuntime, invokeOrFallback } from "../lib/tauri";
import {
  AddSourceFolderInput,
  AnnotationVersion,
  BrowserPayload,
  CreateWorkspaceInput,
  CreateCvatTaskInput,
  CvatSettings,
  CvatTask,
  DatasetMapPayload,
  DatasetMapScope,
  DatasetReviewStatus,
  EmbeddingJob,
  EmbeddingRuntimeProbe,
  EmbeddingRuntimePreference,
  ExportHistoryEntry,
  ExportPreview,
  ExportPreviewInput,
  ImageDetailPayload,
  ImportReviewRow,
  OpenCvatInput,
  OpenWorkspaceInput,
  RecentWorkspace,
  RemoveSourceFolderInput,
  RescanSourceFolderInput,
  SampleSelectionInput,
  SampleSelectionSummary,
  SampleSet,
  SampleSetMembers,
  SaveImportReviewInput,
  ScanProgress,
  SourceFolder,
  StartExportInput,
  StartExportResult,
  SyncCvatTaskInput,
  WorkspaceCreateTargetCheck,
  WorkspaceOverview,
} from "../types/workspace";

function buildWorkspaceTargetPath(parentPath: string, name: string) {
  const trimmedParentPath = parentPath.trim().replace(/[\\/]+$/, "");
  const trimmedName = name.trim();

  if (!trimmedParentPath) {
    return trimmedName;
  }
  if (!trimmedName) {
    return trimmedParentPath;
  }

  return `${trimmedParentPath}\\${trimmedName}`;
}

export async function createWorkspace(input: CreateWorkspaceInput) {
  if (!hasTauriRuntime()) {
    const targetPath = buildWorkspaceTargetPath(input.parentPath, input.name);
    return {
      ...sampleWorkspaceOverview,
      id: input.name,
      name: input.name,
      workspacePath: targetPath,
    } satisfies WorkspaceOverview;
  }

  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<WorkspaceOverview>("create_workspace", { input });
}

export async function checkCreateWorkspaceTarget(input: CreateWorkspaceInput) {
  if (!hasTauriRuntime()) {
    return {
      targetPath: buildWorkspaceTargetPath(input.parentPath, input.name),
      status: "available",
    } satisfies WorkspaceCreateTargetCheck;
  }

  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<WorkspaceCreateTargetCheck>("check_create_workspace_target", { input });
}

export async function openWorkspace(input: OpenWorkspaceInput) {
  return invokeOrFallback<WorkspaceOverview>("open_workspace", { input }, sampleWorkspaceOverview);
}

export async function addSourceFolder(input: AddSourceFolderInput) {
  if (!hasTauriRuntime()) {
    const folderName = input.sourcePath.split(/[/\\]/).filter(Boolean).pop() ?? "new_source";
    return [
      {
        id: `source-${Date.now()}`,
        name: folderName,
        path: input.sourcePath,
        type: "RAW" as const,
        status: "review" as const,
        imageCount: 0,
        categoryCount: 0,
        corruptedImageCount: 0,
        corruptedImagePaths: [],
        lastScanAt: "Not scanned yet",
      },
      ...sampleWorkspaceOverview.sources,
    ];
  }

  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<SourceFolder[]>("add_source_folder", { input });
}

export async function rescanSourceFolder(input: RescanSourceFolderInput) {
  if (!hasTauriRuntime()) {
    return sampleWorkspaceOverview.sources.map((source) =>
      source.id === input.sourceId
        ? {
            ...source,
            imageCount: 12,
            categoryCount: source.type === "RAW" ? 0 : 3,
            lastScanAt: new Date().toISOString(),
          }
        : source,
    );
  }

  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<SourceFolder[]>("rescan_source_folder", { input });
}

export async function removeSourceFolder(input: RemoveSourceFolderInput) {
  if (!hasTauriRuntime()) {
    return sampleWorkspaceOverview.sources.filter((source) => source.id !== input.sourceId);
  }

  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<SourceFolder[]>("remove_source_folder", { input });
}

export async function listRecentWorkspaces() {
  return invokeOrFallback<RecentWorkspace[]>(
    "list_recent_workspaces",
    undefined,
    sampleRecentWorkspaces,
  );
}

export async function removeRecentWorkspace(workspaceId: string) {
  if (!hasTauriRuntime()) {
    return sampleRecentWorkspaces.filter((workspace) => workspace.id !== workspaceId);
  }

  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<RecentWorkspace[]>("remove_recent_workspace", { workspaceId });
}

export async function getWorkspaceOverview(workspaceId: string) {
  return invokeOrFallback<WorkspaceOverview>(
    "get_workspace_overview",
    { workspaceId },
    sampleWorkspaceOverview,
  );
}

export async function getSourceFolders(workspaceId: string) {
  return invokeOrFallback<SourceFolder[]>(
    "get_source_folders",
    { workspaceId },
    sampleWorkspaceOverview.sources,
  );
}

export async function getScanProgress(workspaceId: string) {
  return invokeOrFallback<ScanProgress[]>(
    "get_scan_progress",
    { workspaceId },
    [],
  );
}

export async function getImportReview(workspaceId: string) {
  return invokeOrFallback<ImportReviewRow[]>(
    "get_import_review",
    { workspaceId },
    sampleImportReviewRows,
  );
}

export async function saveImportReview(input: SaveImportReviewInput) {
  if (!hasTauriRuntime()) {
    return input.rows;
  }

  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<ImportReviewRow[]>("save_import_review", { input });
}

export async function getBrowserPayload(workspaceId: string) {
  return invokeOrFallback<BrowserPayload>(
    "get_browser_payload",
    { workspaceId },
    sampleBrowserPayload,
  );
}

export async function getDatasetMapPayload(input: {
  workspaceId: string;
  scope: DatasetMapScope;
  modelId?: string;
}) {
  return invokeOrFallback<DatasetMapPayload>(
    "get_dataset_map_payload",
    { input },
    {
      ...sampleDatasetMapPayload,
      workspaceId: input.workspaceId,
      scope: input.scope,
      modelId: input.modelId ?? sampleDatasetMapPayload.modelId,
    },
  );
}

export async function probeEmbeddingRuntime(input: {
  workspaceId: string;
  preference: EmbeddingRuntimePreference;
}) {
  return invokeOrFallback<EmbeddingRuntimeProbe>(
    "probe_embedding_runtime",
    { input },
    sampleDatasetMapPayload.runtime,
  );
}

export async function startEmbeddingJob(input: {
  workspaceId: string;
  scope: DatasetMapScope;
  modelId: string;
  runtimePreference: EmbeddingRuntimePreference;
}) {
  if (!hasTauriRuntime()) {
    return {
      id: `job-${Date.now()}`,
      workspaceId: input.workspaceId,
      scope: input.scope,
      modelId: input.modelId,
      runtimePreference: input.runtimePreference,
      runtimeBackend: "cpu" as const,
      status: "completed" as const,
      processedItems: sampleDatasetMapPayload.points.length,
      totalItems: sampleDatasetMapPayload.points.length,
      message: "Mock embedding job completed.",
      updatedAt: new Date().toISOString(),
    };
  }

  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<EmbeddingJob>("start_embedding_job", { input });
}

export async function saveDatasetMapReviews(input: {
  workspaceId: string;
  scope: DatasetMapScope;
  updates: Array<{ targetId: string; status: DatasetReviewStatus; reason?: string; note?: string }>;
}) {
  if (!hasTauriRuntime()) {
    return input.updates;
  }

  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<typeof input.updates>("save_dataset_map_reviews", { input });
}

export async function runSampleSelection(input: SampleSelectionInput) {
  if (!hasTauriRuntime()) {
    const total = sampleDatasetMapPayload.points.length;
    const target =
      input.targetImages ??
      Math.max(1, Math.round(total * (input.targetRatio ?? 0.3)));
    const selected = Math.min(target, total);
    return {
      sampleSet: input.name,
      mode: input.mode,
      selectedImages: selected,
      selectedObjects: selected,
      excludedOutliers: input.removeOutliers ? Math.round(total * 0.02) : 0,
      saturated: selected >= total,
      seed: input.seed ?? 42,
      totalImages: total,
    } satisfies SampleSelectionSummary;
  }

  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<SampleSelectionSummary>("run_sample_selection", { input });
}

export async function listSampleSets(workspaceId: string) {
  return invokeOrFallback<SampleSet[]>(
    "list_sample_sets",
    { input: { workspaceId } },
    [],
  );
}

export async function deleteSampleSet(workspaceId: string, name: string) {
  if (!hasTauriRuntime()) {
    return;
  }

  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<void>("delete_sample_set", { input: { workspaceId, name } });
}

export async function getSampleSetMembers(workspaceId: string, name: string) {
  return invokeOrFallback<SampleSetMembers>(
    "get_sample_set_members",
    { input: { workspaceId, name } },
    { imageIds: [], objectIds: [] },
  );
}

export async function getImageDetail(workspaceId: string, imageId: string) {
  return invokeOrFallback<ImageDetailPayload>(
    "get_image_detail",
    { workspaceId, imageId },
    sampleImageDetailPayload,
  );
}

export async function getCvatTasks(workspaceId: string) {
  return invokeOrFallback<CvatTask[]>("get_cvat_tasks", { workspaceId }, sampleCvatTasks);
}

export async function getCvatSettings(workspaceId: string) {
  return invokeOrFallback<CvatSettings>(
    "get_cvat_settings",
    { workspaceId },
    { baseUrl: "http://localhost:8080", accessToken: "" },
  );
}

export async function saveCvatSettings(workspaceId: string, settings: CvatSettings) {
  if (!hasTauriRuntime()) {
    return settings;
  }

  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<CvatSettings>("save_cvat_settings", { workspaceId, settings });
}

export async function testCvatSettings(workspaceId: string) {
  if (!hasTauriRuntime()) {
    return;
  }

  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<void>("test_cvat_settings", { workspaceId });
}

export async function createCvatTask(input: CreateCvatTaskInput) {
  if (!hasTauriRuntime()) {
    return sampleCvatTasks;
  }

  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<CvatTask[]>("create_cvat_task", { input });
}

export async function openCvat(input: OpenCvatInput) {
  if (!hasTauriRuntime()) {
    return;
  }

  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<void>("open_cvat", { input });
}

export async function syncCvatTask(input: SyncCvatTaskInput) {
  if (!hasTauriRuntime()) {
    return sampleCvatTasks;
  }

  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<CvatTask[]>("sync_cvat_task", { input });
}

export async function getAnnotationVersions(workspaceId: string) {
  return invokeOrFallback<AnnotationVersion[]>(
    "get_annotation_versions",
    { workspaceId },
    sampleAnnotationVersions,
  );
}

export async function getExportPreview(input: ExportPreviewInput) {
  return invokeOrFallback<ExportPreview>(
    "get_export_preview",
    { input },
    sampleExportPreview,
  );
}

export async function getExportHistory(workspaceId: string) {
  return invokeOrFallback<ExportHistoryEntry[]>(
    "get_export_history",
    { workspaceId },
    sampleExportHistory,
  );
}

export async function startExport(input: StartExportInput) {
  if (!hasTauriRuntime()) {
    return {
      outputFormat: input.outputFormat,
      outputPath: input.outputPath,
      exportedImages: sampleExportPreview.includedImages,
      exportedBoxes: sampleExportPreview.includedBoxes,
    } satisfies StartExportResult;
  }

  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<StartExportResult>("start_export", { input });
}

export async function openExportFolder(path: string) {
  if (!hasTauriRuntime()) {
    return;
  }

  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<void>("open_export_folder", { path });
}
