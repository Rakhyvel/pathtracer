use nalgebra_glm::{Vec3, vec3};

pub struct ONB {
    pub u: Vec3,
    pub v: Vec3,
    pub w: Vec3,
}

impl ONB {
    pub fn from_w(w: Vec3) -> Self {
        let sign = if w.z >= 0.0 { 1.0 } else { -1.0 };
        let a = -1.0 / (sign + w.z);
        let b = w.x * w.y * a;

        let u = vec3(1.0 + sign * w.x * w.x * a, sign * b, -sign * w.x);

        let v = vec3(b, sign + w.y * w.y * a, -w.y);

        Self { u, v, w }
    }

    /// Returns a normal vector (if `local` was normal)
    pub fn to_world(&self, local: Vec3) -> Vec3 {
        self.u * local.x + self.v * local.y + self.w * local.z
    }
}

mod tests {
    #[allow(unused)]
    use super::*;

    #[test]
    fn is_normalized() {
        let onb = ONB::from_w(vec3(1.0, 0.0, 0.0));
        let dir = onb.to_world(vec3(0.0, 1.0, 0.0));

        assert!((dir.norm() - 1.0).abs() < 0.01);
    }
}
