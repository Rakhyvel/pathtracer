use apricot::{
    app::{App, Scene},
    camera::{Camera, ProjectionKind},
    opengl::{Texture, create_program},
    ray::Ray,
    rectangle::Rectangle,
    render_core::{Mesh, TextureId},
};
use obj::raw::object;
use rand::Rng;
use rayon::prelude::*;
use sdl2::keyboard::Scancode;
use std::f32::consts::PI;

use crate::{
    dielectric::Dielectric,
    emissive::Emissive,
    glossy::Glossy,
    hit_info::HitInfo,
    lambertian::Lambertian,
    material_mgr::{self, MaterialMgr},
    mesh::MaterialMesh,
    metallic::Metallic,
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
    samples_per_pixel: Vec<usize>,
    pixels: Vec<u8>,
    material_mgr: MaterialMgr,
    objects: Vec<Box<dyn Object>>,

    // View stuff
    facing: f32,
    pitch: f32,
}

pub const QUAD_XY_DATA: &[u8] = include_bytes!("../res/quad-xy.obj");
pub const ICO_DATA: &[u8] = include_bytes!("../res/ico-sphere.obj");
pub const CUBE_DATA: &[u8] = include_bytes!("../res/cube.obj");

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

        if false && (player_vel_vec.norm() > 0.0 || app.mouse_vel.norm() > 0.0) {
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
            self.samples_per_pixel.iter_mut().for_each(|s| *s /= 2);
        }
    }

    fn render(&mut self, app: &App) {
        const SAMPLES_PER_FRAME: usize = 50;
        let width = self.width;
        let height = self.height;
        let objects = &self.objects;
        let material_mgr = &self.material_mgr;

        self.image
            .par_iter_mut()
            .zip(self.pixels.par_chunks_mut(4))
            .zip(self.samples_per_pixel.par_iter_mut())
            .enumerate()
            .for_each(|(i, ((image_pixel, pixel_out), sample_count))| {
                // recover which sample number this pixel is on from the frame counter
                let mut rng = rand::thread_rng();
                let x = i % width;
                let y = i / width;

                for _ in 0..SAMPLES_PER_FRAME {
                    *sample_count += 1;
                    let jitter_x = x as f32 + rng.gen_range(-0.5..0.5);
                    let jitter_y = y as f32 + rng.gen_range(-0.5..0.5);
                    let ray = self
                        .camera
                        .get_ray(jitter_x, jitter_y, width as f32, height as f32);

                    let curr_pixel = trace(
                        &ray,
                        0,
                        &mut nalgebra_glm::Vec3::new(10.0, 1.0, 1.0),
                        objects,
                        material_mgr,
                    );
                    *image_pixel += (curr_pixel - *image_pixel) / *sample_count as f32;
                }

                let mut hdr = *image_pixel;
                let denom = hdr + nalgebra_glm::vec3(1.0, 1.0, 1.0);
                hdr.x /= denom.x;
                hdr.y /= denom.y;
                hdr.z /= denom.z;

                pixel_out[0] = (hdr.x * 255.0) as u8;
                pixel_out[1] = (hdr.y * 255.0) as u8;
                pixel_out[2] = (hdr.z * 255.0) as u8;
                pixel_out[3] = 255;
            });

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
        const TILE_SIZE: i32 = 5;

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

        let position = nalgebra_glm::vec3(-19.0, 0.0, 0.0);
        let lookat = nalgebra_glm::vec3(0.0, 0.0, 0.0);
        let up = nalgebra_glm::vec3(0.0, 1.0, 0.0);

        let camera = Camera::new(
            position,
            lookat,
            up,
            ProjectionKind::Perspective {
                fov: 40.0,
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
                color: nalgebra_glm::vec3(1.0, 0.8, 0.5) * 200.0,
            }),
            Some("emissive"),
        );
        let lambert_white = material_mgr.add(
            Box::new(Lambertian {
                albedo: nalgebra_glm::vec3(0.9, 0.9, 0.9),
            }),
            Some("lambert_white"),
        );
        let lambert_blue = material_mgr.add(
            Box::new(Lambertian {
                albedo: nalgebra_glm::vec3(0.0, 0.3, 0.7),
            }),
            Some("lambert_blue"),
        );
        let lambert_red = material_mgr.add(
            Box::new(Lambertian {
                albedo: nalgebra_glm::vec3(0.9, 0.0, 0.0),
            }),
            Some("lambert_red"),
        );
        let lambert_green = material_mgr.add(
            Box::new(Lambertian {
                albedo: nalgebra_glm::vec3(0.0, 0.7, 0.0),
            }),
            Some("lambert_green"),
        );
        let dielectric_blue = material_mgr.add(
            Box::new(Dielectric {
                ior: 1.5,
                tint: nalgebra_glm::vec3(0.95, 0.98, 1.0),
            }),
            Some("dielectric_blue"),
        );
        let dielectric_green = material_mgr.add(
            Box::new(Dielectric {
                ior: 3.01,
                tint: nalgebra_glm::vec3(0.7, 0.95, 0.7),
            }),
            Some("dielectric_green"),
        );
        let metallic_red = material_mgr.add(
            Box::new(Metallic {
                roughness: 0.1,
                albedo: nalgebra_glm::vec3(0.9, 0.7, 0.4),
            }),
            Some("metallic_red"),
        );
        let glossy = material_mgr.add(
            Box::new(Glossy {
                roughness: 0.03,
                albedo: nalgebra_glm::vec3(0.95, 0.93, 0.88),
            }),
            Some("glossy"),
        );

        let radius = 1.0;
        let bounds = 5.0;

        // Setup objects
        let objects: Vec<Box<dyn Object>> = vec![
            Box::new(MaterialMesh::new(
                QUAD_XY_DATA,
                emissive,
                nalgebra_glm::translation(&nalgebra_glm::vec3(0.0, 4.9, 0.0))
                    * nalgebra_glm::rotation(
                        std::f32::consts::FRAC_PI_2,
                        &nalgebra_glm::vec3(1.0, 0.0, 0.0),
                    )
                    * nalgebra_glm::scaling(&nalgebra_glm::vec3(0.75, 0.75, 1.0)),
            )),
            // room
            Box::new(MaterialMesh::new(
                QUAD_XY_DATA, // back
                lambert_white,
                nalgebra_glm::translation(&nalgebra_glm::vec3(5.0, 0.0, 0.0))
                    * nalgebra_glm::rotation(
                        std::f32::consts::FRAC_PI_2,
                        &nalgebra_glm::vec3(0.0, 1.0, 0.0),
                    )
                    * nalgebra_glm::scaling(&nalgebra_glm::vec3(5.0, 5.0, 1.0)),
            )),
            Box::new(MaterialMesh::new(
                QUAD_XY_DATA, // bottom
                lambert_white,
                nalgebra_glm::translation(&nalgebra_glm::vec3(0.0, -5.0, 0.0))
                    * nalgebra_glm::rotation(
                        std::f32::consts::FRAC_PI_2,
                        &nalgebra_glm::vec3(1.0, 0.0, 0.0),
                    )
                    * nalgebra_glm::scaling(&nalgebra_glm::vec3(9.0, 5.0, 1.0)),
            )),
            Box::new(MaterialMesh::new(
                QUAD_XY_DATA, // top
                lambert_white,
                nalgebra_glm::translation(&nalgebra_glm::vec3(0.0, 5.0, 0.0))
                    * nalgebra_glm::rotation(
                        std::f32::consts::FRAC_PI_2,
                        &nalgebra_glm::vec3(1.0, 0.0, 0.0),
                    )
                    * nalgebra_glm::scaling(&nalgebra_glm::vec3(9.0, 5.0, 1.0)),
            )),
            Box::new(MaterialMesh::new(
                QUAD_XY_DATA, // left
                lambert_red,
                nalgebra_glm::translation(&nalgebra_glm::vec3(0.0, 0.0, -5.0))
                    * nalgebra_glm::rotation(
                        std::f32::consts::FRAC_PI_2,
                        &nalgebra_glm::vec3(0.0, 0.0, 1.0),
                    )
                    * nalgebra_glm::scaling(&nalgebra_glm::vec3(5.0, 9.0, 1.0)),
            )),
            Box::new(MaterialMesh::new(
                QUAD_XY_DATA, // right
                lambert_blue,
                nalgebra_glm::translation(&nalgebra_glm::vec3(0.0, 0.0, 5.0))
                    * nalgebra_glm::rotation(
                        std::f32::consts::FRAC_PI_2,
                        &nalgebra_glm::vec3(0.0, 0.0, 1.0),
                    )
                    * nalgebra_glm::scaling(&nalgebra_glm::vec3(5.0, 9.0, 1.0)),
            )),
            // objects
            // Box::new(MaterialMesh::new(
            //     ICO_DATA,
            //     metallic_red,
            //     nalgebra_glm::translation(&nalgebra_glm::vec3(-2.4, 1.0 - bounds, 0.0))
            //         * nalgebra_glm::rotation(
            //             std::f32::consts::FRAC_PI_4,
            //             &nalgebra_glm::vec3(0.0, 1.0, 0.0),
            //         )
            //         * nalgebra_glm::scaling(&nalgebra_glm::vec3(1.0, 1.0, 1.0)),
            // )),
            // Box::new(MaterialMesh::new(
            //     CUBE_DATA,
            //     glossy,
            //     nalgebra_glm::translation(&nalgebra_glm::vec3(2.0, 3.5 - bounds, 2.0))
            //         * nalgebra_glm::rotation(
            //             std::f32::consts::FRAC_PI_4,
            //             &nalgebra_glm::vec3(0.0, 1.0, 0.0),
            //         )
            //         * nalgebra_glm::scaling(&nalgebra_glm::vec3(2.0, 3.5, 2.0)),
            // )),
            // Box::new(MaterialSphere::new(
            //     nalgebra_glm::vec3(0.0, -1.0, -2.5),
            //     2.0,
            //     dielectric_blue,
            // )),
            Box::new(MaterialSphere::new(
                nalgebra_glm::vec3(0.0, -2.5, 0.0),
                2.5,
                glossy,
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
            samples_per_pixel: vec![0; width * height],
            pixels,
            material_mgr,
            objects,
            facing: 0.0,
            pitch: 0.0,
        }
    }
}

fn trace(
    ray: &Ray,
    depth: i32,
    throughput: &mut nalgebra_glm::Vec3,
    objects: &[Box<dyn Object>],
    material_mgr: &MaterialMgr,
) -> nalgebra_glm::Vec3 {
    if depth > 100 {
        return nalgebra_glm::Vec3::zeros();
    }
    if depth > 5 {
        let luminance = 0.2126 * throughput.x + 0.7152 * throughput.y + 0.0722 * throughput.z;
        let survive_prob = luminance.clamp(0.05, 1.0);
        let mut rng = rand::thread_rng();
        if rng.r#gen::<f32>() > survive_prob {
            return nalgebra_glm::Vec3::zeros();
        }
        *throughput /= survive_prob;
    }

    let hit = cast_ray(ray, objects);

    if hit.is_none() {
        return sky(ray) * 0.0;
    }
    let hit = hit.unwrap();

    let hit_material = material_mgr.get_from_id(hit.material).unwrap();

    let emitted = hit_material.emission();

    match hit_material.scatter(ray, &hit) {
        None => emitted,
        Some(scatter) => {
            let mut next_throughput: nalgebra_glm::Vec3 =
                throughput.component_mul(&scatter.attenuation);
            let incoming = trace(
                &scatter.ray,
                depth + 1,
                &mut next_throughput,
                objects,
                material_mgr,
            );
            emitted + scatter.attenuation.component_mul(&incoming)
        }
    }
}

fn cast_ray(ray: &Ray, objects: &[Box<dyn Object>]) -> Option<HitInfo> {
    let mut closest_hit: Option<HitInfo> = None;

    for obj in objects {
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
    let d = ray.dir().normalize();
    let t = 0.5 * (d.y + 1.0);
    let color =
        (1.0 - t) * nalgebra_glm::vec3(1.0, 1.0, 1.0) + t * nalgebra_glm::vec3(0.5, 0.7, 1.0);
    color
}
