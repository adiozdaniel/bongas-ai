pub fn pad_or_truncate(mut features: Vec<f32>, target_dim: usize) -> Vec<f32> {
    if features.len() < target_dim {
        features.resize(target_dim, 0.0);
    } else if features.len() > target_dim {
        features.truncate(target_dim);
    }
    features
}

/// Cosine similarity between two vectors - SIMD optimized
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot = dot_product(a, b);
    
    // Auto-vectorized norms
    let norm_a: f32 = a.iter().map(|&x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|&x| x * x).sum::<f32>().sqrt();

    if norm_a > 0.0 && norm_b > 0.0 {
        dot / (norm_a * norm_b)
    } else {
        0.0
    }
}

/// Dot product between two vectors - optimized for LLVM auto-vectorization (SIMD)
pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    
    // Explicitly using a functional pattern that LLVM targets for SIMD
    a.iter()
        .zip(b)
        .map(|(&x, &y)| x * y)
        .fold(0.0, |acc, val| acc + val)
}

/// Euclidean distance converted to similarity - SIMD friendly
pub fn euclidean_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dist_sq: f32 = a.iter()
        .zip(b)
        .map(|(&x, &y)| {
            let diff = x - y;
            diff * diff
        })
        .sum();
        
    1.0 / (1.0 + dist_sq.sqrt())
}

/// Applies a weight and base score to a slice of scores using SIMD-friendly patterns.
/// final_score = (current_score * (1.0 - weight)) + (boost_value * weight)
pub fn apply_weights_simd(scores: &mut [f32], boosts: &[f32], weight: f32) {
    let inv_weight = 1.0 - weight;
    
    // This loop is a prime candidate for AVX2/NEON auto-vectorization
    for (score, &boost) in scores.iter_mut().zip(boosts.iter()) {
        *score = (*score * inv_weight) + (boost * weight);
    }
}