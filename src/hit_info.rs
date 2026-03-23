use crate::material_mgr::MaterialId;

#[derive(Clone)]
pub struct HitInfo {
    pub point: nalgebra_glm::Vec3,
    pub normal: nalgebra_glm::Vec3,
    pub depth: f32,
    pub material: MaterialId,
}
