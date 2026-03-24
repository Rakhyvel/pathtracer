use apricot::{ray::Ray, sphere::Sphere};

use crate::{hit_info::HitInfo, material_mgr::MaterialId, object::Object};

pub struct MaterialSphere {
    sphere: Sphere,
    mat_id: MaterialId,
}

const EPS: f32 = 1e-5;

impl MaterialSphere {
    pub fn new(center: nalgebra_glm::Vec3, radius: f32, mat_id: MaterialId) -> Self {
        Self {
            sphere: Sphere::new(center, radius),
            mat_id,
        }
    }
}

impl Object for MaterialSphere {
    fn intersect(&self, ray: &Ray) -> Option<HitInfo> {
        let m: nalgebra_glm::Vec3 = ray.origin - self.sphere.center;
        let b = m.dot(&ray.dir);
        let c = m.dot(&m) - self.sphere.radius * self.sphere.radius;

        // ray outside sphere and pointing away
        if c > 0.0 && b > 0.0 {
            return None;
        }

        let discr = b * b - c;
        if discr < 0.0 {
            return None;
        }

        let sqrt_discr = discr.sqrt();

        // try nearest hit first
        let mut t = -b - sqrt_discr;

        // if behind origin or too close, try far intersection
        if t < EPS {
            t = -b + sqrt_discr;
            if t < EPS {
                return None;
            }
        }

        let point = ray.at(t);

        let outward = (point - self.sphere.center).normalize();
        let front_face = ray.dir.dot(&outward) < 0.0;
        let normal = if front_face { outward } else { -outward };

        Some(HitInfo {
            point,
            normal,
            depth: t,
            material: self.mat_id,
        })
    }
}

mod tests {
    use apricot::render_core::OpaqueId;

    use super::*;

    #[test]
    fn sphere_hit_center() {
        let sphere = MaterialSphere {
            sphere: Sphere {
                center: nalgebra_glm::vec3(0.0, 0.0, 0.0),
                radius: 1.0,
            },
            mat_id: MaterialId::new(0),
        };
        let ray = Ray {
            origin: nalgebra_glm::vec3(0.0, 0.0, -3.0),
            dir: nalgebra_glm::vec3(0.0, 0.0, 1.0),
        };

        let hit = sphere.intersect(&ray).expect("Ray should hit sphere");
        assert!((hit.point - nalgebra_glm::vec3(0.0, 0.0, -1.0)).magnitude() < EPS);
        assert!((hit.normal - nalgebra_glm::vec3(0.0, 0.0, -1.0)).magnitude() < EPS);
        assert!((hit.depth - 2.0).abs() < EPS);
    }

    #[test]
    fn sphere_miss() {
        let sphere = MaterialSphere {
            sphere: Sphere {
                center: nalgebra_glm::vec3(0.0, 0.0, 0.0),
                radius: 1.0,
            },
            mat_id: MaterialId::new(0),
        };
        let ray = Ray {
            origin: nalgebra_glm::vec3(0.0, 0.0, -3.0),
            dir: nalgebra_glm::vec3(0.0, 1.0, 0.0).normalize(),
        };

        assert!(sphere.intersect(&ray).is_none());
    }

    #[test]
    fn sphere_inside_hit() {
        let sphere = MaterialSphere {
            sphere: Sphere {
                center: nalgebra_glm::vec3(0.0, 0.0, 0.0),
                radius: 1.0,
            },
            mat_id: MaterialId::new(0),
        };
        let ray = Ray {
            origin: nalgebra_glm::vec3(0.0, 0.0, 0.0),
            dir: nalgebra_glm::vec3(0.0, 0.0, 1.0).normalize(),
        };

        let hit = sphere.intersect(&ray).expect("Ray should hit sphere");
        assert!((hit.point - nalgebra_glm::vec3(0.0, 0.0, 1.0)).magnitude() < EPS);
        assert!((hit.normal - nalgebra_glm::vec3(0.0, 0.0, -1.0)).magnitude() < EPS);
        assert!((hit.depth - 1.0).abs() < EPS);
    }
}
