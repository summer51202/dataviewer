use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;

use reqwest::blocking::{multipart, Client};
use serde_json::json;

use crate::models::CvatSettings;

pub struct RemoteTaskInfo {
    pub remote_task_id: i64,
    pub remote_url: String,
}

fn normalize_base_url(base_url: &str) -> String {
    base_url.trim().trim_end_matches('/').to_string()
}

fn authorized_client(_settings: &CvatSettings) -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|error| format!("failed to create CVAT HTTP client: {error}"))
}

fn auth_header_value(settings: &CvatSettings) -> String {
    format!("Token {}", settings.access_token.trim())
}

pub fn validate_settings(settings: &CvatSettings) -> Result<(), String> {
    let client = authorized_client(settings)?;
    let base_url = normalize_base_url(&settings.base_url);
    let response = client
        .get(format!("{base_url}/api/server/about"))
        .header("Authorization", auth_header_value(settings))
        .send()
        .map_err(|error| format!("failed to reach CVAT server: {error}"))?;

    if !response.status().is_success() {
        return Err(format!("CVAT server rejected the request: HTTP {}", response.status()));
    }

    Ok(())
}

pub fn create_task(
    settings: &CvatSettings,
    task_name: &str,
    labels: &[String],
) -> Result<RemoteTaskInfo, String> {
    if labels.is_empty() {
        return Err("CVAT task requires at least one label; complete import review or create unified labels first".into());
    }

    let client = authorized_client(settings)?;
    let base_url = normalize_base_url(&settings.base_url);
    let label_payload = labels
        .iter()
        .map(|name| json!({ "name": name }))
        .collect::<Vec<_>>();

    let response = client
        .post(format!("{base_url}/api/tasks"))
        .header("Authorization", auth_header_value(settings))
        .json(&json!({
            "name": task_name,
            "labels": label_payload,
        }))
        .send()
        .map_err(|error| format!("failed to create CVAT task: {error}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(format!("CVAT task creation failed: HTTP {status} {body}"));
    }

    let payload = response
        .json::<serde_json::Value>()
        .map_err(|error| format!("failed to parse CVAT task creation response: {error}"))?;
    let remote_task_id = payload
        .get("id")
        .and_then(|value| value.as_i64())
        .ok_or_else(|| "CVAT task creation response did not include task id".to_string())?;

    Ok(RemoteTaskInfo {
        remote_task_id,
        remote_url: format!("{base_url}/tasks/{remote_task_id}"),
    })
}

pub fn upload_task_images(
    settings: &CvatSettings,
    remote_task_id: i64,
    image_paths: &[&Path],
) -> Result<(), String> {
    if image_paths.is_empty() {
        return Err("no staged images are available for CVAT upload".into());
    }

    let client = authorized_client(settings)?;
    let base_url = normalize_base_url(&settings.base_url);
    let mut form = multipart::Form::new()
        .text("image_quality", "70")
        .text("sorting_method", "lexicographical");

    for image_path in image_paths {
        let file_name = image_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("image.bin")
            .to_string();
        let bytes = fs::read(image_path)
            .map_err(|error| format!("failed to read staged image for CVAT upload: {error}"))?;
        let part = multipart::Part::bytes(bytes).file_name(file_name);
        form = form.part("client_files", part);
    }

    let response = client
        .post(format!("{base_url}/api/tasks/{remote_task_id}/data"))
        .header("Authorization", auth_header_value(settings))
        .multipart(form)
        .send()
        .map_err(|error| format!("failed to upload images to CVAT task: {error}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(format!("CVAT image upload failed: HTTP {status} {body}"));
    }

    let payload = response
        .json::<serde_json::Value>()
        .map_err(|error| format!("failed to parse CVAT upload response: {error}"))?;
    let request_id = payload
        .get("rq_id")
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| payload.get("rq_id").and_then(|value| value.as_i64()).map(|value| value.to_string()))
        .ok_or_else(|| "CVAT upload response did not include rq_id".to_string())?;

    wait_for_request(settings, &request_id)
}

fn wait_for_request(settings: &CvatSettings, request_id: &str) -> Result<(), String> {
    let client = authorized_client(settings)?;
    let base_url = normalize_base_url(&settings.base_url);

    for _ in 0..60 {
        let response = client
            .get(format!("{base_url}/api/requests/{request_id}"))
            .header("Authorization", auth_header_value(settings))
            .send()
            .map_err(|error| format!("failed to poll CVAT request status: {error}"))?;

        if !response.status().is_success() {
            return Err(format!("CVAT request status polling failed: HTTP {}", response.status()));
        }

        let payload = response
            .json::<serde_json::Value>()
            .map_err(|error| format!("failed to parse CVAT request status response: {error}"))?;
        let status = payload
            .get("status")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_lowercase();

        if status == "finished" {
            return Ok(());
        }
        if status == "failed" {
            let message = payload
                .get("message")
                .and_then(|value| value.as_str())
                .unwrap_or("unknown CVAT request failure");
            return Err(format!("CVAT background request failed: {message}"));
        }

        thread::sleep(Duration::from_secs(2));
    }

    Err("timed out while waiting for CVAT to finish processing uploaded task data".into())
}
