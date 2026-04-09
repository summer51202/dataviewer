use crate::mock;
use crate::models::{BrowserPayload, ImageDetailPayload};
use crate::workspace_service;

#[tauri::command]
pub fn get_browser_payload(workspace_id: String) -> BrowserPayload {
    workspace_service::load_browser_payload_by_id(&workspace_id)
        .unwrap_or_else(|_| mock::browser_payload())
}

#[tauri::command]
pub fn get_image_detail(workspace_id: String, image_id: String) -> ImageDetailPayload {
    workspace_service::load_image_detail_by_id(&workspace_id, &image_id)
        .unwrap_or_else(|_| mock::image_detail_payload())
}
