use criterion::{black_box, criterion_group, criterion_main, Criterion};
use bongas_ai::pipeline::{PipelineExecutor, ExecutionContext, ScoredItem};
use std::sync::Arc;

fn bench_engine(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let context = rt.block_on(ExecutionContext::test_context());
    
    let config = bongas_ai::config::PipelineConfig::default();
    let breaker_registry = Arc::new(bongas_ai::circuit_breaker::CircuitBreakerRegistry::default());
    let observer = Arc::new(bongas_ai::circuit_breaker::observer::NoOpObserver);
    
    let executor = PipelineExecutor::new(config, breaker_registry, observer, None);

    let definition = bongas_ai::db::PipelineDefinition {
        stages: vec![],
        fallback_stages: None,
    };

    let linked = executor.link(&definition).unwrap();

    c.bench_function("engine_execute_linked", |b| {
        b.to_async(&rt).iter(|| async {
            let _res: Vec<ScoredItem> = executor.execute_linked(black_box(&linked), black_box(&context)).await.unwrap();
        })
    });
}

criterion_group!(benches, bench_engine);
criterion_main!(benches);
