use crate::mock;
use crate::models::{
    AnnotationVersion, CreateCvatTaskInput, CvatSettings, CvatTask, OpenCvatInput,
    SyncCvatTaskInput,
};
use crate::workspace_service;

#[tauri::command]
pub fn get_cvat_tasks(workspace_id: String) -> Vec<CvatTask> {
    workspace_service::load_cvat_tasks_by_id(&workspace_id)
        .unwrap_or_else(|_| mock::cvat_tasks())
}

#[tauri::command]
pub fn get_cvat_settings(workspace_id: String) -> Result<CvatSettings, String> {
    workspace_service::get_cvat_settings_by_id(&workspace_id)
}

#[tauri::command]
pub fn save_cvat_settings(
    workspace_id: String,
    settings: CvatSettings,
) -> Result<CvatSettings, String> {
    workspace_service::save_cvat_settings_by_id(&workspace_id, settings)
}

#[tauri::command]
pub fn test_cvat_settings(workspace_id: String) -> Result<(), String> {
    workspace_service::test_cvat_settings_by_id(&workspace_id)
}

#[tauri::command]
pub fn create_cvat_task(input: CreateCvatTaskInput) -> Result<Vec<CvatTask>, String> {
    workspace_service::create_cvat_task(input)
}

#[tauri::command]
pub fn open_cvat(input: OpenCvatInput) -> Result<(), String> {
    workspace_service::open_cvat(input)
}

#[tauri::command]
pub fn sync_cvat_task(input: SyncCvatTaskInput) -> Result<Vec<CvatTask>, String> {
    workspace_service::sync_cvat_task(input)
}

#[tauri::command]
pub fn get_annotation_versions(workspace_id: String) -> Vec<AnnotationVersion> {
    workspace_service::load_annotation_versions_by_id(&workspace_id)
        .unwrap_or_else(|_| mock::annotation_versions())
}
