use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::runtime::Runtime;
use zylcode_mcp::registry::ToolRegistry;
use zylcode_mcp::config::{McpToolConfig, McpTransport};
use zylcode_mcp::tool::DynamicTool;

fn bench_tool_registry_register(c: &mut Criterion) {
    let mut group = c.benchmark_group("tool_registry_register");

    for num_tools in [10, 50, 100, 500, 1000].iter() {
        let rt = Runtime::new().unwrap();
        let registry = Arc::new(ToolRegistry::new());

        group.throughput(Throughput::Elements(*num_tools as u64));
        group.bench_with_input(
            BenchmarkId::new("register", num_tools),
            &(*num_tools, registry.clone()),
            |b, (n, reg)| {
                b.iter_custom(|iters| {
                    let start = std::time::Instant::now();
                    for _ in 0..iters {
                        rt.block_on(async {
                            for i in 0..*n {
                                let cfg = McpToolConfig {
                                    id: format!("tool_{}", i),
                                    command: "echo".into(),
                                    transport: McpTransport::Stdio,
                                    env: Default::default(),
                                    enabled: true,
                                    description: None,
                                };
                                reg.register(Arc::new(DynamicTool::new(cfg))).await;
                            }
                            reg.clear().await;
                        });
                    }
                    start.elapsed()
                })
            },
        );
    }
    group.finish();
}

fn bench_tool_registry_get(c: &mut Criterion) {
    let mut group = c.benchmark_group("tool_registry_get");

    for num_tools in [10, 50, 100, 500, 1000].iter() {
        let rt = Runtime::new().unwrap();
        let registry = Arc::new(ToolRegistry::new());

        // Pre-populate
        rt.block_on(async {
            for i in 0..*num_tools {
                let cfg = McpToolConfig {
                    id: format!("tool_{}", i),
                    command: "echo".into(),
                    transport: McpTransport::Stdio,
                    env: Default::default(),
                    enabled: true,
                    description: None,
                };
                registry.register(Arc::new(DynamicTool::new(cfg))).await;
            }
        });

        group.bench_with_input(
            BenchmarkId::new("get", num_tools),
            &(registry.clone(), *num_tools, rt),
            |b, (reg, n, rt)| {
                let counter = Arc::new(AtomicUsize::new(0));
                b.iter(|| {
                    let idx = counter.fetch_add(1, Ordering::Relaxed) % *n;
                    rt.block_on(async {
                        black_box(reg.get(&format!("tool_{}", idx)).await);
                    })
                })
            },
        );
    }
    group.finish();
}

fn bench_tool_registry_list(c: &mut Criterion) {
    let mut group = c.benchmark_group("tool_registry_list");

    for num_tools in [10, 50, 100, 500, 1000].iter() {
        let rt = Runtime::new().unwrap();
        let registry = Arc::new(ToolRegistry::new());

        // Pre-populate
        rt.block_on(async {
            for i in 0..*num_tools {
                let cfg = McpToolConfig {
                    id: format!("tool_{}", i),
                    command: "echo".into(),
                    transport: McpTransport::Stdio,
                    env: Default::default(),
                    enabled: true,
                    description: None,
                };
                registry.register(Arc::new(DynamicTool::new(cfg))).await;
            }
        });

        group.throughput(Throughput::Elements(*num_tools as u64));
        group.bench_with_input(
            BenchmarkId::new("list", num_tools),
            &(registry.clone(), rt),
            |b, (reg, rt)| {
                b.iter(|| {
                    rt.block_on(async {
                        black_box(reg.list().await);
                    })
                })
            },
        );
    }
    group.finish();
}

fn bench_tool_registry_concurrent_read(c: &mut Criterion) {
    let mut group = c.benchmark_group("tool_registry_concurrent_read");

    for num_tools in [100, 500, 1000].iter() {
        for concurrency in [2, 4, 8, 16].iter() {
            let rt = Runtime::new().unwrap();
            let registry = Arc::new(ToolRegistry::new());

            // Pre-populate
            rt.block_on(async {
                for i in 0..*num_tools {
                    let cfg = McpToolConfig {
                        id: format!("tool_{}", i),
                        command: "echo".into(),
                        transport: McpTransport::Stdio,
                        env: Default::default(),
                        enabled: true,
                        description: None,
                    };
                    registry.register(Arc::new(DynamicTool::new(cfg))).await;
                }
            });

            group.throughput(Throughput::Elements((*num_tools * *concurrency) as u64));
            let bench_id = format!("read_tools_{}_concurrency_{}", num_tools, concurrency);
            // Clone what we need for the benchmark
            let registry_clone = registry.clone();
            let n = *num_tools;
            let conc = *concurrency;
            let rt_clone = rt;
            group.bench_with_input(
                BenchmarkId::new(bench_id, 0),
                &(), // dummy input, we use captured vars
                |b, _| {
                    b.iter_custom(|iters| {
                        let start = std::time::Instant::now();
                        for _ in 0..iters {
                            rt_clone.block_on(async {
                                let mut handles = vec![];
                                let reg = registry_clone.clone();
                                for _ in 0..conc {
                                    let reg = reg.clone();
                                    handles.push(tokio::spawn(async move {
                                        for i in 0..n {
                                            let _ = reg.get(&format!("tool_{}", i)).await;
                                        }
                                    }));
                                }
                                for h in handles {
                                    let _ = h.await;
                                }
                            });
                        }
                        start.elapsed()
                    })
                },
            );
        }
    }
    group.finish();
}

fn bench_tool_registry_concurrent_write(c: &mut Criterion) {
    let mut group = c.benchmark_group("tool_registry_concurrent_write");

    for num_tools in [100, 500, 1000].iter() {
        for concurrency in [2, 4, 8].iter() {
            let rt = Runtime::new().unwrap();
            let registry = Arc::new(ToolRegistry::new());

            let bench_id = format!("write_tools_{}_concurrency_{}", num_tools, concurrency);
            let reg = registry.clone();
            let n = *num_tools;
            let conc = *concurrency;
            let rt_clone = rt;
            group.bench_with_input(
                BenchmarkId::new(bench_id, 0),
                &(),
                |b, _| {
                    b.iter_custom(|iters| {
                        let start = std::time::Instant::now();
                        for _ in 0..iters {
                            rt_clone.block_on(async {
                                let mut handles = vec![];
                                let reg = reg.clone();
                                for thread in 0..conc {
                                    let reg = reg.clone();
                                    handles.push(tokio::spawn(async move {
                                        for i in 0..n {
                                            let cfg = McpToolConfig {
                                                id: format!("tool_{}_{}", thread, i),
                                                command: "echo".into(),
                                                transport: McpTransport::Stdio,
                                                env: Default::default(),
                                                enabled: true,
                                                description: None,
                                            };
                                            reg.register(Arc::new(DynamicTool::new(cfg))).await;
                                        }
                                    }));
                                }
                                for h in handles {
                                    let _ = h.await;
                                }
                                reg.clear().await;
                            });
                        }
                        start.elapsed()
                    })
                },
            );
        }
    }
    group.finish();
}

fn bench_tool_registry_mixed_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("tool_registry_mixed_workload");

    for num_tools in [100, 500].iter() {
        for concurrency in [4, 8].iter() {
            let rt = Runtime::new().unwrap();
            let registry = Arc::new(ToolRegistry::new());

            // Pre-populate some tools
            rt.block_on(async {
                for i in 0..*num_tools {
                    let cfg = McpToolConfig {
                        id: format!("tool_{}", i),
                        command: "echo".into(),
                        transport: McpTransport::Stdio,
                        env: Default::default(),
                        enabled: true,
                        description: None,
                    };
                    registry.register(Arc::new(DynamicTool::new(cfg))).await;
                }
            });

            let bench_id = format!("mixed_tools_{}_concurrency_{}", num_tools, concurrency);
            let reg = registry.clone();
            let n = *num_tools;
            let conc = *concurrency;
            let rt_clone = rt;
            group.bench_with_input(
                BenchmarkId::new(bench_id, 0),
                &(),
                |b, _| {
                    b.iter_custom(|iters| {
                        let start = std::time::Instant::now();
                        for _ in 0..iters {
                            rt_clone.block_on(async {
                                let mut handles = vec![];
                                let reg = reg.clone();
                                for thread in 0..conc {
                                    let reg = reg.clone();
                                    handles.push(tokio::spawn(async move {
                                        // 70% reads, 30% writes
                                        for i in 0..n {
                                            if i % 10 < 7 {
                                                let _ = reg.get(&format!("tool_{}", i % n)).await;
                                            } else {
                                                let cfg = McpToolConfig {
                                                    id: format!("new_tool_{}_{}", thread, i),
                                                    command: "echo".into(),
                                                    transport: McpTransport::Stdio,
                                                    env: Default::default(),
                                                    enabled: true,
                                                    description: None,
                                                };
                                                reg.register(Arc::new(DynamicTool::new(cfg))).await;
                                            }
                                        }
                                    }));
                                }
                                for h in handles {
                                    let _ = h.await;
                                }
                            });
                        }
                        start.elapsed()
                    })
                },
            );
        }
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_tool_registry_register,
    bench_tool_registry_get,
    bench_tool_registry_list,
    bench_tool_registry_concurrent_read,
    bench_tool_registry_concurrent_write,
    bench_tool_registry_mixed_workload
);
criterion_main!(benches);