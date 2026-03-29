use apricot::ray::Ray;
use rand::rngs::SmallRng;

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

    fn scatter(&self, _ray: &Ray, _hit: &HitInfo, _rng: &mut SmallRng) -> Option<ScatterResult> {
        None
    }
}
