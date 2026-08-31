use criterion::{black_box, criterion_group, criterion_main, Criterion};
use zylcode_core::pipeline::ArtifactPipeline;

fn payload() -> String {
    let mut s = String::from("<zylcode-response>\n");
    for i in 0..20 {
        s.push_str(&format!(
            r#"<artifact kind="RustModule" path="src/gen{i}.rs"><content><![CDATA[pub fn f{i}() {{}}]]></content></artifact>"#
        ));
    }
    s.push_str("</zylcode-response>");
    s
}

fn bench_parse_artifacts(c: &mut Criterion) {
    let raw = payload();
    c.bench_function("parse_artifacts_20", |b| {
        b.iter(|| black_box(ArtifactPipeline::parse_artifacts(black_box(&raw))))
    });
}

fn bench_parse_fences(c: &mut Criterion) {
    let raw = (0..20)
        .map(|i| format!("```rust\npub fn f{i}() {{}}\n```\n"))
        .collect::<String>();
    c.bench_function("parse_fences_20", |b| {
        b.iter(|| black_box(ArtifactPipeline::parse_artifacts(black_box(&raw))))
    });
}

criterion_group!(benches, bench_parse_artifacts, bench_parse_fences);
criterion_main!(benches);
