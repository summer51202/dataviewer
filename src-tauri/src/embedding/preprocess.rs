use crate::embedding::jobs::EmbeddingJobItem;
use crate::embedding::runtime::EmbeddingModelDefinition;
use image::{imageops, RgbImage};

#[derive(Clone, Debug, PartialEq)]
pub struct PreprocessedTensor {
    pub shape: [usize; 4],
    pub values: Vec<f32>,
}

pub fn preprocess_item(
    item: &EmbeddingJobItem,
    model: &EmbeddingModelDefinition,
) -> Result<PreprocessedTensor, String> {
    let image = image::open(&item.original_path)
        .map_err(|error| format!("failed to decode image '{}': {error}", item.original_path))?
        .to_rgb8();
    preprocess_item_from_rgb(&image, item, model)
}

pub fn preprocess_item_from_rgb(
    image: &RgbImage,
    item: &EmbeddingJobItem,
    model: &EmbeddingModelDefinition,
) -> Result<PreprocessedTensor, String> {
    let cropped = crop_image(&image, item);
    let size = model.input_size.max(1);
    let resized = imageops::resize(&cropped, size, size, imageops::FilterType::Triangle);
    let (mean, std) = normalization_for_family(&model.family);
    let mut values = Vec::with_capacity((3 * size * size) as usize);

    for channel in 0..3 {
        for y in 0..size {
            for x in 0..size {
                let pixel = resized.get_pixel(x, y);
                let normalized = pixel[channel] as f32 / 255.0;
                values.push((normalized - mean[channel]) / std[channel]);
            }
        }
    }

    Ok(PreprocessedTensor {
        shape: [1, 3, size as usize, size as usize],
        values,
    })
}

fn crop_image(image: &RgbImage, item: &EmbeddingJobItem) -> RgbImage {
    let Some(bbox) = &item.bbox else {
        return image.clone();
    };

    let image_width = image.width() as f64;
    let image_height = image.height() as f64;
    let left = bbox.x.max(0.0).min(image_width - 1.0).floor() as u32;
    let top = bbox.y.max(0.0).min(image_height - 1.0).floor() as u32;
    let right = (bbox.x + bbox.width)
        .max(left as f64 + 1.0)
        .min(image_width)
        .ceil() as u32;
    let bottom = (bbox.y + bbox.height)
        .max(top as f64 + 1.0)
        .min(image_height)
        .ceil() as u32;
    let width = right.saturating_sub(left).max(1);
    let height = bottom.saturating_sub(top).max(1);

    imageops::crop_imm(image, left, top, width, height).to_image()
}

fn normalization_for_family(family: &str) -> ([f32; 3], [f32; 3]) {
    if family.eq_ignore_ascii_case("clip") {
        (
            [0.48145466, 0.4578275, 0.40821073],
            [0.26862954, 0.26130258, 0.27577711],
        )
    } else {
        ([0.485, 0.456, 0.406], [0.229, 0.224, 0.225])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedding::jobs::CropRect;
    use image::{Rgb, RgbImage};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn make_temp_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("dataviewer-preprocess-{name}-{unique}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_test_image(name: &str, width: u32, height: u32, color: [u8; 3]) -> (PathBuf, PathBuf) {
        let dir = make_temp_dir(name);
        let image_path = dir.join("sample.png");
        let image = RgbImage::from_pixel(width, height, Rgb(color));
        image.save(&image_path).unwrap();
        (dir, image_path)
    }

    fn item_for(image_path: &PathBuf, bbox: Option<CropRect>) -> EmbeddingJobItem {
        EmbeddingJobItem {
            target_id: "target-1".to_string(),
            image_id: "img-1".to_string(),
            annotation_id: bbox.as_ref().map(|_| "ann-1".to_string()),
            original_path: image_path.to_string_lossy().to_string(),
            bbox,
        }
    }

    fn model(family: &str, size: u32) -> EmbeddingModelDefinition {
        EmbeddingModelDefinition::new("test-model", family, "Test Model", 3, size)
    }

    #[test]
    fn full_image_preprocess_returns_nchw_tensor_shape() {
        let (dir, image_path) = write_test_image("full", 3, 2, [128, 64, 32]);
        let tensor = preprocess_item(&item_for(&image_path, None), &model("clip", 4)).unwrap();

        assert_eq!(tensor.shape, [1, 3, 4, 4]);
        assert_eq!(tensor.values.len(), 1 * 3 * 4 * 4);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn object_crop_clamps_bbox_to_image_bounds() {
        let (dir, image_path) = write_test_image("crop", 4, 4, [255, 0, 0]);
        let tensor = preprocess_item(
            &item_for(
                &image_path,
                Some(CropRect {
                    x: -2.0,
                    y: -2.0,
                    width: 3.0,
                    height: 3.0,
                }),
            ),
            &model("clip", 2),
        )
        .unwrap();

        assert_eq!(tensor.shape, [1, 3, 2, 2]);
        assert_eq!(tensor.values.len(), 12);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn clip_and_dino_normalization_differ_for_same_pixel() {
        let (dir, image_path) = write_test_image("normalize", 1, 1, [128, 128, 128]);
        let item = item_for(&image_path, None);
        let clip = preprocess_item(&item, &model("clip", 1)).unwrap();
        let dino = preprocess_item(&item, &model("dinov2", 1)).unwrap();

        assert_ne!(clip.values, dino.values);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn unreadable_image_returns_clear_error() {
        let dir = make_temp_dir("invalid");
        let image_path = dir.join("broken.png");
        fs::write(&image_path, b"not an image").unwrap();

        let error = preprocess_item(&item_for(&image_path, None), &model("clip", 1)).unwrap_err();

        assert!(error.contains("failed to decode image"));

        let _ = fs::remove_dir_all(dir);
    }
}
