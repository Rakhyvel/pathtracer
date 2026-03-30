use apricot::ray::Ray;
use apricot::render_core::OpaqueId;
use criterion::{Criterion, criterion_group, criterion_main};
use nalgebra_glm::vec3;
use pathtracer::hit_info::HitInfo;
use pathtracer::lambertian::Lambertian;
use pathtracer::material::Material;
use pathtracer::material_mgr::MaterialId;
use rand::SeedableRng;
use rand::rngs::SmallRng;

fn material_benchmark(c: &mut Criterion) {
    let lambert = Lambertian {
        albedo: vec3(1.0, 1.0, 1.0),
    };

    let ray = Ray::new(vec3(-1.0, 0.0, 0.0), vec3(1.0, 0.0, 0.0));

    let hit_info = HitInfo {
        depth: 1.0,
        normal: vec3(0.0, 1.0, 0.0),
        point: vec3(0.0, 0.0, 0.0),
        material: MaterialId::new(0),
    };

    let mut rng = SmallRng::from_entropy();

    c.bench_function("lambert scatter", |b| {
        b.iter(|| std::hint::black_box(lambert.scatter(&ray, &hit_info, &mut rng)))
    });
}

criterion_group!(benches, material_benchmark);
criterion_main!(benches);
