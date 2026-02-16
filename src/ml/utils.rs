pub fn pad_or_truncate(mut features: Vec<f32>, target_dim: usize) -> Vec<f32> {
    if features.len() < target_dim {
        features.resize(target_dim, 0.0);
    } else if features.len() > target_dim {
        features.truncate(target_dim);
    }
    features
}

/// Cosine similarity between two vectors
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a > 0.0 && norm_b > 0.0 {
        dot / (norm_a * norm_b)
    } else {
        0.0
    }
}

/// Dot product between two vectors
pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Euclidean distance converted to similarity (closer = higher score)
pub fn euclidean_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dist: f32 = a
        .iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f32>()
        .sqrt();
    // Convert distance to similarity
    1.0 / (1.0 + dist)
}