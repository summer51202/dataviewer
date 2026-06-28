use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeBackend {
    Cuda,
    WindowsGpu,
    Cpu,
}

impl RuntimeBackend {
    pub fn as_str(self) -> &'static str {
        match self {
            RuntimeBackend::Cuda => "cuda",
            RuntimeBackend::WindowsGpu => "windows-gpu",
            RuntimeBackend::Cpu => "cpu",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeProbeResult {
    pub selected_backend: RuntimeBackend,
    pub fallback_reason: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmbeddingModelDefinition {
    pub id: String,
    pub family: String,
    pub display_name: String,
    pub embedding_dim: u32,
    pub input_size: u32,
    pub onnx_file_name: String,
}

impl EmbeddingModelDefinition {
    pub fn new(
        id: &str,
        family: &str,
        display_name: &str,
        embedding_dim: u32,
        input_size: u32,
    ) -> Self {
        Self {
            id: id.to_string(),
            family: family.to_string(),
            display_name: display_name.to_string(),
            embedding_dim,
            input_size,
            onnx_file_name: format!("{id}.onnx"),
        }
    }
}

pub fn default_model_registry() -> Vec<EmbeddingModelDefinition> {
    vec![
        EmbeddingModelDefinition::new("clip-vit-b32", "clip", "CLIP ViT-B/32", 512, 224),
        EmbeddingModelDefinition::new("dinov2-small", "dinov2", "DINOv2 Small", 384, 224),
    ]
}

pub fn resolve_model_asset(workspace_root: &Path, model_id: &str) -> PathBuf {
    workspace_root
        .join(".dataviewer")
        .join("models")
        .join(format!("{model_id}.onnx"))
}

pub fn provider_order(preference: &str) -> Vec<RuntimeBackend> {
    match preference {
        "cuda" => vec![RuntimeBackend::Cuda, RuntimeBackend::Cpu],
        "windows-gpu" => vec![RuntimeBackend::WindowsGpu, RuntimeBackend::Cpu],
        "cpu" => vec![RuntimeBackend::Cpu],
        _ => vec![
            RuntimeBackend::Cuda,
            RuntimeBackend::WindowsGpu,
            RuntimeBackend::Cpu,
        ],
    }
}

pub fn probe_runtime(preference: &str) -> RuntimeProbeResult {
    let fallback_reason = match provider_order(preference).as_slice() {
        [RuntimeBackend::Cpu] => None,
        candidates => {
            let attempted = candidates
                .iter()
                .filter(|backend| **backend != RuntimeBackend::Cpu)
                .map(|backend| backend.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            Some(format!(
                "ONNX Runtime provider probing is not enabled yet; attempted {attempted}; using CPU."
            ))
        }
    };

    RuntimeProbeResult {
        selected_backend: RuntimeBackend::Cpu,
        fallback_reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_runtime_returns_stable_cpu_fallback() {
        let result = probe_runtime("cuda");

        assert_eq!(result.selected_backend, RuntimeBackend::Cpu);
        assert!(result.fallback_reason.is_some());
    }

    #[test]
    fn default_model_registry_includes_clip_and_dinov2() {
        let registry = default_model_registry();

        assert_eq!(registry.len(), 2);
        assert_eq!(registry[0].id, "clip-vit-b32");
        assert_eq!(registry[0].family, "clip");
        assert_eq!(registry[0].embedding_dim, 512);
        assert_eq!(registry[1].id, "dinov2-small");
        assert_eq!(registry[1].family, "dinov2");
        assert_eq!(registry[1].embedding_dim, 384);
    }

    #[test]
    fn resolve_model_asset_prefers_workspace_model_cache() {
        let path = resolve_model_asset(std::path::Path::new("D:\\workspace"), "clip-vit-b32");

        assert_eq!(
            path,
            std::path::PathBuf::from("D:\\workspace")
                .join(".dataviewer")
                .join("models")
                .join("clip-vit-b32.onnx")
        );
    }

    #[test]
    fn provider_order_matches_runtime_preference() {
        assert_eq!(
            provider_order("auto"),
            vec![RuntimeBackend::Cuda, RuntimeBackend::WindowsGpu, RuntimeBackend::Cpu]
        );
        assert_eq!(
            provider_order("cuda"),
            vec![RuntimeBackend::Cuda, RuntimeBackend::Cpu]
        );
        assert_eq!(
            provider_order("windows-gpu"),
            vec![RuntimeBackend::WindowsGpu, RuntimeBackend::Cpu]
        );
        assert_eq!(provider_order("cpu"), vec![RuntimeBackend::Cpu]);
    }
}
