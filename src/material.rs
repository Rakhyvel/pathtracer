use apricot::ray::Ray;

use crate::hit_info::HitInfo;

pub trait Material {
    fn emission(&self) -> nalgebra_glm::Vec3 {
        return nalgebra_glm::vec3(0.0, 0.0, 0.0);
    }

    fn scatter(&self, ray: &Ray, hit: &HitInfo) -> Option<ScatterResult>;
}

pub struct ScatterResult {
    pub ray: Ray,
    pub attenuation: nalgebra_glm::Vec3,
}
