use apricot::{aabb::AABB, bvh::BVH, ray::Ray, tri::Tri};
use obj::{Obj, TexturedVertex, load_obj};

use crate::{hit_info::HitInfo, material_mgr::MaterialId, object::Object};

pub struct MaterialMesh {
    triangles: TriangleSoup,
    mat_id: MaterialId,
    model_matrix: nalgebra_glm::Mat4,
    // Cached at construction, never recomputed
    inv_model_matrix: nalgebra_glm::Mat4,
    normal_matrix: nalgebra_glm::Mat4,
}

struct TriangleSoup {
    pub bvh: BVH<Tri>,
}

const EPS: f32 = 1e-4;

impl MaterialMesh {
    pub fn new(obj_file_data: &[u8], mat_id: MaterialId, model_matrix: nalgebra_glm::Mat4) -> Self {
        let inv_model_matrix = nalgebra_glm::inverse(&model_matrix);
        let normal_matrix = nalgebra_glm::inverse_transpose(model_matrix);
        Self {
            triangles: TriangleSoup::from_obj(obj_file_data),
            mat_id,
            model_matrix,
            inv_model_matrix,
            normal_matrix,
        }
    }

    fn intersect_triangle(&self, ray: &Ray, tri: &Tri) -> Option<HitInfo> {
        let e1 = tri.v1() - tri.v0();
        let e2 = tri.v2() - tri.v0();

        let h = ray.dir().cross(&e2);
        let det = e1.dot(&h);

        // Ray parallel to triangle
        if det.abs() < EPS {
            return None;
        }

        let inv_det = 1.0 / det;
        let s = ray.origin() - tri.v0();
        let u = inv_det * s.dot(&h);
        // didn't know you could do this!
        if !(0.0..=1.0).contains(&u) {
            return None;
        }

        let q = s.cross(&e1);
        let v = inv_det * ray.dir().dot(&q);
        if v < 0.0 || u + v > 1.0 {
            return None;
        }

        let t = inv_det * e2.dot(&q);
        if t < EPS {
            return None;
        }

        let point = ray.at(t);
        let outward = e1.cross(&e2).normalize();
        let front_face = ray.dir().dot(&outward) < 0.0;
        let normal = if front_face { outward } else { -outward };

        Some(HitInfo {
            point,
            normal,
            depth: t,
            material: self.mat_id,
        })
    }
}

impl Object for MaterialMesh {
    fn intersect(&self, ray: &Ray) -> Option<HitInfo> {
        let local_ray = Ray::new(
            (self.inv_model_matrix * ray.origin().push(1.0)).xyz(),
            (self.inv_model_matrix * ray.dir().push(0.0)).xyz(),
        );

        self.triangles
            .bvh
            .iter_ray(&local_ray)
            .filter_map(|tri| self.intersect_triangle(&local_ray, &tri))
            .min_by(|a, b| a.depth.partial_cmp(&b.depth).unwrap())
            .map(|hit| HitInfo {
                point: (self.model_matrix * hit.point.push(1.0)).xyz(),
                normal: (self.normal_matrix * hit.normal.push(0.0))
                    .xyz()
                    .normalize(),
                depth: hit.depth,
                material: hit.material,
            })
    }
}

impl TriangleSoup {
    pub fn from_obj(obj_file_data: &[u8]) -> Self {
        let obj: Obj<TexturedVertex> = load_obj(obj_file_data).unwrap();
        let verts = flatten_positions(&obj.vertices);
        let indices = vec_u32_from_vec_u16(&obj.indices);

        let mut bvh = BVH::new();

        for tri_indices in indices.chunks(3) {
            let (i0, i1, i2) = (
                tri_indices[0] as usize,
                tri_indices[1] as usize,
                tri_indices[2] as usize,
            );
            let tri = Tri::new(
                nalgebra_glm::vec3(verts[i0 * 3], verts[i0 * 3 + 1], verts[i0 * 3 + 2]),
                nalgebra_glm::vec3(verts[i1 * 3], verts[i1 * 3 + 1], verts[i1 * 3 + 2]),
                nalgebra_glm::vec3(verts[i2 * 3], verts[i2 * 3 + 1], verts[i2 * 3 + 2]),
            );
            let aabb = AABB::from_points(vec![
                (tri.v0().push(1.0)).xyz(),
                (tri.v1().push(1.0)).xyz(),
                (tri.v2().push(1.0)).xyz(),
            ]);
            bvh.insert(tri, aabb);
        }

        Self { bvh }
    }
}

fn flatten_positions(vertices: &Vec<TexturedVertex>) -> Vec<f32> {
    let mut retval = vec![];
    for vertex in vertices {
        retval.push(vertex.position[0]);
        retval.push(vertex.position[1]);
        retval.push(vertex.position[2]);
    }
    retval
}

fn vec_u32_from_vec_u16(input: &Vec<u16>) -> Vec<u32> {
    let mut retval = vec![];
    for x in input {
        retval.push(*x as u32);
    }
    retval
}
