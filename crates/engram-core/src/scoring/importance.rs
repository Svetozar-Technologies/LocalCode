use crate::memory::types::*;

/// Score based on how frequently a memory is accessed.
/// Logarithmic scale to prevent runaway scores.
pub fn importance_score(temporal: &TemporalInfo) -> f32 {
    let count = temporal.access_count as f32;
    // log2(count + 1) / log2(100) — normalizes to ~1.0 at 100 accesses
    (count + 1.0).log2() / 100.0f32.log2()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_importance_scaling() {
        let mut temporal = TemporalInfo::new(chrono::Utc::now());
        let score0 = importance_score(&temporal);

        temporal.access_count = 50;
        let score50 = importance_score(&temporal);

        temporal.access_count = 100;
        let score100 = importance_score(&temporal);

        assert!(score50 > score0);
        assert!(score100 > score50);
        assert!((score100 - 1.0).abs() < 0.1);
    }
}
