//! Temporal relevance scoring.
//!
//! More recent memories score higher. Uses an exponential decay
//! with a configurable half-life.

use crate::memory::types::*;
use chrono::{DateTime, Utc};

/// Default half-life in hours (24 hours)
const DEFAULT_HALF_LIFE_HOURS: f64 = 24.0;

/// Calculate temporal relevance score for a memory.
/// Returns a value from 0.0 (very old) to 1.0 (just now).
pub fn temporal_score(created_at: DateTime<Utc>, now: DateTime<Utc>, half_life_hours: Option<f64>) -> f32 {
    let elapsed_hours = (now - created_at).num_seconds().max(0) as f64 / 3600.0;
    let half_life = half_life_hours.unwrap_or(DEFAULT_HALF_LIFE_HOURS);

    // Exponential decay: score = 2^(-elapsed / half_life)
    let score = (-elapsed_hours / half_life * std::f64::consts::LN_2).exp();
    score as f32
}

/// Score based on last access time (rewards recently-used memories)
pub fn access_recency_score(last_accessed: DateTime<Utc>, now: DateTime<Utc>) -> f32 {
    temporal_score(last_accessed, now, Some(48.0)) // 48h half-life for access
}

/// Combined temporal score considering both creation and access time
pub fn combined_temporal_score(temporal: &TemporalInfo, now: DateTime<Utc>) -> f32 {
    let creation_score = temporal_score(temporal.created_at, now, None);
    let access_score = access_recency_score(temporal.last_accessed, now);

    // Weight access recency higher than creation time
    0.4 * creation_score + 0.6 * access_score
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temporal_score_recent() {
        let now = Utc::now();
        let score = temporal_score(now, now, None);
        assert!((score - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_temporal_score_old() {
        let now = Utc::now();
        let old = now - chrono::Duration::days(7);
        let score = temporal_score(old, now, None);
        assert!(score < 0.1, "Week-old memory should have low temporal score, got {}", score);
    }

    #[test]
    fn test_temporal_score_half_life() {
        let now = Utc::now();
        let half = now - chrono::Duration::hours(24);
        let score = temporal_score(half, now, Some(24.0));
        assert!((score - 0.5).abs() < 0.05, "Expected ~0.5 at half-life, got {}", score);
    }
}
