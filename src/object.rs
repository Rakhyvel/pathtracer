use apricot::{plane::Plane, ray::Ray, sphere::Sphere};

use crate::hit_info::HitInfo;

pub trait Object {
    fn intersect(&self, ray: &Ray) -> Option<HitInfo>;
}

const EPS: f32 = 1e-5;

impl Object for Sphere {
    fn intersect(&self, ray: &Ray) -> Option<HitInfo> {
        let m: nalgebra_glm::Vec3 = ray.origin - self.center;
        let b = m.dot(&ray.dir);
        let c = m.dot(&m) - self.radius * self.radius;

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

        let outward = (point - self.center).normalize();
        let front_face = ray.dir.dot(&outward) < 0.0;
        let normal = if front_face { outward } else { -outward };

        Some(HitInfo {
            point,
            normal,
            depth: t,
        })
    }
}

impl Object for Plane {
    fn intersect(&self, ray: &Ray) -> Option<HitInfo> {
        let denom = self.normal().dot(&ray.dir);
        if denom.abs() < EPS {
            // parellel
            return None;
        }

        let t = -(self.normal().dot(&ray.origin) + self.dist) / denom;

        if t < 0.0 {
            return None; // intersection behind ray origin
        }

        let point = ray.origin + ray.dir * t;

        let outward = self.normal();
        let front_face = ray.dir.dot(&outward) < 0.0;
        let normal = if front_face { outward } else { -outward };

        Some(HitInfo {
            point,
            normal: normal,
            depth: t,
        })
    }
}

mod tests {
    use super::*;

    #[test]
    fn sphere_hit_center() {
        let sphere = Sphere {
            center: nalgebra_glm::vec3(0.0, 0.0, 0.0),
            radius: 1.0,
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
        let sphere = Sphere {
            center: nalgebra_glm::vec3(0.0, 0.0, 0.0),
            radius: 1.0,
        };
        let ray = Ray {
            origin: nalgebra_glm::vec3(0.0, 0.0, -3.0),
            dir: nalgebra_glm::vec3(0.0, 1.0, 0.0).normalize(),
        };

        assert!(sphere.intersect(&ray).is_none());
    }

    #[test]
    fn sphere_inside_hit() {
        let sphere = Sphere {
            center: nalgebra_glm::vec3(0.0, 0.0, 0.0),
            radius: 1.0,
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

    #[test]
    fn plane_hit_with_ray() {
        let plane = Plane::from_center_normal(
            nalgebra_glm::vec3(0.0, 0.0, 0.0),
            nalgebra_glm::vec3(0.0, 0.0, 1.0),
        );
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
        let plane = Plane::from_center_normal(
            nalgebra_glm::vec3(0.0, 0.0, 0.0),
            nalgebra_glm::vec3(0.0, 0.0, 1.0),
        );
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
        let plane = Plane::from_center_normal(
            nalgebra_glm::vec3(0.0, 0.0, 0.0),
            nalgebra_glm::vec3(0.0, 0.0, 1.0),
        );
        let ray = Ray {
            origin: nalgebra_glm::vec3(0.0, 0.0, -3.0),
            dir: nalgebra_glm::vec3(1.0, 0.0, 0.0).normalize(),
        };

        assert!(plane.intersect(&ray).is_none());
    }
}
