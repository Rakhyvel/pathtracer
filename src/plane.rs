use apricot::{plane::Plane, ray::Ray};

use crate::{hit_info::HitInfo, material_mgr::MaterialId, object::Object};

const EPS: f32 = 1e-5;
pub struct MaterialPlane {
    plane: Plane,
    mat_id: MaterialId,
}

impl MaterialPlane {
    pub fn new(normal: nalgebra_glm::Vec3, dist: f32, mat_id: MaterialId) -> Self {
        Self {
            plane: Plane::new(normal, dist),
            mat_id,
        }
    }
}

impl Object for MaterialPlane {
    fn intersect(&self, ray: &Ray) -> Option<HitInfo> {
        let denom = self.plane.normal().dot(&ray.dir);
        if denom.abs() < EPS {
            // parellel
            return None;
        }

        let t = -(self.plane.normal().dot(&ray.origin) + self.plane.dist) / denom;

        if t < 0.0 {
            return None; // intersection behind ray origin
        }

        let point = ray.origin + ray.dir * t;

        let outward = self.plane.normal();
        let front_face = ray.dir.dot(&outward) < 0.0;
        let normal = if front_face { outward } else { -outward };

        Some(HitInfo {
            point,
            normal: normal,
            depth: t,
            material: self.mat_id,
        })
    }
}

mod tests {
    use apricot::render_core::OpaqueId;

    use super::*;

    #[test]
    fn plane_hit_with_ray() {
        let plane = MaterialPlane {
            plane: Plane::from_center_normal(
                nalgebra_glm::vec3(0.0, 0.0, 0.0),
                nalgebra_glm::vec3(0.0, 0.0, 1.0),
            ),
            mat_id: MaterialId::new(0),
        };
        let ray = Ray {
            origin: nalgebra_glm::vec3(0.0, 0.0, 3.0),
            dir: nalgebra_glm::vec3(0.0, 0.0, -1.0).normalize(),
        };

        let hit = plane.intersect(&ray).expect("Ray should hit plane");
        assert!((hit.point - nalgebra_glm::vec3(0.0, 0.0, 0.0)).magnitude() < EPS);
        assert!((hit.normal - nalgebra_glm::vec3(0.0, 0.0, 1.0)).magnitude() < EPS);
        assert!((hit.depth - 3.0).abs() < EPS);
    }

    #[test]
    fn plane_hit_against_ray() {
        let plane = MaterialPlane {
            plane: Plane::from_center_normal(
                nalgebra_glm::vec3(0.0, 0.0, 0.0),
                nalgebra_glm::vec3(0.0, 0.0, 1.0),
            ),
            mat_id: MaterialId::new(0),
        };
        let ray = Ray {
            origin: nalgebra_glm::vec3(0.0, 0.0, -3.0),
            dir: nalgebra_glm::vec3(0.0, 0.0, 1.0).normalize(),
        };

        let hit = plane.intersect(&ray).expect("Ray should hit plane");
        assert!((hit.point - nalgebra_glm::vec3(0.0, 0.0, 0.0)).magnitude() < EPS);
        assert!((hit.normal - nalgebra_glm::vec3(0.0, 0.0, -1.0)).magnitude() < EPS);
        assert!((hit.depth - 3.0).abs() < EPS);
    }

    #[test]
    fn plane_parallel_miss() {
        let plane = MaterialPlane {
            plane: Plane::from_center_normal(
                nalgebra_glm::vec3(0.0, 0.0, 0.0),
                nalgebra_glm::vec3(0.0, 0.0, 1.0),
            ),
            mat_id: MaterialId::new(0),
        };
        let ray = Ray {
            origin: nalgebra_glm::vec3(0.0, 0.0, -3.0),
            dir: nalgebra_glm::vec3(1.0, 0.0, 0.0).normalize(),
        };

        assert!(plane.intersect(&ray).is_none());
    }
}
