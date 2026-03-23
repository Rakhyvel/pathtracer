use crate::{
    hit_info::HitInfo,
    material::{Material, ScatterResult},
};

use apricot::ray::Ray;
use rand::{Rng, SeedableRng, rngs::StdRng};

fn random_unit_vector() -> nalgebra_glm::Vec3 {
    let mut rng = rand::rngs::StdRng::from_entropy();
    let theta: f32 = rng.gen_range(0.0..2.0 * 3.0);
    let phi: f32 = (rng.gen_range(-1.0..1.0) as f32).acos(); // cos⁻¹(z) for uniform sphere

    let x = phi.sin() * theta.cos();
    let y = phi.sin() * theta.sin();
    let z = phi.cos();

    nalgebra_glm::vec3(x, y, z)
}

pub struct Lambertian {
    pub albedo: nalgebra_glm::Vec3,
}

const EPS: f32 = 1e-4;

impl Material for Lambertian {
    fn scatter(&self, _ray: &Ray, hit: &HitInfo) -> Option<ScatterResult> {
        let mut dir = hit.normal + random_unit_vector();

        if dir.norm() < 1e-6 {
            dir = hit.normal;
        }

        Some(ScatterResult {
            ray: Ray {
                origin: hit.point + hit.normal * EPS,
                dir,
            },
            attenuation: self.albedo,
        })
    }
}
