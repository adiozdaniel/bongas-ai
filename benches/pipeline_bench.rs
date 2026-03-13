use criterion::{black_box, criterion_group, criterion_main, Criterion};
use bongas_ai::pipeline::ranking::sort_by_score::service::SortByScoreStage;
use bongas_ai::pipeline::processing::deduplicate::service::DeduplicateStage;
use bongas_ai::pipeline::recovery::fetch_behavioral_tribe::service::FetchBehavioralTribeStage;
use bongas_ai::pipeline::ranking::boost_hyper_local_pulse::service::BoostHyperLocalPulseStage;
use bongas_ai::pipeline::ExecutionContext;
use bongas_ai::pipeline::ScoredItem;
use bongas_ai::pipeline::PipelineStage;
use serde_json::json;

fn bench_stages(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let context = rt.block_on(ExecutionContext::test_context());
    
    let mut items = Vec::new();
    for i in 0..100 {
        items.push(ScoredItem::new(i, (100 - i) as f32, json!({})));
    }

    let sort_stage = SortByScoreStage;
    let sort_params = json!({"descending": true});

    c.bench_function("sort_by_score_100", |b| {
        b.to_async(&rt).iter(|| async {
            let _res: Vec<ScoredItem> = sort_stage.execute(black_box(&context), black_box(&sort_params), black_box(items.clone())).await.unwrap();
        })
    });

    let dedup_stage = DeduplicateStage;
    let dedup_params = json!({});

    c.bench_function("deduplicate_100", |b| {
        b.to_async(&rt).iter(|| async {
            let _res: Vec<ScoredItem> = dedup_stage.execute(black_box(&context), black_box(&dedup_params), black_box(items.clone())).await.unwrap();
        })
    });

    let tribe_stage = FetchBehavioralTribeStage;
    let tribe_params = json!({"time_window_hours": 24, "limit": 50});

    c.bench_function("fetch_behavioral_tribe", |b| {
        b.to_async(&rt).iter(|| async {
            let _res: Vec<ScoredItem> = tribe_stage.execute(black_box(&context), black_box(&tribe_params), black_box(Vec::new())).await.unwrap();
        })
    });

    let boost_stage = BoostHyperLocalPulseStage;
    let boost_params = json!({});
    let context_with_loc = context.clone().with_location("Nairobi".to_string());

    c.bench_function("boost_hyper_local_pulse_100", |b| {
        b.to_async(&rt).iter(|| async {
            let _res: Vec<ScoredItem> = boost_stage.execute(black_box(&context_with_loc), black_box(&boost_params), black_box(items.clone())).await.unwrap();
        })
    });
}

criterion_group!(benches, bench_stages);
criterion_main!(benches);
