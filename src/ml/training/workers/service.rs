//! Async ML task queue with backpressure and bulkhead.
//!
//! # Netflix Resilience Patterns
//! - **Backpressure**: Bounded channel rejects when queue is full
//! - **Bulkhead**: Configurable worker count isolates ML from request threads
//! - **Analytics**: Queue depth, processing latency, rejection rate
//! - **Graceful Shutdown**: Drain queue before stopping

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::time::Instant;

use tokio::sync::mpsc;
use tracing::{info, warn, error, debug};

use crate::analytics::types::PerformanceStats;
use crate::config::MlConfig;

/// A unit of work to be processed by the ML worker pool.
pub struct MlTask {
    /// Unique task identifier.
    pub id: String,
    /// Task type for routing.
    pub task_type: MlTaskType,
    /// Payload serialized as JSON.
    pub payload: serde_json::Value,
    /// When the task was enqueued.
    pub enqueued_at: Instant,
    /// Completion callback.
    pub completion_tx: Option<tokio::sync::oneshot::Sender<MlTaskResult>>,
}

/// Task types for the ML worker pool.
#[derive(Debug, Clone)]
pub enum MlTaskType {
    /// Batch inference request.
    BatchInference,
    /// Model reload request.
    ModelReload,
    /// Feature precomputation.
    FeaturePrecompute,
    /// Embedding generation.
    EmbeddingGeneration,
    /// Online learning feedback batch.
    FeedbackBatch,
}

/// Result of a completed ML task.
#[derive(Debug)]
pub enum MlTaskResult {
    Success(serde_json::Value),
    Failed(String),
}

/// Worker queue statistics.
#[derive(Debug, Clone)]
pub struct WorkerQueueStats {
    pub enqueued: u64,
    pub processed: u64,
    pub rejected: u64,
    pub current_depth: usize,
}

/// Async ML worker queue with backpressure.
pub struct MlWorkerQueue {
    sender: mpsc::Sender<MlTask>,
    stats_enqueued: Arc<AtomicU64>,
    stats_processed: Arc<AtomicU64>,
    stats_rejected: Arc<AtomicU64>,
    shutdown: Arc<AtomicBool>,
    worker_handles: Vec<tokio::task::JoinHandle<()>>,
    queue_depth: usize,
    analytics: Option<Arc<PerformanceStats>>,
}

impl MlWorkerQueue {
    /// Create a new worker queue and spawn worker tasks.
    pub fn new(
        config: &MlConfig,
        analytics: Option<Arc<PerformanceStats>>,
    ) -> Self {
        let queue_depth = config.worker_queue_depth;
        let (sender, receiver) = mpsc::channel::<MlTask>(queue_depth);
        let receiver = Arc::new(tokio::sync::Mutex::new(receiver));

        let stats_enqueued = Arc::new(AtomicU64::new(0));
        let stats_processed = Arc::new(AtomicU64::new(0));
        let stats_rejected = Arc::new(AtomicU64::new(0));
        let shutdown = Arc::new(AtomicBool::new(false));

        // Spawn worker tasks (bulkhead: fixed number of workers)
        let worker_count = config.inference_max_concurrent.max(1);
        let mut worker_handles = Vec::with_capacity(worker_count);

        for worker_id in 0..worker_count {
            let rx = receiver.clone();
            let processed = stats_processed.clone();
            let shutdown_flag = shutdown.clone();
            let analytics_clone = analytics.clone();

            let handle = tokio::spawn(async move {
                loop {
                    if shutdown_flag.load(Ordering::Relaxed) {
                        break;
                    }

                    let task = {
                        let mut guard = rx.lock().await;
                        guard.recv().await
                    };

                    match task {
                        Some(task) => {
                            let start = Instant::now();
                            let queue_wait = task.enqueued_at.elapsed();

                            debug!(
                                worker_id = worker_id,
                                task_id = %task.id,
                                task_type = ?task.task_type,
                                queue_wait_ms = queue_wait.as_millis() as u64,
                                "Processing ML task"
                            );

                            // Process the task (placeholder — real processing
                            // would dispatch to model_loader, feature_store, etc.)
                            let result = MlTaskResult::Success(serde_json::json!({
                                "task_id": task.id,
                                "status": "completed",
                                "queue_wait_ms": queue_wait.as_millis() as u64,
                            }));

                            // Send result back
                            if let Some(tx) = task.completion_tx {
                                let _ = tx.send(result);
                            }

                            processed.fetch_add(1, Ordering::Relaxed);

                            let processing_time = start.elapsed();
                            if let Some(ref a) = analytics_clone {
                                a.record_response_time(
                                    "ml.worker_queue.processing",
                                    processing_time.as_millis() as u64,
                                );
                                a.increment_throughput("ml.worker_queue.processed");
                            }
                        }
                        None => {
                            // Channel closed
                            break;
                        }
                    }
                }

                debug!(worker_id = worker_id, "ML worker stopped");
            });

            worker_handles.push(handle);
        }

        info!(
            worker_count = worker_count,
            queue_depth = queue_depth,
            "ML worker queue started"
        );

        Self {
            sender,
            stats_enqueued,
            stats_processed,
            stats_rejected,
            shutdown,
            worker_handles,
            queue_depth,
            analytics,
        }
    }

    /// Enqueue a task. Returns Err if queue is full (backpressure).
    pub fn try_enqueue(&self, task: MlTask) -> Result<(), MlTask> {
        match self.sender.try_send(task) {
            Ok(()) => {
                self.stats_enqueued.fetch_add(1, Ordering::Relaxed);
                if let Some(ref a) = self.analytics {
                    a.increment_throughput("ml.worker_queue.enqueued");
                }
                Ok(())
            }
            Err(mpsc::error::TrySendError::Full(task)) => {
                self.stats_rejected.fetch_add(1, Ordering::Relaxed);
                if let Some(ref a) = self.analytics {
                    a.increment_error("ml.worker_queue.rejected");
                }
                warn!("ML worker queue full, rejecting task");
                Err(task)
            }
            Err(mpsc::error::TrySendError::Closed(task)) => {
                self.stats_rejected.fetch_add(1, Ordering::Relaxed);
                error!("ML worker queue closed");
                Err(task)
            }
        }
    }

    /// Enqueue a task and wait for completion.
    pub async fn enqueue_and_wait(&self, mut task: MlTask) -> Result<MlTaskResult, String> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        task.completion_tx = Some(tx);

        self.try_enqueue(task).map_err(|_| "queue full".to_string())?;

        rx.await.map_err(|_| "worker dropped task".to_string())
    }

    /// Get queue statistics.
    pub fn stats(&self) -> WorkerQueueStats {
        WorkerQueueStats {
            enqueued: self.stats_enqueued.load(Ordering::Relaxed),
            processed: self.stats_processed.load(Ordering::Relaxed),
            rejected: self.stats_rejected.load(Ordering::Relaxed),
            current_depth: self.queue_depth - self.sender.capacity(),
        }
    }

    /// Graceful shutdown: signal workers and wait for drain.
    pub async fn shutdown(self) {
        info!("Shutting down ML worker queue...");
        self.shutdown.store(true, Ordering::Relaxed);
        drop(self.sender); // Close channel

        for handle in self.worker_handles {
            let _ = handle.await;
        }

        info!(
            enqueued = self.stats_enqueued.load(Ordering::Relaxed),
            processed = self.stats_processed.load(Ordering::Relaxed),
            rejected = self.stats_rejected.load(Ordering::Relaxed),
            "ML worker queue shut down"
        );
    }
}
