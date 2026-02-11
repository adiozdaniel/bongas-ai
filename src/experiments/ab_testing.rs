use anyhow::Result;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABTest {
    pub test_id: String,
    pub variants: Vec<String>,
    pub traffic_split: HashMap<String, f64>,  // variant -> probability
    pub active: bool,
}

impl ABTest {
    pub fn new(test_id: String, variants: Vec<(String, f64)>) -> Self {
        let traffic_split: HashMap<String, f64> = variants.into_iter().collect();

        Self {
            test_id,
            variants: traffic_split.keys().cloned().collect(),
            traffic_split,
            active: true,
        }
    }

    /// Assign user to variant
    pub fn assign_variant(&self, user_id: i32) -> Result<String> {
        if !self.active {
            return Err(anyhow::anyhow!("Test is not active"));
        }

        // Deterministic assignment based on user_id hash
        let hash = self.hash_user_id(user_id);
        let mut cumulative_prob = 0.0;

        for variant in &self.variants {
            let prob = self.traffic_split.get(variant).unwrap_or(&0.0);
            cumulative_prob += prob;

            if hash < cumulative_prob {
                return Ok(variant.clone());
            }
        }

        // Fallback to first variant
        Ok(self.variants[0].clone())
    }

    fn hash_user_id(&self, user_id: i32) -> f64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.test_id.hash(&mut hasher);
        user_id.hash(&mut hasher);
        let hash = hasher.finish();

        (hash % 10000) as f64 / 10000.0
    }
}

pub struct ABTestManager {
    tests: HashMap<String, ABTest>,
}

impl ABTestManager {
    pub fn new() -> Self {
        Self {
            tests: HashMap::new(),
        }
    }

    pub fn create_test(&mut self, test: ABTest) {
        self.tests.insert(test.test_id.clone(), test);
    }

    pub fn assign_variant(&self, test_id: &str, user_id: i32) -> Result<String> {
        let test = self.tests
            .get(test_id)
            .ok_or_else(|| anyhow::anyhow!("Test not found: {}", test_id))?;

        test.assign_variant(user_id)
    }

    pub fn deactivate_test(&mut self, test_id: &str) -> Result<()> {
        let test = self.tests
            .get_mut(test_id)
            .ok_or_else(|| anyhow::anyhow!("Test not found: {}", test_id))?;

        test.active = false;
        Ok(())
    }
}