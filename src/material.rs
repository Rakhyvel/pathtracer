use apricot::ray::Ray;
use rand::Rng;

use crate::hit_info::HitInfo;

pub trait Material: Send + Sync {
    fn emission(&self) -> nalgebra_glm::Vec3 {
        return nalgebra_glm::vec3(0.0, 0.0, 0.0);
    }

    fn scatter(&self, ray: &Ray, hit: &HitInfo) -> Option<ScatterResult>;
}

pub struct ScatterResult {
    pub ray: Ray,
    pub attenuation: nalgebra_glm::Vec3,
}

pub fn random_unit_vector(rng: &mut impl Rng) -> nalgebra_glm::Vec3 {
    loop {
        let v = nalgebra_glm::vec3(
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
        );
        if v.norm_squared() < 1.0 {
            return v.normalize();
        }
    }
}
