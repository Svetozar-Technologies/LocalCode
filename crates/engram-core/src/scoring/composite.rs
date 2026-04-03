use crate::memory::types::*;

/// Weighted fusion scorer that combines all retrieval signals.
pub struct CompositeScorer {
    pub weights: ScoringWeights,
}

impl CompositeScorer {
    pub fn new(weights: ScoringWeights) -> Self {
        Self { weights }
    }

    pub fn with_defaults() -> Self {
        Self {
            weights: ScoringWeights::default(),
        }
    }

    /// Calculate the final composite score from a breakdown
    pub fn score(&self, breakdown: &ScoreBreakdown) -> f32 {
        breakdown.vector_score * self.weights.vector
            + breakdown.temporal_score * self.weights.temporal
            + breakdown.strength_score * self.weights.strength
            + breakdown.graph_score * self.weights.graph
            + breakdown.keyword_score * self.weights.keyword
    }

    /// Normalize weights to sum to 1.0
    pub fn normalize_weights(&mut self) {
        let sum = self.weights.vector
            + self.weights.temporal
            + self.weights.strength
            + self.weights.graph
            + self.weights.keyword;

        if sum > 0.0 {
            self.weights.vector /= sum;
            self.weights.temporal /= sum;
            self.weights.strength /= sum;
            self.weights.graph /= sum;
            self.weights.keyword /= sum;
        }
    }
}

impl Default for CompositeScorer {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_composite_scoring() {
        let scorer = CompositeScorer::with_defaults();
        let breakdown = ScoreBreakdown {
            vector_score: 0.8,
            temporal_score: 0.6,
            strength_score: 0.9,
            graph_score: 0.5,
            keyword_score: 0.7,
        };

        let score = scorer.score(&breakdown);
        assert!(score > 0.0 && score <= 1.0);
    }

    #[test]
    fn test_weight_normalization() {
        let mut scorer = CompositeScorer::new(ScoringWeights {
            vector: 2.0,
            temporal: 1.0,
            strength: 1.0,
            graph: 1.0,
            keyword: 1.0,
        });

        scorer.normalize_weights();
        let sum = scorer.weights.vector + scorer.weights.temporal + scorer.weights.strength
            + scorer.weights.graph + scorer.weights.keyword;
        assert!((sum - 1.0).abs() < 0.001);
    }
}
