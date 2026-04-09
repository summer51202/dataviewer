use std::path::{Path, PathBuf};

pub const WORKSPACE_HIDDEN_DIR: &str = ".dataviewer";
pub const WORKSPACE_DB_FILE: &str = "workspace.db";
pub const WORKSPACE_MANIFEST_FILE: &str = "workspace.json";
pub const RECENT_WORKSPACES_FILE: &str = "recent-workspaces.json";
pub const APP_VERSION: &str = "0.1.0";
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Clone)]
pub struct WorkspacePaths {
    pub root: PathBuf,
    pub hidden_dir: PathBuf,
    pub db_path: PathBuf,
    pub manifest_path: PathBuf,
    pub cache_dir: PathBuf,
    pub temp_dir: PathBuf,
    pub exports_dir: PathBuf,
}

pub fn build_workspace_paths(root: &Path) -> WorkspacePaths {
    let hidden_dir = root.join(WORKSPACE_HIDDEN_DIR);

    WorkspacePaths {
        root: root.to_path_buf(),
        db_path: hidden_dir.join(WORKSPACE_DB_FILE),
        manifest_path: hidden_dir.join(WORKSPACE_MANIFEST_FILE),
        cache_dir: hidden_dir.join("cache"),
        temp_dir: hidden_dir.join("temp"),
        exports_dir: hidden_dir.join("exports"),
        hidden_dir,
    }
}

pub fn app_data_dir() -> Result<PathBuf, String> {
    if let Ok(appdata) = std::env::var("APPDATA") {
        return Ok(PathBuf::from(appdata).join("DataViewer"));
    }

    std::env::current_dir()
        .map(|dir| dir.join(".dataviewer-app"))
        .map_err(|error| format!("failed to resolve app data dir: {error}"))
}

pub fn recent_workspaces_path() -> Result<PathBuf, String> {
    Ok(app_data_dir()?.join(RECENT_WORKSPACES_FILE))
}
