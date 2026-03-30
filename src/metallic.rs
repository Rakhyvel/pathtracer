use apricot::ray::Ray;
use rand::rngs::SmallRng;

use crate::{
    hit_info::HitInfo,
    material::{ScatterResult, reflect, sample_ggx},
};

pub struct Metallic {
    pub albedo: nalgebra_glm::Vec3,
    pub roughness: f32,
}

const EPS: f32 = 1e-4;

impl Metallic {
    pub fn scatter(&self, ray: &Ray, hit: &HitInfo, rng: &mut SmallRng) -> Option<ScatterResult> {
        let i = ray.dir().normalize();
        let n = hit.normal;

        let h = sample_ggx(n, self.roughness, rng);
        let scattered = reflect(i, h).normalize();

        // If the scattered ray points into the surface, absorb it
        if scattered.dot(&n) <= 0.0 {
            return None;
        }

        let cos_theta = (-i.dot(&h)).max(0.0);
        let fresnel = self.albedo
            + (nalgebra_glm::Vec3::repeat(1.0) - self.albedo) * (1.0 - cos_theta).powf(5.0);

        Some(ScatterResult {
            ray: Ray::new(hit.point + n * EPS, scattered),
            attenuation: fresnel,
        })
    }
}
