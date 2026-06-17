pub fn deterministic_projection(seed: &str) -> (f64, f64) {
    let mut hash = 0_u64;
    for byte in seed.as_bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(*byte as u64);
    }
    let x = ((hash & 0xffff) as f64 / 65535.0) * 2.0 - 1.0;
    let y = (((hash >> 16) & 0xffff) as f64 / 65535.0) * 2.0 - 1.0;
    (x, y)
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

    let dimensions = inputs.iter().map(|input| input.vector.len()).max().unwrap_or(0);
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
            first.iter().map(|point| point.target_id.as_str()).collect::<Vec<_>>(),
            vec!["ann-a", "ann-b", "ann-c"]
        );
        assert!(first
            .iter()
            .all(|point| (-1.0..=1.0).contains(&point.x) && (-1.0..=1.0).contains(&point.y)));
        assert!(first.iter().any(|point| point.x.abs() > 0.0));
    }
}
