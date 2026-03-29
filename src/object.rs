use apricot::ray::Ray;

use crate::hit_info::HitInfo;

pub trait Object: Sync + Send {
    fn intersect(&self, ray: &Ray) -> Option<HitInfo>;
}
