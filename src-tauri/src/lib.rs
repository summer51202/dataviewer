mod commands;
mod cvat_api;
mod db;
mod embedding;
mod mock;
mod models;
mod paths;
mod workspace_service;

use commands::{
    add_source_folder, check_create_workspace_target, create_cvat_task, create_workspace,
    get_annotation_versions, get_browser_payload, get_cvat_settings, get_cvat_tasks,
    get_dataset_map_payload, get_export_history, get_export_preview, get_image_detail,
    get_import_review, get_scan_progress, get_source_folders, get_workspace_overview,
    delete_sample_set, get_sample_set_members, list_recent_workspaces, list_sample_sets, open_cvat,
    open_export_folder, open_workspace, probe_embedding_runtime, remove_recent_workspace,
    remove_source_folder, rescan_source_folder, run_sample_selection, save_cvat_settings,
    save_dataset_map_reviews, save_import_review, start_embedding_job, start_export, sync_cvat_task,
    test_cvat_settings,
};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            create_workspace,
            check_create_workspace_target,
            open_workspace,
            add_source_folder,
            rescan_source_folder,
            remove_source_folder,
            list_recent_workspaces,
            remove_recent_workspace,
            get_workspace_overview,
            get_source_folders,
            get_scan_progress,
            get_import_review,
            save_import_review,
            get_browser_payload,
            get_image_detail,
            get_dataset_map_payload,
            probe_embedding_runtime,
            start_embedding_job,
            save_dataset_map_reviews,
            get_cvat_tasks,
            get_cvat_settings,
            save_cvat_settings,
            test_cvat_settings,
            create_cvat_task,
            open_cvat,
            sync_cvat_task,
            get_annotation_versions,
            get_export_preview,
            get_export_history,
            open_export_folder,
            start_export,
            run_sample_selection,
            list_sample_sets,
            delete_sample_set,
            get_sample_set_members,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run DataViewer shell");
}
