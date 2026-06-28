use crate::models::{
    AnnotationVersion, BoundingBoxRecord, BoxSummary, BrowserPayload, CvatTask, ExportPreview, ImageCard,
    ImageDetailPayload, ImportReviewRow, RecentWorkspace, SourceFolder, SplitCounts,
    UnifiedCategory, WorkspaceOverview,
};

pub fn recent_workspaces() -> Vec<RecentWorkspace> {
    vec![
        RecentWorkspace {
            id: "factory-defect-v1".into(),
            name: "factory-defect-v1".into(),
            workspace_path: "C:\\Workspaces\\factory-defect-v1".into(),
            health_status: "warning".into(),
            last_opened_at: Some("2026-03-31T12:30:00Z".into()),
            available: true,
        },
        RecentWorkspace {
            id: "retail-products-v2".into(),
            name: "retail-products-v2".into(),
            workspace_path: "D:\\Workspaces\\retail-products-v2".into(),
            health_status: "healthy".into(),
            last_opened_at: Some("2026-03-30T09:20:00Z".into()),
            available: true,
        },
    ]
}

pub fn workspace_overview() -> WorkspaceOverview {
    WorkspaceOverview {
        id: "factory-defect-v1".into(),
        name: "factory-defect-v1".into(),
        workspace_path: "C:\\Workspaces\\factory-defect-v1".into(),
        health_status: "warning".into(),
        sources: source_folders(),
        categories: unified_categories(),
    }
}

pub fn source_folders() -> Vec<SourceFolder> {
    vec![
        SourceFolder {
            id: "source-coco-old".into(),
            name: "coco_old".into(),
            path: "D:\\datasets\\coco_old".into(),
            r#type: "COCO".into(),
            status: "ready".into(),
            image_count: 1450,
            category_count: 6,
            corrupted_image_count: 0,
            corrupted_image_paths: vec![],
            last_scan_at: "2026-03-31 10:30".into(),
        },
        SourceFolder {
            id: "source-yolo-batch-2".into(),
            name: "yolo_batch_2".into(),
            path: "D:\\datasets\\yolo_batch_2".into(),
            r#type: "YOLO".into(),
            status: "warning".into(),
            image_count: 980,
            category_count: 4,
            corrupted_image_count: 2,
            corrupted_image_paths: vec![
                "D:\\datasets\\yolo_batch_2\\images\\broken_001.jpg".into(),
                "D:\\datasets\\yolo_batch_2\\images\\broken_009.jpg".into(),
            ],
            last_scan_at: "2026-03-31 10:35".into(),
        },
        SourceFolder {
            id: "source-raw-2026-03".into(),
            name: "raw_2026_03".into(),
            path: "D:\\datasets\\raw_2026_03".into(),
            r#type: "RAW".into(),
            status: "warning".into(),
            image_count: 620,
            category_count: 0,
            corrupted_image_count: 1,
            corrupted_image_paths: vec![
                "D:\\datasets\\raw_2026_03\\cameraA\\broken_014.jpg".into(),
            ],
            last_scan_at: "2026-03-31 10:40".into(),
        },
    ]
}

pub fn unified_categories() -> Vec<UnifiedCategory> {
    vec![
        UnifiedCategory { id: "cat-screw".into(), name: "screw".into(), image_count: 1240 },
        UnifiedCategory { id: "cat-nut".into(), name: "nut".into(), image_count: 830 },
        UnifiedCategory { id: "cat-washer".into(), name: "washer".into(), image_count: 412 },
        UnifiedCategory { id: "cat-pallet".into(), name: "pallet".into(), image_count: 207 },
    ]
}

pub fn import_review_rows() -> Vec<ImportReviewRow> {
    vec![
        ImportReviewRow {
            source_category_id: "source-cat-car".into(),
            source_category: "car".into(),
            source_path: "D:\\datasets\\coco_old".into(),
            count: 540,
            source_total_image_count: 1450,
            suggested_action: "same-name candidate".into(),
            target_unified_category: Some("car".into()),
            final_action: "Merge".into(),
        },
        ImportReviewRow {
            source_category_id: "source-cat-cars".into(),
            source_category: "cars".into(),
            source_path: "D:\\datasets\\coco_old".into(),
            count: 112,
            source_total_image_count: 1450,
            suggested_action: "similar-name".into(),
            target_unified_category: Some("car".into()),
            final_action: "Merge".into(),
        },
        ImportReviewRow {
            source_category_id: "source-cat-background-obj".into(),
            source_category: "background_obj".into(),
            source_path: "D:\\datasets\\coco_old".into(),
            count: 31,
            source_total_image_count: 1450,
            suggested_action: "no match".into(),
            target_unified_category: None,
            final_action: "Ignore".into(),
        },
    ]
}

pub fn browser_payload() -> BrowserPayload {
    BrowserPayload {
        sources: source_folders(),
        categories: unified_categories(),
        images: vec![
            ImageCard {
                id: "img-001".into(),
                filename: "frame_0001.jpg".into(),
                source_id: "source-coco-old".into(),
                source_name: "coco_old".into(),
                original_path: r"D:\Datasets\coco_old\images\frame_0001.jpg".into(),
                annotation_status: "annotated".into(),
                image_health_status: "healthy".into(),
                image_health_error: None,
                annotation_count: 2,
                max_box_area_ratio: Some(0.118),
                box_summaries: vec![
                    BoxSummary { category_name: "screw".into(), area_ratio: Some(0.057) },
                    BoxSummary { category_name: "nut".into(), area_ratio: Some(0.118) },
                ],
                category_ids: vec!["cat-screw".into(), "cat-nut".into()],
                categories: vec!["screw".into(), "nut".into()],
            },
            ImageCard {
                id: "img-002".into(),
                filename: "frame_0002.jpg".into(),
                source_id: "source-yolo-batch-2".into(),
                source_name: "yolo_batch_2".into(),
                original_path: r"D:\Datasets\yolo_batch_2\images\frame_0002.jpg".into(),
                annotation_status: "annotated".into(),
                image_health_status: "healthy".into(),
                image_health_error: None,
                annotation_count: 1,
                max_box_area_ratio: Some(0.084),
                box_summaries: vec![
                    BoxSummary { category_name: "screw".into(), area_ratio: Some(0.084) },
                ],
                category_ids: vec!["cat-screw".into()],
                categories: vec!["screw".into()],
            },
            ImageCard {
                id: "img-003".into(),
                filename: "frame_0003.jpg".into(),
                source_id: "source-raw-2026-03".into(),
                source_name: "raw_2026_03".into(),
                original_path: r"D:\Datasets\raw_2026_03\cameraA\frame_0003.jpg".into(),
                annotation_status: "unannotated".into(),
                image_health_status: "healthy".into(),
                image_health_error: None,
                annotation_count: 0,
                max_box_area_ratio: None,
                box_summaries: vec![],
                category_ids: vec![],
                categories: vec![],
            },
        ],
    }
}

pub fn image_detail_payload() -> ImageDetailPayload {
    ImageDetailPayload {
        id: "img-001".into(),
        filename: "frame_0001.jpg".into(),
        source_id: "source-coco-old".into(),
        source_name: "coco_old".into(),
        original_path: "D:\\datasets\\coco_old\\images\\frame_0001.jpg".into(),
        annotation_status: "annotated".into(),
        image_health_status: "healthy".into(),
        image_health_error: None,
        categories: vec!["screw".into(), "nut".into()],
        width: Some(1280),
        height: Some(720),
        boxes: vec![
            BoundingBoxRecord {
                id: "box-001".into(),
                category_name: "screw".into(),
                annotation_format: "coco".into(),
                bbox_x: 180.0,
                bbox_y: 140.0,
                bbox_width: 240.0,
                bbox_height: 220.0,
            },
            BoundingBoxRecord {
                id: "box-002".into(),
                category_name: "nut".into(),
                annotation_format: "coco".into(),
                bbox_x: 700.0,
                bbox_y: 260.0,
                bbox_width: 180.0,
                bbox_height: 160.0,
            },
        ],
    }
}

pub fn cvat_tasks() -> Vec<CvatTask> {
    vec![
        CvatTask {
            id: "cvat-task-001".into(),
            name: "raw_2026_03_batch_01".into(),
            image_count: 120,
            status: "In Progress".into(),
            project_name: "defect_v1".into(),
            last_sync_at: None,
            temp_folder: None,
            remote_task_id: Some(101),
            remote_url: Some("http://localhost:8080/tasks/101".into()),
        },
        CvatTask {
            id: "cvat-task-002".into(),
            name: "raw_2026_03_batch_02".into(),
            image_count: 80,
            status: "Ready Sync".into(),
            project_name: "defect_v1".into(),
            last_sync_at: Some("2026-03-31 11:10".into()),
            temp_folder: None,
            remote_task_id: Some(102),
            remote_url: Some("http://localhost:8080/tasks/102".into()),
        },
    ]
}

pub fn annotation_versions() -> Vec<AnnotationVersion> {
    vec![
        AnnotationVersion {
            id: "version-3".into(),
            label: "v3".into(),
            created_at: "2026-03-31 11:25".into(),
            source_task: "raw_2026_03_batch_02".into(),
            image_count: 80,
            box_count: 412,
            notes: "sync from CVAT".into(),
        },
        AnnotationVersion {
            id: "version-2".into(),
            label: "v2".into(),
            created_at: "2026-03-31 10:55".into(),
            source_task: "raw_2026_03_batch_01".into(),
            image_count: 120,
            box_count: 618,
            notes: "sync from CVAT".into(),
        },
    ]
}

pub fn export_preview() -> ExportPreview {
    ExportPreview {
        category_count: 3,
        included_images: 1984,
        excluded_images: 231,
        included_boxes: 8451,
        dataset_map_excluded_images: 12,
        dataset_map_excluded_boxes: 37,
        filename_conflicts: 4,
        conflict_details: vec![crate::models::ExportFilenameConflict {
            file_name: "0001.jpg".into(),
            items: vec![
                crate::models::ExportConflictItem {
                    image_id: "img-1".into(),
                    source_id: "source-a".into(),
                    original_path: "D:\\datasets\\set-a\\0001.jpg".into(),
                },
                crate::models::ExportConflictItem {
                    image_id: "img-2".into(),
                    source_id: "source-b".into(),
                    original_path: "D:\\datasets\\set-b\\0001.jpg".into(),
                },
            ],
        }],
        split_counts: SplitCounts {
            train: 1388,
            valid: 298,
            test: 298,
        },
        output_path: "D:\\exports\\factory-defect-v1-coco".into(),
    }
}
