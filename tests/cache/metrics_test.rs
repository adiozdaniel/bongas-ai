use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::cache::metrics::{CacheMetrics, CacheStatsSnapshot};

#[tokio::test]
async fn test_cache_metrics_basic() {
    let metrics = CacheMetrics::new();

    // Initial state
    assert_eq!(metrics.get_stats().l1_hits, 0);
    assert_eq!(metrics.get_stats().l1_misses, 0);
    assert_eq!(metrics.get_stats().l2_hits, 0);
    assert_eq!(metrics.get_stats().l2_misses, 0);
    assert_eq!(metrics.get_stats().invalidations, 0);
    assert_eq!(metrics.get_stats().warmings, 0);
    assert_eq!(metrics.get_hit_rate(), 0.0);
    assert_eq!(metrics.get_l1_hit_rate(), 0.0);
    assert_eq!(metrics.get_l2_hit_rate(), 0.0);
}

#[tokio::test]
async fn test_l1_hit_rate_calculation() {
    let metrics = CacheMetrics::new();

    // Simulate L1 hits and misses
    metrics.record_l1_hit();
    metrics.record_l1_hit();
    metrics.record_l1_miss();

    let stats = metrics.get_stats();
    assert_eq!(stats.l1_hits, 2);
    assert_eq!(stats.l1_misses, 1);
    assert_eq!(stats.l1_hit_rate, 2.0 / 3.0);
}

#[tokio::test]
async fn test_l2_hit_rate_calculation() {
    let metrics = CacheMetrics::new();

    // Simulate L2 hits and misses
    metrics.record_l2_hit();
    metrics.record_l2_hit();
    metrics.record_l2_hit();
    metrics.record_l2_miss();

    let stats = metrics.get_stats();
    assert_eq!(stats.l2_hits, 3);
    assert_eq!(stats.l2_misses, 1);
    assert_eq!(stats.l2_hit_rate, 3.0 / 4.0);
}

#[tokio::test]
async fn test_overall_hit_rate_calculation() {
    let metrics = CacheMetrics::new();

    // Simulate mixed L1 and L2 activity
    metrics.record_l1_hit();  // Hit L1
    metrics.record_l1_miss(); // Miss L1, hit L2
    metrics.record_l2_hit();
    metrics.record_l2_miss(); // Miss both

    let stats = metrics.get_stats();
    assert_eq!(stats.overall_hit_rate, 3.0 / 4.0); // 3 hits out of 4 total requests
}

#[tokio::test]
async fn test_zero_division_protection() {
    let metrics = CacheMetrics::new();

    // Test with no activity
    assert_eq!(metrics.get_hit_rate(), 0.0);
    assert_eq!(metrics.get_l1_hit_rate(), 0.0);
    assert_eq!(metrics.get_l2_hit_rate(), 0.0);

    // Test L1 with only hits
    metrics.record_l1_hit();
    metrics.record_l1_hit();
    assert_eq!(metrics.get_l1_hit_rate(), 1.0);

    // Test L2 with only misses
    metrics.record_l2_miss();
    metrics.record_l2_miss();
    assert_eq!(metrics.get_l2_hit_rate(), 0.0);
}

#[tokio::test]
async fn test_metrics_counters() {
    let metrics = CacheMetrics::new();

    // Record various events
    metrics.record_l1_hit();
    metrics.record_l1_miss();
    metrics.record_l2_hit();
    metrics.record_l2_miss();
    metrics.record_invalidation();
    metrics.record_warming();

    let stats = metrics.get_stats();
    assert_eq!(stats.l1_hits, 1);
    assert_eq!(stats.l1_misses, 1);
    assert_eq!(stats.l2_hits, 1);
    assert_eq!(stats.l2_misses, 1);
    assert_eq!(stats.invalidations, 1);
    assert_eq!(stats.warmings, 1);
}

#[tokio::test]
async fn test_metrics_reset() {
    let metrics = CacheMetrics::new();

    // Add some activity
    metrics.record_l1_hit();
    metrics.record_l1_hit();
    metrics.record_l2_hit();
    metrics.record_invalidation();

    // Verify activity
    let stats = metrics.get_stats();
    assert!(stats.l1_hits > 0);
    assert!(stats.l2_hits > 0);
    assert!(stats.invalidations > 0);

    // Reset
    metrics.reset();

    // Verify reset
    let stats_after_reset = metrics.get_stats();
    assert_eq!(stats_after_reset.l1_hits, 0);
    assert_eq!(stats_after_reset.l1_misses, 0);
    assert_eq!(stats_after_reset.l2_hits, 0);
    assert_eq!(stats_after_reset.l2_misses, 0);
    assert_eq!(stats_after_reset.invalidations, 0);
    assert_eq!(stats_after_reset.warmings, 0);
    assert_eq!(stats_after_reset.overall_hit_rate, 0.0);
}

#[tokio::test]
async fn test_concurrent_access() {
    use std::thread;
    use std::time::Duration;

    let metrics = Arc::new(CacheMetrics::new());
    let mut handles = vec![];

    // Spawn multiple threads to record metrics concurrently
    for _ in 0..10 {
        let metrics_clone = metrics.clone();
        let handle = thread::spawn(move || {
            for _ in 0..100 {
                metrics_clone.record_l1_hit();
                metrics_clone.record_l2_hit();
                metrics_clone.record_invalidation();
                thread::sleep(Duration::from_micros(10));
            }
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify final counts
    let stats = metrics.get_stats();
    assert_eq!(stats.l1_hits, 1000); // 10 threads * 100 hits each
    assert_eq!(stats.l2_hits, 1000);
    assert_eq!(stats.invalidations, 1000);
}

#[tokio::test]
async fn test_hit_rate_edge_cases() {
    let metrics = CacheMetrics::new();

    // Test with only L1 activity
    metrics.record_l1_hit();
    metrics.record_l1_hit();
    let stats = metrics.get_stats();
    assert_eq!(stats.overall_hit_rate, 1.0); // Only L1 hits, no L2 activity
    assert_eq!(stats.l1_hit_rate, 1.0);
    assert_eq!(stats.l2_hit_rate, 0.0); // No L2 activity

    // Test with only L2 activity
    let metrics2 = CacheMetrics::new();
    metrics2.record_l2_hit();
    metrics2.record_l2_miss();
    let stats2 = metrics2.get_stats();
    assert_eq!(stats2.overall_hit_rate, 0.5);
    assert_eq!(stats2.l1_hit_rate, 0.0); // No L1 activity
    assert_eq!(stats2.l2_hit_rate, 0.5);
}