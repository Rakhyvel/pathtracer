use apricot::ray::Ray;

use crate::{
    hit_info::HitInfo,
    material::{Material, ScatterResult, random_unit_vector},
};

pub struct Metallic {
    pub albedo: nalgebra_glm::Vec3,
    pub roughness: f32,
}

const EPS: f32 = 1e-4;

impl Material for Metallic {
    fn scatter(&self, ray: &Ray, hit: &HitInfo) -> Option<ScatterResult> {
        let i = ray.dir().normalize();
        let n = hit.normal;

        let reflect_dir = i - 2.0 * i.dot(&n) * n;

        // Add roughness by perturbing the reflect direction with a random unit vector
        let mut rng = rand::thread_rng();
        let fuzz = random_unit_vector(&mut rng) * self.roughness;
        let scattered = (reflect_dir + fuzz).normalize();

        // If the scattered ray points into the surface, absorb it
        if scattered.dot(&n) <= 0.0 {
            return None;
        }

        Some(ScatterResult {
            ray: Ray::new(hit.point + n * EPS, scattered),
            attenuation: self.albedo,
        })
    }
}
