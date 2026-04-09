use crate::mock;
use crate::models::{ExportHistoryEntry, ExportPreview, ExportPreviewInput, StartExportInput, StartExportResult};
use crate::workspace_service;
use std::path::PathBuf;
use std::process::Command;

#[tauri::command]
pub fn get_export_preview(input: ExportPreviewInput) -> ExportPreview {
    workspace_service::load_export_preview_by_id(input)
        .unwrap_or_else(|_| mock::export_preview())
}

#[tauri::command]
pub fn get_export_history(workspace_id: String) -> Vec<ExportHistoryEntry> {
    workspace_service::load_export_history_by_id(&workspace_id).unwrap_or_default()
}

#[tauri::command]
pub fn start_export(input: StartExportInput) -> Result<StartExportResult, String> {
    workspace_service::start_export(input)
}

#[tauri::command]
pub fn open_export_folder(path: String) -> Result<(), String> {
    let folder = PathBuf::from(&path);
    if !folder.exists() || !folder.is_dir() {
        return Err("export folder does not exist".into());
    }

    Command::new("explorer")
        .arg(folder)
        .spawn()
        .map_err(|error| format!("failed to open export folder in explorer: {error}"))?;

    Ok(())
}
