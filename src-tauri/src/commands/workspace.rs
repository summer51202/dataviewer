use crate::mock;
use crate::models::{
    AddSourceFolderInput, CreateWorkspaceInput, ImportReviewRow, OpenWorkspaceInput,
    RecentWorkspace, RemoveSourceFolderInput, RescanSourceFolderInput, SaveImportReviewInput,
    ScanProgress, SourceFolder, WorkspaceCreateTargetCheck, WorkspaceOverview,
};
use crate::workspace_service;

#[tauri::command]
pub fn create_workspace(input: CreateWorkspaceInput) -> Result<WorkspaceOverview, String> {
    workspace_service::create_workspace(input)
}

#[tauri::command]
pub fn open_workspace(input: OpenWorkspaceInput) -> Result<WorkspaceOverview, String> {
    workspace_service::open_workspace(input)
}

#[tauri::command]
pub fn check_create_workspace_target(input: CreateWorkspaceInput) -> Result<WorkspaceCreateTargetCheck, String> {
    workspace_service::check_create_workspace_target(input)
}

#[tauri::command]
pub fn add_source_folder(input: AddSourceFolderInput) -> Result<Vec<SourceFolder>, String> {
    workspace_service::add_source_folder(input)
}

#[tauri::command]
pub fn rescan_source_folder(input: RescanSourceFolderInput) -> Result<Vec<SourceFolder>, String> {
    workspace_service::rescan_source_folder(input)
}

#[tauri::command]
pub fn remove_source_folder(input: RemoveSourceFolderInput) -> Result<Vec<SourceFolder>, String> {
    workspace_service::remove_source_folder(input)
}

#[tauri::command]
pub fn list_recent_workspaces() -> Vec<RecentWorkspace> {
    workspace_service::list_recent_workspaces().unwrap_or_else(|_| mock::recent_workspaces())
}

#[tauri::command]
pub fn remove_recent_workspace(workspace_id: String) -> Result<Vec<RecentWorkspace>, String> {
    workspace_service::remove_recent_workspace(&workspace_id)
}

#[tauri::command]
pub fn get_workspace_overview(workspace_id: String) -> WorkspaceOverview {
    workspace_service::load_workspace_overview_by_id(&workspace_id)
        .unwrap_or_else(|_| mock::workspace_overview())
}

#[tauri::command]
pub fn get_source_folders(workspace_id: String) -> Vec<SourceFolder> {
    workspace_service::load_source_folders_by_id(&workspace_id)
        .unwrap_or_else(|_| mock::source_folders())
}

#[tauri::command]
pub fn get_scan_progress(workspace_id: String) -> Vec<ScanProgress> {
    workspace_service::load_scan_progress_by_id(&workspace_id)
}

#[tauri::command]
pub fn get_import_review(workspace_id: String) -> Vec<ImportReviewRow> {
    workspace_service::load_import_review_by_id(&workspace_id)
        .unwrap_or_else(|_| mock::import_review_rows())
}

#[tauri::command]
pub fn save_import_review(input: SaveImportReviewInput) -> Result<Vec<ImportReviewRow>, String> {
    workspace_service::save_import_review(input)
}
