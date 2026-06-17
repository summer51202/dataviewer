pub fn deterministic_projection(seed: &str) -> (f64, f64) {
    let mut hash = 0_u64;
    for byte in seed.as_bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(*byte as u64);
    }
    let x = ((hash & 0xffff) as f64 / 65535.0) * 2.0 - 1.0;
    let y = (((hash >> 16) & 0xffff) as f64 / 65535.0) * 2.0 - 1.0;
    (x, y)
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
}
