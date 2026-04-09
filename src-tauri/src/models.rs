use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentWorkspace {
    pub id: String,
    pub name: String,
    pub workspace_path: String,
    pub health_status: String,
    pub last_opened_at: Option<String>,
    pub available: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceFolder {
    pub id: String,
    pub name: String,
    pub path: String,
    pub r#type: String,
    pub status: String,
    pub image_count: u32,
    pub category_count: u32,
    pub corrupted_image_count: u32,
    pub corrupted_image_paths: Vec<String>,
    pub last_scan_at: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    pub source_id: String,
    pub source_name: String,
    pub stage: String,
    pub processed: u32,
    pub total: u32,
}

#[derive(Clone)]
pub struct StoredSourceFolder {
    pub id: String,
    pub workspace_id: String,
    pub path: String,
    pub source_type: String,
    pub status: String,
    pub last_scan_at: Option<String>,
    pub image_count: u32,
    pub category_count: u32,
    pub corrupted_image_count: u32,
    pub corrupted_image_paths: Vec<String>,
}

#[derive(Clone)]
pub struct StoredImageRecord {
    pub id: String,
    pub workspace_id: String,
    pub source_id: String,
    pub file_name: String,
    pub original_path: String,
    pub relative_path: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub annotation_status: String,
    pub health_status: String,
    pub health_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone)]
pub struct StoredCategoryRecord {
    pub id: String,
    pub workspace_id: String,
    pub source_id: String,
    pub name: String,
    pub normalized_name: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone)]
pub struct StoredAnnotationRecord {
    pub id: String,
    pub workspace_id: String,
    pub image_id: String,
    pub source_id: String,
    pub source_category_id: Option<String>,
    pub category_id: Option<String>,
    pub bbox_x: f64,
    pub bbox_y: f64,
    pub bbox_width: f64,
    pub bbox_height: f64,
    pub annotation_format: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedCategory {
    pub id: String,
    pub name: String,
    pub image_count: u32,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceOverview {
    pub id: String,
    pub name: String,
    pub workspace_path: String,
    pub health_status: String,
    pub sources: Vec<SourceFolder>,
    pub categories: Vec<UnifiedCategory>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReviewRow {
    pub source_category_id: String,
    pub source_category: String,
    pub source_path: String,
    pub count: u32,
    pub source_total_image_count: u32,
    pub suggested_action: String,
    pub target_unified_category: Option<String>,
    pub final_action: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoxSummary {
    pub category_name: String,
    pub area_ratio: Option<f64>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageCard {
    pub id: String,
    pub filename: String,
    pub source_id: String,
    pub source_name: String,
    pub original_path: String,
    pub annotation_status: String,
    pub image_health_status: String,
    pub image_health_error: Option<String>,
    pub annotation_count: u32,
    pub max_box_area_ratio: Option<f64>,
    pub box_summaries: Vec<BoxSummary>,
    pub category_ids: Vec<String>,
    pub categories: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserPayload {
    pub sources: Vec<SourceFolder>,
    pub categories: Vec<UnifiedCategory>,
    pub images: Vec<ImageCard>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoundingBoxRecord {
    pub id: String,
    pub category_name: String,
    pub annotation_format: String,
    pub bbox_x: f64,
    pub bbox_y: f64,
    pub bbox_width: f64,
    pub bbox_height: f64,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageDetailPayload {
    pub id: String,
    pub filename: String,
    pub source_id: String,
    pub source_name: String,
    pub original_path: String,
    pub annotation_status: String,
    pub image_health_status: String,
    pub image_health_error: Option<String>,
    pub categories: Vec<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub boxes: Vec<BoundingBoxRecord>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CvatTask {
    pub id: String,
    pub name: String,
    pub image_count: u32,
    pub status: String,
    pub project_name: String,
    pub last_sync_at: Option<String>,
    pub temp_folder: Option<String>,
    pub remote_task_id: Option<i64>,
    pub remote_url: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationVersion {
    pub id: String,
    pub label: String,
    pub created_at: String,
    pub source_task: String,
    pub image_count: u32,
    pub box_count: u32,
    pub notes: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportHistoryEntry {
    pub id: String,
    pub output_format: String,
    pub output_path: String,
    pub created_at: String,
    pub status: String,
    pub exported_images: u32,
    pub exported_boxes: u32,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SplitCounts {
    pub train: u32,
    pub valid: u32,
    pub test: u32,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportConflictItem {
    pub image_id: String,
    pub source_id: String,
    pub original_path: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportFilenameConflict {
    pub file_name: String,
    pub items: Vec<ExportConflictItem>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPreview {
    pub category_count: u32,
    pub included_images: u32,
    pub excluded_images: u32,
    pub included_boxes: u32,
    pub filename_conflicts: u32,
    pub conflict_details: Vec<ExportFilenameConflict>,
    pub split_counts: SplitCounts,
    pub output_path: String,
}

#[derive(Clone)]
pub struct ExportAnnotationRecord {
    pub category_key: String,
    pub category_name: String,
    pub annotation_format: String,
    pub bbox_x: f64,
    pub bbox_y: f64,
    pub bbox_width: f64,
    pub bbox_height: f64,
}

#[derive(Clone)]
pub struct ExportImageRecord {
    pub id: String,
    pub file_name: String,
    pub original_path: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub annotations: Vec<ExportAnnotationRecord>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceManifest {
    pub id: String,
    pub name: String,
    pub workspace_path: String,
    pub created_at: String,
    pub app_version: String,
    pub schema_version: u32,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkspaceInput {
    pub name: String,
    pub parent_path: String,
    pub allow_existing_target: Option<bool>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceCreateTargetCheck {
    pub target_path: String,
    pub status: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenWorkspaceInput {
    pub workspace_path: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddSourceFolderInput {
    pub workspace_id: String,
    pub source_path: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RescanSourceFolderInput {
    pub workspace_id: String,
    pub source_id: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveSourceFolderInput {
    pub workspace_id: String,
    pub source_id: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveImportReviewInput {
    pub workspace_id: String,
    pub rows: Vec<ImportReviewRow>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CvatSettings {
    pub base_url: String,
    pub access_token: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCvatTaskInput {
    pub workspace_id: String,
    pub image_ids: Vec<String>,
    pub task_name: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncCvatTaskInput {
    pub workspace_id: String,
    pub task_id: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenCvatInput {
    pub workspace_id: String,
    pub task_id: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPreviewInput {
    pub workspace_id: String,
    pub image_ids: Option<Vec<String>>,
    pub source_ids: Option<Vec<String>>,
    pub category_ids: Option<Vec<String>>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartExportInput {
    pub workspace_id: String,
    pub output_format: String,
    pub train_ratio: u32,
    pub valid_ratio: u32,
    pub test_ratio: u32,
    pub random_seed: u64,
    pub output_path: String,
    pub allow_auto_rename_conflicts: bool,
    pub image_ids: Option<Vec<String>>,
    pub source_ids: Option<Vec<String>>,
    pub category_ids: Option<Vec<String>>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartExportResult {
    pub output_format: String,
    pub output_path: String,
    pub exported_images: u32,
    pub exported_boxes: u32,
}
