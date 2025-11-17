use criterion::{Criterion, criterion_group, criterion_main};

fn bench_template_matching(c: &mut Criterion) {
}

criterion_group!(benches, bench_template_matching);
criterion_main!(benches);