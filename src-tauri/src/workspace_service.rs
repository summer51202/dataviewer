use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{LazyLock, Mutex};

use chrono::Utc;

use crate::cvat_api;
use crate::db;
use crate::embedding::projection::deterministic_projection;
use crate::models::{
    AddSourceFolderInput, AnnotationVersion, BrowserPayload, CreateWorkspaceInput,
    CreateCvatTaskInput, CvatSettings, CvatTask, DatasetMapPayload, DatasetMapPayloadInput,
    DatasetMapReviewInput, DatasetReviewUpdate, EmbeddingJob, EmbeddingJobInput,
    EmbeddingModelOption, EmbeddingRuntimeCapability, EmbeddingRuntimeProbe,
    EmbeddingRuntimeProbeInput, ExportHistoryEntry, ExportImageRecord, ExportPreviewInput,
    ImageDetailPayload, ImportReviewRow, OpenCvatInput, OpenWorkspaceInput, RecentWorkspace,
    RemoveSourceFolderInput, RescanSourceFolderInput, SaveImportReviewInput, ScanProgress,
    SourceFolder, StartExportInput, StartExportResult,
    StoredAnnotationRecord, StoredCategoryRecord, StoredImageRecord,
    SyncCvatTaskInput, WorkspaceCreateTargetCheck, WorkspaceManifest, WorkspaceOverview,
};
use crate::paths::{
    build_workspace_paths, recent_workspaces_path, APP_VERSION, SCHEMA_VERSION,
};

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "bmp", "webp", "tif", "tiff"];
static ACTIVE_SCANS: LazyLock<Mutex<HashMap<String, Vec<ScanProgress>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn create_workspace(input: CreateWorkspaceInput) -> Result<WorkspaceOverview, String> {
    let trimmed_name = input.name.trim();
    let root = resolve_workspace_creation_root(trimmed_name, &input.parent_path)?;
    let paths = build_workspace_paths(&root);

    validate_workspace_creation_target(
        &paths.root,
        &paths.hidden_dir,
        input.allow_existing_target.unwrap_or(false),
    )?;
    create_workspace_directories(&paths)?;

    let now = Utc::now().to_rfc3339();
    let workspace_id = slugify_workspace_name(trimmed_name);

    let manifest = WorkspaceManifest {
        id: workspace_id.clone(),
        name: trimmed_name.to_string(),
        workspace_path: normalize_path_string(&paths.root),
        created_at: now.clone(),
        app_version: APP_VERSION.to_string(),
        schema_version: SCHEMA_VERSION,
    };

    db::initialize_database(&paths.db_path)?;
    db::upsert_workspace_meta(
        &paths.db_path,
        &workspace_id,
        trimmed_name,
        &manifest.workspace_path,
        &now,
        &now,
    )?;

    write_manifest(&paths.manifest_path, &manifest)?;

    let overview = db::read_workspace_overview(&paths.db_path)?;
    upsert_recent_workspace(&overview)?;

    Ok(overview)
}

pub fn open_workspace(input: OpenWorkspaceInput) -> Result<WorkspaceOverview, String> {
    let root = PathBuf::from(&input.workspace_path);
    let paths = build_workspace_paths(&root);

    ensure_workspace_exists(&paths.root, &paths.manifest_path, &paths.db_path)?;
    let _manifest = read_manifest(&paths.manifest_path)?;

    db::initialize_database(&paths.db_path)?;
    let overview = db::read_workspace_overview(&paths.db_path)?;
    upsert_recent_workspace(&overview)?;

    Ok(overview)
}

pub fn check_create_workspace_target(input: CreateWorkspaceInput) -> Result<WorkspaceCreateTargetCheck, String> {
    let trimmed_name = input.name.trim();
    let root = resolve_workspace_creation_root(trimmed_name, &input.parent_path)?;
    let paths = build_workspace_paths(&root);
    let status = inspect_workspace_creation_target(&paths.root, &paths.hidden_dir)?;

    Ok(WorkspaceCreateTargetCheck {
        target_path: normalize_path_string(&paths.root),
        status: status.to_string(),
    })
}

pub fn add_source_folder(input: AddSourceFolderInput) -> Result<Vec<SourceFolder>, String> {
    let paths = resolve_workspace_paths_by_id(&input.workspace_id)?;
    let source_root = PathBuf::from(&input.source_path);

    if !source_root.exists() {
        return Err("source folder does not exist".into());
    }

    if !source_root.is_dir() {
        return Err("selected source path is not a directory".into());
    }

    let source_type = detect_source_type(&source_root);
    let status = match source_type.as_str() {
        "RAW" => "review",
        _ => "ready",
    };
    let source_id = build_source_id(&source_root);
    let normalized_source_path = normalize_path_string(&source_root);

    db::insert_source_folder(
        &paths.db_path,
        &source_id,
        &input.workspace_id,
        &normalized_source_path,
        &source_type,
        status,
        None,
    )?;

    let scan = scan_source_folder(&source_root, &input.workspace_id, &source_id);
    let now = Utc::now().to_rfc3339();
    db::replace_source_categories(&paths.db_path, &source_id, &scan.category_records)?;
    db::replace_source_images(&paths.db_path, &source_id, &scan.image_records)?;
    db::replace_source_annotations(&paths.db_path, &source_id, &scan.annotation_records)?;
    db::update_source_folder_scan(
        &paths.db_path,
        &source_id,
        &scan.source_type,
        &scan.status,
        &now,
        scan.image_count,
        scan.category_count,
    )?;

    db::read_source_folders(&paths.db_path)
}

pub fn rescan_source_folder(input: RescanSourceFolderInput) -> Result<Vec<SourceFolder>, String> {
    let paths = resolve_workspace_paths_by_id(&input.workspace_id)?;
    let stored = db::read_source_folder_row(&paths.db_path, &input.source_id)?;

    if stored.workspace_id != input.workspace_id {
        return Err("source folder does not belong to this workspace".into());
    }

    let source_root = PathBuf::from(&stored.path);
    if !source_root.exists() || !source_root.is_dir() {
        return Err("source folder path is no longer available".into());
    }

    let scan = scan_source_folder(&source_root, &stored.workspace_id, &stored.id);
    let now = Utc::now().to_rfc3339();

    db::replace_source_categories(&paths.db_path, &stored.id, &scan.category_records)?;
    db::replace_source_images(&paths.db_path, &stored.id, &scan.image_records)?;
    db::replace_source_annotations(&paths.db_path, &stored.id, &scan.annotation_records)?;

    db::update_source_folder_scan(
        &paths.db_path,
        &stored.id,
        &scan.source_type,
        &scan.status,
        &now,
        scan.image_count,
        scan.category_count,
    )?;

    db::read_source_folders(&paths.db_path)
}

pub fn remove_source_folder(input: RemoveSourceFolderInput) -> Result<Vec<SourceFolder>, String> {
    let paths = resolve_workspace_paths_by_id(&input.workspace_id)?;
    let stored = db::read_source_folder_row(&paths.db_path, &input.source_id)?;

    if stored.workspace_id != input.workspace_id {
        return Err("source folder does not belong to this workspace".into());
    }

    db::delete_source_folder(&paths.db_path, &input.source_id)?;
    db::read_source_folders(&paths.db_path)
}

pub fn list_recent_workspaces() -> Result<Vec<RecentWorkspace>, String> {
    let mut items = load_recent_workspaces_file()?;

    for item in &mut items {
        let is_available = PathBuf::from(&item.workspace_path).exists();
        item.available = is_available;
        item.health_status = if is_available { "healthy".into() } else { "warning".into() };
    }

    items.sort_by(|a, b| b.last_opened_at.cmp(&a.last_opened_at));

    Ok(items)
}

pub fn remove_recent_workspace(workspace_id: &str) -> Result<Vec<RecentWorkspace>, String> {
    let path = recent_workspaces_path()?;

    if !path.exists() {
        return Ok(Vec::new());
    }

    let mut items = load_recent_workspaces_file()?;
    items.retain(|item| item.id != workspace_id);
    write_recent_workspaces_file(&path, &items)?;

    list_recent_workspaces()
}

pub fn load_workspace_overview_by_id(workspace_id: &str) -> Result<WorkspaceOverview, String> {
    let paths = resolve_workspace_paths_by_id(workspace_id)?;
    db::read_workspace_overview(&paths.db_path)
}

pub fn load_source_folders_by_id(workspace_id: &str) -> Result<Vec<SourceFolder>, String> {
    let paths = resolve_workspace_paths_by_id(workspace_id)?;
    db::read_source_folders(&paths.db_path)
}

pub fn load_scan_progress_by_id(workspace_id: &str) -> Vec<ScanProgress> {
    ACTIVE_SCANS
        .lock()
        .ok()
        .and_then(|state| state.get(workspace_id).cloned())
        .unwrap_or_default()
}

pub fn load_browser_payload_by_id(workspace_id: &str) -> Result<BrowserPayload, String> {
    let paths = resolve_workspace_paths_by_id(workspace_id)?;
    db::read_browser_payload(&paths.db_path)
}

pub fn load_image_detail_by_id(
    workspace_id: &str,
    image_id: &str,
) -> Result<ImageDetailPayload, String> {
    let paths = resolve_workspace_paths_by_id(workspace_id)?;
    db::read_image_detail(&paths.db_path, image_id)
}

pub fn load_dataset_map_payload_by_id(
    input: DatasetMapPayloadInput,
) -> Result<DatasetMapPayload, String> {
    let models = default_embedding_models();
    let model_id = input
        .model_id
        .clone()
        .filter(|id| models.iter().any(|model| model.id == *id))
        .unwrap_or_else(|| "clip-vit-b32".to_string());
    let workspace_id = input.workspace_id.clone();
    let scope = input.scope.clone();
    let paths = resolve_workspace_paths_by_id(&workspace_id)?;
    let points = db::read_dataset_map_points(&paths.db_path, &workspace_id, &scope, &model_id)?;

    Ok(DatasetMapPayload {
        workspace_id,
        scope,
        model_id,
        models,
        runtime: default_embedding_runtime_probe("auto"),
        points,
        jobs: vec![],
    })
}

pub fn probe_embedding_runtime_by_id(
    input: EmbeddingRuntimeProbeInput,
) -> Result<EmbeddingRuntimeProbe, String> {
    Ok(default_embedding_runtime_probe(&input.preference))
}

pub fn start_embedding_job_by_id(input: EmbeddingJobInput) -> Result<EmbeddingJob, String> {
    let paths = resolve_workspace_paths_by_id(&input.workspace_id)?;
    let processed_items = generate_bootstrap_embedding_projections(
        &paths.db_path,
        &input.workspace_id,
        &input.scope,
        &input.model_id,
    )?;

    Ok(EmbeddingJob {
        id: format!("embedding-job-{}", Utc::now().timestamp_millis()),
        scope: input.scope,
        model_id: input.model_id,
        runtime_preference: input.runtime_preference,
        runtime_backend: Some("cpu".to_string()),
        status: "completed".to_string(),
        processed_items,
        total_items: processed_items,
        message: Some(format!(
            "Generated deterministic bootstrap projections for {processed_items} items."
        )),
        updated_at: Utc::now().to_rfc3339(),
    })
}

pub fn save_dataset_map_reviews_by_id(
    input: DatasetMapReviewInput,
) -> Result<Vec<DatasetReviewUpdate>, String> {
    Ok(input.updates)
}

fn default_embedding_models() -> Vec<EmbeddingModelOption> {
    vec![
        EmbeddingModelOption {
            id: "clip-vit-b32".to_string(),
            family: "clip".to_string(),
            display_name: "CLIP ViT-B/32".to_string(),
            embedding_dim: 512,
            input_size: 224,
            available: true,
            download_required: false,
        },
        EmbeddingModelOption {
            id: "dinov2-small".to_string(),
            family: "dinov2".to_string(),
            display_name: "DINOv2 Small".to_string(),
            embedding_dim: 384,
            input_size: 224,
            available: true,
            download_required: false,
        },
    ]
}

fn default_embedding_runtime_probe(preference: &str) -> EmbeddingRuntimeProbe {
    let fallback_reason = if preference == "cpu" {
        None
    } else {
        Some("Using CPU until ONNX Runtime provider probing is wired.".to_string())
    };

    EmbeddingRuntimeProbe {
        preference: preference.to_string(),
        selected_backend: "cpu".to_string(),
        capabilities: vec![
            EmbeddingRuntimeCapability {
                backend: "cuda".to_string(),
                available: false,
                label: "NVIDIA CUDA".to_string(),
                detail: "No CUDA provider detected in the current backend build.".to_string(),
            },
            EmbeddingRuntimeCapability {
                backend: "windows-gpu".to_string(),
                available: false,
                label: "Windows GPU".to_string(),
                detail: "DirectML provider detection will be added with packaged runtime support."
                    .to_string(),
            },
            EmbeddingRuntimeCapability {
                backend: "cpu".to_string(),
                available: true,
                label: "CPU".to_string(),
                detail: "Available on this Windows desktop build.".to_string(),
            },
        ],
        fallback_reason,
    }
}

fn generate_bootstrap_embedding_projections(
    db_path: &Path,
    workspace_id: &str,
    scope: &str,
    model_id: &str,
) -> Result<u32, String> {
    let targets = db::read_dataset_map_projection_targets(db_path, workspace_id, scope)?;
    let now = Utc::now().to_rfc3339();
    let projections = targets
        .iter()
        .map(|target| {
            let seed = format!("{scope}:{model_id}:{}", target.target_id);
            let (x, y) = deterministic_projection(&seed);
            db::EmbeddingProjectionRow {
                id: format!(
                    "projection-{workspace_id}-{scope}-{model_id}-{}-bootstrap",
                    target.target_id
                ),
                workspace_id: workspace_id.to_string(),
                scope: scope.to_string(),
                target_id: target.target_id.clone(),
                model_id: model_id.to_string(),
                projection_method: "bootstrap-deterministic".to_string(),
                x,
                y,
                created_at: now.clone(),
            }
        })
        .collect::<Vec<_>>();

    db::upsert_embedding_projections(db_path, &projections)?;

    Ok(projections.len() as u32)
}

pub fn load_export_preview_by_id(input: ExportPreviewInput) -> Result<crate::models::ExportPreview, String> {
    let paths = resolve_workspace_paths_by_id(&input.workspace_id)?;
    let overview = db::read_workspace_overview(&paths.db_path)?;
    let output_path = normalize_path_string(&paths.exports_dir.join(format!("{}-coco", overview.name)));
    let browser_payload = db::read_browser_payload(&paths.db_path)?;
    let all_export_images = db::read_export_images(&paths.db_path)?;
    let filtered_images = filter_export_images(
        all_export_images,
        &browser_payload,
        input.image_ids.as_deref(),
        input.source_ids.as_deref().unwrap_or(&[]),
        input.category_ids.as_deref().unwrap_or(&[]),
    );
    let scoped_image_count = count_scoped_browser_images(
        &browser_payload,
        input.image_ids.as_deref(),
        input.source_ids.as_deref().unwrap_or(&[]),
    );
    Ok(build_export_preview(
        filtered_images,
        &browser_payload,
        scoped_image_count,
        output_path,
    ))
}

pub fn load_export_history_by_id(workspace_id: &str) -> Result<Vec<ExportHistoryEntry>, String> {
    let paths = resolve_workspace_paths_by_id(workspace_id)?;
    db::read_export_history(&paths.db_path)
}

pub fn start_export(input: StartExportInput) -> Result<StartExportResult, String> {
    let paths = resolve_workspace_paths_by_id(&input.workspace_id)?;
    let browser_payload = db::read_browser_payload(&paths.db_path)?;
    let all_images = db::read_export_images(&paths.db_path)?;
    let mut images = filter_export_images(
        all_images,
        &browser_payload,
        input.image_ids.as_deref(),
        input.source_ids.as_deref().unwrap_or(&[]),
        input.category_ids.as_deref().unwrap_or(&[]),
    );
    let conflicts = collect_export_conflicts(&images, &browser_payload);
    if !conflicts.is_empty() && !input.allow_auto_rename_conflicts {
        return Err("filename conflicts detected; review conflicts and enable auto rename before exporting".into());
    }

    if images.is_empty() {
        return Err("no annotated images are available for export under the current filters".into());
    }


    sort_images_for_seed(&mut images, input.random_seed);
    let split = compute_split_counts(
        images.len() as u32,
        input.train_ratio,
        input.valid_ratio,
        input.test_ratio,
    );

    let output_root = PathBuf::from(&input.output_path);
    let train_count = split.train as usize;
    let valid_count = split.valid as usize;
    let train_images = &images[..train_count.min(images.len())];
    let valid_end = (train_count + valid_count).min(images.len());
    let valid_images = &images[train_count.min(images.len())..valid_end];
    let test_images = &images[valid_end..];

    let export_boxes = if input.output_format.eq_ignore_ascii_case("COCO") {
        let train_dir = output_root.join("train");
        let valid_dir = output_root.join("valid");
        let test_dir = output_root.join("test");
        fs::create_dir_all(&train_dir)
            .and_then(|_| fs::create_dir_all(&valid_dir))
            .and_then(|_| fs::create_dir_all(&test_dir))
            .map_err(|error| format!("failed to create export directories: {error}"))?;

        let mut export_boxes = 0_u32;
        export_boxes += export_coco_split("train", train_images, &train_dir)?;
        export_boxes += export_coco_split("valid", valid_images, &valid_dir)?;
        export_boxes += export_coco_split("test", test_images, &test_dir)?;
        export_boxes
    } else if input.output_format.eq_ignore_ascii_case("YOLO") {
        export_yolo_dataset(&output_root, train_images, valid_images, test_images)?
    } else {
        return Err("unsupported export format".into());
    };

    let created_at = Utc::now().to_rfc3339();
    let export_job_id = format!("export-{}", Utc::now().timestamp_millis());
    let normalized_output_path = normalize_path_string(&output_root);
    db::insert_export_job(
        &paths.db_path,
        &input.workspace_id,
        &export_job_id,
        &input.output_format.to_uppercase(),
        &normalized_output_path,
        &created_at,
        "completed",
        images.len() as u32,
        export_boxes,
    )?;

    Ok(StartExportResult {
        output_format: input.output_format.to_uppercase(),
        output_path: normalized_output_path,
        exported_images: images.len() as u32,
        exported_boxes: export_boxes,
    })
}

pub fn load_import_review_by_id(workspace_id: &str) -> Result<Vec<ImportReviewRow>, String> {
    let paths = resolve_workspace_paths_by_id(workspace_id)?;
    db::read_import_review_rows(&paths.db_path)
}

pub fn save_import_review(input: SaveImportReviewInput) -> Result<Vec<ImportReviewRow>, String> {
    let paths = resolve_workspace_paths_by_id(&input.workspace_id)?;
    db::save_import_review_rows(&paths.db_path, &input.workspace_id, &input.rows)?;
    db::read_import_review_rows(&paths.db_path)
}

pub fn load_cvat_tasks_by_id(workspace_id: &str) -> Result<Vec<CvatTask>, String> {
    let paths = resolve_workspace_paths_by_id(workspace_id)?;
    Ok(db::read_cvat_tasks(&paths.db_path)?
        .into_iter()
        .map(decorate_cvat_task_status)
        .collect())
}

pub fn load_annotation_versions_by_id(workspace_id: &str) -> Result<Vec<AnnotationVersion>, String> {
    let paths = resolve_workspace_paths_by_id(workspace_id)?;
    db::read_annotation_versions(&paths.db_path)
}

pub fn create_cvat_task(input: CreateCvatTaskInput) -> Result<Vec<CvatTask>, String> {
    if input.image_ids.is_empty() {
        return Err("no images selected for CVAT task".into());
    }

    let paths = resolve_workspace_paths_by_id(&input.workspace_id)?;
    let selected_images = db::read_images_for_ids(&paths.db_path, &input.image_ids)?;
    if selected_images.is_empty() {
        return Err("selected images are not available in this workspace".into());
    }
    if selected_images.iter().any(|image| image.4 != "unannotated") {
        return Err("only unannotated images can be staged for CVAT in the current workflow".into());
    }

    let task_id = format!("cvat-task-{}", Utc::now().timestamp_millis());
    let task_name = input
        .task_name
        .unwrap_or_else(|| format!("selection-{}", Utc::now().format("%Y%m%d-%H%M%S")));
    let task_root = paths.temp_dir.join("cvat").join(&task_id);
    let task_images_dir = task_root.join("images");
    let task_annotations_dir = task_root.join("annotations");
    fs::create_dir_all(&task_images_dir)
        .map_err(|error| format!("failed to create CVAT temp task directory: {error}"))?;
    fs::create_dir_all(&task_annotations_dir)
        .map_err(|error| format!("failed to create CVAT annotation directory: {error}"))?;

    let workspace_overview = db::read_workspace_overview(&paths.db_path)?;
    let mut labels = workspace_overview
        .categories
        .into_iter()
        .map(|category| category.name.trim().to_string())
        .filter(|name| !name.is_empty())
        .collect::<Vec<_>>();
    labels.sort();
    labels.dedup();

    let mut used_names = HashMap::<String, usize>::new();
    let mut manifest_images = Vec::<serde_json::Value>::new();
    let mut staged_image_paths = Vec::<PathBuf>::new();
    for (image_id, source_id, file_name, original_path, _) in selected_images {
        let source_path = PathBuf::from(&original_path);
        if !source_path.exists() {
            continue;
        }

        let safe_name = make_unique_file_name(&file_name, &mut used_names);
        let destination = task_images_dir.join(&safe_name);
        fs::copy(&source_path, &destination).map_err(|error| {
            format!("failed to stage selected image for CVAT task: {error}")
        })?;
        staged_image_paths.push(destination.clone());
        manifest_images.push(serde_json::json!({
            "imageId": image_id,
            "sourceId": source_id,
            "fileName": file_name,
            "stagedFileName": safe_name,
            "originalPath": original_path,
        }));
    }

    if manifest_images.is_empty() {
        return Err("no selected images were available to stage for CVAT".into());
    }

    let manifest = serde_json::json!({
        "taskId": task_id,
        "taskName": task_name,
        "workspaceId": input.workspace_id,
        "imageCount": manifest_images.len(),
        "labels": labels,
        "images": manifest_images,
    });
    fs::write(
        task_root.join("task.json"),
        serde_json::to_string_pretty(&manifest)
            .map_err(|error| format!("failed to serialize CVAT task manifest: {error}"))?,
    )
    .map_err(|error| format!("failed to write CVAT task manifest: {error}"))?;

    let settings = load_optional_cvat_settings(&paths)?;
    let initial_status = if settings.is_some() { "Connecting" } else { "Prepared" };
    let project_name = if settings.is_some() {
        "remote-cvat".to_string()
    } else {
        "local-staging".to_string()
    };

    db::insert_cvat_task_with_workspace(
        &paths.db_path,
        &input.workspace_id,
        &CvatTask {
            id: task_id.clone(),
            name: task_name.clone(),
            image_count: manifest_images.len() as u32,
            status: initial_status.into(),
            project_name,
            last_sync_at: None,
            temp_folder: Some(normalize_path_string(&task_root)),
            remote_task_id: None,
            remote_url: None,
        },
    )?;

    if let Some(settings) = settings {
        let remote = match cvat_api::create_task(&settings, &task_name, &labels) {
            Ok(remote) => remote,
            Err(error) => {
                let _ = db::update_cvat_task_remote_info(&paths.db_path, &task_id, "Remote Error", None, None);
                return Err(error);
            }
        };

        db::update_cvat_task_remote_info(
            &paths.db_path,
            &task_id,
            "Uploading",
            Some(remote.remote_task_id),
            Some(&remote.remote_url),
        )?;

        let staged_refs = staged_image_paths.iter().map(PathBuf::as_path).collect::<Vec<_>>();
        if let Err(error) = cvat_api::upload_task_images(&settings, remote.remote_task_id, &staged_refs) {
            let _ = db::update_cvat_task_remote_info(
                &paths.db_path,
                &task_id,
                "Remote Error",
                Some(remote.remote_task_id),
                Some(&remote.remote_url),
            );
            return Err(error);
        }

        db::update_cvat_task_remote_info(
            &paths.db_path,
            &task_id,
            "In CVAT",
            Some(remote.remote_task_id),
            Some(&remote.remote_url),
        )?;
    }

    load_cvat_tasks_by_id(&input.workspace_id)
}

pub fn get_cvat_settings_by_id(workspace_id: &str) -> Result<CvatSettings, String> {
    let paths = resolve_workspace_paths_by_id(workspace_id)?;
    Ok(load_optional_cvat_settings(&paths)?.unwrap_or(CvatSettings {
        base_url: "http://localhost:8080".into(),
        access_token: String::new(),
    }))
}

pub fn save_cvat_settings_by_id(
    workspace_id: &str,
    settings: CvatSettings,
) -> Result<CvatSettings, String> {
    let paths = resolve_workspace_paths_by_id(workspace_id)?;
    let sanitized = sanitize_cvat_settings(&settings)?;
    write_cvat_settings(&paths, &sanitized)?;
    Ok(sanitized)
}

pub fn test_cvat_settings_by_id(workspace_id: &str) -> Result<(), String> {
    let paths = resolve_workspace_paths_by_id(workspace_id)?;
    let settings = read_cvat_settings(&paths)?;
    cvat_api::validate_settings(&settings)
}

pub fn open_cvat(input: OpenCvatInput) -> Result<(), String> {
    let paths = resolve_workspace_paths_by_id(&input.workspace_id)?;
    let target_url = if let Some(task_id) = input.task_id.as_deref() {
        let (task_workspace_id, _task_name, _temp_folder, _last_sync_at, _status, remote_task_id, remote_url) =
            db::read_cvat_task_metadata(&paths.db_path, task_id)?;
        if task_workspace_id != input.workspace_id {
            return Err("CVAT task does not belong to this workspace".into());
        }

        if let Some(remote_url) = remote_url {
            remote_url
        } else if let Some(remote_task_id) = remote_task_id {
            if let Some(settings) = load_optional_cvat_settings(&paths)? {
                format!("{}/{}", cvat_tasks_url(&settings.base_url), remote_task_id)
            } else {
                "http://localhost:8080/tasks".into()
            }
        } else if let Some(settings) = load_optional_cvat_settings(&paths)? {
            cvat_tasks_url(&settings.base_url)
        } else {
            "http://localhost:8080/tasks".into()
        }
    } else if let Some(settings) = load_optional_cvat_settings(&paths)? {
        cvat_tasks_url(&settings.base_url)
    } else {
        "http://localhost:8080/tasks".into()
    };

    Command::new("cmd")
        .args(["/C", "start", "", &target_url])
        .spawn()
        .map_err(|error| format!("failed to open CVAT in browser: {error}"))?;

    Ok(())
}

pub fn sync_cvat_task(input: SyncCvatTaskInput) -> Result<Vec<CvatTask>, String> {
    let paths = resolve_workspace_paths_by_id(&input.workspace_id)?;
    let (task_workspace_id, task_name, temp_folder, _last_sync_at, _status, _remote_task_id, _remote_url) =
        db::read_cvat_task_metadata(&paths.db_path, &input.task_id)?;

    if task_workspace_id != input.workspace_id {
        return Err("CVAT task does not belong to this workspace".into());
    }

    let task_root = temp_folder
        .map(PathBuf::from)
        .ok_or_else(|| "CVAT task temp folder is missing".to_string())?;
    if !task_root.exists() || !task_root.is_dir() {
        return Err("CVAT task temp folder is no longer available".into());
    }

    let annotation_file = find_cvat_sync_annotation_file(&task_root)
        .ok_or_else(|| "no CVAT annotation export file found in task folder".to_string())?;
    let manifest_path = task_root.join("task.json");
    if !manifest_path.exists() {
        return Err("CVAT task manifest is missing".into());
    }

    let manifest_json: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&manifest_path)
            .map_err(|error| format!("failed to read CVAT task manifest: {error}"))?,
    )
    .map_err(|error| format!("failed to parse CVAT task manifest: {error}"))?;
    let manifest_images = manifest_json
        .get("images")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    if manifest_images.is_empty() {
        return Err("CVAT task manifest does not contain staged images".into());
    }

    let manifest_image_ids = manifest_images
        .iter()
        .filter_map(|image| image.get("imageId").and_then(|value| value.as_str()).map(str::to_string))
        .collect::<Vec<_>>();
    let fallback_source_lookup = db::read_images_for_ids(&paths.db_path, &manifest_image_ids)?
        .into_iter()
        .map(|(image_id, source_id, _, _, _)| (image_id, source_id))
        .collect::<HashMap<_, _>>();

    let mut staged_image_lookup = HashMap::<String, (String, String)>::new();
    for image in manifest_images {
        let Some(image_id) = image.get("imageId").and_then(|value| value.as_str()) else {
            continue;
        };
        let Some(staged_file_name) = image
            .get("stagedFileName")
            .or_else(|| image.get("fileName"))
            .and_then(|value| value.as_str())
        else {
            continue;
        };
        let source_id = image
            .get("sourceId")
            .and_then(|value| value.as_str())
            .map(str::to_string)
            .or_else(|| fallback_source_lookup.get(image_id).cloned())
            .ok_or_else(|| format!("failed to resolve source id for staged image {image_id}"))?;
        let staged_file_name = staged_file_name.to_string();
        staged_image_lookup.insert(staged_file_name.clone(), (image_id.to_string(), source_id.clone()));
        if let Some(file_name_only) = Path::new(&staged_file_name).file_name().and_then(|value| value.to_str()) {
            staged_image_lookup.insert(file_name_only.to_string(), (image_id.to_string(), source_id));
        }
    }

    let annotation_json: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&annotation_file)
            .map_err(|error| format!("failed to read CVAT annotation export: {error}"))?,
    )
    .map_err(|error| format!("failed to parse CVAT annotation export: {error}"))?;

    let category_lookup = annotation_json
        .get("categories")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|category| {
            Some((
                json_id_to_string(category.get("id"))?,
                category.get("name")?.as_str()?.to_string(),
            ))
        })
        .collect::<HashMap<_, _>>();
    let image_lookup = annotation_json
        .get("images")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|image| {
            Some((
                json_id_to_string(image.get("id"))?,
                image.get("file_name")?.as_str()?.to_string(),
            ))
        })
        .collect::<HashMap<_, _>>();

    let mut synced_annotations = Vec::<(String, String, String, f64, f64, f64, f64)>::new();
    for annotation in annotation_json
        .get("annotations")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default()
    {
        let Some(original_image_id) = json_id_to_string(annotation.get("image_id")) else {
            continue;
        };
        let Some(category_id) = json_id_to_string(annotation.get("category_id")) else {
            continue;
        };
        let Some(file_name) = image_lookup.get(&original_image_id) else {
            continue;
        };
        let image_key = Path::new(file_name)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(file_name);
        let Some((image_id, source_id)) = staged_image_lookup
            .get(file_name)
            .or_else(|| staged_image_lookup.get(image_key))
            .cloned()
        else {
            continue;
        };
        let Some(category_name) = category_lookup.get(&category_id).cloned() else {
            continue;
        };
        let Some(bbox) = annotation.get("bbox").and_then(|value| value.as_array()) else {
            continue;
        };
        let Some(bbox_x) = bbox.get(0).and_then(parse_json_number) else {
            continue;
        };
        let Some(bbox_y) = bbox.get(1).and_then(parse_json_number) else {
            continue;
        };
        let Some(bbox_width) = bbox.get(2).and_then(parse_json_number) else {
            continue;
        };
        let Some(bbox_height) = bbox.get(3).and_then(parse_json_number) else {
            continue;
        };

        synced_annotations.push((
            image_id,
            source_id,
            category_name,
            bbox_x,
            bbox_y,
            bbox_width,
            bbox_height,
        ));
    }

    if synced_annotations.is_empty() {
        return Err("no valid COCO annotations were found in the CVAT export file".into());
    }

    let _version = db::apply_cvat_sync_annotations(
        &paths.db_path,
        &input.workspace_id,
        &input.task_id,
        &task_name,
        &manifest_image_ids,
        &synced_annotations,
    )?;
    let now = Utc::now().to_rfc3339();
    db::update_cvat_task_sync_state(&paths.db_path, &input.task_id, "Synced", Some(&now))?;

    load_cvat_tasks_by_id(&input.workspace_id)
}

fn decorate_cvat_task_status(task: CvatTask) -> CvatTask {
    let has_sync_file = task
        .temp_folder
        .as_ref()
        .map(PathBuf::from)
        .and_then(|path| find_cvat_sync_annotation_file(&path))
        .is_some();

    if has_sync_file && !task.status.eq_ignore_ascii_case("synced") {
        return CvatTask {
            status: "Ready Sync".into(),
            ..task
        };
    }

    task
}

fn find_cvat_sync_annotation_file(task_root: &Path) -> Option<PathBuf> {
    let candidates = [
        task_root.join("annotations").join("instances_default.json"),
        task_root.join("annotations").join("instances.json"),
        task_root.join("annotations").join("annotations.json"),
        task_root.join("instances_default.json"),
        task_root.join("instances.json"),
        task_root.join("annotations.json"),
    ];

    for candidate in candidates {
        if candidate.exists() && candidate.is_file() {
            return Some(candidate);
        }
    }

    find_file_by_name(task_root, "instances_default.json")
        .or_else(|| find_file_by_name(task_root, "instances.json"))
        .or_else(|| find_file_by_name(task_root, "annotations.json"))
}

fn cvat_settings_path(paths: &crate::paths::WorkspacePaths) -> PathBuf {
    paths.hidden_dir.join("cvat-settings.json")
}

fn sanitize_cvat_settings(settings: &CvatSettings) -> Result<CvatSettings, String> {
    let base_url = settings.base_url.trim().trim_end_matches('/').to_string();
    let access_token = settings.access_token.trim().to_string();

    if base_url.is_empty() {
        return Err("CVAT base URL is required".into());
    }
    if access_token.is_empty() {
        return Err("CVAT access token is required".into());
    }

    Ok(CvatSettings {
        base_url,
        access_token,
    })
}

fn read_cvat_settings(paths: &crate::paths::WorkspacePaths) -> Result<CvatSettings, String> {
    let settings_path = cvat_settings_path(paths);
    let content = fs::read_to_string(&settings_path)
        .map_err(|error| format!("failed to read CVAT settings: {error}"))?;
    let settings = serde_json::from_str::<CvatSettings>(&content)
        .map_err(|error| format!("failed to parse CVAT settings: {error}"))?;
    sanitize_cvat_settings(&settings)
}

fn load_optional_cvat_settings(
    paths: &crate::paths::WorkspacePaths,
) -> Result<Option<CvatSettings>, String> {
    let settings_path = cvat_settings_path(paths);
    if !settings_path.exists() {
        return Ok(None);
    }

    read_cvat_settings(paths).map(Some)
}

fn write_cvat_settings(
    paths: &crate::paths::WorkspacePaths,
    settings: &CvatSettings,
) -> Result<(), String> {
    let settings_path = cvat_settings_path(paths);
    let content = serde_json::to_string_pretty(settings)
        .map_err(|error| format!("failed to serialize CVAT settings: {error}"))?;
    fs::write(settings_path, content)
        .map_err(|error| format!("failed to write CVAT settings: {error}"))
}

fn cvat_tasks_url(base_url: &str) -> String {
    format!("{}/tasks", base_url.trim().trim_end_matches('/'))
}
fn resolve_workspace_paths_by_id(workspace_id: &str) -> Result<crate::paths::WorkspacePaths, String> {
    let recent = list_recent_workspaces()?;
    let item = recent
        .into_iter()
        .find(|entry| entry.id == workspace_id)
        .ok_or_else(|| format!("workspace not found in recent list: {workspace_id}"))?;

    let root = PathBuf::from(item.workspace_path);
    let paths = build_workspace_paths(&root);
    ensure_workspace_exists(&paths.root, &paths.manifest_path, &paths.db_path)?;
    Ok(paths)
}

struct ScanResult {
    source_type: String,
    status: String,
    image_count: u32,
    category_count: u32,
    image_records: Vec<StoredImageRecord>,
    category_records: Vec<StoredCategoryRecord>,
    annotation_records: Vec<StoredAnnotationRecord>,
}

struct ScanProgressGuard {
    workspace_id: String,
    source_id: String,
}

impl Drop for ScanProgressGuard {
    fn drop(&mut self) {
        clear_scan_progress(&self.workspace_id, &self.source_id);
    }
}

fn register_scan_progress(
    workspace_id: &str,
    source_id: &str,
    source_name: &str,
    stage: &str,
    processed: u32,
    total: u32,
) {
    update_scan_progress(workspace_id, source_id, source_name, stage, processed, total);
}

fn update_scan_progress(
    workspace_id: &str,
    source_id: &str,
    source_name: &str,
    stage: &str,
    processed: u32,
    total: u32,
) {
    if let Ok(mut state) = ACTIVE_SCANS.lock() {
        let items = state.entry(workspace_id.to_string()).or_default();
        if let Some(entry) = items.iter_mut().find(|entry| entry.source_id == source_id) {
            entry.stage = stage.to_string();
            entry.processed = processed;
            entry.total = total;
            entry.source_name = source_name.to_string();
        } else {
            items.push(ScanProgress {
                source_id: source_id.to_string(),
                source_name: source_name.to_string(),
                stage: stage.to_string(),
                processed,
                total,
            });
        }
    }
}

fn clear_scan_progress(workspace_id: &str, source_id: &str) {
    if let Ok(mut state) = ACTIVE_SCANS.lock() {
        if let Some(items) = state.get_mut(workspace_id) {
            items.retain(|entry| entry.source_id != source_id);
            if items.is_empty() {
                state.remove(workspace_id);
            }
        }
    }
}

fn count_corrupted_images(image_records: &[StoredImageRecord]) -> u32 {
    image_records
        .iter()
        .filter(|image| image.health_status == "corrupted")
        .count() as u32
}

fn resolve_source_status(default_status: &str, image_records: &[StoredImageRecord]) -> String {
    if count_corrupted_images(image_records) > 0 {
        "warning".into()
    } else {
        default_status.into()
    }
}

fn probe_image_dimensions_for_path(path: &Path) -> Result<(u32, u32), String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("failed to read image file: {error}"))?;
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .ok_or_else(|| "image file is missing an extension".to_string())?;

    match extension.as_str() {
        "png" => parse_png_dimensions(&bytes),
        "jpg" | "jpeg" => parse_jpeg_dimensions(&bytes),
        "bmp" => parse_bmp_dimensions(&bytes),
        "webp" => parse_webp_dimensions(&bytes),
        "tif" | "tiff" => parse_tiff_dimensions(&bytes),
        _ => Err("unsupported image format".into()),
    }
}

fn parse_png_dimensions(bytes: &[u8]) -> Result<(u32, u32), String> {
    const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    if bytes.len() < 24 || &bytes[..8] != PNG_SIGNATURE || &bytes[12..16] != b"IHDR" {
        return Err("invalid PNG header".into());
    }

    let width = read_u32_be(bytes, 16)?;
    let height = read_u32_be(bytes, 20)?;
    ensure_positive_dimensions(width, height)
}

fn parse_jpeg_dimensions(bytes: &[u8]) -> Result<(u32, u32), String> {
    if bytes.len() < 4 || bytes[0] != 0xFF || bytes[1] != 0xD8 {
        return Err("invalid JPEG header".into());
    }

    let mut index = 2usize;
    while index + 1 < bytes.len() {
        while index < bytes.len() && bytes[index] != 0xFF {
            index += 1;
        }
        while index < bytes.len() && bytes[index] == 0xFF {
            index += 1;
        }
        if index >= bytes.len() {
            break;
        }

        let marker = bytes[index];
        index += 1;

        if marker == 0xD8 || marker == 0xD9 || marker == 0x01 || (0xD0..=0xD7).contains(&marker) {
            continue;
        }

        if index + 2 > bytes.len() {
            return Err("truncated JPEG segment".into());
        }
        let segment_length = read_u16_be(bytes, index)? as usize;
        if segment_length < 2 {
            return Err("invalid JPEG segment length".into());
        }
        index += 2;

        let segment_end = index + segment_length.saturating_sub(2);
        if segment_end > bytes.len() {
            return Err("truncated JPEG segment payload".into());
        }

        if matches!(marker, 0xC0 | 0xC1 | 0xC2 | 0xC3 | 0xC5 | 0xC6 | 0xC7 | 0xC9 | 0xCA | 0xCB | 0xCD | 0xCE | 0xCF) {
            if index + 5 > segment_end {
                return Err("truncated JPEG frame header".into());
            }
            let height = read_u16_be(bytes, index + 1)? as u32;
            let width = read_u16_be(bytes, index + 3)? as u32;
            return ensure_positive_dimensions(width, height);
        }

        index = segment_end;
    }

    Err("JPEG dimensions not found".into())
}

fn parse_bmp_dimensions(bytes: &[u8]) -> Result<(u32, u32), String> {
    if bytes.len() < 26 || &bytes[..2] != b"BM" {
        return Err("invalid BMP header".into());
    }

    let dib_header_size = read_u32_le(bytes, 14)?;
    match dib_header_size {
        12 => {
            let width = read_u16_le(bytes, 18)? as u32;
            let height = read_u16_le(bytes, 20)? as u32;
            ensure_positive_dimensions(width, height)
        }
        size if size >= 40 => {
            let width = read_i32_le(bytes, 18)?;
            let height = read_i32_le(bytes, 22)?;
            ensure_positive_dimensions(width.unsigned_abs(), height.unsigned_abs())
        }
        _ => Err("unsupported BMP header".into()),
    }
}

fn parse_webp_dimensions(bytes: &[u8]) -> Result<(u32, u32), String> {
    if bytes.len() < 30 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WEBP" {
        return Err("invalid WEBP header".into());
    }

    match &bytes[12..16] {
        b"VP8X" => {
            let width = 1 + read_u24_le(bytes, 24)?;
            let height = 1 + read_u24_le(bytes, 27)?;
            ensure_positive_dimensions(width, height)
        }
        b"VP8L" => {
            if bytes.len() < 25 || bytes[20] != 0x2F {
                return Err("invalid WEBP lossless header".into());
            }
            let packed = read_u32_le(bytes, 21)?;
            let width = 1 + (packed & 0x3FFF);
            let height = 1 + ((packed >> 14) & 0x3FFF);
            ensure_positive_dimensions(width, height)
        }
        b"VP8 " => {
            if bytes.len() < 30 || bytes[23] != 0x9D || bytes[24] != 0x01 || bytes[25] != 0x2A {
                return Err("invalid WEBP lossy header".into());
            }
            let width = (read_u16_le(bytes, 26)? & 0x3FFF) as u32;
            let height = (read_u16_le(bytes, 28)? & 0x3FFF) as u32;
            ensure_positive_dimensions(width, height)
        }
        _ => Err("unsupported WEBP chunk".into()),
    }
}

fn parse_tiff_dimensions(bytes: &[u8]) -> Result<(u32, u32), String> {
    if bytes.len() < 8 {
        return Err("invalid TIFF header".into());
    }

    let little_endian = match &bytes[..4] {
        b"II*\0" => true,
        b"MM\0*" => false,
        _ => return Err("invalid TIFF header".into()),
    };

    let first_ifd_offset = read_u32_tiff(bytes, 4, little_endian)? as usize;
    if first_ifd_offset + 2 > bytes.len() {
        return Err("invalid TIFF IFD offset".into());
    }

    let entry_count = read_u16_tiff(bytes, first_ifd_offset, little_endian)? as usize;
    let mut width = None;
    let mut height = None;

    for index in 0..entry_count {
        let entry_offset = first_ifd_offset + 2 + (index * 12);
        if entry_offset + 12 > bytes.len() {
            return Err("truncated TIFF directory entry".into());
        }

        let tag = read_u16_tiff(bytes, entry_offset, little_endian)?;
        if tag != 256 && tag != 257 {
            continue;
        }

        let value = read_tiff_entry_value(bytes, entry_offset, little_endian)?;
        if tag == 256 {
            width = Some(value);
        } else {
            height = Some(value);
        }
    }

    match (width, height) {
        (Some(width), Some(height)) => ensure_positive_dimensions(width, height),
        _ => Err("TIFF dimensions not found".into()),
    }
}

fn read_tiff_entry_value(bytes: &[u8], entry_offset: usize, little_endian: bool) -> Result<u32, String> {
    let field_type = read_u16_tiff(bytes, entry_offset + 2, little_endian)?;
    let count = read_u32_tiff(bytes, entry_offset + 4, little_endian)?;
    if count == 0 {
        return Err("invalid TIFF tag count".into());
    }

    match field_type {
        3 => {
            if count == 1 {
                if little_endian {
                    Ok(read_u16_le(bytes, entry_offset + 8)? as u32)
                } else {
                    Ok(read_u16_be(bytes, entry_offset + 8)? as u32)
                }
            } else {
                let value_offset = read_u32_tiff(bytes, entry_offset + 8, little_endian)? as usize;
                if little_endian {
                    Ok(read_u16_le(bytes, value_offset)? as u32)
                } else {
                    Ok(read_u16_be(bytes, value_offset)? as u32)
                }
            }
        }
        4 => {
            if count == 1 {
                read_u32_tiff(bytes, entry_offset + 8, little_endian)
            } else {
                let value_offset = read_u32_tiff(bytes, entry_offset + 8, little_endian)? as usize;
                read_u32_tiff(bytes, value_offset, little_endian)
            }
        }
        _ => Err("unsupported TIFF dimension field type".into()),
    }
}

fn read_u16_tiff(bytes: &[u8], offset: usize, little_endian: bool) -> Result<u16, String> {
    if little_endian {
        read_u16_le(bytes, offset)
    } else {
        read_u16_be(bytes, offset)
    }
}

fn read_u32_tiff(bytes: &[u8], offset: usize, little_endian: bool) -> Result<u32, String> {
    if little_endian {
        read_u32_le(bytes, offset)
    } else {
        read_u32_be(bytes, offset)
    }
}

fn read_u16_be(bytes: &[u8], offset: usize) -> Result<u16, String> {
    let slice = bytes
        .get(offset..offset + 2)
        .ok_or_else(|| "unexpected end of file".to_string())?;
    Ok(u16::from_be_bytes([slice[0], slice[1]]))
}

fn read_u16_le(bytes: &[u8], offset: usize) -> Result<u16, String> {
    let slice = bytes
        .get(offset..offset + 2)
        .ok_or_else(|| "unexpected end of file".to_string())?;
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

fn read_u32_be(bytes: &[u8], offset: usize) -> Result<u32, String> {
    let slice = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| "unexpected end of file".to_string())?;
    Ok(u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Result<u32, String> {
    let slice = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| "unexpected end of file".to_string())?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_i32_le(bytes: &[u8], offset: usize) -> Result<i32, String> {
    let slice = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| "unexpected end of file".to_string())?;
    Ok(i32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_u24_le(bytes: &[u8], offset: usize) -> Result<u32, String> {
    let slice = bytes
        .get(offset..offset + 3)
        .ok_or_else(|| "unexpected end of file".to_string())?;
    Ok((slice[0] as u32) | ((slice[1] as u32) << 8) | ((slice[2] as u32) << 16))
}

fn ensure_positive_dimensions(width: u32, height: u32) -> Result<(u32, u32), String> {
    if width == 0 || height == 0 {
        return Err("image dimensions must be greater than zero".into());
    }

    Ok((width, height))
}
fn build_image_record(
    workspace_id: &str,
    source_id: &str,
    file_name: String,
    image_path: &Path,
    relative_path: Option<String>,
    created_at: &str,
    updated_at: &str,
) -> StoredImageRecord {
    let (width, height, health_status, health_error) = match probe_image_dimensions_for_path(image_path) {
        Ok((width, height)) => (Some(width), Some(height), "healthy".to_string(), None),
        Err(error) => (None, None, "corrupted".to_string(), Some(error)),
    };

    StoredImageRecord {
        id: build_image_id(source_id, image_path),
        workspace_id: workspace_id.to_string(),
        source_id: source_id.to_string(),
        file_name,
        original_path: normalize_path_string(image_path),
        relative_path,
        width,
        height,
        annotation_status: "unannotated".into(),
        health_status,
        health_error,
        created_at: created_at.to_string(),
        updated_at: updated_at.to_string(),
    }
}

fn scan_source_folder(source_root: &Path, workspace_id: &str, source_id: &str) -> ScanResult {
    let source_name = source_root
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("source")
        .to_string();
    register_scan_progress(workspace_id, source_id, &source_name, "Inspecting source folder", 0, 0);
    let _guard = ScanProgressGuard {
        workspace_id: workspace_id.to_string(),
        source_id: source_id.to_string(),
    };
    let source_type = detect_source_type(source_root);

    match source_type.as_str() {
        "YOLO" => scan_yolo_source(source_root, workspace_id, source_id),
        "COCO" => scan_coco_source(source_root, workspace_id, source_id),
        "RAW" => {
            let image_records = collect_raw_image_records(source_root, workspace_id, source_id);
            ScanResult {
                source_type,
                status: resolve_source_status("review", &image_records),
                image_count: image_records.len() as u32,
                category_count: 0,
                image_records,
                category_records: Vec::new(),
                annotation_records: Vec::new(),
            }
        }
        _ => ScanResult {
            source_type,
            status: "review".into(),
            image_count: 0,
            category_count: 0,
            image_records: Vec::new(),
            category_records: Vec::new(),
            annotation_records: Vec::new(),
        },
    }
}

fn scan_yolo_source(source_root: &Path, workspace_id: &str, source_id: &str) -> ScanResult {
    let category_names = read_yolo_category_names(source_root);
    let now = Utc::now().to_rfc3339();
    let source_name = source_root
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("source")
        .to_string();

    let category_records = category_names
        .iter()
        .enumerate()
        .map(|(index, name)| StoredCategoryRecord {
            id: build_category_id(source_id, index),
            workspace_id: workspace_id.to_string(),
            source_id: source_id.to_string(),
            name: name.clone(),
            normalized_name: slugify_workspace_name(name),
            created_at: now.clone(),
            updated_at: now.clone(),
        })
        .collect::<Vec<_>>();

    let mut category_id_by_index = HashMap::new();
    for (index, category) in category_records.iter().enumerate() {
        category_id_by_index.insert(index as u32, category.id.clone());
    }

    let image_paths = collect_yolo_image_paths(source_root);
    let total_images = image_paths.len() as u32;
    update_scan_progress(
        workspace_id,
        source_id,
        &source_name,
        "Scanning YOLO images",
        0,
        total_images,
    );
    let labels_root = source_root.join("labels");
    let images_root = source_root.join("images");

    let mut image_records = Vec::new();
    let mut annotation_records = Vec::new();

    for (index, image_path) in image_paths.into_iter().enumerate() {
        let file_name = image_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("image")
            .to_string();
        let mut image_record = build_image_record(
            workspace_id,
            source_id,
            file_name,
            &image_path,
            image_path
                .strip_prefix(source_root)
                .ok()
                .map(|value| value.to_string_lossy().to_string()),
            &now,
            &now,
        );

        if image_record.health_status == "healthy" {
            let label_path = resolve_yolo_label_path(source_root, &images_root, &labels_root, &image_path);
            let parsed_annotations = parse_yolo_annotations(
                &label_path,
                workspace_id,
                source_id,
                &image_record.id,
                &category_id_by_index,
            );
            if !parsed_annotations.is_empty() {
                image_record.annotation_status = "annotated".into();
            }
            annotation_records.extend(parsed_annotations);
        }

        image_records.push(image_record);

        update_scan_progress(
            workspace_id,
            source_id,
            &source_name,
            "Scanning YOLO images",
            (index as u32) + 1,
            total_images,
        );
    }

    image_records.sort_by(|a, b| a.file_name.cmp(&b.file_name));

    ScanResult {
        source_type: "YOLO".into(),
        status: resolve_source_status("ready", &image_records),
        image_count: image_records.len() as u32,
        category_count: category_records.len() as u32,
        image_records,
        category_records,
        annotation_records,
    }
}

fn scan_coco_source(source_root: &Path, workspace_id: &str, source_id: &str) -> ScanResult {
    let source_name = source_root
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("source")
        .to_string();
    let Some(annotation_file) = find_coco_annotation_file(source_root) else {
        return ScanResult {
            source_type: "COCO".into(),
            status: "ready".into(),
            image_count: count_images_recursive(source_root),
            category_count: 0,
            image_records: Vec::new(),
            category_records: Vec::new(),
            annotation_records: Vec::new(),
        };
    };

    let Ok(content) = fs::read_to_string(&annotation_file) else {
        return ScanResult {
            source_type: "COCO".into(),
            status: "ready".into(),
            image_count: count_images_recursive(source_root),
            category_count: 0,
            image_records: Vec::new(),
            category_records: Vec::new(),
            annotation_records: Vec::new(),
        };
    };

    let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) else {
        return ScanResult {
            source_type: "COCO".into(),
            status: "ready".into(),
            image_count: count_images_recursive(source_root),
            category_count: 0,
            image_records: Vec::new(),
            category_records: Vec::new(),
            annotation_records: Vec::new(),
        };
    };

    let now = Utc::now().to_rfc3339();
    let category_values = json
        .get("categories")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let image_values = json
        .get("images")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let annotation_values = json
        .get("annotations")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();

    let mut category_records = Vec::new();
    let mut source_category_lookup = HashMap::new();
    for (index, category) in category_values.iter().enumerate() {
        let Some(category_name) = category.get("name").and_then(|value| value.as_str()) else {
            continue;
        };
        let Some(original_category_id) = json_id_to_string(category.get("id")) else {
            continue;
        };

        let record = StoredCategoryRecord {
            id: build_category_id(source_id, index),
            workspace_id: workspace_id.to_string(),
            source_id: source_id.to_string(),
            name: category_name.to_string(),
            normalized_name: slugify_workspace_name(category_name),
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        source_category_lookup.insert(original_category_id, record.id.clone());
        category_records.push(record);
    }

    let mut image_meta = HashMap::new();
    let total_images = image_values.len() as u32;
    update_scan_progress(
        workspace_id,
        source_id,
        &source_name,
        "Scanning COCO images",
        0,
        total_images,
    );
    for (index, image) in image_values.iter().enumerate() {
        let Some(original_image_id) = json_id_to_string(image.get("id")) else {
            continue;
        };
        let Some(file_name) = image.get("file_name").and_then(|value| value.as_str()) else {
            continue;
        };

        image_meta.insert(original_image_id, file_name.to_string());
        update_scan_progress(
            workspace_id,
            source_id,
            &source_name,
            "Scanning COCO images",
            (index as u32) + 1,
            total_images,
        );
    }

    let mut image_records = Vec::new();
    let mut annotation_records = Vec::new();
    let mut annotation_count_by_image = HashMap::<String, usize>::new();
    let mut image_id_lookup = HashMap::<String, String>::new();

    for (original_image_id, file_name) in &image_meta {
        let resolved_path = resolve_coco_image_path(source_root, &annotation_file, file_name);
        let mut image_record = build_image_record(
            workspace_id,
            source_id,
            Path::new(file_name)
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or(file_name)
                .to_string(),
            &resolved_path,
            resolved_path
                .strip_prefix(source_root)
                .ok()
                .map(|value| value.to_string_lossy().to_string())
                .or_else(|| Some(file_name.clone())),
            &now,
            &now,
        );

        if image_record.health_status == "healthy" {
            image_id_lookup.insert(original_image_id.clone(), image_record.id.clone());
        }

        image_record.annotation_status = "unannotated".into();
        image_records.push(image_record);
    }

    for (line_index, annotation) in annotation_values.iter().enumerate() {
        let Some(original_image_id) = annotation
            .get("image_id")
            .and_then(|value| json_id_to_string(Some(value)))
        else {
            continue;
        };
        let Some(image_id) = image_id_lookup.get(&original_image_id) else {
            continue;
        };
        let Some(original_category_id) = annotation
            .get("category_id")
            .and_then(|value| json_id_to_string(Some(value)))
        else {
            continue;
        };
        let Some(source_category_id) = source_category_lookup.get(&original_category_id) else {
            continue;
        };
        let Some(bbox) = annotation.get("bbox").and_then(|value| value.as_array()) else {
            continue;
        };
        if bbox.len() < 4 {
            continue;
        }
        let Some(bbox_x) = bbox.first().and_then(parse_json_number) else {
            continue;
        };
        let Some(bbox_y) = bbox.get(1).and_then(parse_json_number) else {
            continue;
        };
        let Some(bbox_width) = bbox.get(2).and_then(parse_json_number) else {
            continue;
        };
        let Some(bbox_height) = bbox.get(3).and_then(parse_json_number) else {
            continue;
        };

        *annotation_count_by_image.entry(image_id.clone()).or_insert(0) += 1;
        annotation_records.push(StoredAnnotationRecord {
            id: format!("ann-{source_id}-{line_index}"),
            workspace_id: workspace_id.to_string(),
            image_id: image_id.clone(),
            source_id: source_id.to_string(),
            source_category_id: Some(source_category_id.clone()),
            category_id: None,
            bbox_x,
            bbox_y,
            bbox_width,
            bbox_height,
            annotation_format: "coco".into(),
            created_at: now.clone(),
            updated_at: now.clone(),
        });
    }

    for image in &mut image_records {
        if annotation_count_by_image.contains_key(&image.id) {
            image.annotation_status = "annotated".into();
        }
    }

    image_records.sort_by(|a, b| a.file_name.cmp(&b.file_name));

    ScanResult {
        source_type: "COCO".into(),
        status: resolve_source_status("ready", &image_records),
        image_count: image_records.len() as u32,
        category_count: category_records.len() as u32,
        image_records,
        category_records,
        annotation_records,
    }
}

fn detect_source_type(source_root: &Path) -> String {
    let has_data_yaml = source_root.join("data.yaml").exists();
    let has_images_dir = source_root.join("images").is_dir();
    let has_labels_dir = source_root.join("labels").is_dir();
    if has_data_yaml || (has_images_dir && has_labels_dir) {
        return "YOLO".into();
    }

    let has_annotations_dir = source_root.join("annotations").is_dir();
    let has_coco_json = fs::read_dir(source_root)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.flatten())
        .any(|entry| {
            let path = entry.path();
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("json"))
                .unwrap_or(false)
        });
    if has_annotations_dir || has_coco_json {
        return "COCO".into();
    }

    "RAW".into()
}

fn collect_raw_image_records(root: &Path, workspace_id: &str, source_id: &str) -> Vec<StoredImageRecord> {
    let now = Utc::now().to_rfc3339();
    let source_name = root
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("source")
        .to_string();
    let image_paths = collect_image_paths_recursive(root);
    let total_images = image_paths.len() as u32;
    update_scan_progress(
        workspace_id,
        source_id,
        &source_name,
        "Scanning RAW images",
        0,
        total_images,
    );

    image_paths
        .into_iter()
        .enumerate()
        .map(|(index, entry_path)| {
            let file_name = entry_path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("image")
                .to_string();
            let relative_path = entry_path
                .strip_prefix(root)
                .ok()
                .map(|value| value.to_string_lossy().to_string());

            let record = build_image_record(
                workspace_id,
                source_id,
                file_name,
                &entry_path,
                relative_path,
                &now,
                &now,
            );

            update_scan_progress(
                workspace_id,
                source_id,
                &source_name,
                "Scanning RAW images",
                (index as u32) + 1,
                total_images,
            );

            record
        })
        .collect()
}

fn collect_yolo_image_paths(root: &Path) -> Vec<PathBuf> {
    let images_root = root.join("images");
    let mut paths = if images_root.is_dir() {
        collect_image_paths_recursive(&images_root)
    } else {
        collect_image_paths_recursive_filtered(root, &["labels", "annotations", ".dataviewer"])
    };

    paths.sort();
    paths
}

fn collect_image_paths_recursive(root: &Path) -> Vec<PathBuf> {
    collect_image_paths_recursive_filtered(root, &[])
}

fn collect_image_paths_recursive_filtered(root: &Path, ignored_dir_names: &[&str]) -> Vec<PathBuf> {
    let mut results = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(path) = stack.pop() {
        let Ok(entries) = fs::read_dir(path) else {
            continue;
        };

        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                let skip = entry_path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .map(|name| ignored_dir_names.iter().any(|ignored| name.eq_ignore_ascii_case(ignored)))
                    .unwrap_or(false);
                if !skip {
                    stack.push(entry_path);
                }
                continue;
            }

            if is_supported_image(&entry_path) {
                results.push(entry_path);
            }
        }
    }

    results
}

fn count_images_recursive(root: &Path) -> u32 {
    collect_image_paths_recursive(root).len() as u32
}

fn read_yolo_category_names(root: &Path) -> Vec<String> {
    let data_yaml = root.join("data.yaml");
    if data_yaml.exists() {
        if let Ok(content) = fs::read_to_string(&data_yaml) {
            let names = parse_yolo_names_from_yaml(&content);
            if !names.is_empty() {
                return names;
            }
        }
    }

    let classes_txt = root.join("classes.txt");
    if classes_txt.exists() {
        if let Ok(content) = fs::read_to_string(classes_txt) {
            let names = content
                .lines()
                .map(|line| sanitize_yaml_name(line))
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>();
            if !names.is_empty() {
                return names;
            }
        }
    }

    Vec::new()
}

fn parse_yolo_names_from_yaml(content: &str) -> Vec<String> {
    let lines = content.lines().collect::<Vec<_>>();
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("names:") {
            let inline = value.trim();
            if inline.starts_with('[') && inline.ends_with(']') {
                return parse_inline_yaml_list(inline);
            }

            let mut names = Vec::new();
            for next_line in lines.iter().skip(index + 1) {
                if next_line.trim().is_empty() {
                    continue;
                }
                let raw = next_line.trim();
                let starts_new_top_level = !next_line.starts_with(' ') && !next_line.starts_with('\t');
                if starts_new_top_level {
                    break;
                }
                if let Some((_, value)) = raw.split_once(':') {
                    let parsed = sanitize_yaml_name(value);
                    if !parsed.is_empty() {
                        names.push(parsed);
                    }
                    continue;
                }
                if let Some(value) = raw.strip_prefix('-') {
                    let parsed = sanitize_yaml_name(value);
                    if !parsed.is_empty() {
                        names.push(parsed);
                    }
                }
            }

            if !names.is_empty() {
                return names;
            }
        }
    }

    Vec::new()
}

fn parse_inline_yaml_list(value: &str) -> Vec<String> {
    let inner = value
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']');

    inner
        .split(',')
        .map(sanitize_yaml_name)
        .filter(|item| !item.is_empty())
        .collect()
}

fn sanitize_yaml_name(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string()
}

fn resolve_yolo_label_path(
    source_root: &Path,
    images_root: &Path,
    labels_root: &Path,
    image_path: &Path,
) -> PathBuf {
    if let Some(parent) = image_path.parent() {
        let parent_name = parent.file_name().and_then(|value| value.to_str());
        if matches!(parent_name, Some(name) if name.eq_ignore_ascii_case("images")) {
            if let Some(split_root) = parent.parent() {
                let sibling = split_root
                    .join("labels")
                    .join(image_path.file_name().unwrap_or_default())
                    .with_extension("txt");
                if sibling.exists() {
                    return sibling;
                }
            }
        }
    }

    if labels_root.is_dir() {
        if images_root.is_dir() {
            if let Ok(relative) = image_path.strip_prefix(images_root) {
                let mirrored = labels_root.join(relative).with_extension("txt");
                if mirrored.exists() {
                    return mirrored;
                }
            }
        }

        if let Ok(relative) = image_path.strip_prefix(source_root) {
            let mirrored = labels_root.join(relative).with_extension("txt");
            if mirrored.exists() {
                return mirrored;
            }
        }

        if let Some(found) = find_label_by_file_stem(labels_root, image_path) {
            return found;
        }
    }

    image_path.with_extension("txt")
}

fn find_label_by_file_stem(labels_root: &Path, image_path: &Path) -> Option<PathBuf> {
    let image_stem = image_path.file_stem()?.to_str()?;
    let mut stack = vec![labels_root.to_path_buf()];

    while let Some(path) = stack.pop() {
        let Ok(entries) = fs::read_dir(path) else {
            continue;
        };

        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                stack.push(entry_path);
                continue;
            }

            let is_txt = entry_path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("txt"))
                .unwrap_or(false);
            if !is_txt {
                continue;
            }

            let label_stem = entry_path.file_stem().and_then(|value| value.to_str());
            if label_stem == Some(image_stem) {
                return Some(entry_path);
            }
        }
    }

    None
}

fn parse_yolo_annotations(
    label_path: &Path,
    workspace_id: &str,
    source_id: &str,
    image_id: &str,
    category_id_by_index: &HashMap<u32, String>,
) -> Vec<StoredAnnotationRecord> {
    if !label_path.exists() {
        return Vec::new();
    }

    let Ok(content) = fs::read_to_string(label_path) else {
        return Vec::new();
    };
    let now = Utc::now().to_rfc3339();
    let mut annotations = Vec::new();

    for (line_index, line) in content.lines().enumerate() {
        let parts = line.split_whitespace().collect::<Vec<_>>();
        if parts.len() < 5 {
            continue;
        }

        let Ok(class_index) = parts[0].parse::<u32>() else {
            continue;
        };
        let Ok(bbox_x) = parts[1].parse::<f64>() else {
            continue;
        };
        let Ok(bbox_y) = parts[2].parse::<f64>() else {
            continue;
        };
        let Ok(bbox_width) = parts[3].parse::<f64>() else {
            continue;
        };
        let Ok(bbox_height) = parts[4].parse::<f64>() else {
            continue;
        };

        annotations.push(StoredAnnotationRecord {
            id: format!("ann-{image_id}-{line_index}"),
            workspace_id: workspace_id.to_string(),
            image_id: image_id.to_string(),
            source_id: source_id.to_string(),
            source_category_id: category_id_by_index.get(&class_index).cloned(),
            category_id: None,
            bbox_x,
            bbox_y,
            bbox_width,
            bbox_height,
            annotation_format: "yolo".into(),
            created_at: now.clone(),
            updated_at: now.clone(),
        });
    }

    annotations
}

fn find_coco_annotation_file(root: &Path) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if root.join("annotations").is_dir() {
        if let Ok(entries) = fs::read_dir(root.join("annotations")) {
            for entry in entries.flatten() {
                let path = entry.path();
                let is_json = path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("json"))
                    .unwrap_or(false);
                if is_json {
                    candidates.push(path);
                }
            }
        }
    }

    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            let is_json = path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("json"))
                .unwrap_or(false);
            if is_json {
                candidates.push(path);
            }
        }
    }

    for candidate in candidates {
        let Ok(content) = fs::read_to_string(&candidate) else {
            continue;
        };
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) else {
            continue;
        };
        let has_images = json.get("images").and_then(|value| value.as_array()).is_some();
        let has_annotations = json.get("annotations").and_then(|value| value.as_array()).is_some();
        let has_categories = json.get("categories").and_then(|value| value.as_array()).is_some();
        if has_images && has_annotations && has_categories {
            return Some(candidate);
        }
    }

    None
}

fn resolve_coco_image_path(source_root: &Path, annotation_file: &Path, file_name: &str) -> PathBuf {
    let file_path = PathBuf::from(file_name);
    if file_path.is_absolute() && file_path.exists() {
        return file_path;
    }

    let candidates = [
        source_root.join(file_name),
        source_root.join("images").join(file_name),
        annotation_file
            .parent()
            .map(|parent| parent.join(file_name))
            .unwrap_or_else(|| source_root.join(file_name)),
    ];

    for candidate in candidates {
        if candidate.exists() {
            return candidate;
        }
    }

    if let Some(found) = find_file_by_name(source_root, Path::new(file_name).file_name().and_then(|v| v.to_str()).unwrap_or(file_name)) {
        return found;
    }

    source_root.join(file_name)
}

fn find_file_by_name(root: &Path, target_file_name: &str) -> Option<PathBuf> {
    let mut stack = vec![root.to_path_buf()];

    while let Some(path) = stack.pop() {
        let Ok(entries) = fs::read_dir(path) else {
            continue;
        };

        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                stack.push(entry_path);
                continue;
            }

            let file_name = entry_path.file_name().and_then(|value| value.to_str());
            if file_name == Some(target_file_name) {
                return Some(entry_path);
            }
        }
    }

    None
}

fn json_id_to_string(value: Option<&serde_json::Value>) -> Option<String> {
    let value = value?;
    if let Some(text) = value.as_str() {
        return Some(text.to_string());
    }
    if let Some(number) = value.as_i64() {
        return Some(number.to_string());
    }
    if let Some(number) = value.as_u64() {
        return Some(number.to_string());
    }
    None
}

fn count_coco_categories(root: &Path) -> u32 {
    let Some(candidate) = find_coco_annotation_file(root) else {
        return 0;
    };
    let Ok(content) = fs::read_to_string(candidate) else {
        return 0;
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) else {
        return 0;
    };
    json.get("categories")
        .and_then(|value| value.as_array())
        .map(|categories| categories.len() as u32)
        .unwrap_or(0)
}

fn is_supported_image(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| IMAGE_EXTENSIONS.iter().any(|candidate| ext.eq_ignore_ascii_case(candidate)))
        .unwrap_or(false)
}

fn build_source_id(source_root: &Path) -> String {
    let base = source_root
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("source");
    format!("source-{}-{}", slugify_workspace_name(base), Utc::now().timestamp_millis())
}

fn build_image_id(source_id: &str, image_path: &Path) -> String {
    let relative = normalize_path_string(image_path);
    format!("img-{source_id}-{}", slugify_workspace_name(&relative))
}

fn build_category_id(source_id: &str, category_index: usize) -> String {
    format!("cat-{source_id}-{category_index}")
}

fn make_unique_file_name(file_name: &str, used_names: &mut HashMap<String, usize>) -> String {
    let next = used_names.entry(file_name.to_string()).or_insert(0);
    if *next == 0 {
        *next = 1;
        return file_name.to_string();
    }

    let path = Path::new(file_name);
    let stem = path.file_stem().and_then(|value| value.to_str()).unwrap_or("image");
    let ext = path.extension().and_then(|value| value.to_str()).unwrap_or("");
    let candidate = if ext.is_empty() {
        format!("{stem}-{}", *next)
    } else {
        format!("{stem}-{}.{}", *next, ext)
    };
    *next += 1;
    candidate
}

fn sort_images_for_seed(images: &mut [ExportImageRecord], random_seed: u64) {
    images.sort_by_key(|image| {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        random_seed.hash(&mut hasher);
        image.id.hash(&mut hasher);
        hasher.finish()
    });
}

fn compute_split_counts(total: u32, train_ratio: u32, valid_ratio: u32, test_ratio: u32) -> crate::models::SplitCounts {
    let ratio_sum = train_ratio + valid_ratio + test_ratio;
    if total == 0 || ratio_sum == 0 {
        return crate::models::SplitCounts {
            train: 0,
            valid: 0,
            test: 0,
        };
    }

    let train = ((total as f64) * (train_ratio as f64 / ratio_sum as f64)).floor() as u32;
    let valid = ((total as f64) * (valid_ratio as f64 / ratio_sum as f64)).floor() as u32;
    let test = total.saturating_sub(train).saturating_sub(valid);

    crate::models::SplitCounts { train, valid, test }
}

fn export_coco_split(
    split_name: &str,
    images: &[ExportImageRecord],
    split_dir: &Path,
) -> Result<u32, String> {
    let mut used_file_names = HashSet::<String>::new();
    let mut exported_images = Vec::<serde_json::Value>::new();
    let mut exported_annotations = Vec::<serde_json::Value>::new();
    let mut category_ids = BTreeMap::<String, u32>::new();
    let mut category_names = BTreeMap::<String, String>::new();
    let mut annotation_id = 1_u32;

    for (index, image) in images.iter().enumerate() {
        let safe_file_name = ensure_unique_for_export(&image.file_name, &mut used_file_names);
        let destination = split_dir.join(&safe_file_name);
        fs::copy(&image.original_path, &destination)
            .map_err(|error| format!("failed to copy image into export split '{split_name}': {error}"))?;

        let (width, height) = resolve_export_dimensions(image)?;
        let image_numeric_id = (index as u32) + 1;
        exported_images.push(serde_json::json!({
            "id": image_numeric_id,
            "file_name": safe_file_name,
            "width": width,
            "height": height
        }));

        for annotation in &image.annotations {
            let next_category_id = if let Some(existing) = category_ids.get(&annotation.category_key) {
                *existing
            } else {
                let next = (category_ids.len() as u32) + 1;
                category_ids.insert(annotation.category_key.clone(), next);
                category_names.insert(annotation.category_key.clone(), annotation.category_name.clone());
                next
            };

            let bbox = if annotation.annotation_format.eq_ignore_ascii_case("yolo") {
                vec![
                    (annotation.bbox_x - annotation.bbox_width / 2.0) * width as f64,
                    (annotation.bbox_y - annotation.bbox_height / 2.0) * height as f64,
                    annotation.bbox_width * width as f64,
                    annotation.bbox_height * height as f64,
                ]
            } else {
                vec![
                    annotation.bbox_x,
                    annotation.bbox_y,
                    annotation.bbox_width,
                    annotation.bbox_height,
                ]
            };

            exported_annotations.push(serde_json::json!({
                "id": annotation_id,
                "image_id": image_numeric_id,
                "category_id": next_category_id,
                "bbox": bbox,
                "area": bbox[2].max(0.0) * bbox[3].max(0.0),
                "iscrowd": 0
            }));
            annotation_id = annotation_id.saturating_add(1);
        }
    }

    let categories = category_ids
        .iter()
        .map(|(key, id)| {
            serde_json::json!({
                "id": id,
                "name": category_names.get(key).cloned().unwrap_or_else(|| key.clone()),
                "supercategory": "object"
            })
        })
        .collect::<Vec<_>>();

    let payload = serde_json::json!({
        "images": exported_images,
        "annotations": exported_annotations,
        "categories": categories
    });
    fs::write(
        split_dir.join("_annotations.coco.json"),
        serde_json::to_string_pretty(&payload)
            .map_err(|error| format!("failed to serialize COCO export payload: {error}"))?,
    )
    .map_err(|error| format!("failed to write COCO export payload: {error}"))?;

    Ok(annotation_id.saturating_sub(1))
}

fn export_yolo_dataset(
    output_root: &Path,
    train_images: &[ExportImageRecord],
    valid_images: &[ExportImageRecord],
    test_images: &[ExportImageRecord],
) -> Result<u32, String> {
    let categories = collect_export_categories(train_images, valid_images, test_images);
    if categories.is_empty() {
        return Err("no categories available for YOLO export".into());
    }

    let train_images_dir = output_root.join("train").join("images");
    let train_labels_dir = output_root.join("train").join("labels");
    let valid_images_dir = output_root.join("valid").join("images");
    let valid_labels_dir = output_root.join("valid").join("labels");
    let test_images_dir = output_root.join("test").join("images");
    let test_labels_dir = output_root.join("test").join("labels");

    for dir in [
        &train_images_dir,
        &train_labels_dir,
        &valid_images_dir,
        &valid_labels_dir,
        &test_images_dir,
        &test_labels_dir,
    ] {
        fs::create_dir_all(dir)
            .map_err(|error| format!("failed to create YOLO export directories: {error}"))?;
    }

    let mut export_boxes = 0_u32;
    export_boxes += export_yolo_split(train_images, &train_images_dir, &train_labels_dir, &categories)?;
    export_boxes += export_yolo_split(valid_images, &valid_images_dir, &valid_labels_dir, &categories)?;
    export_boxes += export_yolo_split(test_images, &test_images_dir, &test_labels_dir, &categories)?;

    let mut yaml = String::new();
    yaml.push_str("train: train/images\n");
    yaml.push_str("val: valid/images\n");
    yaml.push_str("test: test/images\n");
    yaml.push_str(&format!("nc: {}\n", categories.len()));
    yaml.push_str("names:\n");
    for (index, (_, name)) in categories.iter().enumerate() {
        yaml.push_str(&format!("  {index}: {name}\n"));
    }

    fs::write(output_root.join("data.yaml"), yaml)
        .map_err(|error| format!("failed to write YOLO data.yaml: {error}"))?;

    Ok(export_boxes)
}

fn export_yolo_split(
    images: &[ExportImageRecord],
    images_dir: &Path,
    labels_dir: &Path,
    categories: &[(String, String)],
) -> Result<u32, String> {
    let category_index = categories
        .iter()
        .enumerate()
        .map(|(index, (key, _))| (key.clone(), index as u32))
        .collect::<HashMap<_, _>>();
    let mut used_names = HashSet::<String>::new();
    let mut exported_boxes = 0_u32;

    for image in images {
        let safe_file_name = ensure_unique_for_export(&image.file_name, &mut used_names);
        let destination = images_dir.join(&safe_file_name);
        fs::copy(&image.original_path, &destination)
            .map_err(|error| format!("failed to copy image into YOLO export split: {error}"))?;

        let (width, height) = resolve_export_dimensions(image)?;
        let mut label_lines = Vec::<String>::new();
        for annotation in &image.annotations {
            let Some(class_index) = category_index.get(&annotation.category_key) else {
                continue;
            };

            let (cx, cy, bw, bh) = if annotation.annotation_format.eq_ignore_ascii_case("yolo") {
                (
                    annotation.bbox_x,
                    annotation.bbox_y,
                    annotation.bbox_width,
                    annotation.bbox_height,
                )
            } else {
                (
                    (annotation.bbox_x + annotation.bbox_width / 2.0) / width as f64,
                    (annotation.bbox_y + annotation.bbox_height / 2.0) / height as f64,
                    annotation.bbox_width / width as f64,
                    annotation.bbox_height / height as f64,
                )
            };

            label_lines.push(format!(
                "{} {:.6} {:.6} {:.6} {:.6}",
                class_index, cx, cy, bw, bh
            ));
            exported_boxes = exported_boxes.saturating_add(1);
        }

        let label_path = labels_dir.join(Path::new(&safe_file_name).with_extension("txt"));
        fs::write(label_path, label_lines.join("\n"))
            .map_err(|error| format!("failed to write YOLO label file: {error}"))?;
    }

    Ok(exported_boxes)
}

fn filter_export_images(
    images: Vec<ExportImageRecord>,
    browser_payload: &BrowserPayload,
    selected_image_ids: Option<&[String]>,
    selected_source_ids: &[String],
    selected_category_ids: &[String],
) -> Vec<ExportImageRecord> {
    let image_lookup = browser_payload
        .images
        .iter()
        .map(|image| (image.id.as_str(), image))
        .collect::<HashMap<_, _>>();
    let selected_image_id_set = selected_image_ids
        .unwrap_or(&[])
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let has_image_scope = selected_image_ids.is_some();

    images
        .into_iter()
        .filter_map(|mut image| {
            let browser_image = image_lookup.get(image.id.as_str())?;
            if has_image_scope && !selected_image_id_set.contains(image.id.as_str()) {
                return None;
            }
            if !selected_source_ids.is_empty() && !selected_source_ids.iter().any(|id| id == &browser_image.source_id) {
                return None;
            }
            if !selected_category_ids.is_empty() {
                image.annotations.retain(|annotation| {
                    selected_category_ids.iter().any(|id| id == &annotation.category_key)
                });
            }
            if image.annotations.is_empty() {
                return None;
            }
            Some(image)
        })
        .collect()
}

fn count_scoped_browser_images(
    browser_payload: &BrowserPayload,
    selected_image_ids: Option<&[String]>,
    selected_source_ids: &[String],
) -> u32 {
    let selected_image_id_set = selected_image_ids
        .unwrap_or(&[])
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let has_image_scope = selected_image_ids.is_some();

    browser_payload
        .images
        .iter()
        .filter(|image| {
            if has_image_scope {
                return selected_image_id_set.contains(image.id.as_str());
            }

            selected_source_ids.is_empty() || selected_source_ids.iter().any(|id| id == &image.source_id)
        })
        .count() as u32
}

fn collect_export_conflicts(
    images: &[ExportImageRecord],
    browser_payload: &BrowserPayload,
) -> Vec<crate::models::ExportFilenameConflict> {
    let source_lookup = browser_payload
        .images
        .iter()
        .map(|image| (image.id.as_str(), image.source_id.clone()))
        .collect::<HashMap<_, _>>();
    let mut grouped = BTreeMap::<String, Vec<crate::models::ExportConflictItem>>::new();

    for image in images {
        grouped.entry(image.file_name.clone()).or_default().push(crate::models::ExportConflictItem {
            image_id: image.id.clone(),
            source_id: source_lookup.get(image.id.as_str()).cloned().unwrap_or_default(),
            original_path: image.original_path.clone(),
        });
    }

    grouped
        .into_iter()
        .filter(|(_, items)| items.len() > 1)
        .map(|(file_name, items)| crate::models::ExportFilenameConflict { file_name, items })
        .collect()
}

fn build_export_preview(
    images: Vec<ExportImageRecord>,
    browser_payload: &BrowserPayload,
    scoped_image_count: u32,
    output_path: String,
) -> crate::models::ExportPreview {
    let included_images = images.len() as u32;
    let included_boxes = images.iter().map(|image| image.annotations.len() as u32).sum();
    let filename_conflicts = collect_export_conflicts(&images, browser_payload);
    let train = ((included_images as f64) * 0.70).floor() as u32;
    let valid = ((included_images as f64) * 0.15).floor() as u32;
    let test = included_images.saturating_sub(train).saturating_sub(valid);
    let category_count = images
        .iter()
        .flat_map(|image| image.annotations.iter().map(|annotation| annotation.category_key.as_str()))
        .collect::<HashSet<_>>()
        .len() as u32;

    crate::models::ExportPreview {
        category_count,
        included_images,
        excluded_images: scoped_image_count.saturating_sub(included_images),
        included_boxes,
        filename_conflicts: filename_conflicts.len() as u32,
        conflict_details: filename_conflicts,
        split_counts: crate::models::SplitCounts { train, valid, test },
        output_path,
    }
}

fn collect_export_categories(
    train_images: &[ExportImageRecord],
    valid_images: &[ExportImageRecord],
    test_images: &[ExportImageRecord],
) -> Vec<(String, String)> {
    let mut category_map = BTreeMap::<String, String>::new();
    for image in train_images
        .iter()
        .chain(valid_images.iter())
        .chain(test_images.iter())
    {
        for annotation in &image.annotations {
            category_map
                .entry(annotation.category_key.clone())
                .or_insert_with(|| annotation.category_name.clone());
        }
    }

    category_map.into_iter().collect()
}

fn ensure_unique_for_export(file_name: &str, used_names: &mut HashSet<String>) -> String {
    if used_names.insert(file_name.to_string()) {
        return file_name.to_string();
    }

    let path = Path::new(file_name);
    let stem = path.file_stem().and_then(|value| value.to_str()).unwrap_or("image");
    let ext = path.extension().and_then(|value| value.to_str()).unwrap_or("");

    let mut index = 1_u32;
    loop {
        let candidate = if ext.is_empty() {
            format!("{stem}-{index}")
        } else {
            format!("{stem}-{index}.{ext}")
        };

        if used_names.insert(candidate.clone()) {
            return candidate;
        }
        index = index.saturating_add(1);
    }
}

fn resolve_export_dimensions(image: &ExportImageRecord) -> Result<(u32, u32), String> {
    if let (Some(width), Some(height)) = (image.width, image.height) {
        return Ok((width, height));
    }

    probe_image_dimensions_for_path(Path::new(&image.original_path))
}

fn resolve_workspace_creation_root(name: &str, parent_path: &str) -> Result<PathBuf, String> {
    validate_workspace_name(name)?;

    let parent = PathBuf::from(parent_path.trim());
    if !parent.exists() {
        return Err("workspace parent folder does not exist".into());
    }
    if !parent.is_dir() {
        return Err("selected workspace parent path is not a directory".into());
    }

    Ok(parent.join(name))
}

fn validate_workspace_name(name: &str) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("workspace name is required".into());
    }
    if trimmed.chars().any(|ch| matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || ch.is_control()) {
        return Err("workspace name contains characters that are not allowed on Windows".into());
    }
    if trimmed.ends_with(' ') || trimmed.ends_with('.') {
        return Err("workspace name cannot end with a space or period on Windows".into());
    }

    let upper = trimmed.to_ascii_uppercase();
    let reserved = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
        "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    if reserved.iter().any(|item| *item == upper) {
        return Err("workspace name uses a reserved Windows device name".into());
    }

    Ok(())
}

fn inspect_workspace_creation_target(root: &Path, hidden_dir: &Path) -> Result<&'static str, String> {
    if hidden_dir.exists() {
        return Ok("existing-workspace");
    }

    if root.exists() {
        if !root.is_dir() {
            return Err("target workspace path exists but is not a directory".into());
        }

        return if is_directory_empty(root)? {
            Ok("existing-empty")
        } else {
            Ok("existing-nonempty")
        };
    }

    Ok("available")
}

fn validate_workspace_creation_target(root: &Path, hidden_dir: &Path, allow_existing_target: bool) -> Result<(), String> {
    match inspect_workspace_creation_target(root, hidden_dir)? {
        "available" => Ok(()),
        "existing-empty" if allow_existing_target => Ok(()),
        "existing-empty" => Err("target folder already exists and is empty; confirm before initializing the workspace there".into()),
        "existing-workspace" => Err("target folder is already a DataViewer workspace".into()),
        "existing-nonempty" => Err("target folder already exists and is not empty".into()),
        _ => Err("workspace target status could not be determined".into()),
    }
}

fn is_directory_empty(root: &Path) -> Result<bool, String> {
    let mut entries = fs::read_dir(root)
        .map_err(|error| format!("failed to inspect target workspace folder: {error}"))?;
    Ok(entries.next().is_none())
}

fn create_workspace_directories(paths: &crate::paths::WorkspacePaths) -> Result<(), String> {
    fs::create_dir_all(&paths.hidden_dir)
        .map_err(|error| format!("failed to create workspace hidden directory: {error}"))?;
    fs::create_dir_all(&paths.cache_dir)
        .map_err(|error| format!("failed to create cache directory: {error}"))?;
    fs::create_dir_all(&paths.temp_dir)
        .map_err(|error| format!("failed to create temp directory: {error}"))?;
    fs::create_dir_all(&paths.exports_dir)
        .map_err(|error| format!("failed to create exports directory: {error}"))?;

    Ok(())
}

fn ensure_workspace_exists(root: &Path, manifest_path: &Path, db_path: &Path) -> Result<(), String> {
    if !root.exists() {
        return Err("workspace folder does not exist".into());
    }
    if !manifest_path.exists() {
        return Err("workspace manifest file is missing".into());
    }
    if !db_path.exists() {
        return Err("workspace database file is missing".into());
    }

    Ok(())
}

fn write_manifest(manifest_path: &Path, manifest: &WorkspaceManifest) -> Result<(), String> {
    let content = serde_json::to_string_pretty(manifest)
        .map_err(|error| format!("failed to serialize workspace manifest: {error}"))?;

    fs::write(manifest_path, content)
        .map_err(|error| format!("failed to write workspace manifest: {error}"))
}

fn read_manifest(manifest_path: &Path) -> Result<WorkspaceManifest, String> {
    let content = fs::read_to_string(manifest_path)
        .map_err(|error| format!("failed to read workspace manifest: {error}"))?;

    serde_json::from_str(&content)
        .map_err(|error| format!("failed to parse workspace manifest: {error}"))
}

fn load_recent_workspaces_file() -> Result<Vec<RecentWorkspace>, String> {
    let path = recent_workspaces_path()?;

    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(path)
        .map_err(|error| format!("failed to read recent workspaces file: {error}"))?;

    serde_json::from_str(&content)
        .map_err(|error| format!("failed to parse recent workspaces file: {error}"))
}

fn upsert_recent_workspace(overview: &WorkspaceOverview) -> Result<(), String> {
    let path = recent_workspaces_path()?;
    let mut items = load_recent_workspaces_file().unwrap_or_default();
    let now = Utc::now().to_rfc3339();

    let next_item = RecentWorkspace {
        id: overview.id.clone(),
        name: overview.name.clone(),
        workspace_path: overview.workspace_path.clone(),
        health_status: "healthy".into(),
        last_opened_at: Some(now),
        available: true,
    };

    items.retain(|item| item.id != overview.id);
    items.push(next_item);
    items.sort_by(|a, b| b.last_opened_at.cmp(&a.last_opened_at));

    write_recent_workspaces_file(&path, &items)
}

fn write_recent_workspaces_file(path: &Path, items: &[RecentWorkspace]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "failed to resolve recent workspaces parent directory".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create recent workspaces directory: {error}"))?;

    let content = serde_json::to_string_pretty(items)
        .map_err(|error| format!("failed to serialize recent workspaces file: {error}"))?;

    fs::write(path, content)
        .map_err(|error| format!("failed to write recent workspaces file: {error}"))
}

fn normalize_path_string(path: &Path) -> String {
    path.to_string_lossy().replace('/', "\\")
}


fn parse_json_number(value: &serde_json::Value) -> Option<f64> {
    if let Some(number) = value.as_f64() {
        return Some(number);
    }

    value.as_str()?.trim().parse::<f64>().ok()
}
fn slugify_workspace_name(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn make_temp_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("dataviewer-{name}-{unique}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn compute_split_counts_handles_zero_total() {
        let split = compute_split_counts(0, 70, 15, 15);
        assert_eq!(split.train, 0);
        assert_eq!(split.valid, 0);
        assert_eq!(split.test, 0);
    }

    #[test]
    fn compute_split_counts_preserves_total_count() {
        let split = compute_split_counts(17, 70, 15, 15);
        assert_eq!(split.train + split.valid + split.test, 17);
        assert_eq!(split.train, 11);
        assert_eq!(split.valid, 2);
        assert_eq!(split.test, 4);
    }

    #[test]
    fn resolve_yolo_label_path_supports_split_layout() {
        let root = make_temp_dir("yolo-layout");
        let images_root = root.join("images");
        let labels_root = root.join("labels");
        let split_images = root.join("train").join("images");
        let split_labels = root.join("train").join("labels");
        fs::create_dir_all(&images_root).unwrap();
        fs::create_dir_all(&labels_root).unwrap();
        fs::create_dir_all(&split_images).unwrap();
        fs::create_dir_all(&split_labels).unwrap();

        let image_path = split_images.join("sample.jpg");
        let label_path = split_labels.join("sample.txt");
        fs::write(&image_path, b"jpg").unwrap();
        fs::write(&label_path, b"0 0.5 0.5 0.25 0.25").unwrap();

        let resolved = resolve_yolo_label_path(&root, &images_root, &labels_root, &image_path);
        assert_eq!(resolved, label_path);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn detect_source_type_identifies_common_layouts() {
        let raw_root = make_temp_dir("raw-source");
        fs::write(raw_root.join("image.jpg"), b"jpg").unwrap();
        assert_eq!(detect_source_type(&raw_root), "RAW");

        let yolo_root = make_temp_dir("yolo-source");
        fs::create_dir_all(yolo_root.join("images")).unwrap();
        fs::create_dir_all(yolo_root.join("labels")).unwrap();
        fs::write(yolo_root.join("data.yaml"), b"names: [helmet]").unwrap();
        assert_eq!(detect_source_type(&yolo_root), "YOLO");

        let coco_root = make_temp_dir("coco-source");
        fs::create_dir_all(coco_root.join("annotations")).unwrap();
        fs::write(
            coco_root.join("annotations").join("instances.json"),
            r#"{"images":[],"annotations":[],"categories":[]}"#,
        )
        .unwrap();
        assert_eq!(detect_source_type(&coco_root), "COCO");

        let _ = fs::remove_dir_all(raw_root);
        let _ = fs::remove_dir_all(yolo_root);
        let _ = fs::remove_dir_all(coco_root);
    }

    #[test]
    fn parse_json_number_accepts_numeric_strings() {
        assert_eq!(parse_json_number(&serde_json::json!(444.0)), Some(444.0));
        assert_eq!(parse_json_number(&serde_json::json!("444.00")), Some(444.0));
        assert_eq!(parse_json_number(&serde_json::json!(" 508.00 ")), Some(508.0));
        assert_eq!(parse_json_number(&serde_json::json!("abc")), None);
    }

    #[test]
    fn export_yolo_split_writes_label_files() {
        let root = make_temp_dir("yolo-export");
        let images_dir = root.join("train").join("images");
        let labels_dir = root.join("train").join("labels");
        fs::create_dir_all(&images_dir).unwrap();
        fs::create_dir_all(&labels_dir).unwrap();

        let source_image = root.join("source.jpg");
        fs::write(&source_image, b"jpg").unwrap();
        let images = vec![ExportImageRecord {
            id: "img-1".into(),
            file_name: "source.jpg".into(),
            original_path: normalize_path_string(&source_image),
            width: Some(100),
            height: Some(80),
            annotations: vec![crate::models::ExportAnnotationRecord {
                category_key: "helmet".into(),
                category_name: "helmet".into(),
                annotation_format: "coco".into(),
                bbox_x: 10.0,
                bbox_y: 20.0,
                bbox_width: 30.0,
                bbox_height: 16.0,
            }],
        }];
        let categories = vec![("helmet".to_string(), "helmet".to_string())];

        let exported_boxes = export_yolo_split(&images, &images_dir, &labels_dir, &categories).unwrap();
        assert_eq!(exported_boxes, 1);

        let label = fs::read_to_string(labels_dir.join("source.txt")).unwrap();
        assert_eq!(label.trim(), "0 0.250000 0.350000 0.300000 0.200000");

        let _ = fs::remove_dir_all(root);
    }
}

















