use std::{cell::RefCell, f32::consts::PI};

use apricot::ray::Ray;
use nalgebra_glm::{Vec3, vec3};
use rand::{Rng, SeedableRng, rngs::SmallRng};

use crate::{
    dielectric::Dielectric, emissive::Emissive, glossy::Glossy, hit_info::HitInfo,
    lambertian::Lambertian, metallic::Metallic, onb::ONB,
};

pub enum MaterialEnum {
    Dielectric(Dielectric),
    Emissive(Emissive),
    Glossy(Glossy),
    Lambertian(Lambertian),
    Metallic(Metallic),
}

impl MaterialEnum {
    #[inline(always)]
    pub fn emission(&self) -> Vec3 {
        match self {
            MaterialEnum::Emissive(e) => e.color,
            _ => vec3(0.0, 0.0, 0.0),
        }
    }

    #[inline(always)]
    pub fn scatter(&self, ray: &Ray, hit: &HitInfo, rng: &mut SmallRng) -> Option<ScatterResult> {
        match self {
            MaterialEnum::Dielectric(d) => d.scatter(ray, hit, rng),
            MaterialEnum::Emissive(_e) => None,
            MaterialEnum::Glossy(g) => g.scatter(ray, hit, rng),
            MaterialEnum::Lambertian(l) => l.scatter(ray, hit, rng),
            MaterialEnum::Metallic(m) => m.scatter(ray, hit, rng),
        }
    }
}

pub struct ScatterResult {
    pub ray: Ray,
    pub attenuation: Vec3,
}

// Thread-local SmallRng
thread_local! {
    pub static THREAD_RNG: RefCell<SmallRng> = RefCell::new(SmallRng::from_entropy());
}

pub fn reflect(i: Vec3, n: Vec3) -> Vec3 {
    i - 2.0 * i.dot(&n) * n
}

pub fn random_unit_vector(rng: &mut impl Rng) -> Vec3 {
    loop {
        let v = vec3(
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
        );
        if v.norm_squared() < 1.0 {
            return v.normalize();
        }
    }
}

pub fn random_cosine_direction(rng: &mut impl Rng) -> Vec3 {
    let r1: f32 = rng.r#gen();
    let r2: f32 = rng.r#gen();

    let phi = 2.0 * PI * r1;
    let r = r2.sqrt();

    let (sin_phi, cos_phi) = phi.sin_cos();

    let x = r * cos_phi;
    let y = r * sin_phi;
    let z = (1.0 - r2).sqrt();

    Vec3::new(x, y, z)
}

mod tests {
    #[allow(unused)]
    use super::*;

    #[test]
    fn test_mean_z() {
        let mut rng = SmallRng::from_entropy();
        let mut sum = 0.0;
        let n = 1_000_000;

        for _ in 0..n {
            let v = random_cosine_direction(&mut rng);
            sum += v.z;
        }
        let mean = sum / n as f32;

        // Shouldn't be biased
        assert!((mean - 0.66).abs() < 0.1);
    }

    #[test]
    fn all_normal() {
        let mut rng = SmallRng::from_entropy();
        let n = 1_000_000;

        for _ in 0..n {
            let v = random_cosine_direction(&mut rng);
            let v_norm = v.norm();
            println!("{}", v_norm);
            assert!((v_norm - 1.0).abs() < 0.1);
        }
    }
}

pub fn sample_ggx(n: Vec3, roughness: f32, rng: &mut impl rand::Rng) -> Vec3 {
    let alpha = roughness * roughness;

    let r1: f32 = rng.r#gen();
    let r2: f32 = rng.r#gen();

    let phi = 2.0 * PI * r1;

    let cos_theta = ((1.0 - r2) / (1.0 + (alpha - 1.0) * r2)).sqrt();
    let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();

    let h_local = vec3(sin_theta * phi.cos(), sin_theta * phi.sin(), cos_theta);

    let onb = ONB::from_w(n);
    onb.to_world(h_local).normalize()
}
