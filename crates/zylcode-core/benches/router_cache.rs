use criterion::{black_box, criterion_group, criterion_main, Criterion};
use zylcode_core::router::{RouterConfig, SpeculativeCache, TokenRouter};

fn bench_cache_hit(c: &mut Criterion) {
    let cfg = RouterConfig::default();
    let cache = std::sync::Arc::new(SpeculativeCache::new(128, std::time::Duration::from_secs(600)));
    let router = TokenRouter::with_cache(cfg, cache).unwrap();
    // Prime cache via synthetic offline path
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let _ = router.dispatch_prompt("bench prompt", "system").await;
    });
    c.bench_function("router_cache_hit", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                black_box(router.dispatch_prompt("bench prompt", "system").await.unwrap());
            })
        })
    });
}

fn bench_cache_miss(c: &mut Criterion) {
    let cfg = RouterConfig::default();
    let router = TokenRouter::new(cfg).unwrap();
    let mut i = 0u64;
    c.bench_function("router_cache_miss", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            i += 1;
            let prompt = format!("bench miss {i}");
            rt.block_on(async {
                black_box(router.dispatch_prompt(&prompt, "system").await.unwrap());
            })
        })
    });
}

criterion_group!(benches, bench_cache_hit, bench_cache_miss);
criterion_main!(benches);
