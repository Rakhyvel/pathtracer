use apricot::{
    app::{App, Scene},
    camera::{Camera, ProjectionKind},
    opengl::{Texture, create_program},
    ray::Ray,
    rectangle::Rectangle,
    render_core::TextureId,
};
use sdl2::keyboard::Scancode;
use std::f32::consts::PI;

use crate::{
    dielectric::Dielectric, emissive::Emissive, hit_info::HitInfo, lambertian::Lambertian,
    material_mgr::MaterialMgr, object::Object, plane::MaterialPlane, sphere::MaterialSphere,
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

    // View stuff
    facing: f32,
    pitch: f32,
}

pub const QUAD_XY_DATA: &[u8] = include_bytes!("../res/quad-xy.obj");

impl Scene for Tracer {
    fn update(&mut self, app: &App) {
        let curr_w_state = app.keys[Scancode::W as usize];
        let curr_s_state = app.keys[Scancode::S as usize];
        let curr_a_state = app.keys[Scancode::A as usize];
        let curr_d_state = app.keys[Scancode::D as usize];
        let curr_space_state = app.keys[Scancode::Space as usize];
        let curr_shift_state = app.keys[Scancode::LShift as usize];

        let up = self.camera.up();
        let facing_vec = nalgebra_glm::vec3(self.facing.cos(), 0.0, self.facing.sin());
        let sideways_vec = nalgebra_glm::cross(&up, &facing_vec);
        let mut player_vel_vec: nalgebra_glm::Vec3 = nalgebra_glm::zero();
        if curr_w_state {
            player_vel_vec += facing_vec;
        }
        if curr_s_state {
            player_vel_vec += -facing_vec;
        }
        if curr_a_state {
            player_vel_vec += sideways_vec;
        }
        if curr_d_state {
            player_vel_vec += -sideways_vec;
        }
        if curr_space_state {
            player_vel_vec += up;
        }
        if curr_shift_state {
            player_vel_vec += -up;
        }

        if player_vel_vec.norm() > 0.0 || app.mouse_vel.norm() > 0.0 {
            const VIEW_SPEED: f32 = 0.01;
            const WALK_SPEED: f32 = 0.04;
            self.facing += VIEW_SPEED * app.mouse_vel.x;
            self.pitch = (self.pitch + VIEW_SPEED * app.mouse_vel.y)
                .max(VIEW_SPEED - PI / 2.0)
                .min(PI / 2.0 - VIEW_SPEED);

            let prev_cam_pos = self.camera.position();
            let new_cam_pos = prev_cam_pos + player_vel_vec * WALK_SPEED;
            self.camera.set_position(new_cam_pos);

            let rot_matrix = nalgebra_glm::rotate_y(
                &nalgebra_glm::rotate_z(&nalgebra_glm::one(), self.facing),
                self.pitch,
            );
            let facing_vec = (rot_matrix * nalgebra_glm::vec4(1.0, 0.0, 0.0, 0.0)).xzy();
            self.camera.set_lookat(new_cam_pos + facing_vec);
            self.n /= 2;
        }
    }

    fn render(&mut self, app: &App) {
        for _n in 0..10 {
            self.n += 1;
            for y in 0..self.height {
                for x in 0..self.width {
                    let i = y * self.width + x;
                    let ray = self.camera.get_ray(
                        x as f32,
                        y as f32,
                        self.width as f32,
                        self.height as f32,
                    );
                    let curr_pixel = self.trace(&ray, 0);
                    let prev_pixel = self.image[i];
                    self.image[i] += (curr_pixel - prev_pixel) / (self.n as f32);

                    let mut hdr = self.image[i];
                    let denom = hdr + nalgebra_glm::vec3(1.0, 1.0, 1.0);
                    hdr.x /= denom.x;
                    hdr.y /= denom.y;
                    hdr.z /= denom.z;

                    self.pixels[i * 4 + 0] = (hdr.x * 255.0) as u8;
                    self.pixels[i * 4 + 1] = (hdr.y * 255.0) as u8;
                    self.pixels[i * 4 + 2] = (hdr.z * 255.0) as u8;
                    self.pixels[i * 4 + 3] = 255;
                }
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

        let position = nalgebra_glm::vec3(0.0, 0.0, 0.0);
        let lookat = nalgebra_glm::vec3(-1.0, 0.0, 0.0);
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
                color: nalgebra_glm::vec3(1.0, 1.0, 1.0) * 10.0,
            }),
            Some("emissive"),
        );
        let lambert_white = material_mgr.add(
            Box::new(Lambertian {
                albedo: nalgebra_glm::vec3(1.0, 1.0, 1.0),
            }),
            Some("lambert_white"),
        );
        let lambert_blue = material_mgr.add(
            Box::new(Lambertian {
                albedo: nalgebra_glm::vec3(0.0, 0.5, 1.0),
            }),
            Some("lambert_blue"),
        );
        let dielectric_blue = material_mgr.add(
            Box::new(Dielectric {
                ior: 1.52,
                tint: nalgebra_glm::vec3(0.9, 0.9, 0.95),
            }),
            Some("dielectric_blue"),
        );
        let dielectric_green = material_mgr.add(
            Box::new(Dielectric {
                ior: 1.52,
                tint: nalgebra_glm::vec3(0.9, 0.95, 0.9),
            }),
            Some("dielectric_green"),
        );
        let dielectric_red = material_mgr.add(
            Box::new(Dielectric {
                ior: 1.52,
                tint: nalgebra_glm::vec3(0.95, 0.9, 0.9),
            }),
            Some("dielectric_red"),
        );

        // Setup objects
        let objects: Vec<Box<dyn Object>> = vec![
            Box::new(MaterialSphere::new(
                nalgebra_glm::vec3(40.0, 40.0, 40.0),
                10.0,
                emissive,
            )),
            Box::new(MaterialSphere::new(
                nalgebra_glm::vec3(-2.0, 0.0, -0.0),
                1.0,
                dielectric_red,
            )),
            Box::new(MaterialSphere::new(
                nalgebra_glm::vec3(-0.0, 0.0, 0.0),
                1.0,
                dielectric_green,
            )),
            Box::new(MaterialSphere::new(
                nalgebra_glm::vec3(2.0, 0.0, 0.0),
                1.0,
                dielectric_blue,
            )),
            Box::new(MaterialPlane::new(
                nalgebra_glm::vec3(0.0, 1.0, 0.0),
                1.0,
                lambert_white,
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
            facing: 3.14,
            pitch: 0.0,
        }
    }

    fn trace(&self, ray: &Ray, depth: i32) -> nalgebra_glm::Vec3 {
        if depth > 100 {
            return nalgebra_glm::zero();
        }

        let hit = self.cast_ray(ray);

        if hit.is_none() {
            return Self::sky(ray);
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

    fn sky(ray: &Ray) -> nalgebra_glm::Vec3 {
        let d = ray.dir.normalize();
        let t = 0.5 * (d.y + 1.0);
        let color =
            (1.0 - t) * nalgebra_glm::vec3(1.0, 1.0, 1.0) + t * nalgebra_glm::vec3(0.5, 0.7, 1.0);
        color
    }
}
