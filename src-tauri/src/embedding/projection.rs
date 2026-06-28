pub fn deterministic_projection(seed: &str) -> (f64, f64) {
    let hash = stable_hash(seed.as_bytes());
    // Route BOTH coordinates through splitmix64 (with distinct seeds) so the raw
    // FNV-1a high bits don't band the x axis into a few columns for sequential ids.
    let hx = splitmix64(hash ^ 0x2545_f491_4f6c_dd1d);
    let hy = splitmix64(hash ^ 0x9e37_79b9_7f4a_7c15);
    let x = unit_from_hash(hx) * 2.0 - 1.0;
    let y = unit_from_hash(hy) * 2.0 - 1.0;
    (x, y)
}

fn stable_hash(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    hash
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn unit_from_hash(value: u64) -> f64 {
    let mantissa = value >> 11;
    mantissa as f64 / ((1_u64 << 53) - 1) as f64
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProjectionInput {
    pub target_id: String,
    pub vector: Vec<f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProjectionPoint {
    pub target_id: String,
    pub x: f64,
    pub y: f64,
}

pub fn pca_projection(inputs: &[ProjectionInput]) -> Vec<ProjectionPoint> {
    if inputs.is_empty() {
        return Vec::new();
    }

    let dimensions = inputs
        .iter()
        .map(|input| input.vector.len())
        .max()
        .unwrap_or(0);
    if dimensions == 0 {
        return inputs
            .iter()
            .map(|input| ProjectionPoint {
                target_id: input.target_id.clone(),
                x: 0.0,
                y: 0.0,
            })
            .collect();
    }

    let mut means = vec![0.0; dimensions];
    for input in inputs {
        for (index, value) in input.vector.iter().enumerate() {
            means[index] += *value as f64;
        }
    }
    for mean in &mut means {
        *mean /= inputs.len() as f64;
    }

    let centered = inputs
        .iter()
        .map(|input| {
            (0..dimensions)
                .map(|index| input.vector.get(index).copied().unwrap_or(0.0) as f64 - means[index])
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let first_component = principal_component(&centered, None);
    let second_component = principal_component(&centered, Some(&first_component));
    let x_scores = centered
        .iter()
        .map(|vector| dot(vector, &first_component))
        .collect::<Vec<_>>();
    let y_scores = centered
        .iter()
        .map(|vector| dot(vector, &second_component))
        .collect::<Vec<_>>();
    let x_scale = max_abs(&x_scores);
    let y_scale = max_abs(&y_scores);

    inputs
        .iter()
        .zip(x_scores.iter().zip(y_scores.iter()))
        .map(|(input, (x, y))| ProjectionPoint {
            target_id: input.target_id.clone(),
            x: normalize_score(*x, x_scale),
            y: normalize_score(*y, y_scale),
        })
        .collect()
}

fn principal_component(vectors: &[Vec<f64>], orthogonal_to: Option<&[f64]>) -> Vec<f64> {
    let dimensions = vectors.first().map(|vector| vector.len()).unwrap_or(0);
    if dimensions == 0 {
        return Vec::new();
    }

    let mut component = (0..dimensions)
        .map(|index| if index % 2 == 0 { 1.0 } else { -1.0 })
        .collect::<Vec<_>>();
    if let Some(existing) = orthogonal_to {
        orthogonalize(&mut component, existing);
    }
    normalize_vector(&mut component);

    for _ in 0..32 {
        let mut next = vec![0.0; dimensions];
        for vector in vectors {
            let score = dot(vector, &component);
            for (index, value) in vector.iter().enumerate() {
                next[index] += value * score;
            }
        }
        if let Some(existing) = orthogonal_to {
            orthogonalize(&mut next, existing);
        }
        if !normalize_vector(&mut next) {
            return vec![0.0; dimensions];
        }
        component = next;
    }

    component
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right.iter())
        .map(|(left, right)| left * right)
        .sum()
}

fn orthogonalize(vector: &mut [f64], existing: &[f64]) {
    let overlap = dot(vector, existing);
    for (value, existing_value) in vector.iter_mut().zip(existing.iter()) {
        *value -= overlap * existing_value;
    }
}

fn normalize_vector(vector: &mut [f64]) -> bool {
    let norm = vector.iter().map(|value| value * value).sum::<f64>().sqrt();
    if norm <= f64::EPSILON {
        return false;
    }
    for value in vector {
        *value /= norm;
    }
    true
}

fn max_abs(values: &[f64]) -> f64 {
    values
        .iter()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max)
}

fn normalize_score(value: f64, scale: f64) -> f64 {
    if scale <= f64::EPSILON {
        0.0
    } else {
        (value / scale).clamp(-1.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_projection_is_stable_and_bounded() {
        let first = deterministic_projection("object:clip-vit-b32:ann-1");
        let second = deterministic_projection("object:clip-vit-b32:ann-1");

        assert_eq!(first, second);
        assert!((-1.0..=1.0).contains(&first.0));
        assert!((-1.0..=1.0).contains(&first.1));
    }

    #[test]
    fn deterministic_projection_spreads_similar_ids() {
        let points = (0..64)
            .map(|index| deterministic_projection(&format!("object:fast-preview:ann-{index}")))
            .collect::<Vec<_>>();
        let unique_cells = points
            .iter()
            .map(|(x, y)| ((x * 20.0).round() as i32, (y * 20.0).round() as i32))
            .collect::<std::collections::HashSet<_>>();

        assert!(unique_cells.len() > 56);

        // Each axis must spread on its own — guards against one axis banding into
        // a few columns/rows for sequential ids (regression guard).
        let unique_x = points
            .iter()
            .map(|(x, _)| (x * 20.0).round() as i32)
            .collect::<std::collections::HashSet<_>>();
        let unique_y = points
            .iter()
            .map(|(_, y)| (y * 20.0).round() as i32)
            .collect::<std::collections::HashSet<_>>();
        assert!(unique_x.len() > 24, "x axis is banded: {} bins", unique_x.len());
        assert!(unique_y.len() > 24, "y axis is banded: {} bins", unique_y.len());
    }

    #[test]
    fn pca_projection_is_stable_bounded_and_keeps_target_ids() {
        let inputs = vec![
            ProjectionInput {
                target_id: "ann-a".to_string(),
                vector: vec![0.0, 0.0, 1.0],
            },
            ProjectionInput {
                target_id: "ann-b".to_string(),
                vector: vec![1.0, 0.0, 0.0],
            },
            ProjectionInput {
                target_id: "ann-c".to_string(),
                vector: vec![0.0, 1.0, 0.0],
            },
        ];

        let first = pca_projection(&inputs);
        let second = pca_projection(&inputs);

        assert_eq!(first, second);
        assert_eq!(
            first
                .iter()
                .map(|point| point.target_id.as_str())
                .collect::<Vec<_>>(),
            vec!["ann-a", "ann-b", "ann-c"]
        );
        assert!(first
            .iter()
            .all(|point| (-1.0..=1.0).contains(&point.x) && (-1.0..=1.0).contains(&point.y)));
        assert!(first.iter().any(|point| point.x.abs() > 0.0));
    }
}
