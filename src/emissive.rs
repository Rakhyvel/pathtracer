use apricot::ray::Ray;

use crate::{
    hit_info::HitInfo,
    material::{Material, ScatterResult},
};

pub struct Emissive {
    pub color: nalgebra_glm::Vec3,
}

impl Material for Emissive {
    fn emission(&self) -> nalgebra_glm::Vec3 {
        self.color
    }

    fn scatter(&self, _ray: &Ray, hit: &HitInfo) -> Option<ScatterResult> {
        None
    }
}
