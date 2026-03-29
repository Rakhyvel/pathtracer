use std::{cell::RefCell, f32::consts::PI};

use apricot::ray::Ray;
use rand::{Rng, SeedableRng, rngs::SmallRng};

use crate::{hit_info::HitInfo, onb::ONB};

pub trait Material: Send + Sync {
    fn emission(&self) -> nalgebra_glm::Vec3 {
        return nalgebra_glm::vec3(0.0, 0.0, 0.0);
    }

    fn scatter(&self, ray: &Ray, hit: &HitInfo, rng: &mut SmallRng) -> Option<ScatterResult>;
}

pub struct ScatterResult {
    pub ray: Ray,
    pub attenuation: nalgebra_glm::Vec3,
}

// Thread-local SmallRng
thread_local! {
    pub static THREAD_RNG: RefCell<SmallRng> = RefCell::new(SmallRng::from_entropy());
}

pub fn reflect(i: nalgebra_glm::Vec3, n: nalgebra_glm::Vec3) -> nalgebra_glm::Vec3 {
    i - 2.0 * i.dot(&n) * n
}

pub fn random_unit_vector(rng: &mut impl Rng) -> nalgebra_glm::Vec3 {
    loop {
        let v = nalgebra_glm::vec3(
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
        );
        if v.norm_squared() < 1.0 {
            return v.normalize();
        }
    }
}

pub fn random_cosine_direction(rng: &mut impl Rng) -> nalgebra_glm::Vec3 {
    loop {
        let x: f32 = rng.gen_range(-1.0..1.0);
        let y: f32 = rng.gen_range(-1.0..1.0);
        let r2 = x * x + y * y;
        if r2 < 1.0 {
            let z = 1.0 - r2; // approx sqrt(1 - r^2) without calling sqrt
            return nalgebra_glm::vec3(x, y, z);
        }
    }
}

pub fn sample_ggx(
    n: nalgebra_glm::Vec3,
    roughness: f32,
    rng: &mut impl rand::Rng,
) -> nalgebra_glm::Vec3 {
    let alpha = roughness * roughness;

    let r1: f32 = rng.r#gen();
    let r2: f32 = rng.r#gen();

    let phi = 2.0 * PI * r1;

    let cos_theta = ((1.0 - r2) / (1.0 + (alpha - 1.0) * r2)).sqrt();
    let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();

    let h_local = nalgebra_glm::vec3(sin_theta * phi.cos(), sin_theta * phi.sin(), cos_theta);

    let onb = ONB::from_w(n);
    onb.to_world(h_local).normalize()
}
