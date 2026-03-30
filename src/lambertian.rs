use crate::{
    hit_info::HitInfo,
    material::{Material, ScatterResult, random_cosine_direction},
    onb::ONB,
};

use apricot::ray::Ray;
use rand::rngs::SmallRng;

pub struct Lambertian {
    pub albedo: nalgebra_glm::Vec3,
}

const EPS: f32 = 1e-4;

impl Material for Lambertian {
    fn scatter(&self, _ray: &Ray, hit: &HitInfo, rng: &mut SmallRng) -> Option<ScatterResult> {
        let onb = ONB::from_w(hit.normal);
        let dir = onb.to_world(random_cosine_direction(rng));

        Some(ScatterResult {
            ray: Ray::new(hit.point + hit.normal * EPS, dir),
            attenuation: self.albedo,
        })
    }
}
