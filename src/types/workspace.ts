export type HealthStatus = "healthy" | "warning";
export type SourceType = "COCO" | "YOLO" | "RAW";
export type SourceStatus = "ready" | "warning" | "review";
export type ImageHealthStatus = "healthy" | "corrupted";
export type AnnotationStatus = "annotated" | "unannotated";
export type AnnotationFilter = "all" | AnnotationStatus;
export type AnnotationCountFilter = "" | "0" | "1" | "2" | "3" | "4" | "5+";

export type RecentWorkspace = {
  id: string;
  name: string;
  workspacePath: string;
  healthStatus: HealthStatus;
  lastOpenedAt?: string | null;
  available: boolean;
};

export type CreateWorkspaceInput = {
  name: string;
  parentPath: string;
  allowExistingTarget?: boolean;
};

export type WorkspaceCreateTargetStatus =
  | "available"
  | "existing-empty"
  | "existing-workspace"
  | "existing-nonempty";

export type WorkspaceCreateTargetCheck = {
  targetPath: string;
  status: WorkspaceCreateTargetStatus;
};

export type OpenWorkspaceInput = {
  workspacePath: string;
};

export type AddSourceFolderInput = {
  workspaceId: string;
  sourcePath: string;
};

export type RescanSourceFolderInput = {
  workspaceId: string;
  sourceId: string;
};

export type RemoveSourceFolderInput = {
  workspaceId: string;
  sourceId: string;
};

export type SourceFolder = {
  id: string;
  name: string;
  path: string;
  type: SourceType;
  status: SourceStatus;
  imageCount: number;
  categoryCount: number;
  corruptedImageCount: number;
  corruptedImagePaths: string[];
  lastScanAt: string;
};

export type ScanProgress = {
  sourceId: string;
  sourceName: string;
  stage: string;
  processed: number;
  total: number;
};

export type UnifiedCategory = {
  id: string;
  name: string;
  imageCount: number;
};

export type WorkspaceOverview = {
  id: string;
  name: string;
  workspacePath: string;
  healthStatus: HealthStatus;
  sources: SourceFolder[];
  categories: UnifiedCategory[];
};

export type ImportReviewRow = {
  sourceCategoryId: string;
  sourceCategory: string;
  sourcePath: string;
  count: number;
  sourceTotalImageCount: number;
  suggestedAction: string;
  targetUnifiedCategory?: string;
  finalAction: "Merge" | "Create New" | "Ignore";
};

export type SaveImportReviewInput = {
  workspaceId: string;
  rows: ImportReviewRow[];
};

export type BoxSummary = {
  categoryName: string;
  areaRatio?: number | null;
};

export type ImageCard = {
  id: string;
  filename: string;
  sourceId: string;
  sourceName: string;
  originalPath: string;
  annotationStatus: AnnotationStatus;
  imageHealthStatus: ImageHealthStatus;
  imageHealthError?: string | null;
  annotationCount: number;
  maxBoxAreaRatio?: number | null;
  boxSummaries: BoxSummary[];
  categoryIds: string[];
  categories: string[];
};

export type BrowserPayload = {
  sources: SourceFolder[];
  categories: UnifiedCategory[];
  images: ImageCard[];
};

export type BoundingBoxRecord = {
  id: string;
  categoryName: string;
  annotationFormat: string;
  bboxX: number;
  bboxY: number;
  bboxWidth: number;
  bboxHeight: number;
};

export type ImageDetailPayload = {
  id: string;
  filename: string;
  sourceId: string;
  sourceName: string;
  originalPath: string;
  annotationStatus: AnnotationStatus;
  imageHealthStatus: ImageHealthStatus;
  imageHealthError?: string | null;
  categories: string[];
  width?: number | null;
  height?: number | null;
  boxes: BoundingBoxRecord[];
};

export type CvatTask = {
  id: string;
  name: string;
  imageCount: number;
  status: string;
  projectName: string;
  lastSyncAt: string | null;
  tempFolder?: string | null;
  remoteTaskId?: number | null;
  remoteUrl?: string | null;
};

export type CvatSettings = {
  baseUrl: string;
  accessToken: string;
};

export type CreateCvatTaskInput = {
  workspaceId: string;
  imageIds: string[];
  taskName?: string;
};

export type SyncCvatTaskInput = {
  workspaceId: string;
  taskId: string;
};

export type OpenCvatInput = {
  workspaceId: string;
  taskId?: string;
};

export type AnnotationVersion = {
  id: string;
  label: string;
  createdAt: string;
  sourceTask: string;
  imageCount: number;
  boxCount: number;
  notes: string;
};

export type ExportHistoryEntry = {
  id: string;
  outputFormat: string;
  outputPath: string;
  createdAt: string;
  status: string;
  exportedImages: number;
  exportedBoxes: number;
};

export type ExportPreview = {
  categoryCount: number;
  includedImages: number;
  excludedImages: number;
  includedBoxes: number;
  filenameConflicts: number;
  conflictDetails: {
    fileName: string;
    items: {
      imageId: string;
      sourceId: string;
      originalPath: string;
    }[];
  }[];
  splitCounts: {
    train: number;
    valid: number;
    test: number;
  };
  outputPath: string;
};

export type ExportPreviewInput = {
  workspaceId: string;
  imageIds?: string[];
  sourceIds?: string[];
  categoryIds?: string[];
};

export type StartExportInput = {
  workspaceId: string;
  outputFormat: string;
  trainRatio: number;
  validRatio: number;
  testRatio: number;
  randomSeed: number;
  outputPath: string;
  allowAutoRenameConflicts: boolean;
  imageIds?: string[];
  sourceIds?: string[];
  categoryIds?: string[];
};

export type StartExportResult = {
  outputFormat: string;
  outputPath: string;
  exportedImages: number;
  exportedBoxes: number;
};

export type DatasetMapScope = "object" | "image";
export type EmbeddingFamily = "clip" | "dinov2";
export type EmbeddingRuntimePreference = "auto" | "cuda" | "windows-gpu" | "cpu";
export type EmbeddingRuntimeBackend = "cuda" | "windows-gpu" | "cpu";
export type EmbeddingJobStatus =
  | "queued"
  | "running"
  | "completed"
  | "failed"
  | "cancelled";
export type DatasetReviewStatus =
  | "unreviewed"
  | "needs-review"
  | "keep"
  | "fix"
  | "exclude";

export type EmbeddingModelOption = {
  id: string;
  family: EmbeddingFamily;
  displayName: string;
  embeddingDim: number;
  inputSize: number;
  available: boolean;
  downloadRequired: boolean;
};

export type EmbeddingRuntimeCapability = {
  backend: EmbeddingRuntimeBackend;
  available: boolean;
  label: string;
  detail: string;
};

export type EmbeddingRuntimeProbe = {
  preference: EmbeddingRuntimePreference;
  selectedBackend: EmbeddingRuntimeBackend;
  capabilities: EmbeddingRuntimeCapability[];
  fallbackReason?: string | null;
};

export type DatasetMapPoint = {
  id: string;
  scope: DatasetMapScope;
  imageId: string;
  annotationId?: string | null;
  filename: string;
  sourceId: string;
  sourceName: string;
  categoryId?: string | null;
  categoryName?: string | null;
  bbox?: {
    x: number;
    y: number;
    width: number;
    height: number;
    areaRatio?: number | null;
  } | null;
  x: number;
  y: number;
  reviewStatus: DatasetReviewStatus;
};

export type EmbeddingJob = {
  id: string;
  scope: DatasetMapScope;
  modelId: string;
  runtimePreference: EmbeddingRuntimePreference;
  runtimeBackend?: EmbeddingRuntimeBackend | null;
  status: EmbeddingJobStatus;
  processedItems: number;
  totalItems: number;
  message?: string | null;
  updatedAt: string;
};

export type DatasetMapPayload = {
  workspaceId: string;
  scope: DatasetMapScope;
  modelId: string;
  models: EmbeddingModelOption[];
  runtime: EmbeddingRuntimeProbe;
  points: DatasetMapPoint[];
  jobs: EmbeddingJob[];
};
