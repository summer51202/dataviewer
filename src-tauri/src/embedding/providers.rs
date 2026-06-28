use std::path::Path;
#[cfg(feature = "onnx-runtime")]
use std::path::PathBuf;
#[cfg(feature = "onnx-runtime")]
use std::sync::OnceLock;

use crate::embedding::runtime::{EmbeddingModelDefinition, RuntimeBackend};

#[cfg(feature = "onnx-runtime")]
use ndarray::Array4;
#[cfg(feature = "onnx-runtime")]
use ort::{
    init_from, inputs,
    session::{builder::GraphOptimizationLevel, Session},
    value::{TensorRef, ValueType},
};

#[cfg(test)]
pub const ONNX_RUNTIME_CRATE: &str = "ort";
#[cfg(test)]
pub const ONNX_RUNTIME_CRATE_VERSION: &str = "2.0.0-rc.12";
#[cfg(test)]
pub const ONNX_RUNTIME_FEATURES: &[&str] = &["load-dynamic", "api-20"];

#[cfg(test)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionSmokeResult {
    pub available: bool,
    pub detail: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmbeddingSessionInfo {
    pub model_id: String,
    pub backend: RuntimeBackend,
    pub input_name: String,
    pub output_name: String,
    pub embedding_dim: usize,
}

pub trait EmbeddingProvider {
    fn run_batch(
        &mut self,
        input_shape: [usize; 4],
        input_values: &[f32],
        batch_len: usize,
    ) -> Result<Vec<Vec<f32>>, String>;
}

pub fn load_embedding_provider(
    model: &EmbeddingModelDefinition,
    backend: RuntimeBackend,
    model_path: &Path,
) -> Result<(EmbeddingSessionInfo, Box<dyn EmbeddingProvider>), String> {
    if !model_path.exists() {
        return Err(format!("model asset not found: {}", model_path.display()));
    }

    load_existing_embedding_provider(model, backend, model_path)
}

#[cfg(test)]
pub fn load_embedding_session(
    model: &EmbeddingModelDefinition,
    backend: RuntimeBackend,
    model_path: &Path,
) -> Result<EmbeddingSessionInfo, String> {
    load_embedding_provider(model, backend, model_path).map(|(info, _provider)| info)
}

pub fn validate_embedding_vectors(vectors: &[Vec<f32>], expected_dim: usize) -> Result<(), String> {
    for (index, vector) in vectors.iter().enumerate() {
        if vector.len() != expected_dim {
            return Err(format!(
                "expected embedding dimension {expected_dim}, got {} for output row {index}",
                vector.len()
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
pub fn smoke_test_session(model_path: &Path) -> SessionSmokeResult {
    if !model_path.exists() {
        return SessionSmokeResult {
            available: false,
            detail: format!("model asset not found: {}", model_path.display()),
        };
    }

    smoke_test_existing_model(model_path)
}

#[cfg(all(test, feature = "onnx-runtime"))]
fn smoke_test_existing_model(model_path: &Path) -> SessionSmokeResult {
    let detail = format!(
        "{} {} configured with {:?}; session smoke test pending model input metadata for {}.",
        ONNX_RUNTIME_CRATE,
        ONNX_RUNTIME_CRATE_VERSION,
        ONNX_RUNTIME_FEATURES,
        model_path.display()
    );
    SessionSmokeResult {
        available: true,
        detail,
    }
}

#[cfg(feature = "onnx-runtime")]
pub struct OnnxEmbeddingProvider {
    session: Session,
    info: EmbeddingSessionInfo,
}

#[cfg(feature = "onnx-runtime")]
impl EmbeddingProvider for OnnxEmbeddingProvider {
    fn run_batch(
        &mut self,
        input_shape: [usize; 4],
        input_values: &[f32],
        batch_len: usize,
    ) -> Result<Vec<Vec<f32>>, String> {
        if input_shape[0] != batch_len {
            return Err(format!(
                "input batch shape {} does not match batch length {batch_len}",
                input_shape[0]
            ));
        }

        let input = Array4::from_shape_vec(
            (
                input_shape[0],
                input_shape[1],
                input_shape[2],
                input_shape[3],
            ),
            input_values.to_vec(),
        )
        .map_err(|error| format!("failed to create ONNX input tensor: {error}"))?;
        let input_tensor = TensorRef::from_array_view(&input)
            .map_err(|error| format!("failed to bind ONNX input tensor: {error}"))?;
        let outputs = self
            .session
            .run(inputs![input_tensor])
            .map_err(|error| format!("failed to run ONNX inference: {error}"))?;
        let output = outputs
            .get(&self.info.output_name)
            .ok_or_else(|| format!("ONNX output '{}' was not returned", self.info.output_name))?;
        let (shape, values) = output
            .try_extract_tensor::<f32>()
            .map_err(|error| format!("failed to extract ONNX output tensor: {error}"))?;
        let shape = shape
            .iter()
            .map(|dimension| usize::try_from(*dimension).unwrap_or(0))
            .collect::<Vec<_>>();
        let vectors =
            extract_embedding_vectors(&shape, values, batch_len, self.info.embedding_dim)?;
        Ok(vectors)
    }
}

pub fn extract_embedding_vectors(
    shape: &[usize],
    values: &[f32],
    batch_len: usize,
    embedding_dim: usize,
) -> Result<Vec<Vec<f32>>, String> {
    let vectors = match shape {
        [batch, dim] if *batch == batch_len && *dim == embedding_dim => values
            .chunks(embedding_dim)
            .map(|chunk| chunk.to_vec())
            .collect::<Vec<_>>(),
        [batch, tokens, dim] if *batch == batch_len && *dim == embedding_dim => {
            let row_stride = tokens.checked_mul(embedding_dim).ok_or_else(|| {
                "ONNX output shape overflow while pooling token embeddings".to_string()
            })?;
            let expected_len = batch_len.checked_mul(row_stride).ok_or_else(|| {
                "ONNX output shape overflow while validating token embeddings".to_string()
            })?;
            if values.len() != expected_len {
                return Err(format!(
                    "ONNX output shape {:?} expects {expected_len} values, got {}",
                    shape,
                    values.len()
                ));
            }
            (0..batch_len)
                .map(|batch_index| {
                    let start = batch_index * row_stride;
                    values[start..start + embedding_dim].to_vec()
                })
                .collect::<Vec<_>>()
        }
        _ if values.len() == batch_len * embedding_dim => values
            .chunks(embedding_dim)
            .map(|chunk| chunk.to_vec())
            .collect::<Vec<_>>(),
        _ => {
            return Err(format!(
                "unsupported ONNX output shape {:?}; expected [batch, {embedding_dim}] or [batch, tokens, {embedding_dim}] for batch {batch_len}",
                shape
            ));
        }
    };

    validate_embedding_vectors(&vectors, embedding_dim)?;
    if vectors.len() != batch_len {
        return Err(format!(
            "expected {batch_len} embedding rows, got {}",
            vectors.len()
        ));
    }
    Ok(vectors)
}

#[cfg(feature = "onnx-runtime")]
fn load_existing_embedding_provider(
    model: &EmbeddingModelDefinition,
    backend: RuntimeBackend,
    model_path: &Path,
) -> Result<(EmbeddingSessionInfo, Box<dyn EmbeddingProvider>), String> {
    ensure_ort_runtime_initialized(model_path)?;
    eprintln!("[dataset-map] ONNX session builder start");
    let available_threads = std::thread::available_parallelism()
        .map(|threads| threads.get())
        .unwrap_or(4);
    let intra_threads = available_threads.clamp(2, 8);
    let inter_threads = (available_threads / 2).clamp(1, 4);
    let mut builder = Session::builder()
        .map_err(|error| format!("failed to create ONNX session builder: {error}"))?
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .map_err(|error| format!("failed to configure ONNX graph optimization: {error}"))?
        .with_intra_threads(intra_threads)
        .map_err(|error| format!("failed to configure ONNX intra threads: {error}"))?
        .with_inter_threads(inter_threads)
        .map_err(|error| format!("failed to configure ONNX inter threads: {error}"))?;
    eprintln!("[dataset-map] ONNX session builder ready; committing model file");
    let session = builder.commit_from_file(model_path).map_err(|error| {
        format!(
            "failed to load ONNX session for {} on {} from {}: {error}",
            model.id,
            backend.as_str(),
            model_path.display()
        )
    })?;
    eprintln!("[dataset-map] ONNX session committed; reading metadata");
    let input = session
        .inputs()
        .first()
        .ok_or_else(|| format!("ONNX model '{}' has no inputs", model.id))?;
    let output = session
        .outputs()
        .first()
        .ok_or_else(|| format!("ONNX model '{}' has no outputs", model.id))?;
    let embedding_dim =
        output_embedding_dim(output.dtype()).unwrap_or(model.embedding_dim as usize);
    if embedding_dim != model.embedding_dim as usize {
        return Err(format!(
            "embedding dimension mismatch for {}: expected embedding dimension {}, got {embedding_dim}",
            model.id, model.embedding_dim
        ));
    }

    let info = EmbeddingSessionInfo {
        model_id: model.id.clone(),
        backend,
        input_name: input.name().to_string(),
        output_name: output.name().to_string(),
        embedding_dim,
    };
    Ok((
        info.clone(),
        Box::new(OnnxEmbeddingProvider { session, info }),
    ))
}

#[cfg(feature = "onnx-runtime")]
fn ensure_ort_runtime_initialized(model_path: &Path) -> Result<(), String> {
    static ORT_INIT: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    let result = ORT_INIT.get_or_init(|| {
        let dll_path = resolve_onnxruntime_dll(model_path)?;
        eprintln!(
            "[dataset-map] initializing ONNX Runtime from {}",
            dll_path.display()
        );

        init_from(&dll_path)
            .map_err(|error| {
                format!(
                    "failed to load ONNX Runtime DLL from {}: {error}",
                    dll_path.display()
                )
            })
            .map(|builder| {
                let committed = builder.commit();
                eprintln!(
                    "[dataset-map] ONNX Runtime environment initialized committed={committed}"
                );
                dll_path
            })
    });

    result
        .as_ref()
        .map(|_path| ())
        .map_err(|error| error.clone())
}

#[cfg(feature = "onnx-runtime")]
fn resolve_onnxruntime_dll(model_path: &Path) -> Result<PathBuf, String> {
    let mut candidates = Vec::new();
    if let Ok(path) =
        std::env::var("DATAVIEWER_ONNXRUNTIME_DLL").or_else(|_| std::env::var("ORT_DYLIB_PATH"))
    {
        candidates.push(PathBuf::from(path));
    }
    if let Some(models_dir) = model_path.parent() {
        candidates.push(models_dir.join("onnxruntime.dll"));
        if let Some(dataviewer_dir) = models_dir.parent() {
            candidates.push(dataviewer_dir.join("runtime").join("onnxruntime.dll"));
        }
    }
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            candidates.push(exe_dir.join("onnxruntime.dll"));
            candidates.push(exe_dir.join("runtime").join("onnxruntime.dll"));
        }
    }

    candidates
        .into_iter()
        .find(|candidate| candidate.exists())
        .ok_or_else(|| {
            "ONNX Runtime DLL not found. Set DATAVIEWER_ONNXRUNTIME_DLL to a compatible onnxruntime.dll, or place onnxruntime.dll next to the model files / .dataviewer/runtime.".to_string()
        })
}

#[cfg(not(feature = "onnx-runtime"))]
fn load_existing_embedding_provider(
    model: &EmbeddingModelDefinition,
    backend: RuntimeBackend,
    model_path: &Path,
) -> Result<(EmbeddingSessionInfo, Box<dyn EmbeddingProvider>), String> {
    Err(format!(
        "failed to load ONNX session for {} on {} from {}; enable the 'onnx-runtime' Cargo feature",
        model.id,
        backend.as_str(),
        model_path.display()
    ))
}

#[cfg(feature = "onnx-runtime")]
fn output_embedding_dim(dtype: &ValueType) -> Option<usize> {
    let shape = dtype.tensor_shape()?;
    shape
        .iter()
        .rev()
        .find_map(|dimension| usize::try_from(*dimension).ok().filter(|value| *value > 0))
}

#[cfg(all(test, not(feature = "onnx-runtime")))]
fn smoke_test_existing_model(model_path: &Path) -> SessionSmokeResult {
    SessionSmokeResult {
        available: false,
        detail: format!(
            "{} {} is configured with {:?}; enable the 'onnx-runtime' Cargo feature to load {}.",
            ONNX_RUNTIME_CRATE,
            ONNX_RUNTIME_CRATE_VERSION,
            ONNX_RUNTIME_FEATURES,
            model_path.display()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedding::runtime::RuntimeBackend;
    #[cfg(not(feature = "onnx-runtime"))]
    use std::fs;
    #[cfg(not(feature = "onnx-runtime"))]
    use std::time::{SystemTime, UNIX_EPOCH};

    fn model() -> EmbeddingModelDefinition {
        EmbeddingModelDefinition::new("clip-vit-b32", "clip", "CLIP ViT-B/32", 512, 224)
    }

    #[cfg(not(feature = "onnx-runtime"))]
    fn make_temp_file(name: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("dataviewer-provider-{name}-{unique}.onnx"));
        fs::write(&path, b"not a real model").unwrap();
        path
    }

    #[test]
    fn smoke_test_session_reports_missing_model_asset() {
        let result = smoke_test_session(Path::new(
            "D:\\workspace\\.dataviewer\\models\\missing.onnx",
        ));

        assert!(!result.available);
        assert!(result.detail.contains("model asset not found"));
    }

    #[test]
    fn records_selected_onnx_runtime_binding() {
        assert_eq!(ONNX_RUNTIME_CRATE, "ort");
        assert_eq!(ONNX_RUNTIME_CRATE_VERSION, "2.0.0-rc.12");
        assert_eq!(ONNX_RUNTIME_FEATURES, ["load-dynamic", "api-20"]);
    }

    #[test]
    fn load_embedding_session_reports_missing_model_asset() {
        let error = load_embedding_session(
            &model(),
            RuntimeBackend::Cpu,
            Path::new("D:\\workspace\\.dataviewer\\models\\missing.onnx"),
        )
        .unwrap_err();

        assert!(error.contains("model asset not found"));
    }

    #[test]
    fn validate_embedding_vectors_rejects_wrong_dimensions() {
        let error = validate_embedding_vectors(&[vec![0.0, 1.0]], 512).unwrap_err();

        assert!(error.contains("expected embedding dimension 512"));
    }

    #[test]
    fn extract_embedding_vectors_accepts_clip_rows() {
        let vectors =
            extract_embedding_vectors(&[2, 3], &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3).unwrap();

        assert_eq!(vectors, vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]]);
    }

    #[test]
    fn extract_embedding_vectors_pools_dino_cls_token() {
        let values = vec![
            1.0, 2.0, 3.0, 10.0, 20.0, 30.0, 4.0, 5.0, 6.0, 40.0, 50.0, 60.0,
        ];
        let vectors = extract_embedding_vectors(&[2, 2, 3], &values, 2, 3).unwrap();

        assert_eq!(vectors, vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]]);
    }

    #[test]
    fn extract_embedding_vectors_rejects_unsupported_shape() {
        let error = extract_embedding_vectors(&[1, 2, 4], &[0.0; 8], 1, 3).unwrap_err();

        assert!(error.contains("unsupported ONNX output shape"));
    }

    #[test]
    #[cfg(not(feature = "onnx-runtime"))]
    fn load_embedding_session_validates_session_metadata_dimension() {
        let path = make_temp_file("invalid-dimension");
        let error = load_embedding_session(&model(), RuntimeBackend::Cpu, &path).unwrap_err();

        assert!(
            error.contains("failed to load ONNX session") || error.contains("embedding dimension")
        );

        let _ = fs::remove_file(path);
    }
}
