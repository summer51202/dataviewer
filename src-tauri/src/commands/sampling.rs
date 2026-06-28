use crate::models::{
    DeleteSampleSetInput, SampleSelectionInput, SampleSelectionSummary, SampleSet,
    SampleSetListInput, SampleSetMembers, SampleSetMembersInput,
};
use crate::workspace_service;

#[tauri::command]
pub fn run_sample_selection(
    input: SampleSelectionInput,
) -> Result<SampleSelectionSummary, String> {
    workspace_service::run_sample_selection_by_id(input)
}

#[tauri::command]
pub fn list_sample_sets(input: SampleSetListInput) -> Result<Vec<SampleSet>, String> {
    workspace_service::list_sample_sets_by_id(input)
}

#[tauri::command]
pub fn delete_sample_set(input: DeleteSampleSetInput) -> Result<(), String> {
    workspace_service::delete_sample_set_by_id(input)
}

#[tauri::command]
pub fn get_sample_set_members(
    input: SampleSetMembersInput,
) -> Result<SampleSetMembers, String> {
    workspace_service::get_sample_set_members_by_id(input)
}
