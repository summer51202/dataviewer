use std::collections::BTreeMap;
use std::path::Path;

use rusqlite::{params, Connection};

use crate::models::{
    AnnotationVersion, BoundingBoxRecord, BoxSummary, BrowserPayload, DatasetMapBbox,
    DatasetMapPoint, ExportAnnotationRecord, ExportConflictItem,
    ExportFilenameConflict, ExportHistoryEntry, ExportImageRecord, ImageCard,
    ImageDetailPayload, ImportReviewRow, SourceFolder, StoredAnnotationRecord,
    StoredCategoryRecord, StoredImageRecord, StoredSourceFolder, UnifiedCategory,
    WorkspaceOverview,
};
use crate::paths::APP_VERSION;

pub struct DatasetMapProjectionTarget {
    pub target_id: String,
}

pub struct EmbeddingProjectionRow {
    pub id: String,
    pub workspace_id: String,
    pub scope: String,
    pub target_id: String,
    pub model_id: String,
    pub projection_method: String,
    pub x: f64,
    pub y: f64,
    pub created_at: String,
}

const INIT_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS workspace_meta (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    workspace_path TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    app_version TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS source_folders (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    path TEXT NOT NULL,
    source_type TEXT NOT NULL,
    status TEXT NOT NULL,
    last_scan_at TEXT,
    image_count INTEGER NOT NULL DEFAULT 0,
    category_count INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY(workspace_id) REFERENCES workspace_meta(id)
);

CREATE TABLE IF NOT EXISTS images (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    source_id TEXT NOT NULL,
    file_name TEXT NOT NULL,
    original_path TEXT NOT NULL,
    relative_path TEXT,
    width INTEGER,
    height INTEGER,
    annotation_status TEXT NOT NULL DEFAULT 'unannotated',
    health_status TEXT NOT NULL DEFAULT 'healthy',
    health_error TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY(workspace_id) REFERENCES workspace_meta(id),
    FOREIGN KEY(source_id) REFERENCES source_folders(id)
);

CREATE TABLE IF NOT EXISTS categories (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    source_id TEXT,
    name TEXT NOT NULL,
    normalized_name TEXT,
    category_role TEXT NOT NULL DEFAULT 'source',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY(workspace_id) REFERENCES workspace_meta(id),
    FOREIGN KEY(source_id) REFERENCES source_folders(id)
);

CREATE TABLE IF NOT EXISTS source_category_mappings (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    source_id TEXT NOT NULL,
    source_category_id TEXT NOT NULL,
    target_category_id TEXT,
    final_action TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY(workspace_id) REFERENCES workspace_meta(id),
    FOREIGN KEY(source_id) REFERENCES source_folders(id),
    FOREIGN KEY(source_category_id) REFERENCES categories(id),
    FOREIGN KEY(target_category_id) REFERENCES categories(id)
);

CREATE TABLE IF NOT EXISTS annotations (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    image_id TEXT NOT NULL,
    source_id TEXT NOT NULL,
    source_category_id TEXT,
    category_id TEXT,
    annotation_version_id TEXT,
    bbox_x REAL,
    bbox_y REAL,
    bbox_width REAL,
    bbox_height REAL,
    annotation_format TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY(workspace_id) REFERENCES workspace_meta(id),
    FOREIGN KEY(image_id) REFERENCES images(id),
    FOREIGN KEY(source_id) REFERENCES source_folders(id),
    FOREIGN KEY(source_category_id) REFERENCES categories(id),
    FOREIGN KEY(category_id) REFERENCES categories(id),
    FOREIGN KEY(annotation_version_id) REFERENCES annotation_versions(id)
);

CREATE TABLE IF NOT EXISTS annotation_versions (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    label TEXT NOT NULL,
    created_at TEXT NOT NULL,
    source_task TEXT,
    notes TEXT,
    FOREIGN KEY(workspace_id) REFERENCES workspace_meta(id)
);

CREATE TABLE IF NOT EXISTS export_jobs (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    output_format TEXT NOT NULL,
    output_path TEXT NOT NULL,
    created_at TEXT NOT NULL,
    status TEXT NOT NULL,
    FOREIGN KEY(workspace_id) REFERENCES workspace_meta(id)
);

CREATE TABLE IF NOT EXISTS cvat_tasks (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    name TEXT NOT NULL,
    image_count INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL,
    project_name TEXT NOT NULL,
    temp_folder TEXT,
    last_sync_at TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY(workspace_id) REFERENCES workspace_meta(id)
);

CREATE TABLE IF NOT EXISTS embedding_models (
    id TEXT PRIMARY KEY,
    family TEXT NOT NULL,
    model_id TEXT NOT NULL,
    display_name TEXT NOT NULL,
    embedding_dim INTEGER NOT NULL,
    input_size INTEGER NOT NULL,
    preprocess_version TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS embedding_jobs (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    scope TEXT NOT NULL,
    model_id TEXT NOT NULL,
    runtime_preference TEXT NOT NULL,
    runtime_backend TEXT,
    status TEXT NOT NULL,
    processed_items INTEGER NOT NULL DEFAULT 0,
    total_items INTEGER NOT NULL DEFAULT 0,
    message TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY(workspace_id) REFERENCES workspace_meta(id)
);

CREATE TABLE IF NOT EXISTS embeddings (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    scope TEXT NOT NULL,
    target_id TEXT NOT NULL,
    image_id TEXT NOT NULL,
    annotation_id TEXT,
    model_id TEXT NOT NULL,
    runtime_backend TEXT NOT NULL,
    vector BLOB NOT NULL,
    vector_norm REAL,
    created_at TEXT NOT NULL,
    UNIQUE(workspace_id, scope, target_id, model_id),
    FOREIGN KEY(workspace_id) REFERENCES workspace_meta(id),
    FOREIGN KEY(image_id) REFERENCES images(id),
    FOREIGN KEY(annotation_id) REFERENCES annotations(id)
);

CREATE TABLE IF NOT EXISTS embedding_projections (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    scope TEXT NOT NULL,
    target_id TEXT NOT NULL,
    model_id TEXT NOT NULL,
    projection_method TEXT NOT NULL,
    x REAL NOT NULL,
    y REAL NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(workspace_id, scope, target_id, model_id, projection_method),
    FOREIGN KEY(workspace_id) REFERENCES workspace_meta(id)
);

CREATE TABLE IF NOT EXISTS dataset_review_marks (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    scope TEXT NOT NULL,
    target_id TEXT NOT NULL,
    status TEXT NOT NULL,
    reason TEXT,
    note TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(workspace_id, scope, target_id),
    FOREIGN KEY(workspace_id) REFERENCES workspace_meta(id)
);

CREATE INDEX IF NOT EXISTS idx_source_folders_workspace_id ON source_folders(workspace_id);
CREATE INDEX IF NOT EXISTS idx_images_source_id ON images(source_id);
CREATE INDEX IF NOT EXISTS idx_images_workspace_id ON images(workspace_id);
CREATE INDEX IF NOT EXISTS idx_categories_source_id ON categories(source_id);
CREATE INDEX IF NOT EXISTS idx_categories_workspace_role ON categories(workspace_id, category_role);
CREATE INDEX IF NOT EXISTS idx_source_category_mappings_source_id ON source_category_mappings(source_id);
CREATE INDEX IF NOT EXISTS idx_source_category_mappings_workspace_id ON source_category_mappings(workspace_id);
CREATE INDEX IF NOT EXISTS idx_annotations_source_id ON annotations(source_id);
CREATE INDEX IF NOT EXISTS idx_annotations_image_id ON annotations(image_id);
CREATE INDEX IF NOT EXISTS idx_annotations_workspace_id ON annotations(workspace_id);
CREATE INDEX IF NOT EXISTS idx_export_jobs_workspace_id ON export_jobs(workspace_id);
CREATE INDEX IF NOT EXISTS idx_cvat_tasks_workspace_id ON cvat_tasks(workspace_id);
CREATE INDEX IF NOT EXISTS idx_embeddings_workspace_scope_model ON embeddings(workspace_id, scope, model_id);
CREATE INDEX IF NOT EXISTS idx_embedding_projections_workspace_scope_model ON embedding_projections(workspace_id, scope, model_id);
CREATE INDEX IF NOT EXISTS idx_dataset_review_marks_workspace_scope ON dataset_review_marks(workspace_id, scope);
"#;

pub fn initialize_database(db_path: &Path) -> Result<(), String> {
    let connection = Connection::open(db_path)
        .map_err(|error| format!("failed to open workspace database: {error}"))?;

    connection
        .execute_batch(INIT_SQL)
        .map_err(|error| format!("failed to initialize workspace database schema: {error}"))?;

    ensure_source_folder_column(&connection, "image_count", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_source_folder_column(&connection, "category_count", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_image_column(&connection, "health_status", "TEXT NOT NULL DEFAULT 'healthy'")?;
    ensure_image_column(&connection, "health_error", "TEXT")?;
    ensure_cvat_task_column(&connection, "remote_task_id", "INTEGER")?;
    ensure_cvat_task_column(&connection, "remote_url", "TEXT")?;
    ensure_export_jobs_column(&connection, "exported_images", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_export_jobs_column(&connection, "exported_boxes", "INTEGER NOT NULL DEFAULT 0")?;

    Ok(())
}

pub fn upsert_workspace_meta(
    db_path: &Path,
    workspace_id: &str,
    name: &str,
    workspace_path: &str,
    created_at: &str,
    updated_at: &str,
) -> Result<(), String> {
    let connection = Connection::open(db_path)
        .map_err(|error| format!("failed to open workspace database: {error}"))?;

    connection
        .execute(
            r#"
            INSERT INTO workspace_meta (id, name, workspace_path, created_at, updated_at, app_version)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                workspace_path = excluded.workspace_path,
                updated_at = excluded.updated_at,
                app_version = excluded.app_version
            "#,
            params![
                workspace_id,
                name,
                workspace_path,
                created_at,
                updated_at,
                APP_VERSION
            ],
        )
        .map_err(|error| format!("failed to upsert workspace metadata: {error}"))?;

    Ok(())
}

pub fn insert_source_folder(
    db_path: &Path,
    source_id: &str,
    workspace_id: &str,
    source_path: &str,
    source_type: &str,
    status: &str,
    last_scan_at: Option<&str>,
) -> Result<(), String> {
    let connection = Connection::open(db_path)
        .map_err(|error| format!("failed to open workspace database: {error}"))?;

    let existing_count: i64 = connection
        .query_row(
            "SELECT COUNT(1) FROM source_folders WHERE workspace_id = ?1 AND path = ?2",
            params![workspace_id, source_path],
            |row| row.get(0),
        )
        .map_err(|error| format!("failed to check existing source folder: {error}"))?;

    if existing_count > 0 {
        return Err("source folder already exists in this workspace".into());
    }

    connection
        .execute(
            r#"
            INSERT INTO source_folders (
                id, workspace_id, path, source_type, status, last_scan_at, image_count, category_count
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, 0)
            "#,
            params![source_id, workspace_id, source_path, source_type, status, last_scan_at],
        )
        .map_err(|error| format!("failed to insert source folder: {error}"))?;

    Ok(())
}

pub fn replace_source_images(
    db_path: &Path,
    source_id: &str,
    image_records: &[StoredImageRecord],
) -> Result<(), String> {
    let mut connection = Connection::open(db_path)
        .map_err(|error| format!("failed to open workspace database: {error}"))?;
    let transaction = connection
        .transaction()
        .map_err(|error| format!("failed to start image replace transaction: {error}"))?;

    transaction
        .execute("DELETE FROM annotations WHERE source_id = ?1", params![source_id])
        .map_err(|error| format!("failed to clear source annotations: {error}"))?;
    transaction
        .execute("DELETE FROM images WHERE source_id = ?1", params![source_id])
        .map_err(|error| format!("failed to clear source images: {error}"))?;

    for image in image_records {
        transaction
            .execute(
                r#"
                INSERT INTO images (
                    id, workspace_id, source_id, file_name, original_path, relative_path, width, height,
                    annotation_status, health_status, health_error, created_at, updated_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                "#,
                params![
                    image.id,
                    image.workspace_id,
                    image.source_id,
                    image.file_name,
                    image.original_path,
                    image.relative_path,
                    image.width,
                    image.height,
                    image.annotation_status,
                    image.health_status,
                    image.health_error,
                    image.created_at,
                    image.updated_at
                ],
            )
            .map_err(|error| format!("failed to insert image record: {error}"))?;
    }

    transaction
        .commit()
        .map_err(|error| format!("failed to commit image replace transaction: {error}"))?;

    Ok(())
}

pub fn replace_source_categories(
    db_path: &Path,
    source_id: &str,
    categories: &[StoredCategoryRecord],
) -> Result<(), String> {
    let mut connection = Connection::open(db_path)
        .map_err(|error| format!("failed to open workspace database: {error}"))?;
    let transaction = connection
        .transaction()
        .map_err(|error| format!("failed to start category replace transaction: {error}"))?;

    transaction
        .execute("DELETE FROM source_category_mappings WHERE source_id = ?1", params![source_id])
        .map_err(|error| format!("failed to clear source category mappings: {error}"))?;
    transaction
        .execute("DELETE FROM categories WHERE source_id = ?1", params![source_id])
        .map_err(|error| format!("failed to clear source categories: {error}"))?;

    for category in categories {
        transaction
            .execute(
                r#"
                INSERT INTO categories (
                    id, workspace_id, source_id, name, normalized_name, category_role, created_at, updated_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, 'source', ?6, ?7)
                "#,
                params![
                    category.id,
                    category.workspace_id,
                    category.source_id,
                    category.name,
                    category.normalized_name,
                    category.created_at,
                    category.updated_at
                ],
            )
            .map_err(|error| format!("failed to insert category record: {error}"))?;
    }

    transaction
        .commit()
        .map_err(|error| format!("failed to commit category replace transaction: {error}"))?;

    Ok(())
}

pub fn replace_source_annotations(
    db_path: &Path,
    source_id: &str,
    annotations: &[StoredAnnotationRecord],
) -> Result<(), String> {
    let mut connection = Connection::open(db_path)
        .map_err(|error| format!("failed to open workspace database: {error}"))?;
    let transaction = connection
        .transaction()
        .map_err(|error| format!("failed to start annotation replace transaction: {error}"))?;

    transaction
        .execute("DELETE FROM annotations WHERE source_id = ?1", params![source_id])
        .map_err(|error| format!("failed to clear source annotations: {error}"))?;

    for annotation in annotations {
        transaction
            .execute(
                r#"
                INSERT INTO annotations (
                    id, workspace_id, image_id, source_id, source_category_id, category_id,
                    annotation_version_id, bbox_x, bbox_y, bbox_width, bbox_height,
                    annotation_format, created_at, updated_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                "#,
                params![
                    annotation.id,
                    annotation.workspace_id,
                    annotation.image_id,
                    annotation.source_id,
                    annotation.source_category_id,
                    annotation.category_id,
                    annotation.bbox_x,
                    annotation.bbox_y,
                    annotation.bbox_width,
                    annotation.bbox_height,
                    annotation.annotation_format,
                    annotation.created_at,
                    annotation.updated_at
                ],
            )
            .map_err(|error| format!("failed to insert annotation record: {error}"))?;
    }

    transaction
        .commit()
        .map_err(|error| format!("failed to commit annotation replace transaction: {error}"))?;

    Ok(())
}

pub fn update_source_folder_scan(
    db_path: &Path,
    source_id: &str,
    source_type: &str,
    status: &str,
    last_scan_at: &str,
    image_count: u32,
    category_count: u32,
) -> Result<(), String> {
    let connection = Connection::open(db_path)
        .map_err(|error| format!("failed to open workspace database: {error}"))?;

    connection
        .execute(
            r#"
            UPDATE source_folders
            SET source_type = ?2,
                status = ?3,
                last_scan_at = ?4,
                image_count = ?5,
                category_count = ?6
            WHERE id = ?1
            "#,
            params![source_id, source_type, status, last_scan_at, image_count, category_count],
        )
        .map_err(|error| format!("failed to update source folder scan metadata: {error}"))?;

    Ok(())
}

pub fn delete_source_folder(db_path: &Path, source_id: &str) -> Result<(), String> {
    let mut connection = Connection::open(db_path)
        .map_err(|error| format!("failed to open workspace database: {error}"))?;

    let transaction = connection
        .transaction()
        .map_err(|error| format!("failed to start source delete transaction: {error}"))?;

    transaction
        .execute("DELETE FROM annotations WHERE source_id = ?1", params![source_id])
        .map_err(|error| format!("failed to remove source annotations: {error}"))?;
    transaction
        .execute("DELETE FROM source_category_mappings WHERE source_id = ?1", params![source_id])
        .map_err(|error| format!("failed to remove source category mappings: {error}"))?;
    transaction
        .execute("DELETE FROM categories WHERE source_id = ?1", params![source_id])
        .map_err(|error| format!("failed to remove source categories: {error}"))?;
    transaction
        .execute("DELETE FROM images WHERE source_id = ?1", params![source_id])
        .map_err(|error| format!("failed to remove source images: {error}"))?;
    transaction
        .execute("DELETE FROM source_folders WHERE id = ?1", params![source_id])
        .map_err(|error| format!("failed to remove source folder: {error}"))?;

    transaction
        .commit()
        .map_err(|error| format!("failed to commit source delete transaction: {error}"))?;

    Ok(())
}

pub fn read_workspace_overview(db_path: &Path) -> Result<WorkspaceOverview, String> {
    let connection = Connection::open(db_path)
        .map_err(|error| format!("failed to open workspace database: {error}"))?;

    let mut statement = connection
        .prepare(
            r#"
            SELECT id, name, workspace_path, updated_at
            FROM workspace_meta
            LIMIT 1
            "#,
        )
        .map_err(|error| format!("failed to prepare workspace overview query: {error}"))?;

    let mut overview = statement
        .query_row([], |row| {
            Ok(WorkspaceOverview {
                id: row.get(0)?,
                name: row.get(1)?,
                workspace_path: row.get(2)?,
                health_status: "healthy".to_string(),
                sources: Vec::new(),
                categories: Vec::new(),
            })
        })
        .map_err(|error| format!("failed to read workspace overview: {error}"))?;

    overview.sources = read_source_folders(db_path)?;
    overview.categories = read_browser_categories(db_path)?;
    Ok(overview)
}

pub fn read_browser_payload(db_path: &Path) -> Result<BrowserPayload, String> {
    Ok(BrowserPayload {
        sources: read_source_folders(db_path)?,
        categories: read_browser_categories(db_path)?,
        images: read_image_cards(db_path)?,
    })
}

pub fn read_dataset_map_points(
    db_path: &Path,
    workspace_id: &str,
    scope: &str,
    model_id: &str,
) -> Result<Vec<DatasetMapPoint>, String> {
    let connection = Connection::open(db_path)
        .map_err(|error| format!("failed to open workspace database: {error}"))?;
    let mut statement = connection
        .prepare(
            r#"
            SELECT
                p.target_id,
                p.scope,
                i.id,
                i.file_name,
                i.source_id,
                COALESCE(sf.path, ''),
                a.id,
                COALESCE(a.category_id, a.source_category_id),
                COALESCE(uc.name, sc.name),
                a.bbox_x,
                a.bbox_y,
                a.bbox_width,
                a.bbox_height,
                i.width,
                i.height,
                p.x,
                p.y,
                COALESCE(r.status, 'unreviewed')
            FROM embedding_projections p
            LEFT JOIN annotations a
                ON p.scope = 'object'
                AND a.id = p.target_id
                AND a.workspace_id = p.workspace_id
            LEFT JOIN images i
                ON i.workspace_id = p.workspace_id
                AND (
                    (p.scope = 'image' AND i.id = p.target_id)
                    OR (p.scope = 'object' AND i.id = a.image_id)
                )
            LEFT JOIN source_folders sf ON sf.id = i.source_id
            LEFT JOIN categories uc ON uc.id = a.category_id
            LEFT JOIN categories sc ON sc.id = a.source_category_id
            LEFT JOIN dataset_review_marks r
                ON r.workspace_id = p.workspace_id
                AND r.scope = p.scope
                AND r.target_id = p.target_id
            WHERE p.workspace_id = ?1
                AND p.scope = ?2
                AND p.model_id = ?3
                AND i.id IS NOT NULL
            ORDER BY i.file_name ASC, p.target_id ASC
            "#,
        )
        .map_err(|error| format!("failed to prepare dataset map points query: {error}"))?;

    let rows = statement
        .query_map(params![workspace_id, scope, model_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, Option<f64>>(9)?,
                row.get::<_, Option<f64>>(10)?,
                row.get::<_, Option<f64>>(11)?,
                row.get::<_, Option<f64>>(12)?,
                row.get::<_, Option<u32>>(13)?,
                row.get::<_, Option<u32>>(14)?,
                row.get::<_, f64>(15)?,
                row.get::<_, f64>(16)?,
                row.get::<_, String>(17)?,
            ))
        })
        .map_err(|error| format!("failed to read dataset map point rows: {error}"))?;

    let mut points = Vec::new();
    for row in rows {
        let (
            target_id,
            scope,
            image_id,
            filename,
            source_id,
            source_path,
            annotation_id,
            category_id,
            category_name,
            bbox_x,
            bbox_y,
            bbox_width,
            bbox_height,
            image_width,
            image_height,
            x,
            y,
            review_status,
        ) = row.map_err(|error| format!("failed to map dataset map point row: {error}"))?;

        let bbox = match (bbox_x, bbox_y, bbox_width, bbox_height) {
            (Some(x), Some(y), Some(width), Some(height)) => {
                let area_ratio = image_width
                    .zip(image_height)
                    .and_then(|(image_width, image_height)| {
                        let image_area = (image_width as f64) * (image_height as f64);
                        if image_area > 0.0 {
                            Some((width * height) / image_area)
                        } else {
                            None
                        }
                    });

                Some(DatasetMapBbox {
                    x,
                    y,
                    width,
                    height,
                    area_ratio,
                })
            }
            _ => None,
        };

        let source_name = Path::new(&source_path)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(&source_id)
            .to_string();

        points.push(DatasetMapPoint {
            id: target_id,
            scope,
            image_id,
            annotation_id,
            filename,
            source_id,
            source_name,
            category_id,
            category_name,
            bbox,
            x,
            y,
            review_status,
        });
    }

    Ok(points)
}

pub fn read_dataset_map_projection_targets(
    db_path: &Path,
    workspace_id: &str,
    scope: &str,
) -> Result<Vec<DatasetMapProjectionTarget>, String> {
    let connection = Connection::open(db_path)
        .map_err(|error| format!("failed to open workspace database: {error}"))?;

    let sql = match scope {
        "object" => {
            r#"
            SELECT a.id
            FROM annotations a
            INNER JOIN images i ON i.id = a.image_id
            WHERE a.workspace_id = ?1
            ORDER BY i.file_name ASC, a.id ASC
            "#
        }
        "image" => {
            r#"
            SELECT i.id
            FROM images i
            WHERE i.workspace_id = ?1
                AND EXISTS (
                    SELECT 1
                    FROM annotations a
                    WHERE a.image_id = i.id
                )
            ORDER BY i.file_name ASC, i.id ASC
            "#
        }
        other => return Err(format!("unsupported dataset map scope '{other}'")),
    };

    let mut statement = connection
        .prepare(sql)
        .map_err(|error| format!("failed to prepare dataset map projection target query: {error}"))?;
    let rows = statement
        .query_map(params![workspace_id], |row| {
            Ok(DatasetMapProjectionTarget {
                target_id: row.get(0)?,
            })
        })
        .map_err(|error| format!("failed to read dataset map projection targets: {error}"))?;

    let mut targets = Vec::new();
    for row in rows {
        targets.push(row.map_err(|error| format!("failed to map dataset map projection target: {error}"))?);
    }

    Ok(targets)
}

pub fn upsert_embedding_projections(
    db_path: &Path,
    projections: &[EmbeddingProjectionRow],
) -> Result<(), String> {
    let mut connection = Connection::open(db_path)
        .map_err(|error| format!("failed to open workspace database: {error}"))?;
    let transaction = connection
        .transaction()
        .map_err(|error| format!("failed to start embedding projection transaction: {error}"))?;

    for projection in projections {
        transaction
            .execute(
                r#"
                INSERT INTO embedding_projections (
                    id,
                    workspace_id,
                    scope,
                    target_id,
                    model_id,
                    projection_method,
                    x,
                    y,
                    created_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                ON CONFLICT(workspace_id, scope, target_id, model_id, projection_method)
                DO UPDATE SET
                    x = excluded.x,
                    y = excluded.y,
                    created_at = excluded.created_at
                "#,
                params![
                    projection.id,
                    projection.workspace_id,
                    projection.scope,
                    projection.target_id,
                    projection.model_id,
                    projection.projection_method,
                    projection.x,
                    projection.y,
                    projection.created_at,
                ],
            )
            .map_err(|error| format!("failed to upsert embedding projection: {error}"))?;
    }

    transaction
        .commit()
        .map_err(|error| format!("failed to commit embedding projection transaction: {error}"))?;

    Ok(())
}

pub fn read_import_review_rows(db_path: &Path) -> Result<Vec<ImportReviewRow>, String> {
    let connection = Connection::open(db_path)
        .map_err(|error| format!("failed to open workspace database: {error}"))?;
    let mut statement = connection
        .prepare(
            r#"
            SELECT
                c.id,
                c.name,
                COALESCE(sf.path, ''),
                COALESCE(NULLIF(c.normalized_name, ''), LOWER(c.name)) AS group_key,
                COUNT(DISTINCT a.image_id) AS image_count,
                COALESCE(sf.image_count, 0) AS source_total_image_count,
                scm.final_action,
                uc.name
            FROM categories c
            LEFT JOIN annotations a ON a.source_category_id = c.id
            LEFT JOIN source_folders sf ON sf.id = c.source_id
            LEFT JOIN source_category_mappings scm ON scm.source_category_id = c.id
            LEFT JOIN categories uc ON uc.id = scm.target_category_id AND uc.category_role = 'unified'
            WHERE c.category_role = 'source'
            GROUP BY c.id, c.name, sf.path, sf.image_count, group_key, scm.final_action, uc.name
            ORDER BY c.name ASC
            "#,
        )
        .map_err(|error| format!("failed to prepare import review query: {error}"))?;

    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?.max(0) as u32,
                row.get::<_, i64>(5)?.max(0) as u32,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
            ))
        })
        .map_err(|error| format!("failed to read import review rows: {error}"))?;

    let mut source_rows = Vec::new();
    let mut group_counts = BTreeMap::<String, usize>::new();
    let mut display_names = BTreeMap::<String, String>::new();

    for row in rows {
        let (
            source_category_id,
            name,
            source_path,
            group_key,
            image_count,
            source_total_image_count,
            saved_final_action,
            saved_target_name,
        ) = row.map_err(|error| format!("failed to map import review row: {error}"))?;
        *group_counts.entry(group_key.clone()).or_insert(0) += 1;
        display_names.entry(group_key.clone()).or_insert_with(|| name.clone());
        source_rows.push((
            source_category_id,
            name,
            source_path,
            group_key,
            image_count,
            source_total_image_count,
            saved_final_action,
            saved_target_name,
        ));
    }

    Ok(source_rows
        .into_iter()
        .map(|(
            source_category_id,
            name,
            source_path,
            group_key,
            image_count,
            source_total_image_count,
            saved_final_action,
            saved_target_name,
        )| {
            let is_duplicate_group = group_counts.get(&group_key).copied().unwrap_or(0) > 1;
            let default_target_name = display_names
                .get(&group_key)
                .cloned()
                .unwrap_or_else(|| name.clone());
            let final_action = saved_final_action.unwrap_or_else(|| {
                if is_duplicate_group { "Merge".into() } else { "Create New".into() }
            });

            ImportReviewRow {
                source_category_id,
                source_category: name,
                source_path,
                count: image_count,
                source_total_image_count,
                suggested_action: if is_duplicate_group { "same-name candidate".into() } else { "new".into() },
                target_unified_category: if final_action == "Ignore" {
                    None
                } else {
                    Some(saved_target_name.unwrap_or(default_target_name))
                },
                final_action,
            }
        })
        .collect())
}
pub fn save_import_review_rows(
    db_path: &Path,
    workspace_id: &str,
    rows: &[ImportReviewRow],
) -> Result<(), String> {
    let mut connection = Connection::open(db_path)
        .map_err(|error| format!("failed to open workspace database: {error}"))?;
    let transaction = connection
        .transaction()
        .map_err(|error| format!("failed to start import review save transaction: {error}"))?;

    transaction
        .execute(
            "UPDATE annotations SET category_id = NULL WHERE workspace_id = ?1",
            params![workspace_id],
        )
        .map_err(|error| format!("failed to clear unified category ids on annotations: {error}"))?;
    transaction
        .execute(
            "DELETE FROM source_category_mappings WHERE workspace_id = ?1",
            params![workspace_id],
        )
        .map_err(|error| format!("failed to clear source category mappings: {error}"))?;
    transaction
        .execute(
            "DELETE FROM categories WHERE workspace_id = ?1 AND category_role = 'unified'",
            params![workspace_id],
        )
        .map_err(|error| format!("failed to clear unified categories: {error}"))?;

    let mut unified_ids = BTreeMap::<String, String>::new();

    for row in rows {
        let normalized_target = row
            .target_unified_category
            .clone()
            .unwrap_or_else(|| row.source_category.clone());
        let normalized_target = slugify_name(&normalized_target);

        let target_category_id = if row.final_action == "Ignore" {
            None
        } else if let Some(existing) = unified_ids.get(&normalized_target) {
            Some(existing.clone())
        } else {
            let display_name = row
                .target_unified_category
                .clone()
                .unwrap_or_else(|| row.source_category.clone());
            let unified_id = format!("unified-{normalized_target}");
            transaction
                .execute(
                    r#"
                    INSERT INTO categories (
                        id, workspace_id, source_id, name, normalized_name, category_role, created_at, updated_at
                    )
                    VALUES (?1, ?2, NULL, ?3, ?4, 'unified', datetime('now'), datetime('now'))
                    "#,
                    params![unified_id, workspace_id, display_name, normalized_target],
                )
                .map_err(|error| format!("failed to insert unified category: {error}"))?;
            unified_ids.insert(normalized_target.clone(), unified_id.clone());
            Some(unified_id)
        };

        let source_meta: (String, String) = transaction
            .query_row(
                "SELECT workspace_id, source_id FROM categories WHERE id = ?1",
                params![row.source_category_id],
                |db_row| Ok((db_row.get(0)?, db_row.get(1)?)),
            )
            .map_err(|error| format!("failed to resolve source category metadata: {error}"))?;

        transaction
            .execute(
                r#"
                INSERT INTO source_category_mappings (
                    id, workspace_id, source_id, source_category_id, target_category_id,
                    final_action, created_at, updated_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'), datetime('now'))
                "#,
                params![
                    format!("map-{}", row.source_category_id),
                    source_meta.0,
                    source_meta.1,
                    row.source_category_id,
                    target_category_id,
                    row.final_action
                ],
            )
            .map_err(|error| format!("failed to insert source category mapping: {error}"))?;
    }

    transaction
        .execute(
            r#"
            UPDATE annotations
            SET category_id = (
                SELECT scm.target_category_id
                FROM source_category_mappings scm
                WHERE scm.source_category_id = annotations.source_category_id
                  AND scm.workspace_id = annotations.workspace_id
                LIMIT 1
            )
            WHERE workspace_id = ?1
            "#,
            params![workspace_id],
        )
        .map_err(|error| format!("failed to apply unified category ids to annotations: {error}"))?;

    transaction
        .commit()
        .map_err(|error| format!("failed to commit import review save transaction: {error}"))?;

    Ok(())
}

pub fn read_source_folders(db_path: &Path) -> Result<Vec<SourceFolder>, String> {
    let rows = read_source_folder_rows(db_path)?;
    Ok(rows.into_iter().map(to_source_folder).collect())
}

pub fn read_source_folder_row(db_path: &Path, source_id: &str) -> Result<StoredSourceFolder, String> {
    let connection = Connection::open(db_path)
        .map_err(|error| format!("failed to open workspace database: {error}"))?;

    connection
        .query_row(
            r#"
            SELECT id, workspace_id, path, source_type, status, last_scan_at, image_count, category_count
            FROM source_folders
            WHERE id = ?1
            "#,
            params![source_id],
            |row| {
                Ok(StoredSourceFolder {
                    id: row.get(0)?,
                    workspace_id: row.get(1)?,
                    path: row.get(2)?,
                    source_type: row.get(3)?,
                    status: row.get(4)?,
                    last_scan_at: row.get(5)?,
                    image_count: row.get(6)?,
                    category_count: row.get(7)?,
                    corrupted_image_count: 0,
                    corrupted_image_paths: Vec::new(),
                })
            },
        )
        .map_err(|error| format!("failed to read source folder: {error}"))
}

pub fn read_cvat_tasks(db_path: &Path) -> Result<Vec<crate::models::CvatTask>, String> {
    let connection = Connection::open(db_path)
        .map_err(|error| format!("failed to open workspace database: {error}"))?;
    let mut statement = connection
        .prepare(
            r#"
            SELECT id, name, image_count, status, project_name, last_sync_at, temp_folder, remote_task_id, remote_url
            FROM cvat_tasks
            ORDER BY created_at DESC
            "#,
        )
        .map_err(|error| format!("failed to prepare cvat tasks query: {error}"))?;

    let rows = statement
        .query_map([], |row| {
            Ok(crate::models::CvatTask {
                id: row.get(0)?,
                name: row.get(1)?,
                image_count: row.get(2)?,
                status: row.get(3)?,
                project_name: row.get(4)?,
                last_sync_at: row.get(5)?,
                temp_folder: row.get(6)?,
                remote_task_id: row.get(7)?,
                remote_url: row.get(8)?,
            })
        })
        .map_err(|error| format!("failed to read cvat tasks: {error}"))?;

    let mut tasks = Vec::new();
    for row in rows {
        tasks.push(row.map_err(|error| format!("failed to map cvat task row: {error}"))?);
    }
    Ok(tasks)
}

pub fn read_cvat_task_metadata(
    db_path: &Path,
    task_id: &str,
) -> Result<(String, String, Option<String>, Option<String>, String, Option<i64>, Option<String>), String> {
    let connection = Connection::open(db_path)
        .map_err(|error| format!("failed to open workspace database: {error}"))?;

    connection
        .query_row(
            r#"
            SELECT workspace_id, name, temp_folder, last_sync_at, status, remote_task_id, remote_url
            FROM cvat_tasks
            WHERE id = ?1
            LIMIT 1
            "#,
            params![task_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?)),
        )
        .map_err(|error| format!("failed to read cvat task metadata: {error}"))
}

pub fn insert_cvat_task(
    db_path: &Path,
    task: &crate::models::CvatTask,
) -> Result<(), String> {
    let connection = Connection::open(db_path)
        .map_err(|error| format!("failed to open workspace database: {error}"))?;
    connection
        .execute(
            r#"
            INSERT INTO cvat_tasks (
                id, workspace_id, name, image_count, status, project_name, temp_folder, last_sync_at, remote_task_id, remote_url, created_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, datetime('now'))
            "#,
            params![
                task.id,
                "",
                task.name,
                task.image_count,
                task.status,
                task.project_name,
                task.temp_folder,
                task.last_sync_at,
                task.remote_task_id,
                task.remote_url,
            ],
        )
        .map_err(|error| format!("failed to insert cvat task: {error}"))?;
    Ok(())
}

pub fn insert_cvat_task_with_workspace(
    db_path: &Path,
    workspace_id: &str,
    task: &crate::models::CvatTask,
) -> Result<(), String> {
    let connection = Connection::open(db_path)
        .map_err(|error| format!("failed to open workspace database: {error}"))?;
    connection
        .execute(
            r#"
            INSERT INTO cvat_tasks (
                id, workspace_id, name, image_count, status, project_name, temp_folder, last_sync_at, remote_task_id, remote_url, created_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, datetime('now'))
            "#,
            params![
                task.id,
                workspace_id,
                task.name,
                task.image_count,
                task.status,
                task.project_name,
                task.temp_folder,
                task.last_sync_at,
                task.remote_task_id,
                task.remote_url,
            ],
        )
        .map_err(|error| format!("failed to insert cvat task: {error}"))?;
    Ok(())
}

pub fn update_cvat_task_remote_info(
    db_path: &Path,
    task_id: &str,
    status: &str,
    remote_task_id: Option<i64>,
    remote_url: Option<&str>,
) -> Result<(), String> {
    let connection = Connection::open(db_path)
        .map_err(|error| format!("failed to open workspace database: {error}"))?;

    connection
        .execute(
            r#"
            UPDATE cvat_tasks
            SET status = ?2,
                remote_task_id = ?3,
                remote_url = ?4
            WHERE id = ?1
            "#,
            params![task_id, status, remote_task_id, remote_url],
        )
        .map_err(|error| format!("failed to update cvat task remote info: {error}"))?;

    Ok(())
}

pub fn update_cvat_task_sync_state(
    db_path: &Path,
    task_id: &str,
    status: &str,
    last_sync_at: Option<&str>,
) -> Result<(), String> {
    let connection = Connection::open(db_path)
        .map_err(|error| format!("failed to open workspace database: {error}"))?;

    connection
        .execute(
            r#"
            UPDATE cvat_tasks
            SET status = ?2,
                last_sync_at = ?3
            WHERE id = ?1
            "#,
            params![task_id, status, last_sync_at],
        )
        .map_err(|error| format!("failed to update cvat task sync state: {error}"))?;

    Ok(())
}

pub fn read_images_for_ids(
    db_path: &Path,
    image_ids: &[String],
) -> Result<Vec<(String, String, String, String, String)>, String> {
    if image_ids.is_empty() {
        return Ok(Vec::new());
    }

    let connection = Connection::open(db_path)
        .map_err(|error| format!("failed to open workspace database: {error}"))?;
    let placeholders = std::iter::repeat("?")
        .take(image_ids.len())
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT id, source_id, file_name, original_path, annotation_status FROM images WHERE id IN ({placeholders}) ORDER BY file_name ASC"
    );
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| format!("failed to prepare selected images query: {error}"))?;
    let params = rusqlite::params_from_iter(image_ids.iter());
    let rows = statement
        .query_map(params, |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)))
        .map_err(|error| format!("failed to read selected images: {error}"))?;

    let mut images = Vec::new();
    for row in rows {
        images.push(row.map_err(|error| format!("failed to map selected image row: {error}"))?);
    }
    Ok(images)
}

pub fn read_annotation_versions(db_path: &Path) -> Result<Vec<AnnotationVersion>, String> {
    let connection = Connection::open(db_path)
        .map_err(|error| format!("failed to open workspace database: {error}"))?;
    let mut statement = connection
        .prepare(
            r#"
            SELECT id, label, created_at, source_task, notes
            FROM annotation_versions
            ORDER BY created_at DESC
            "#,
        )
        .map_err(|error| format!("failed to prepare annotation versions query: {error}"))?;

    let rows = statement
        .query_map([], |row| {
            let version_id: String = row.get(0)?;
            let image_count: u32 = connection
                .query_row(
                    "SELECT COUNT(DISTINCT image_id) FROM annotations WHERE annotation_version_id = ?1",
                    params![version_id.clone()],
                    |count_row| Ok(count_row.get::<_, i64>(0)?.max(0) as u32),
                )?;
            let box_count: u32 = connection
                .query_row(
                    "SELECT COUNT(1) FROM annotations WHERE annotation_version_id = ?1",
                    params![version_id.clone()],
                    |count_row| Ok(count_row.get::<_, i64>(0)?.max(0) as u32),
                )?;

            Ok(AnnotationVersion {
                id: version_id,
                label: row.get(1)?,
                created_at: row.get(2)?,
                source_task: row.get(3)?,
                image_count,
                box_count,
                notes: row.get(4)?,
            })
        })
        .map_err(|error| format!("failed to read annotation versions: {error}"))?;

    let mut versions = Vec::new();
    for row in rows {
        versions.push(row.map_err(|error| format!("failed to map annotation version row: {error}"))?);
    }
    Ok(versions)
}

pub fn apply_cvat_sync_annotations(
    db_path: &Path,
    workspace_id: &str,
    task_id: &str,
    task_name: &str,
    image_ids: &[String],
    annotations: &[(String, String, String, f64, f64, f64, f64)],
) -> Result<AnnotationVersion, String> {
    let mut connection = Connection::open(db_path)
        .map_err(|error| format!("failed to open workspace database: {error}"))?;
    let transaction = connection
        .transaction()
        .map_err(|error| format!("failed to start cvat sync transaction: {error}"))?;

    let existing_version_count: i64 = transaction
        .query_row(
            "SELECT COUNT(1) FROM annotation_versions WHERE workspace_id = ?1",
            params![workspace_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("failed to count annotation versions: {error}"))?;

    let created_at = chrono::Utc::now().to_rfc3339();
    let version_id = format!("version-{}", chrono::Utc::now().timestamp_millis());
    let version_label = format!("v{}", existing_version_count + 1);

    let mut category_lookup = BTreeMap::<String, String>::new();
    let mut category_statement = transaction
        .prepare(
            r#"
            SELECT id, normalized_name, name
            FROM categories
            WHERE workspace_id = ?1 AND category_role = 'unified'
            "#,
        )
        .map_err(|error| format!("failed to prepare unified category lookup: {error}"))?;
    let category_rows = category_statement
        .query_map(params![workspace_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|error| format!("failed to read unified category lookup: {error}"))?;
    for row in category_rows {
        let (category_id, normalized_name, category_name) =
            row.map_err(|error| format!("failed to map unified category row: {error}"))?;
        let key = normalized_name.unwrap_or_else(|| slugify_name(&category_name));
        category_lookup.insert(key, category_id);
    }
    drop(category_statement);

    for (_, _, category_name, _, _, _, _) in annotations {
        let normalized_name = slugify_name(category_name);
        if category_lookup.contains_key(&normalized_name) {
            continue;
        }

        let category_id = format!("unified-{normalized_name}");
        transaction
            .execute(
                r#"
                INSERT INTO categories (
                    id, workspace_id, source_id, name, normalized_name, category_role, created_at, updated_at
                )
                VALUES (?1, ?2, NULL, ?3, ?4, 'unified', ?5, ?6)
                "#,
                params![
                    category_id,
                    workspace_id,
                    category_name,
                    normalized_name,
                    created_at,
                    created_at,
                ],
            )
            .map_err(|error| format!("failed to insert synced unified category: {error}"))?;
        category_lookup.insert(normalized_name, category_id);
    }

    if !image_ids.is_empty() {
        let placeholders = std::iter::repeat("?")
            .take(image_ids.len())
            .collect::<Vec<_>>()
            .join(", ");
        let delete_sql = format!(
            "DELETE FROM annotations WHERE workspace_id = ?1 AND image_id IN ({placeholders})"
        );
        let delete_params = std::iter::once(workspace_id.to_string()).chain(image_ids.iter().cloned());
        transaction
            .execute(&delete_sql, rusqlite::params_from_iter(delete_params))
            .map_err(|error| format!("failed to clear image annotations before cvat sync: {error}"))?;

        let reset_sql = format!(
            "UPDATE images SET annotation_status = 'unannotated', updated_at = ?1 WHERE id IN ({placeholders})"
        );
        let reset_params = std::iter::once(created_at.clone()).chain(image_ids.iter().cloned());
        transaction
            .execute(&reset_sql, rusqlite::params_from_iter(reset_params))
            .map_err(|error| format!("failed to reset image annotation status before cvat sync: {error}"))?;
    }

    transaction
        .execute(
            r#"
            INSERT INTO annotation_versions (
                id, workspace_id, label, created_at, source_task, notes
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![
                version_id,
                workspace_id,
                version_label,
                created_at,
                task_name,
                format!("sync from CVAT task {task_id}"),
            ],
        )
        .map_err(|error| format!("failed to insert annotation version: {error}"))?;

    for (index, (image_id, source_id, category_name, bbox_x, bbox_y, bbox_width, bbox_height)) in annotations.iter().enumerate() {
        let category_id = category_lookup
            .get(&slugify_name(category_name))
            .cloned()
            .ok_or_else(|| format!("failed to resolve unified category for synced annotation: {category_name}"))?;
        transaction
            .execute(
                r#"
                INSERT INTO annotations (
                    id, workspace_id, image_id, source_id, source_category_id, category_id,
                    annotation_version_id, bbox_x, bbox_y, bbox_width, bbox_height,
                    annotation_format, created_at, updated_at
                )
                VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6, ?7, ?8, ?9, ?10, 'coco', ?11, ?12)
                "#,
                params![
                    format!("ann-{task_id}-{index}"),
                    workspace_id,
                    image_id,
                    source_id,
                    category_id,
                    version_id,
                    bbox_x,
                    bbox_y,
                    bbox_width,
                    bbox_height,
                    created_at,
                    created_at,
                ],
            )
            .map_err(|error| format!("failed to insert synced annotation: {error}"))?;
    }

    let annotated_image_ids = annotations
        .iter()
        .map(|(image_id, _, _, _, _, _, _)| image_id.clone())
        .collect::<Vec<_>>();
    if !annotated_image_ids.is_empty() {
        let placeholders = std::iter::repeat("?")
            .take(annotated_image_ids.len())
            .collect::<Vec<_>>()
            .join(", ");
        let update_sql = format!(
            "UPDATE images SET annotation_status = 'annotated', updated_at = ?1 WHERE id IN ({placeholders})"
        );
        let update_params = std::iter::once(created_at.clone()).chain(annotated_image_ids.iter().cloned());
        transaction
            .execute(&update_sql, rusqlite::params_from_iter(update_params))
            .map_err(|error| format!("failed to mark synced images as annotated: {error}"))?;
    }

    transaction
        .commit()
        .map_err(|error| format!("failed to commit cvat sync transaction: {error}"))?;

    Ok(AnnotationVersion {
        id: version_id,
        label: version_label,
        created_at,
        source_task: task_name.to_string(),
        image_count: image_ids.len() as u32,
        box_count: annotations.len() as u32,
        notes: format!("sync from CVAT task {task_id}"),
    })
}

pub fn read_image_detail(db_path: &Path, image_id: &str) -> Result<ImageDetailPayload, String> {
    let connection = Connection::open(db_path)
        .map_err(|error| format!("failed to open workspace database: {error}"))?;
    let unified_count: i64 = connection
        .query_row(
            "SELECT COUNT(1) FROM categories WHERE category_role = 'unified'",
            [],
            |row| row.get(0),
        )
        .map_err(|error| format!("failed to inspect unified category count: {error}"))?;

    let image_row = connection
        .query_row(
            r#"
            SELECT
                i.id,
                i.file_name,
                i.source_id,
                COALESCE(sf.path, ''),
                i.original_path,
                i.annotation_status,
                i.health_status,
                i.health_error,
                i.width,
                i.height
            FROM images i
            LEFT JOIN source_folders sf ON sf.id = i.source_id
            WHERE i.id = ?1
            LIMIT 1
            "#,
            params![image_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<u32>>(8)?,
                    row.get::<_, Option<u32>>(9)?,
                ))
            },
        )
        .map_err(|error| format!("failed to read image detail: {error}"))?;

    let mut statement = if unified_count > 0 {
        connection
            .prepare(
                r#"
                SELECT
                    a.id,
                    COALESCE(uc.name, sc.name, 'unknown'),
                    a.annotation_format,
                    a.bbox_x,
                    a.bbox_y,
                    a.bbox_width,
                    a.bbox_height
                FROM annotations a
                LEFT JOIN categories uc ON uc.id = a.category_id
                LEFT JOIN categories sc ON sc.id = a.source_category_id
                WHERE a.image_id = ?1
                ORDER BY COALESCE(uc.name, sc.name, 'unknown') ASC
                "#,
            )
            .map_err(|error| format!("failed to prepare mapped image detail annotations query: {error}"))?
    } else {
        connection
            .prepare(
                r#"
                SELECT
                    a.id,
                    COALESCE(sc.name, 'unknown'),
                    a.annotation_format,
                    a.bbox_x,
                    a.bbox_y,
                    a.bbox_width,
                    a.bbox_height
                FROM annotations a
                LEFT JOIN categories sc ON sc.id = a.source_category_id
                WHERE a.image_id = ?1
                ORDER BY COALESCE(sc.name, 'unknown') ASC
                "#,
            )
            .map_err(|error| format!("failed to prepare image detail annotations query: {error}"))?
    };

    let rows = statement
        .query_map(params![image_id], |row| {
            Ok(BoundingBoxRecord {
                id: row.get(0)?,
                category_name: row.get(1)?,
                annotation_format: row.get(2)?,
                bbox_x: row.get(3)?,
                bbox_y: row.get(4)?,
                bbox_width: row.get(5)?,
                bbox_height: row.get(6)?,
            })
        })
        .map_err(|error| format!("failed to read image detail annotation rows: {error}"))?;

    let mut boxes = Vec::new();
    let mut categories = Vec::new();
    for row in rows {
        let next = row.map_err(|error| format!("failed to map image detail annotation row: {error}"))?;
        if !categories.contains(&next.category_name) {
            categories.push(next.category_name.clone());
        }
        boxes.push(next);
    }

    let source_name = Path::new(&image_row.3)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(&image_row.3)
        .to_string();

    Ok(ImageDetailPayload {
        id: image_row.0,
        filename: image_row.1,
        source_id: image_row.2,
        source_name,
        original_path: image_row.4,
        annotation_status: image_row.5,
        image_health_status: image_row.6,
        image_health_error: image_row.7,
        categories,
        width: image_row.8,
        height: image_row.9,
        boxes,
    })
}
pub fn read_export_preview(db_path: &Path, default_output_path: &str) -> Result<crate::models::ExportPreview, String> {
    let connection = Connection::open(db_path)
        .map_err(|error| format!("failed to open workspace database: {error}"))?;
    let unified_count: i64 = connection
        .query_row(
            "SELECT COUNT(1) FROM categories WHERE category_role = 'unified'",
            [],
            |row| row.get(0),
        )
        .map_err(|error| format!("failed to inspect unified category count for export preview: {error}"))?;

    let included_images: u32 = connection
        .query_row(
            "SELECT COUNT(1) FROM images WHERE annotation_status = 'annotated'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| format!("failed to count export included images: {error}"))?
        .max(0) as u32;
    let excluded_images: u32 = connection
        .query_row(
            "SELECT COUNT(1) FROM images WHERE annotation_status != 'annotated'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| format!("failed to count export excluded images: {error}"))?
        .max(0) as u32;
    let included_boxes: u32 = connection
        .query_row("SELECT COUNT(1) FROM annotations", [], |row| row.get::<_, i64>(0))
        .map_err(|error| format!("failed to count export boxes: {error}"))?
        .max(0) as u32;

    let category_count: u32 = if unified_count > 0 {
        connection
            .query_row(
                "SELECT COUNT(1) FROM categories WHERE category_role = 'unified'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|error| format!("failed to count unified categories for export preview: {error}"))?
            .max(0) as u32
    } else {
        connection
            .query_row(
                r#"
                SELECT COUNT(1) FROM (
                    SELECT COALESCE(NULLIF(normalized_name, ''), LOWER(name)) AS group_key
                    FROM categories
                    WHERE category_role = 'source'
                    GROUP BY group_key
                )
                "#,
                [],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|error| format!("failed to count auto merged categories for export preview: {error}"))?
            .max(0) as u32
    };

    let filename_conflicts: u32 = connection
        .query_row(
            r#"
            SELECT COUNT(1) FROM (
                SELECT file_name
                FROM images
                WHERE annotation_status = 'annotated'
                GROUP BY file_name
                HAVING COUNT(1) > 1
            )
            "#,
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| format!("failed to count filename conflicts for export preview: {error}"))?
        .max(0) as u32;
    let conflict_details = read_export_conflicts(db_path)?;

    let train = ((included_images as f64) * 0.70).floor() as u32;
    let valid = ((included_images as f64) * 0.15).floor() as u32;
    let test = included_images.saturating_sub(train).saturating_sub(valid);

    Ok(crate::models::ExportPreview {
        category_count,
        included_images,
        excluded_images,
        included_boxes,
        filename_conflicts,
        conflict_details,
        split_counts: crate::models::SplitCounts { train, valid, test },
        output_path: default_output_path.to_string(),
    })
}

pub fn read_export_conflicts(db_path: &Path) -> Result<Vec<ExportFilenameConflict>, String> {
    let connection = Connection::open(db_path)
        .map_err(|error| format!("failed to open workspace database: {error}"))?;

    let mut statement = connection
        .prepare(
            r#"
            SELECT i.file_name, i.id, i.source_id, i.original_path
            FROM images i
            WHERE i.annotation_status = 'annotated' AND i.health_status = 'healthy'
              AND i.file_name IN (
                SELECT file_name
                FROM images
                WHERE annotation_status = 'annotated'
                GROUP BY file_name
                HAVING COUNT(1) > 1
              )
            ORDER BY i.file_name ASC, i.original_path ASC
            "#,
        )
        .map_err(|error| format!("failed to prepare export conflict query: {error}"))?;

    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|error| format!("failed to read export conflicts: {error}"))?;

    let mut grouped = BTreeMap::<String, Vec<ExportConflictItem>>::new();
    for row in rows {
        let (file_name, image_id, source_id, original_path) =
            row.map_err(|error| format!("failed to map export conflict row: {error}"))?;
        grouped
            .entry(file_name)
            .or_default()
            .push(ExportConflictItem {
                image_id,
                source_id,
                original_path,
            });
    }

    Ok(grouped
        .into_iter()
        .map(|(file_name, items)| ExportFilenameConflict { file_name, items })
        .collect())
}

pub fn insert_export_job(
    db_path: &Path,
    workspace_id: &str,
    id: &str,
    output_format: &str,
    output_path: &str,
    created_at: &str,
    status: &str,
    exported_images: u32,
    exported_boxes: u32,
) -> Result<(), String> {
    let connection = Connection::open(db_path)
        .map_err(|error| format!("failed to open workspace database: {error}"))?;

    ensure_export_jobs_column(&connection, "exported_images", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_export_jobs_column(&connection, "exported_boxes", "INTEGER NOT NULL DEFAULT 0")?;

    connection
        .execute(
            r#"
            INSERT INTO export_jobs (
                id, workspace_id, output_format, output_path, created_at, status, exported_images, exported_boxes
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
            params![
                id,
                workspace_id,
                output_format,
                output_path,
                created_at,
                status,
                exported_images,
                exported_boxes
            ],
        )
        .map_err(|error| format!("failed to insert export job: {error}"))?;

    Ok(())
}

pub fn read_export_history(db_path: &Path) -> Result<Vec<ExportHistoryEntry>, String> {
    let connection = Connection::open(db_path)
        .map_err(|error| format!("failed to open workspace database: {error}"))?;

    ensure_export_jobs_column(&connection, "exported_images", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_export_jobs_column(&connection, "exported_boxes", "INTEGER NOT NULL DEFAULT 0")?;

    let mut statement = connection
        .prepare(
            r#"
            SELECT id, output_format, output_path, created_at, status, exported_images, exported_boxes
            FROM export_jobs
            ORDER BY created_at DESC
            "#,
        )
        .map_err(|error| format!("failed to prepare export history query: {error}"))?;

    let rows = statement
        .query_map([], |row| {
            Ok(ExportHistoryEntry {
                id: row.get(0)?,
                output_format: row.get(1)?,
                output_path: row.get(2)?,
                created_at: row.get(3)?,
                status: row.get(4)?,
                exported_images: row.get::<_, i64>(5)?.max(0) as u32,
                exported_boxes: row.get::<_, i64>(6)?.max(0) as u32,
            })
        })
        .map_err(|error| format!("failed to read export history: {error}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|error| format!("failed to map export history row: {error}"))?);
    }
    Ok(items)
}

pub fn read_export_images(db_path: &Path) -> Result<Vec<ExportImageRecord>, String> {
    let connection = Connection::open(db_path)
        .map_err(|error| format!("failed to open workspace database: {error}"))?;
    let unified_count: i64 = connection
        .query_row(
            "SELECT COUNT(1) FROM categories WHERE category_role = 'unified'",
            [],
            |row| row.get(0),
        )
        .map_err(|error| format!("failed to inspect unified category count for export rows: {error}"))?;

    let query = if unified_count > 0 {
        r#"
        SELECT
            i.id,
            i.file_name,
            i.original_path,
            i.width,
            i.height,
            COALESCE(uc.id, COALESCE(NULLIF(sc.normalized_name, ''), LOWER(sc.name))),
            COALESCE(uc.name, sc.name),
            a.annotation_format,
            a.bbox_x,
            a.bbox_y,
            a.bbox_width,
            a.bbox_height
        FROM images i
        INNER JOIN annotations a ON a.image_id = i.id
        LEFT JOIN categories uc ON uc.id = a.category_id AND uc.category_role = 'unified'
        LEFT JOIN categories sc ON sc.id = a.source_category_id
        WHERE i.annotation_status = 'annotated' AND i.health_status = 'healthy'
        ORDER BY i.file_name ASC, COALESCE(uc.name, sc.name) ASC
        "#
    } else {
        r#"
        SELECT
            i.id,
            i.file_name,
            i.original_path,
            i.width,
            i.height,
            COALESCE(NULLIF(sc.normalized_name, ''), LOWER(sc.name)),
            sc.name,
            a.annotation_format,
            a.bbox_x,
            a.bbox_y,
            a.bbox_width,
            a.bbox_height
        FROM images i
        INNER JOIN annotations a ON a.image_id = i.id
        LEFT JOIN categories sc ON sc.id = a.source_category_id
        WHERE i.annotation_status = 'annotated' AND i.health_status = 'healthy'
        ORDER BY i.file_name ASC, sc.name ASC
        "#
    };

    let mut statement = connection
        .prepare(query)
        .map_err(|error| format!("failed to prepare export rows query: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<u32>>(3)?,
                row.get::<_, Option<u32>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, f64>(8)?,
                row.get::<_, f64>(9)?,
                row.get::<_, f64>(10)?,
                row.get::<_, f64>(11)?,
            ))
        })
        .map_err(|error| format!("failed to read export rows: {error}"))?;

    let mut image_map = BTreeMap::<String, ExportImageRecord>::new();
    for row in rows {
        let (
            image_id,
            file_name,
            original_path,
            width,
            height,
            category_key,
            category_name,
            annotation_format,
            bbox_x,
            bbox_y,
            bbox_width,
            bbox_height,
        ) = row.map_err(|error| format!("failed to map export row: {error}"))?;

        let image_entry = image_map.entry(image_id.clone()).or_insert_with(|| ExportImageRecord {
            id: image_id,
            file_name,
            original_path,
            width,
            height,
            annotations: Vec::new(),
        });

        if let (Some(category_key), Some(category_name)) = (category_key, category_name) {
            image_entry.annotations.push(ExportAnnotationRecord {
                category_key,
                category_name,
                annotation_format,
                bbox_x,
                bbox_y,
                bbox_width,
                bbox_height,
            });
        }
    }

    Ok(image_map.into_values().collect())
}

fn read_source_folder_rows(db_path: &Path) -> Result<Vec<StoredSourceFolder>, String> {
    let connection = Connection::open(db_path)
        .map_err(|error| format!("failed to open workspace database: {error}"))?;

    let mut statement = connection
        .prepare(
            r#"
            SELECT
                sf.id,
                sf.workspace_id,
                sf.path,
                sf.source_type,
                sf.status,
                sf.last_scan_at,
                sf.image_count,
                sf.category_count,
                COALESCE((
                    SELECT COUNT(1)
                    FROM images i
                    WHERE i.source_id = sf.id AND i.health_status = 'corrupted'
                ), 0) AS corrupted_image_count,
                COALESCE((
                    SELECT GROUP_CONCAT(i.original_path, CHAR(10))
                    FROM images i
                    WHERE i.source_id = sf.id AND i.health_status = 'corrupted'
                ), '') AS corrupted_image_paths
            FROM source_folders sf
            ORDER BY sf.rowid DESC
            "#,
        )
        .map_err(|error| format!("failed to prepare source folder query: {error}"))?;

    let rows = statement
        .query_map([], |row| {
            let corrupted_paths_raw = row.get::<_, String>(9)?;
            Ok(StoredSourceFolder {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                path: row.get(2)?,
                source_type: row.get(3)?,
                status: row.get(4)?,
                last_scan_at: row.get(5)?,
                image_count: row.get(6)?,
                category_count: row.get(7)?,
                corrupted_image_count: row.get::<_, i64>(8)?.max(0) as u32,
                corrupted_image_paths: corrupted_paths_raw
                    .lines()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
                    .collect(),
            })
        })
        .map_err(|error| format!("failed to read source folders: {error}"))?;

    let mut sources = Vec::new();
    for row in rows {
        sources.push(row.map_err(|error| format!("failed to map source folder row: {error}"))?);
    }

    Ok(sources)
}
fn read_browser_categories(db_path: &Path) -> Result<Vec<UnifiedCategory>, String> {
    let connection = Connection::open(db_path)
        .map_err(|error| format!("failed to open workspace database: {error}"))?;
    let unified_count: i64 = connection
        .query_row(
            "SELECT COUNT(1) FROM categories WHERE category_role = 'unified'",
            [],
            |row| row.get(0),
        )
        .map_err(|error| format!("failed to inspect unified category count: {error}"))?;

    if unified_count > 0 {
        let mut statement = connection
            .prepare(
                r#"
                SELECT uc.id, uc.name, COUNT(DISTINCT a.image_id) AS image_count
                FROM categories uc
                LEFT JOIN annotations a ON a.category_id = uc.id
                WHERE uc.category_role = 'unified'
                GROUP BY uc.id, uc.name
                HAVING COUNT(DISTINCT a.image_id) > 0
                ORDER BY uc.name ASC
                "#,
            )
            .map_err(|error| format!("failed to prepare mapped browser categories query: {error}"))?;

        let rows = statement
            .query_map([], |row| {
                Ok(UnifiedCategory {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    image_count: row.get::<_, i64>(2)?.max(0) as u32,
                })
            })
            .map_err(|error| format!("failed to read mapped browser categories: {error}"))?;

        let mut categories = Vec::new();
        for row in rows {
            categories.push(
                row.map_err(|error| format!("failed to map mapped browser category row: {error}"))?,
            );
        }
        return Ok(categories);
    }

    let mut statement = connection
        .prepare(
            r#"
            SELECT
                COALESCE(NULLIF(c.normalized_name, ''), LOWER(c.name)) AS group_key,
                MIN(c.name) AS display_name,
                COUNT(DISTINCT a.image_id) AS image_count
            FROM categories c
            LEFT JOIN annotations a ON a.source_category_id = c.id
            WHERE c.category_role = 'source'
            GROUP BY group_key
            HAVING COUNT(DISTINCT a.image_id) > 0
            ORDER BY display_name ASC
            "#,
        )
        .map_err(|error| format!("failed to prepare browser categories query: {error}"))?;

    let rows = statement
        .query_map([], |row| {
            Ok(UnifiedCategory {
                id: format!("unified-auto-{}", row.get::<_, String>(0)?),
                name: row.get(1)?,
                image_count: row.get::<_, i64>(2)?.max(0) as u32,
            })
        })
        .map_err(|error| format!("failed to read browser categories: {error}"))?;

    let mut categories = Vec::new();
    for row in rows {
        categories.push(row.map_err(|error| format!("failed to map browser category row: {error}"))?);
    }
    Ok(categories)
}

fn read_image_cards(db_path: &Path) -> Result<Vec<ImageCard>, String> {
    let connection = Connection::open(db_path)
        .map_err(|error| format!("failed to open workspace database: {error}"))?;
    let unified_count: i64 = connection
        .query_row(
            "SELECT COUNT(1) FROM categories WHERE category_role = 'unified'",
            [],
            |row| row.get(0),
        )
        .map_err(|error| format!("failed to inspect unified category count: {error}"))?;

    let query = if unified_count > 0 {
        r#"
        SELECT
            i.id,
            i.file_name,
            i.source_id,
            COALESCE(sf.path, ''),
            i.original_path,
            i.annotation_status,
            i.health_status,
            i.health_error,
            uc.id,
            uc.name,
            i.width,
            i.height,
            a.annotation_format,
            a.bbox_width,
            a.bbox_height
        FROM images i
        LEFT JOIN source_folders sf ON sf.id = i.source_id
        LEFT JOIN annotations a ON a.image_id = i.id
        LEFT JOIN categories uc ON uc.id = a.category_id AND uc.category_role = 'unified'
        WHERE i.health_status = 'healthy'
        ORDER BY i.file_name ASC, uc.name ASC
        "#
    } else {
        r#"
        SELECT
            i.id,
            i.file_name,
            i.source_id,
            COALESCE(sf.path, ''),
            i.original_path,
            i.annotation_status,
            i.health_status,
            i.health_error,
            COALESCE('unified-auto-' || COALESCE(NULLIF(c.normalized_name, ''), LOWER(c.name)), NULL),
            c.name,
            i.width,
            i.height,
            a.annotation_format,
            a.bbox_width,
            a.bbox_height
        FROM images i
        LEFT JOIN source_folders sf ON sf.id = i.source_id
        LEFT JOIN annotations a ON a.image_id = i.id
        LEFT JOIN categories c ON c.id = a.source_category_id
        WHERE i.health_status = 'healthy'
        ORDER BY i.file_name ASC, c.name ASC
        "#
    };

    let mut statement = connection
        .prepare(query)
        .map_err(|error| format!("failed to prepare image card query: {error}"))?;

    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, Option<String>>(9)?,
                row.get::<_, Option<u32>>(10)?,
                row.get::<_, Option<u32>>(11)?,
                row.get::<_, Option<String>>(12)?,
                row.get::<_, Option<f64>>(13)?,
                row.get::<_, Option<f64>>(14)?,
            ))
        })
        .map_err(|error| format!("failed to read image cards: {error}"))?;

    let mut image_map = BTreeMap::<String, ImageCard>::new();
    for row in rows {
        let (
            id,
            filename,
            source_id,
            source_path,
            original_path,
            annotation_status,
            image_health_status,
            image_health_error,
            category_id,
            category_name,
            image_width,
            image_height,
            annotation_format,
            bbox_width,
            bbox_height,
        ) = row.map_err(|error| format!("failed to map image card row: {error}"))?;
        let source_name = Path::new(&source_path)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(&source_path)
            .to_string();

        let entry = image_map.entry(id.clone()).or_insert_with(|| ImageCard {
            id: id.clone(),
            filename,
            source_id,
            source_name,
            original_path,
            annotation_status,
            image_health_status,
            image_health_error,
            annotation_count: 0,
            max_box_area_ratio: None,
            box_summaries: Vec::new(),
            category_ids: Vec::new(),
            categories: Vec::new(),
        });

        if let (Some(category_id), Some(category_name_value)) = (category_id.clone(), category_name.clone()) {
            if !entry.category_ids.contains(&category_id) {
                entry.category_ids.push(category_id);
            }
            if !entry.categories.contains(&category_name_value) {
                entry.categories.push(category_name_value);
            }
        }

        if let Some(annotation_format) = annotation_format {
            entry.annotation_count = entry.annotation_count.saturating_add(1);

            let ratio = if annotation_format.eq_ignore_ascii_case("yolo") {
                match (bbox_width, bbox_height) {
                    (Some(width), Some(height)) => Some(width.max(0.0) * height.max(0.0)),
                    _ => None,
                }
            } else {
                match (bbox_width, bbox_height, image_width, image_height) {
                    (Some(width), Some(height), Some(image_width), Some(image_height))
                        if image_width > 0 && image_height > 0 =>
                    {
                        Some(
                            (width.max(0.0) * height.max(0.0))
                                / (image_width as f64 * image_height as f64),
                        )
                    }
                    _ => None,
                }
            };

            if let Some(ratio) = ratio {
                entry.max_box_area_ratio = Some(
                    entry
                        .max_box_area_ratio
                        .map(|current| current.max(ratio))
                        .unwrap_or(ratio),
                );
            }

            if let Some(category_name_value) = category_name {
                entry.box_summaries.push(BoxSummary {
                    category_name: category_name_value,
                    area_ratio: ratio,
                });
            }
        }
    }

    Ok(image_map.into_values().collect())
}
fn to_source_folder(row: StoredSourceFolder) -> SourceFolder {
    let name = Path::new(&row.path)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(&row.path)
        .to_string();

    SourceFolder {
        id: row.id,
        name,
        path: row.path,
        r#type: row.source_type,
        status: row.status,
        image_count: row.image_count,
        category_count: row.category_count,
        corrupted_image_count: row.corrupted_image_count,
        corrupted_image_paths: row.corrupted_image_paths,
        last_scan_at: row.last_scan_at.unwrap_or_else(|| "-".to_string()),
    }
}
fn ensure_source_folder_column(
    connection: &Connection,
    column_name: &str,
    column_definition: &str,
) -> Result<(), String> {
    let mut statement = connection
        .prepare("PRAGMA table_info(source_folders)")
        .map_err(|error| format!("failed to inspect source_folders table: {error}"))?;

    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| format!("failed to enumerate source_folders columns: {error}"))?;

    for column in columns {
        if column
            .map_err(|error| format!("failed to read source_folders column info: {error}"))?
            == column_name
        {
            return Ok(());
        }
    }

    connection
        .execute(
            &format!("ALTER TABLE source_folders ADD COLUMN {column_name} {column_definition}"),
            [],
        )
        .map_err(|error| format!("failed to add source_folders.{column_name}: {error}"))?;

    Ok(())
}

fn ensure_image_column(
    connection: &Connection,
    column_name: &str,
    column_definition: &str,
) -> Result<(), String> {
    let mut statement = connection
        .prepare("PRAGMA table_info(images)")
        .map_err(|error| format!("failed to inspect images table: {error}"))?;

    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| format!("failed to enumerate images columns: {error}"))?;

    for column in columns {
        if column
            .map_err(|error| format!("failed to read images column info: {error}"))?
            == column_name
        {
            return Ok(());
        }
    }

    connection
        .execute(
            &format!("ALTER TABLE images ADD COLUMN {column_name} {column_definition}"),
            [],
        )
        .map_err(|error| format!("failed to add images.{column_name}: {error}"))?;

    Ok(())
}
fn ensure_cvat_task_column(
    connection: &Connection,
    column_name: &str,
    column_definition: &str,
) -> Result<(), String> {
    let mut statement = connection
        .prepare("PRAGMA table_info(cvat_tasks)")
        .map_err(|error| format!("failed to inspect cvat_tasks table: {error}"))?;

    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| format!("failed to enumerate cvat_tasks columns: {error}"))?;

    for column in columns {
        if column
            .map_err(|error| format!("failed to read cvat_tasks column info: {error}"))?
            == column_name
        {
            return Ok(());
        }
    }

    connection
        .execute(
            &format!("ALTER TABLE cvat_tasks ADD COLUMN {column_name} {column_definition}"),
            [],
        )
        .map_err(|error| format!("failed to add cvat_tasks.{column_name}: {error}"))?;

    Ok(())
}
fn ensure_export_jobs_column(
    connection: &Connection,
    column_name: &str,
    column_definition: &str,
) -> Result<(), String> {
    let mut statement = connection
        .prepare("PRAGMA table_info(export_jobs)")
        .map_err(|error| format!("failed to inspect export_jobs schema: {error}"))?;

    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| format!("failed to query export_jobs schema: {error}"))?;

    let mut exists = false;
    for column in columns {
        let column = column.map_err(|error| format!("failed to read export_jobs column name: {error}"))?;
        if column == column_name {
            exists = true;
            break;
        }
    }

    if !exists {
        let sql = format!("ALTER TABLE export_jobs ADD COLUMN {column_name} {column_definition}");
        connection
            .execute(sql.as_str(), [])
            .map_err(|error| format!("failed to add export_jobs column '{column_name}': {error}"))?;
    }

    Ok(())
}

fn slugify_name(value: &str) -> String {
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

    #[test]
    fn initialize_database_creates_dataset_map_tables() {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let root = std::env::temp_dir().join(format!("dataviewer-db-schema-{timestamp}"));
        std::fs::create_dir_all(&root).unwrap();
        let db_path = root.join("workspace.db");

        initialize_database(&db_path).unwrap();

        let connection = Connection::open(&db_path).unwrap();
        for table_name in [
            "embedding_models",
            "embedding_jobs",
            "embeddings",
            "embedding_projections",
            "dataset_review_marks",
        ] {
            let count: i64 = connection
                .query_row(
                    "SELECT COUNT(1) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    params![table_name],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "missing table {table_name}");
        }

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn upsert_embedding_projections_can_feed_dataset_map_points() {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let root = std::env::temp_dir().join(format!("dataviewer-map-points-{timestamp}"));
        std::fs::create_dir_all(&root).unwrap();
        let db_path = root.join("workspace.db");
        initialize_database(&db_path).unwrap();

        let connection = Connection::open(&db_path).unwrap();
        connection
            .execute(
                "INSERT INTO workspace_meta (id, name, workspace_path, created_at, updated_at, app_version) VALUES (?1, ?2, ?3, ?4, ?4, ?5)",
                params!["ws-1", "Workspace", "D:\\workspace", "2026-06-17T00:00:00Z", APP_VERSION],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO source_folders (id, workspace_id, path, source_type, status, last_scan_at, image_count, category_count) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params!["source-1", "ws-1", "D:\\datasets\\source-1", "COCO", "ready", "2026-06-17T00:00:00Z", 1, 1],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO images (id, workspace_id, source_id, file_name, original_path, width, height, annotation_status, health_status, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
                params!["image-1", "ws-1", "source-1", "frame.jpg", "D:\\datasets\\source-1\\frame.jpg", 100, 50, "annotated", "healthy", "2026-06-17T00:00:00Z"],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO categories (id, workspace_id, source_id, name, normalized_name, category_role, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
                params!["cat-1", "ws-1", "source-1", "screw", "screw", "source", "2026-06-17T00:00:00Z"],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO annotations (id, workspace_id, image_id, source_id, source_category_id, bbox_x, bbox_y, bbox_width, bbox_height, annotation_format, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)",
                params!["ann-1", "ws-1", "image-1", "source-1", "cat-1", 10.0, 5.0, 20.0, 10.0, "coco", "2026-06-17T00:00:00Z"],
            )
            .unwrap();

        upsert_embedding_projections(
            &db_path,
            &[EmbeddingProjectionRow {
                id: "projection-1".into(),
                workspace_id: "ws-1".into(),
                scope: "object".into(),
                target_id: "ann-1".into(),
                model_id: "clip-vit-b32".into(),
                projection_method: "bootstrap-deterministic".into(),
                x: 0.25,
                y: -0.5,
                created_at: "2026-06-17T00:00:00Z".into(),
            }],
        )
        .unwrap();

        let points =
            read_dataset_map_points(&db_path, "ws-1", "object", "clip-vit-b32").unwrap();

        assert_eq!(points.len(), 1);
        assert_eq!(points[0].id, "ann-1");
        assert_eq!(points[0].image_id, "image-1");
        assert_eq!(points[0].category_name.as_deref(), Some("screw"));
        assert_eq!(points[0].review_status, "unreviewed");
        assert_eq!(points[0].bbox.as_ref().and_then(|bbox| bbox.area_ratio), Some(0.04));

        let _ = std::fs::remove_dir_all(root);
    }
}





























