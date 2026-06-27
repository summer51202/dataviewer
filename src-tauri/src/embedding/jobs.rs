#[cfg(test)]
use std::collections::hash_map::DefaultHasher;
#[cfg(test)]
use std::hash::{Hash, Hasher};

use crate::embedding::runtime::RuntimeBackend;

#[derive(Clone, Debug, PartialEq)]
pub struct CropRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EmbeddingJobItem {
    pub target_id: String,
    pub image_id: String,
    pub annotation_id: Option<String>,
    pub original_path: String,
    pub bbox: Option<CropRect>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EmbeddingRecord {
    pub workspace_id: String,
    pub scope: String,
    pub target_id: String,
    pub image_id: String,
    pub annotation_id: Option<String>,
    pub model_id: String,
    pub runtime_backend: String,
    pub vector: Vec<f32>,
    pub created_at: String,
}

pub fn batch_size_for_backend(backend: RuntimeBackend) -> usize {
    match backend {
        RuntimeBackend::Cpu => 8,
        RuntimeBackend::WindowsGpu => 16,
        RuntimeBackend::Cuda => 32,
    }
}

pub fn serialize_f32_vector(values: &[f32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

#[cfg(test)]
pub fn deterministic_embedding_for_item(
    item: &EmbeddingJobItem,
    model_id: &str,
    dimensions: usize,
) -> Vec<f32> {
    (0..dimensions)
        .map(|dimension| {
            let mut hasher = DefaultHasher::new();
            model_id.hash(&mut hasher);
            item.target_id.hash(&mut hasher);
            item.image_id.hash(&mut hasher);
            item.annotation_id.hash(&mut hasher);
            item.original_path.hash(&mut hasher);
            if let Some(bbox) = &item.bbox {
                bbox.x.to_bits().hash(&mut hasher);
                bbox.y.to_bits().hash(&mut hasher);
                bbox.width.to_bits().hash(&mut hasher);
                bbox.height.to_bits().hash(&mut hasher);
            }
            dimension.hash(&mut hasher);

            let normalized = hasher.finish() as f64 / u64::MAX as f64;
            (normalized.mul_add(2.0, -1.0)) as f32
        })
        .collect()
}

pub fn l2_normalize_vector(values: Vec<f32>) -> Vec<f32> {
    let norm = values.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm <= f32::EPSILON {
        return values;
    }

    values.into_iter().map(|value| value / norm).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedding::runtime::RuntimeBackend;

    #[test]
    fn batch_size_matches_runtime_backend() {
        assert_eq!(batch_size_for_backend(RuntimeBackend::Cpu), 8);
        assert_eq!(batch_size_for_backend(RuntimeBackend::WindowsGpu), 16);
        assert_eq!(batch_size_for_backend(RuntimeBackend::Cuda), 32);
    }

    #[test]
    fn serialize_f32_vector_uses_little_endian_bytes() {
        let bytes = serialize_f32_vector(&[1.0, -2.5]);

        assert_eq!(bytes, [0x00, 0x00, 0x80, 0x3f, 0x00, 0x00, 0x20, 0xc0]);
    }

    #[test]
    fn deterministic_embedding_uses_item_identity_and_requested_dimensions() {
        let item = EmbeddingJobItem {
            target_id: "ann-1".to_string(),
            image_id: "img-1".to_string(),
            annotation_id: Some("ann-1".to_string()),
            original_path: "C:\\dataset\\part-a.jpg".to_string(),
            bbox: Some(CropRect {
                x: 10.0,
                y: 20.0,
                width: 30.0,
                height: 40.0,
            }),
        };

        let first = deterministic_embedding_for_item(&item, "clip-vit-b32", 4);
        let second = deterministic_embedding_for_item(&item, "clip-vit-b32", 4);
        let changed_model = deterministic_embedding_for_item(&item, "dinov2-small", 4);

        assert_eq!(first.len(), 4);
        assert_eq!(first, second);
        assert_ne!(first, changed_model);
        assert!(first.iter().all(|value| (-1.0..=1.0).contains(value)));
    }

    #[test]
    fn l2_normalize_vector_scales_non_zero_vectors() {
        let normalized = l2_normalize_vector(vec![3.0, 4.0]);

        assert!((normalized[0] - 0.6).abs() < 0.0001);
        assert!((normalized[1] - 0.8).abs() < 0.0001);
    }

    #[test]
    fn l2_normalize_vector_preserves_zero_vectors() {
        assert_eq!(l2_normalize_vector(vec![0.0, 0.0]), vec![0.0, 0.0]);
    }
}
