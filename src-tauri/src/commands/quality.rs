use crate::models::{
    DatasetMapPayload, DatasetMapPayloadInput, DatasetMapReviewInput, DatasetReviewUpdate,
    EmbeddingJob, EmbeddingJobInput, EmbeddingRuntimeProbe, EmbeddingRuntimeProbeInput,
};
use crate::workspace_service;

#[tauri::command]
pub fn get_dataset_map_payload(input: DatasetMapPayloadInput) -> Result<DatasetMapPayload, String> {
    workspace_service::load_dataset_map_payload_by_id(input)
}

#[tauri::command]
pub fn probe_embedding_runtime(
    input: EmbeddingRuntimeProbeInput,
) -> Result<EmbeddingRuntimeProbe, String> {
    workspace_service::probe_embedding_runtime_by_id(input)
}

#[tauri::command]
pub fn start_embedding_job(input: EmbeddingJobInput) -> Result<EmbeddingJob, String> {
    workspace_service::start_embedding_job_by_id(input)
}

#[tauri::command]
pub fn save_dataset_map_reviews(
    input: DatasetMapReviewInput,
) -> Result<Vec<DatasetReviewUpdate>, String> {
    workspace_service::save_dataset_map_reviews_by_id(input)
}
