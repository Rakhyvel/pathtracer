use apricot::ray::Ray;
use rand::{Rng, rngs::SmallRng};

use crate::{
    hit_info::HitInfo,
    material::{Material, ScatterResult, reflect},
};

pub struct Dielectric {
    pub ior: f32,
    pub tint: nalgebra_glm::Vec3,
}

const EPS: f32 = 1e-4;

impl Material for Dielectric {
    fn scatter(&self, ray: &Ray, hit: &HitInfo, rng: &mut SmallRng) -> Option<ScatterResult> {
        let i = ray.dir().normalize();
        let n = hit.normal;

        let reflect_dir = reflect(i, n);

        let (refract_dir, fresnel) = self.refract(i, n);

        let r: f32 = rng.gen_range(0.0..1.0);
        let choose_reflect = r < fresnel;

        if choose_reflect {
            Some(ScatterResult {
                ray: Ray::new(hit.point + n * EPS, reflect_dir),
                attenuation: nalgebra_glm::vec3(1.0, 1.0, 1.0),
            })
        } else {
            Some(ScatterResult {
                ray: Ray::new(hit.point - n * EPS, refract_dir),
                attenuation: self.tint,
            })
        }
    }
}

impl Dielectric {
    fn refract(&self, i: nalgebra_glm::Vec3, n: nalgebra_glm::Vec3) -> (nalgebra_glm::Vec3, f32) {
        let cosi = -i.dot(&n).clamp(-1.0, 1.0);
        let etai = 1.0;
        let etat = self.ior;
        let eta = etai / etat;

        let k = 1.0 - eta * eta * (1.0 - cosi * cosi);
        if k < 0.0 {
            return (nalgebra_glm::vec3(0.0, 0.0, 0.0), 1.0); // total internal reflection
        }

        let r0 = ((etai - etat) / (etai + etat)).powf(2.0);
        let fresnel = r0 + (1.0 - r0) * (1.0 - cosi).powf(5.0);

        let out = eta * i + (eta * cosi - k.sqrt()) * n;
        (out, fresnel)
    }
}
