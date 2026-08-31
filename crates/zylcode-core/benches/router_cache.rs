use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;
use zylcode_core::pipeline::ArtifactPipeline;
use zylcode_core::router::{
    cache::SpeculativeCache, trim_to_window, ContextTrim, RouterConfig, TokenRouter,
};

/// Generate a multi-megabyte artifact payload for stress testing
fn generate_large_payload(num_artifacts: usize, artifact_size_kb: usize) -> String {
    let mut s = String::from("<zylcode-response>\n");
    let content = "x".repeat(artifact_size_kb * 1024);
    for i in 0..num_artifacts {
        s.push_str(&format!(
            r#"<artifact kind="RustModule" path="src/gen{i}.rs"><content><![CDATA[{}]]></content></artifact>"#,
            content
        ));
    }
    s.push_str("</zylcode-response>");
    s
}

fn bench_parse_artifacts(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_artifacts");

    // Test different artifact counts with fixed size
    for num_artifacts in [1, 5, 10, 20, 50].iter() {
        let raw = generate_large_payload(*num_artifacts, 1); // 1KB each
        group.throughput(Throughput::Bytes(raw.len() as u64));
        group.bench_with_input(BenchmarkId::new("xml_artifacts", num_artifacts), &raw, |b, input| {
            b.iter(|| black_box(ArtifactPipeline::parse_artifacts(black_box(input))))
        });
    }

    // Test different artifact sizes with fixed count
    for artifact_size_kb in [1, 4, 16, 64, 256].iter() {
        let raw = generate_large_payload(10, *artifact_size_kb);
        group.throughput(Throughput::Bytes(raw.len() as u64));
        group.bench_with_input(BenchmarkId::new("xml_size_kb", artifact_size_kb), &raw, |b, input| {
            b.iter(|| black_box(ArtifactPipeline::parse_artifacts(black_box(input))))
        });
    }

    group.finish();
}

fn bench_parse_fences(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_fences");

    for num_fences in [1, 5, 10, 20, 50].iter() {
        let raw = (0..*num_fences)
            .map(|i| format!("```rust\npub fn f{}() {{}}\n```\n", i))
            .collect::<String>();
        group.throughput(Throughput::Bytes(raw.len() as u64));
        group.bench_with_input(BenchmarkId::new("fences_count", num_fences), &raw, |b, input| {
            b.iter(|| black_box(ArtifactPipeline::parse_artifacts(black_box(input))))
        });
    }
    group.finish();
}

fn bench_trim_to_window(c: &mut Criterion) {
    let mut group = c.benchmark_group("trim_to_window");

    // Various prompt sizes with fixed window
    let system = "You are a code generation assistant.";
    let window = 8192u32;

    for prompt_kb in [1, 4, 16, 64, 256, 1024].iter() {
        let prompt = "x".repeat(prompt_kb * 1024);
        group.throughput(Throughput::Bytes(prompt.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("prompt_kb", prompt_kb),
            &(prompt, system, window, ContextTrim::SlidingWindow),
            |b, (p, s, w, strat)| {
                b.iter(|| black_box(trim_to_window(black_box(p), black_box(s), *w, *strat)))
            },
        );
    }
    group.finish();
}

fn bench_speculative_cache_hit(c: &mut Criterion) {
    let mut group = c.benchmark_group("speculative_cache_hit");

    for capacity in [16, 64, 256, 1024].iter() {
        let cache = Arc::new(SpeculativeCache::new(*capacity, Duration::from_secs(600)));
        let rt = Runtime::new().unwrap();
        let cfg = RouterConfig::default();
        let router = TokenRouter::with_cache(cfg, Arc::clone(&cache)).unwrap();

        // Prime the cache
        rt.block_on(async {
            for i in 0..*capacity {
                let _ = router.dispatch_prompt(&format!("prompt {i}"), "system").await;
            }
        });

        group.bench_with_input(
            BenchmarkId::new("capacity", capacity),
            &(router, *capacity),
            |b, (r, _)| {
                b.iter(|| {
                    let rt = Runtime::new().unwrap();
                    rt.block_on(async {
                        black_box(r.dispatch_prompt("prompt 0", "system").await.unwrap());
                    })
                })
            },
        );
    }
    group.finish();
}

fn bench_speculative_cache_miss(c: &mut Criterion) {
    let mut group = c.benchmark_group("speculative_cache_miss");

    for capacity in [16, 64, 256, 1024].iter() {
        let cfg = RouterConfig::default();
        let router = TokenRouter::new(cfg).unwrap();
        let mut i = 0u64;

        group.bench_with_input(
            BenchmarkId::new("capacity", capacity),
            &(router, *capacity),
            |b, (r, _)| {
                b.iter(|| {
                    let rt = Runtime::new().unwrap();
                    i += 1;
                    rt.block_on(async {
                        black_box(r.dispatch_prompt(&format!("miss prompt {i}"), "system").await.unwrap());
                    })
                })
            },
        );
    }
    group.finish();
}

fn bench_router_dispatch_synthetic(c: &mut Criterion) {
    let mut group = c.benchmark_group("router_dispatch_synthetic");

    for prompt_kb in [1, 4, 16, 64].iter() {
        let cfg = RouterConfig::default();
        let router = TokenRouter::new(cfg).unwrap();
        let prompt = "x".repeat(prompt_kb * 1024);
        group.throughput(Throughput::Bytes(prompt.len() as u64));

        group.bench_with_input(
            BenchmarkId::new("prompt_kb", prompt_kb),
            &(router, prompt, "system prompt".to_string()),
            |b, (r, p, s)| {
                b.iter(|| {
                    let rt = Runtime::new().unwrap();
                    rt.block_on(async {
                        black_box(r.dispatch_prompt(black_box(p), black_box(s)).await.unwrap());
                    })
                })
            },
        );
    }
    group.finish();
}

fn bench_token_metrics(c: &mut Criterion) {
    let mut group = c.benchmark_group("token_metrics");

    group.bench_function("record_usage", |b| {
        let metrics = zylcode_core::router::TokenMetrics::default();
        let mut input = 100u64;
        let mut output = 200u64;
        b.iter(|| {
            input += 1;
            output += 2;
            let _: () = metrics.record_usage(input, output);
            black_box(());
        })
    });

    group.bench_function("record_saved", |b| {
        let metrics = zylcode_core::router::TokenMetrics::default();
        let mut saved = 50u64;
        b.iter(|| {
            saved += 1;
            let _: () = metrics.record_saved(saved);
            black_box(());
        })
    });

    group.bench_function("record_fallback", |b| {
        let metrics = zylcode_core::router::TokenMetrics::default();
        b.iter(|| {
            let _: () = metrics.record_fallback();
            black_box(());
        })
    });

    group.bench_function("snapshot", |b| {
        let metrics = zylcode_core::router::TokenMetrics::default();
        metrics.record_usage(1000, 2000);
        b.iter(|| black_box(metrics.snapshot()))
    });

    group.finish();
}

fn bench_cache_hash_key(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_hash_key");

    for prompt_kb in [1, 4, 16, 64, 256].iter() {
        let prompt = "x".repeat(prompt_kb * 1024);
        let system = "system prompt";
        let model = "anthropic/claude-3.5-sonnet";
        group.throughput(Throughput::Bytes(prompt.len() as u64));

        group.bench_with_input(
            BenchmarkId::new("prompt_kb", prompt_kb),
            &(prompt, system, model),
            |b, (p, s, m)| {
                b.iter(|| black_box(SpeculativeCache::hash_key(black_box(p), black_box(s), black_box(m))))
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_parse_artifacts,
    bench_parse_fences,
    bench_trim_to_window,
    bench_speculative_cache_hit,
    bench_speculative_cache_miss,
    bench_router_dispatch_synthetic,
    bench_token_metrics,
    bench_cache_hash_key
);
criterion_main!(benches);