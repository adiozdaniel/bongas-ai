use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, black_box};
use bongas_ai::pipeline::{ScoredItem, PipelineStage};
use bongas_ai::pipeline::stages::sort::{SortByScoreStage, DeduplicateStage};
use bongas_ai::pipeline::context::ExecutionContext;
use serde_json::json;
use tokio::runtime::Runtime;

fn create_test_items(count: usize) -> Vec<ScoredItem> {
    (0..count)
        .map(|i| ScoredItem {
            item_id: i as i32,
            score: (i as f32) / (count as f32),
            metadata: json!({}),
        })
        .collect()
}

fn bench_pipeline_stages(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let context = rt.block_on(ExecutionContext::test_context());
    
    let mut group = c.benchmark_group("pipeline_logic");
    
    // Benchmark SortByScoreStage
    let sort_stage = SortByScoreStage;
    let sort_params = json!({ "descending": true });

    for size in [100, 500, 1000] {
        let items = create_test_items(size);
        group.bench_with_input(BenchmarkId::new("sort_by_score", size), &size, |b, _| {
            b.to_async(&rt).iter(|| async {
                let res: Vec<ScoredItem> = sort_stage.execute(&context, &sort_params, items.clone()).await.unwrap();
                black_box(res)
            })
        });
    }

    // Benchmark DeduplicateStage
    let dedup_stage = DeduplicateStage;
    let dedup_params = json!({});

    for size in [100, 500, 1000] {
        let mut items = create_test_items(size);
        // Add some duplicates
        if size > 10 {
            for i in 0..10 {
                items.push(items[i].clone());
            }
        }
        group.bench_with_input(BenchmarkId::new("deduplicate", size), &size, |b, _| {
            b.to_async(&rt).iter(|| async {
                let res: Vec<ScoredItem> = dedup_stage.execute(&context, &dedup_params, items.clone()).await.unwrap();
                black_box(res)
            })
        });
    }

    group.finish();
}

criterion_group!(benches, bench_pipeline_stages);
criterion_main!(benches);
