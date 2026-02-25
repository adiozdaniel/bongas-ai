use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, black_box};
use bongas_ai::pipeline::executor::PipelineExecutor;
use bongas_ai::pipeline::context::ExecutionContext;
use bongas_ai::pipeline::{ScoredItem, PipelineStage, StageDataKind};
use bongas_ai::db::models::{PipelineDefinition, PipelineStageConfig};
use bongas_ai::circuit_breaker::CircuitBreakerRegistry;
use bongas_ai::resilience::{ResilienceMetricsCollector, MetricsRegistry, ResilienceConfig};
use bongas_ai::pipeline::stages::sort::{SortByScoreStage, DeduplicateStage, LimitStage};
use serde_json::json;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::runtime::Runtime;
use async_trait::async_trait;

struct MockFetchStage {
    items: Vec<ScoredItem>,
}

#[async_trait]
impl PipelineStage for MockFetchStage {
    fn name(&self) -> &str { "mock_fetch" }
    fn input_type(&self) -> StageDataKind { StageDataKind::Empty }
    fn output_type(&self) -> StageDataKind { StageDataKind::ScoredItems }
    async fn execute(&self, _: &ExecutionContext, _: &serde_json::Value, _: Vec<ScoredItem>) -> anyhow::Result<Vec<ScoredItem>> {
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

fn bench_pipeline_executor(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let context = rt.block_on(ExecutionContext::test_context());
    
    let breaker_registry = Arc::new(CircuitBreakerRegistry::default());
    let resilience_metrics = Arc::new(ResilienceMetricsCollector::new(
        Arc::new(MetricsRegistry::new(ResilienceConfig::default())),
    ));

    let mut group = c.benchmark_group("engine_executor");

    for size in [100, 500, 1000] {
        let items = create_test_items(size);
        
        use bongas_ai::pipeline::PipelineRegistry;
        let mut stages: HashMap<String, Arc<dyn PipelineStage>> = HashMap::new();
        stages.insert("mock_fetch".to_string(), Arc::new(MockFetchStage { items: items.clone() }));
        stages.insert("deduplicate".to_string(), Arc::new(DeduplicateStage));
        stages.insert("sort_by_score".to_string(), Arc::new(SortByScoreStage));
        stages.insert("limit".to_string(), Arc::new(LimitStage));
        
        let registry = PipelineRegistry::with_stages(stages);

        let executor = PipelineExecutor::with_registry(
            bongas_ai::config::PipelineConfig::default(),
            breaker_registry.clone(),
            resilience_metrics.clone(),
            None,
            registry,
        );

        let pipeline = PipelineDefinition {
            stages: vec![
                PipelineStageConfig {
                    r#type: "mock_fetch".to_string(),
                    params: json!({}),
                },
                PipelineStageConfig {
                    r#type: "deduplicate".to_string(),
                    params: json!({}),
                },
                PipelineStageConfig {
                    r#type: "sort_by_score".to_string(),
                    params: json!({ "descending": true }),
                },
                PipelineStageConfig {
                    r#type: "limit".to_string(),
                    params: json!({ "limit": 100 }),
                },
            ],
            fallback_stages: None,
        };

        let linked = executor.link(&pipeline).unwrap();

        group.bench_with_input(BenchmarkId::new("slow_path_dynamic", size), &size, |b, _| {
            b.to_async(&rt).iter(|| async {
                let res = executor.execute(&pipeline, &context).await.unwrap();
                black_box(res)
            })
        });

        group.bench_with_input(BenchmarkId::new("fast_path_linked", size), &size, |b, _| {
            b.to_async(&rt).iter(|| async {
                let res = executor.execute_linked(&linked, &context).await.unwrap();
                black_box(res)
            })
        });
    }
    group.finish();
}

criterion_group!(benches, bench_pipeline_executor);
criterion_main!(benches);
