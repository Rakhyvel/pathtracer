use apricot::{
    app::{App, Scene},
    camera::{Camera, ProjectionKind},
    opengl::{Buffer, Texture, Vao, create_program},
    ray::Ray,
    rectangle::Rectangle,
    render_core::TextureId,
};

use crate::{
    emissive::Emissive,
    hit_info::HitInfo,
    lambertian::Lambertian,
    material_mgr::{MaterialId, MaterialMgr},
    object::Object,
    plane::MaterialPlane,
    sphere::MaterialSphere,
};

pub struct Tracer {
    n: usize,
    width: usize,
    height: usize,
    tile_size: usize,
    camera: Camera,
    texture_id: TextureId,
    image: Vec<nalgebra_glm::Vec3>,
    pixels: Vec<u8>,
    material_mgr: MaterialMgr,
    objects: Vec<Box<dyn Object>>,
}

pub const QUAD_XY_DATA: &[u8] = include_bytes!("../res/quad-xy.obj");

impl Scene for Tracer {
    fn update(&mut self, app: &App) {
        // Handle mouse + keyboard
    }

    fn render(&mut self, app: &App) {
        self.n += 1;
        for y in 0..self.height {
            for x in 0..self.width {
                let i = y * self.width + x;
                let ray =
                    self.camera
                        .get_ray(x as f32, y as f32, self.width as f32, self.height as f32);
                let curr_pixel = self.trace(&ray, 0);
                let prev_pixel = self.image[i];
                self.image[i] += (curr_pixel - prev_pixel) / (self.n as f32);

                self.pixels[i * 4 + 0] = (self.image[i].x * 255.0) as u8;
                self.pixels[i * 4 + 1] = (self.image[i].y * 255.0) as u8;
                self.pixels[i * 4 + 2] = (self.image[i].z * 255.0) as u8;
                self.pixels[i * 4 + 3] = 255;
            }
        }

        let texture = app.renderer.get_texture_from_id(self.texture_id).unwrap();
        texture.set_pixels(self.width as i32, self.height as i32, &self.pixels);
        let screen_rect = Rectangle::new(
            0.0,
            0.0,
            self.width as f32 * self.tile_size as f32,
            self.height as f32 * self.tile_size as f32,
        );
        let tex_rect = Rectangle::new(0.0, 0.0, self.width as f32, self.height as f32);
        app.renderer
            .copy_texture(screen_rect, self.texture_id, tex_rect);
    }
}

impl Tracer {
    pub fn new(app: &App) -> Self {
        const TILE_SIZE: i32 = 10;

        let width = (app.window_size.x / TILE_SIZE) as usize;
        let height = (app.window_size.y / TILE_SIZE) as usize;

        app.renderer.add_program(
            create_program(
                include_str!("../shaders/2d.vert"),
                include_str!("../shaders/2d.frag"),
            )
            .unwrap(),
            Some("2d"),
        );
        app.renderer
            .add_mesh_from_obj(QUAD_XY_DATA, Some("quad-xy"));

        let position = nalgebra_glm::vec3(0.0, 0.5, -5.0);
        let lookat = nalgebra_glm::vec3(0.0, 0.5, -4.0);
        let up = nalgebra_glm::vec3(0.0, 1.0, 0.0);

        let camera = Camera::new(
            position,
            lookat,
            up,
            ProjectionKind::Perspective {
                fov: 71.0,
                far: 1000.0,
            },
        );

        let texture = Texture::new();
        let pixels: Vec<u8> = vec![0u8; width * height * 4];
        texture.set_pixels(width as i32, height as i32, &pixels);
        let texture_id = app.renderer.add_texture(texture, Some("texture"));

        let image: Vec<nalgebra_glm::Vec3> =
            vec![nalgebra_glm::vec3(0.0, 0.0, 0.0); width * height * 4];

        // Setup materials
        let mut material_mgr = MaterialMgr::new();
        let emissive = material_mgr.add(
            Box::new(Emissive {
                color: nalgebra_glm::vec3(1.0, 1.0, 1.0),
            }),
            Some("emissive"),
        );
        let lambert = material_mgr.add(
            Box::new(Lambertian {
                albedo: nalgebra_glm::vec3(1.0, 0.5, 0.0),
            }),
            Some("lambertOrange"),
        );

        // Setup objects
        let objects: Vec<Box<dyn Object>> = vec![
            Box::new(MaterialSphere::new(
                nalgebra_glm::vec3(0.0, 0.0, 0.0),
                1.0,
                emissive,
            )),
            Box::new(MaterialPlane::new(
                nalgebra_glm::vec3(0.0, 1.0, 0.0),
                0.0,
                lambert,
            )),
        ];

        Tracer {
            n: 0,
            width,
            height,
            tile_size: TILE_SIZE as usize,
            camera,
            texture_id,
            image,
            pixels,
            material_mgr,
            objects,
        }
    }

    fn trace(&self, ray: &Ray, depth: i32) -> nalgebra_glm::Vec3 {
        if depth > 100 {
            return nalgebra_glm::zero();
        }

        let hit = self.cast_ray(ray);

        if hit.is_none() {
            return Self::sky(ray) * 0.1;
        }
        let hit = hit.unwrap();

        let hit_material = self.material_mgr.get_from_id(hit.material).unwrap();

        let emitted = hit_material.emission();

        match hit_material.scatter(ray, &hit) {
            None => emitted,
            Some(scatter) => {
                let incoming = self.trace(&scatter.ray, depth + 1);
                emitted + scatter.attenuation.component_mul(&incoming)
            }
        }
    }

    fn cast_ray(&self, ray: &Ray) -> Option<HitInfo> {
        let mut closest_hit: Option<HitInfo> = None;

        for obj in &self.objects {
            if let Some(hit) = obj.intersect(ray) {
                match &closest_hit {
                    Some(existing_hit) => {
                        if hit.depth < existing_hit.depth {
                            closest_hit = Some(hit)
                        }
                    }
                    None => closest_hit = Some(hit),
                }
            }
        }

        closest_hit
    }

    fn sky(_ray: &Ray) -> nalgebra_glm::Vec3 {
        nalgebra_glm::vec3(0.2, 0.5, 0.8)
    }
}
