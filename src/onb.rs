pub struct ONB {
    pub u: nalgebra_glm::Vec3,
    pub v: nalgebra_glm::Vec3,
    pub w: nalgebra_glm::Vec3,
}

impl ONB {
    pub fn from_w(w: nalgebra_glm::Vec3) -> Self {
        let a = if w.x.abs() > 0.9 {
            nalgebra_glm::vec3(0.0, 1.0, 0.0)
        } else {
            nalgebra_glm::vec3(1.0, 0.0, 0.0)
        };

        let v = w.cross(&a).normalize();
        let u = w.cross(&v);

        Self { u, v, w }
    }

    pub fn to_world(&self, local: nalgebra_glm::Vec3) -> nalgebra_glm::Vec3 {
        self.u * local.x + self.v * local.y + self.w * local.z
    }
}
