use crate::{
    hit_info::HitInfo,
    material::{Material, ScatterResult, random_unit_vector},
};

use apricot::ray::Ray;
use rand::{Rng, SeedableRng, thread_rng};

pub struct Lambertian {
    pub albedo: nalgebra_glm::Vec3,
}

const EPS: f32 = 1e-4;

impl Material for Lambertian {
    fn scatter(&self, _ray: &Ray, hit: &HitInfo) -> Option<ScatterResult> {
        let mut rng = thread_rng();
        let mut dir = hit.normal + random_unit_vector(&mut rng);

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
