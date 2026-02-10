pub struct ScorePostprocessor;

impl ScorePostprocessor {
    pub fn new() -> Self {
        Self
    }

    /// Normalize scores to [0, 1] range
    pub fn normalize_scores(&self, scores: &[f32]) -> Vec<f32> {
        if scores.is_empty() {
            return Vec::new();
        }

        let min = scores.iter().fold(f32::INFINITY, |a, &b| a.min(b));
        let max = scores.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));

        if (max - min).abs() < 1e-6 {
            return scores.to_vec();
        }

        scores.iter().map(|&s| (s - min) / (max - min)).collect()
    }

    /// Apply sigmoid activation
    pub fn sigmoid(&self, scores: &[f32]) -> Vec<f32> {
        scores.iter().map(|&s| 1.0 / (1.0 + (-s).exp())).collect()
    }

    /// Apply softmax
    pub fn softmax(&self, scores: &[f32]) -> Vec<f32> {
        if scores.is_empty() {
            return Vec::new();
        }

        let max = scores.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        let exps: Vec<f32> = scores.iter().map(|&s| (s - max).exp()).collect();
        let sum: f32 = exps.iter().sum();

        exps.iter().map(|&e| e / sum).collect()
    }

    /// Get top-k items with scores
    pub fn top_k(&self, scores: &[f32], k: usize) -> Vec<(usize, f32)> {
        let mut indexed: Vec<(usize, f32)> = scores
            .iter()
            .enumerate()
            .map(|(i, &s)| (i, s))
            .collect();

        indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        indexed.truncate(k);

        indexed
    }

    /// Calibrate scores using temperature scaling
    pub fn temperature_scale(&self, scores: &[f32], temperature: f32) -> Vec<f32> {
        scores.iter().map(|&s| s / temperature).collect()
    }
}

impl Default for ScorePostprocessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_scores() {
        let processor = ScorePostprocessor::new();
        let scores = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let normalized = processor.normalize_scores(&scores);

        assert_eq!(normalized[0], 0.0);
        assert_eq!(normalized[4], 1.0);
    }

    #[test]
    fn test_softmax() {
        let processor = ScorePostprocessor::new();
        let scores = vec![1.0, 2.0, 3.0];
        let softmax = processor.softmax(&scores);

        let sum: f32 = softmax.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_top_k() {
        let processor = ScorePostprocessor::new();
        let scores = vec![5.0, 2.0, 8.0, 1.0, 9.0];
        let top_2 = processor.top_k(&scores, 2);

        assert_eq!(top_2.len(), 2);
        assert_eq!(top_2[0].0, 4); // Index of highest score (9.0)
        assert_eq!(top_2[1].0, 2); // Index of second highest (8.0)
    }

    #[test]
    fn test_sigmoid() {
        let processor = ScorePostprocessor::new();
        let scores = vec![0.0];
        let sig = processor.sigmoid(&scores);
        assert!((sig[0] - 0.5).abs() < 1e-6);
    }
}
