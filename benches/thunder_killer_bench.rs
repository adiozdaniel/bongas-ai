use criterion::{criterion_group, criterion_main, Criterion, black_box};
use bongas_ai::pipeline::executor::PipelineExecutor;
use bongas_ai::pipeline::context::ExecutionContext;
use bongas_ai::pipeline::{ScoredItem, PipelineStage, StageDataKind};
use bongas_ai::db::models::{PipelineDefinition, PipelineStageConfig};
use bongas_ai::cache::{HotRegistry, HotItem};
use bongas_ai::circuit_breaker::CircuitBreakerRegistry;
use bongas_ai::resilience::{ResilienceMetricsCollector, MetricsRegistry, ResilienceMetricsConfig};
use serde_json::json;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::runtime::Runtime;
use async_trait::async_trait;

// ─── Mock Stages ────────────────────────────────────────────────────────────

struct MockFetchStage {
    items: Vec<ScoredItem>,
    parallelizable: bool,
}

#[async_trait]
impl PipelineStage for MockFetchStage {
    fn name(&self) -> &str { "mock_fetch" }
    fn input_type(&self) -> StageDataKind { StageDataKind::Empty }
    fn output_type(&self) -> StageDataKind { StageDataKind::ScoredItems }
    fn can_parallelize(&self) -> bool { self.parallelizable }
    
    async fn execute(&self, _: &ExecutionContext, _: &serde_json::Value, _: Vec<ScoredItem>) -> anyhow::Result<Vec<ScoredItem>> {
        // Simulate a small network delay
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        Ok(self.items.clone())
    }
}

fn create_test_items(count: usize) -> Vec<ScoredItem> {
    (0..count)
        .map(|i| ScoredItem::new(
            i as i32,
            (i as f32) / (count as f32),
            json!({}),
        ))
        .collect()
}

// ─── Benchmarks ─────────────────────────────────────────────────────────────

fn bench_fast_path_vs_slow_path(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let context = rt.block_on(ExecutionContext::test_context());
    
    let breaker_registry = Arc::new(CircuitBreakerRegistry::default());
    let resilience_metrics = Arc::new(ResilienceMetricsCollector::new(
        Arc::new(MetricsRegistry::new(ResilienceMetricsConfig::default())),
    ));

    let items = create_test_items(100);
    use bongas_ai::pipeline::PipelineRegistry;
    let mut stages: HashMap<String, Arc<dyn PipelineStage>> = HashMap::new();
    stages.insert("fetch".to_string(), Arc::new(MockFetchStage { items: items.clone(), parallelizable: false }));
    let registry = PipelineRegistry::with_stages(stages);

    let mut pipeline_config = bongas_ai::config::PipelineConfig::default();
    pipeline_config.stage_breaker_enabled = false;

    let executor = PipelineExecutor::with_registry(
        pipeline_config,
        breaker_registry.clone(),
        resilience_metrics.clone(),
        None,
        registry,
    );

    let pipeline = PipelineDefinition {
        stages: vec![
            PipelineStageConfig { r#type: "fetch".to_string(), params: json!({}) },
        ],
        fallback_stages: None,
    };

    let linked_pipeline = executor.link(&pipeline).unwrap();

    let mut group = c.benchmark_group("thunder_killer_fast_path");
    
    group.bench_function("slow_path_dynamic_link", |b| {
        b.to_async(&rt).iter(|| async {
            let res = executor.execute(&pipeline, &context).await.unwrap();
            black_box(res)
        })
    });

    group.bench_function("fast_path_pre_linked", |b| {
        b.to_async(&rt).iter(|| async {
            let res = executor.execute_linked(&linked_pipeline, &context).await.unwrap();
            black_box(res)
        })
    });

    group.finish();
}

fn bench_parallel_fetch_gains(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let context = rt.block_on(ExecutionContext::test_context());
    
    let breaker_registry = Arc::new(CircuitBreakerRegistry::default());
    let resilience_metrics = Arc::new(ResilienceMetricsCollector::new(
        Arc::new(MetricsRegistry::new(ResilienceMetricsConfig::default())),
    ));

    let items = create_test_items(100);
    use bongas_ai::pipeline::PipelineRegistry;
    let mut stages: HashMap<String, Arc<dyn PipelineStage>> = HashMap::new();
    stages.insert("fetch_parallel".to_string(), Arc::new(MockFetchStage { items: items.clone(), parallelizable: true }));
    stages.insert("fetch_serial".to_string(), Arc::new(MockFetchStage { items: items.clone(), parallelizable: false }));
    let registry = PipelineRegistry::with_stages(stages);

    let mut pipeline_config = bongas_ai::config::PipelineConfig::default();
    pipeline_config.stage_breaker_enabled = false;

    let executor = PipelineExecutor::with_registry(
        pipeline_config,
        breaker_registry.clone(),
        resilience_metrics.clone(),
        None,
        registry,
    );

    let serial_def = PipelineDefinition {
        stages: vec![
            PipelineStageConfig { r#type: "fetch_serial".to_string(), params: json!({}) },
            PipelineStageConfig { r#type: "fetch_serial".to_string(), params: json!({}) },
            PipelineStageConfig { r#type: "fetch_serial".to_string(), params: json!({}) },
        ],
        fallback_stages: None,
    };

    let parallel_def = PipelineDefinition {
        stages: vec![
            PipelineStageConfig { r#type: "fetch_parallel".to_string(), params: json!({}) },
            PipelineStageConfig { r#type: "fetch_parallel".to_string(), params: json!({}) },
            PipelineStageConfig { r#type: "fetch_parallel".to_string(), params: json!({}) },
        ],
        fallback_stages: None,
    };

    let serial_linked = executor.link(&serial_def).unwrap();
    let parallel_linked = executor.link(&parallel_def).unwrap();

    let mut group = c.benchmark_group("thunder_killer_parallelism");
    
    group.bench_function("3_fetches_serial", |b| {
        b.to_async(&rt).iter(|| async {
            let res = executor.execute_linked(&serial_linked, &context).await.unwrap();
            black_box(res)
        })
    });

    group.bench_function("3_fetches_parallel", |b| {
        b.to_async(&rt).iter(|| async {
            let res = executor.execute_linked(&parallel_linked, &context).await.unwrap();
            black_box(res)
        })
    });

    group.finish();
}

fn bench_hot_registry_retrieval(c: &mut Criterion) {
    let registry = HotRegistry::new();
    let mut items = Vec::new();
    for i in 0..1000 {
        items.push(HotItem {
            item_id: i,
            score: i as f32,
            metadata: json!({"title": format!("Video {}", i)}),
        });
    }
    registry.refresh(items);

    let mut group = c.benchmark_group("thunder_killer_hot_registry");
    
    group.bench_function("get_top_100_from_ram", |b| {
        b.iter(|| {
            let res = registry.get_top_k(100);
            black_box(res)
        })
    });

    group.bench_function("single_item_lookup_dashmap", |b| {
        b.iter(|| {
            let res = registry.get(black_box(500));
            black_box(res)
        })
    });

    group.finish();
}

criterion_group!(
    benches, 
    bench_fast_path_vs_slow_path, 
    bench_parallel_fetch_gains,
    bench_hot_registry_retrieval
);
criterion_main!(benches);
