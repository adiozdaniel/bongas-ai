//! Phase 1: Tribe Orchestrator Worker
//!
//! Groups profiles into behavioral tribes based on embedding similarity.

use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;
use candle_core::{Tensor, Device};
use tokio::sync::broadcast;
use tracing::{info, warn, error};

use crate::db::ResilientPool;
use crate::cache::CacheManager;
use crate::db::models::ProfileFeatures;
use redis::AsyncCommands;

/// Background worker for clustering profiles into behavioral tribes.
pub struct TribeOrchestrator {
    db_pool: Arc<ResilientPool>,
    cache_manager: Arc<CacheManager>,
    pulse_interval: Duration,
    num_tribes: usize,
}

impl TribeOrchestrator {
    pub fn new(
        db_pool: Arc<ResilientPool>,
        cache_manager: Arc<CacheManager>,
        pulse_interval: Duration,
        num_tribes: usize,
    ) -> Self {
        Self {
            db_pool,
            cache_manager,
            pulse_interval,
            num_tribes,
        }
    }

    /// Start the background pulse loop.
    pub async fn start(self: Arc<Self>, mut shutdown_rx: broadcast::Receiver<()>) {
        info!(
            interval_mins = self.pulse_interval.as_secs() / 60,
            num_tribes = self.num_tribes,
            "Tribe Orchestrator started"
        );

        let mut interval = tokio::time::interval(self.pulse_interval);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = self.run_pulse().await {
                        error!(error = %e, "Tribe Orchestrator pulse failed");
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("Tribe Orchestrator shutting down...");
                    break;
                }
            }
        }
    }

    /// Perform a single clustering pulse.
    async fn run_pulse(&self) -> Result<()> {
        let start_time = std::time::Instant::now();
        info!("Starting tribal clustering pulse...");

        // 1. Fetch profiles with embeddings
        let profiles = self.fetch_profiles().await?;
        if profiles.is_empty() {
            warn!("No profiles found for clustering");
            return Ok(());
        }

        // 2. Prepare embedding matrix
        let (profile_ids, matrix) = self.prepare_matrix(&profiles)?;
        if matrix.dims()[0] < self.num_tribes {
            warn!(profiles = matrix.dims()[0], tribes = self.num_tribes, "Not enough profiles to form requested tribes");
            return Ok(());
        }

        // 3. K-Means Clustering
        let tribe_assignments = self.cluster_kmeans(&matrix, self.num_tribes, 10)?;

        // 4. Dual-Sync: Redis & Postgres
        self.sync_tribe_assignments(&profile_ids, &tribe_assignments).await?;

        info!(
            duration_ms = start_time.elapsed().as_millis(),
            profiles = profile_ids.len(),
            "Tribal clustering completed successfully"
        );

        Ok(())
    }

    async fn fetch_profiles(&self) -> Result<Vec<ProfileFeatures>> {
        self.db_pool.execute(|pool| async move {
            sqlx::query_as::<_, ProfileFeatures>(
                "SELECT * FROM bongas.profile_features WHERE embedding IS NOT NULL"

            )
            .fetch_all(&pool)
            .await
        }).await.map_err(|e| anyhow::anyhow!("Failed to fetch profiles: {}", e))
    }

    fn prepare_matrix(&self, profiles: &[ProfileFeatures]) -> Result<(Vec<String>, Tensor)> {
        let mut profile_ids = Vec::new();
        let mut data = Vec::new();
        let mut dim = 0;

        for p in profiles {
            if let Some(ref emb) = p.embedding {
                if dim == 0 { dim = emb.len(); }
                if emb.len() == dim {
                    profile_ids.push(p.profile_id.clone());
                    data.extend_from_slice(emb);
                }
            }
        }

        let nrows = profile_ids.len();
        let matrix = Tensor::from_vec(data, (nrows, dim), &Device::Cpu)?;
        Ok((profile_ids, matrix))
    }

    /// Pure-Rust K-Means implementation using Candle
    fn cluster_kmeans(&self, data: &Tensor, k: usize, max_iter: usize) -> Result<Vec<usize>> {
        let (n_samples, n_features) = data.dims2()?;
        
        // Randomly initialize centroids
        let mut centroid_data = Vec::with_capacity(k * n_features);
        for _ in 0..k {
            let idx = (rand::random::<f32>() * n_samples as f32) as usize;
            let sample = data.get(idx % n_samples)?;
            centroid_data.extend_from_slice(&sample.to_vec1::<f32>()?);
        }
        let mut centroids = Tensor::from_vec(centroid_data, (k, n_features), &Device::Cpu)?;

        let mut assignments = vec![0; n_samples];

        for _ in 0..max_iter {
            // Assignment step (Vectorized)
            // For each sample, find closest centroid
            for (i, assignment) in assignments.iter_mut().enumerate() {
                let sample = data.get(i)?;
                let mut min_dist = f32::MAX;
                let mut best_cluster = 0;

                for j in 0..k {
                    let centroid = centroids.get(j)?;
                    let diff = sample.sub(&centroid)?;
                    let dist = diff.sqr()?.sum_all()?.to_vec0::<f32>()?;
                    
                    if dist < min_dist {
                        min_dist = dist;
                        best_cluster = j;
                    }
                }
                *assignment = best_cluster;
            }

            // Update step
            let mut new_centroid_data = vec![0.0f32; k * n_features];
            let mut counts = vec![0; k];

            for (i, &cluster) in assignments.iter().enumerate() {
                let sample = data.get(i)?.to_vec1::<f32>()?;
                for (j, val) in sample.iter().enumerate() {
                    new_centroid_data[cluster * n_features + j] += val;
                }
                counts[cluster] += 1;
            }

            for (j, count) in counts.iter().enumerate() {
                if *count > 0 {
                    for d in 0..n_features {
                        new_centroid_data[j * n_features + d] /= *count as f32;
                    }
                }
            }

            let new_centroids = Tensor::from_vec(new_centroid_data, (k, n_features), &Device::Cpu)?;
            
            // Convergence check (Simplified)
            centroids = new_centroids;
        }

        Ok(assignments)
    }

    async fn sync_tribe_assignments(&self, profile_ids: &[String], assignments: &[usize]) -> Result<()> {
        // 1. Sync to Redis (Real-time path)
        if let Some(mut conn) = self.cache_manager.l2_connection() {
            for (pid, tribe_id) in profile_ids.iter().zip(assignments.iter()) {
                let key = format!("tribe_map:{}", pid);
                let _: () = conn.set(&key, *tribe_id as i32).await?;
            }
        }

        // 2. Sync to Postgres (System of record)
        for chunk in profile_ids.chunks(100).zip(assignments.chunks(100)) {
            let (ids, tribes) = chunk;
            self.db_pool.execute(move |pool| {
                let ids = ids.to_vec();
                let tribes: Vec<i32> = tribes.iter().map(|&t| t as i32).collect();
                async move {
                    sqlx::query(
                        "UPDATE bongas.profile_features SET tribe_id = UNNEST($1::int[]), features_updated_at = NOW() FROM UNNEST($2::text[]) AS pid WHERE bongas.profile_features.profile_id = pid"
                    )
                    .bind(&tribes)
                    .bind(&ids)
                    .execute(&pool)
                    .await
                }
            }).await?;
        }

        Ok(())
    }
}
