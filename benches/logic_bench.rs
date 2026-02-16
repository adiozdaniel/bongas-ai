use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, black_box};
use bongas_ai::ml::utils::{cosine_similarity, dot_product, euclidean_similarity, pad_or_truncate};

fn bench_math_utils(c: &mut Criterion) {
    let mut group = c.benchmark_group("math_utils");

    for dim in [64, 128, 256, 512, 1024] {
        let v1 = vec![0.5f32; dim];
        let v2 = vec![0.8f32; dim];

        group.bench_with_input(BenchmarkId::new("cosine_similarity", dim), &dim, |b, _| {
            b.iter(|| cosine_similarity(black_box(&v1), black_box(&v2)))
        });

        group.bench_with_input(BenchmarkId::new("dot_product", dim), &dim, |b, _| {
            b.iter(|| dot_product(black_box(&v1), black_box(&v2)))
        });

        group.bench_with_input(BenchmarkId::new("euclidean_similarity", dim), &dim, |b, _| {
            b.iter(|| euclidean_similarity(black_box(&v1), black_box(&v2)))
        });
    }
    group.finish();
}

fn bench_pad_or_truncate(c: &mut Criterion) {
    let mut group = c.benchmark_group("vec_utils");

    for size in [100, 1000, 10000] {
        let v = vec![1.0f32; size];
        let target = size / 2;

        group.bench_with_input(BenchmarkId::new("pad_or_truncate", size), &size, |b, _| {
            b.iter(|| pad_or_truncate(black_box(v.clone()), black_box(target)))
        });
    }
    group.finish();
}

criterion_group!(benches, bench_math_utils, bench_pad_or_truncate);
criterion_main!(benches);
