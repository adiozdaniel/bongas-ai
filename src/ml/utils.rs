pub fn pad_or_truncate(mut features: Vec<f32>, target_dim: usize) -> Vec<f32> {
    if features.len() < target_dim {
        features.resize(target_dim, 0.0);
    } else if features.len() > target_dim {
        features.truncate(target_dim);
    }
    features
}