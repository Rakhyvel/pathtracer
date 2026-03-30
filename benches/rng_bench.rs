use criterion::{Criterion, criterion_group, criterion_main};
use pathtracer::material::{random_cosine_direction, random_unit_vector};
use rand::SeedableRng;
use rand::rngs::SmallRng;

fn rng_benchmark(c: &mut Criterion) {
    let mut rng = SmallRng::from_entropy();

    c.bench_function("cosine hemisphere", |b| {
        b.iter(|| std::hint::black_box(random_cosine_direction(&mut rng)))
    });

    c.bench_function("uniform hemisphere", |b| {
        b.iter(|| std::hint::black_box(random_unit_vector(&mut rng)))
    });
}

criterion_group!(benches, rng_benchmark);
criterion_main!(benches);
