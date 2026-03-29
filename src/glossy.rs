use apricot::ray::Ray;
use rand::Rng;

use crate::{
    hit_info::HitInfo,
    material::{Material, ScatterResult, random_unit_vector},
};

pub struct Glossy {
    pub roughness: f32,
    pub albedo: nalgebra_glm::Vec3,
}

const EPS: f32 = 1e-4;
const IOR: f32 = 1.5;

impl Material for Glossy {
    fn scatter(&self, ray: &Ray, hit: &HitInfo) -> Option<ScatterResult> {
        let i = ray.dir().normalize();
        let n = hit.normal;

        let mut rng = rand::thread_rng();

        // Fresnel
        let cosi = (-i.dot(&n)).max(0.0);
        let r0 = ((1.0 - IOR) / (1.0 + IOR)).powf(2.0); // ~ 0.04, typical for plastic
        let fresnel = r0 + (1.0 - r0) * (1.0 - cosi).powf(5.0);

        // Spectral
        let reflect_dir = i - 2.0 * i.dot(&n) * n;
        let mut scattered =
            (reflect_dir + random_unit_vector(&mut rng) * self.roughness).normalize();
        if scattered.dot(&n) <= 0.0 {
            scattered = reflect_dir;
        }

        // Diffuse
        let diffuse = (n + random_unit_vector(&mut rng)).normalize();

        if rng.r#gen::<f32>() < fresnel {
            Some(ScatterResult {
                ray: Ray::new(hit.point + n * EPS, scattered),
                attenuation: nalgebra_glm::vec3(1.0, 1.0, 1.0),
            })
        } else {
            Some(ScatterResult {
                ray: Ray::new(hit.point + n * EPS, diffuse),
                attenuation: self.albedo,
            })
        }
    }
}
