//! Time-decay scoring for recency-based ranking.

use crate::memory::types::*;
use chrono::{DateTime, Utc};

pub fn recency_score(temporal: &TemporalInfo, now: DateTime<Utc>) -> f32 {
    crate::retrieval::temporal::combined_temporal_score(temporal, now)
}
