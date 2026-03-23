use apricot::ray::Ray;

use crate::hit_info::HitInfo;

pub trait Meterial {
    fn emission(&self) -> nalgebra_glm::Vec3;

    fn scatter(&self, ray: Ray, hit: HitInfo) -> Option<ScatterResult>;
}

pub struct ScatterResult {
    ray: Ray,
    attenuation: nalgebra_glm::Vec3,
}
