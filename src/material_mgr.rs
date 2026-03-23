use std::collections::HashMap;

use apricot::render_core::OpaqueId;

use crate::material::Material;

/// Opaque type used by the texture manager to associate textures.
#[derive(Copy, Clone, Debug)]
pub struct MaterialId(usize);

impl OpaqueId for MaterialId {
    fn new(id: usize) -> Self {
        MaterialId(id)
    }

    fn as_usize(&self) -> usize {
        self.0
    }
}

pub struct MaterialMgr {
    materials: Vec<Box<dyn Material>>,
    keys: HashMap<&'static str, MaterialId>,
}

impl MaterialMgr {
    pub fn new() -> Self {
        Self {
            materials: Vec::new(),
            keys: HashMap::new(),
        }
    }

    pub fn add(&mut self, mat: Box<dyn Material>, name: Option<&'static str>) -> MaterialId {
        let id = MaterialId::new(self.materials.len());
        self.materials.push(mat);
        if name.is_some() {
            self.keys.insert(name.unwrap(), id);
        }
        id
    }

    pub fn get_from_id(&self, id: MaterialId) -> Option<&dyn Material> {
        self.materials.get(id.as_usize()).map(|b| b.as_ref())
    }

    pub fn get_id_from_name(&self, name: &str) -> Option<MaterialId> {
        self.keys.get(name).copied()
    }
}
