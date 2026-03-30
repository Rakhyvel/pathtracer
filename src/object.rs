use apricot::ray::Ray;

use crate::{hit_info::HitInfo, plane::MaterialPlane, sphere::MaterialSphere};

pub enum ObjectEnum {
    Plane(MaterialPlane),
    Sphere(MaterialSphere),
}

impl ObjectEnum {
    #[inline(always)]
    pub fn intersect(&self, ray: &Ray) -> Option<HitInfo> {
        match self {
            ObjectEnum::Plane(p) => p.intersect(ray),
            ObjectEnum::Sphere(s) => s.intersect(ray),
        }
    }
}
