use apricot::{plane::Plane, ray::Ray, sphere::Sphere};

use crate::hit_info::HitInfo;

pub trait Object {
    fn intersect(&self, ray: &Ray) -> Option<HitInfo>;
}
