use apricot::{
    app::{App, Scene},
    camera::{Camera, ProjectionKind},
    opengl::{Texture, create_program},
    ray::Ray,
    rectangle::Rectangle,
    render_core::TextureId,
};
use rand::Rng;
use rayon::prelude::*;
use sdl2::keyboard::Scancode;
use std::{
    f32::consts::PI,
    sync::atomic::{AtomicU32, Ordering},
};

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
    width: usize,
    height: usize,
    tile_size: usize,
    camera: Camera,
    texture_id: TextureId,
    image: Vec<nalgebra_glm::Vec3>,    // image mean accumulator
    image_sq: Vec<nalgebra_glm::Vec3>, // sum of mean accumulator
    converged: Vec<bool>,
    samples_per_pixel: Vec<usize>,
    pixels: Vec<u8>,
    material_mgr: MaterialMgr,
    objects: Vec<Box<dyn Object>>,
}

pub const QUAD_XY_DATA: &[u8] = include_bytes!("../res/quad-xy.obj");
pub const ICO_DATA: &[u8] = include_bytes!("../res/ico-sphere.obj");
pub const CUBE_DATA: &[u8] = include_bytes!("../res/cube.obj");

impl Scene for Tracer {
    fn update(&mut self, app: &App) {
        if app.ticks % (60 * 5) == 0 {
            let total_pixels = self.width * self.height;
            let converged = self.converged.par_iter().filter(|&&c| c).count();
            if converged == total_pixels {
                println!("Image converged!");
            } else {
                println!(
                    "{:.1}% converged",
                    100.0 * converged as f32 / total_pixels as f32
                );
                let total = (self.width * self.height) as f32;
                for t in [0.001, 0.003, 0.01, 0.03, 0.1, 0.3] {
                    let count = self
                        .image
                        .iter()
                        .zip(self.image_sq.iter())
                        .zip(self.samples_per_pixel.iter())
                        .filter(|((mean, m2), n)| confidence_interval(**mean, **m2, **n as f32) < t)
                        .count();
                    println!(
                        "converged at {:.3}: {:.3}%",
                        t,
                        count as f32 / total * 100.0
                    );
                }
            }
        }
    }

    fn render(&mut self, app: &App) {
        const SAMPLES_PER_FRAME: usize = 60;
        const MIN_SAMPLES_BEFORE_SKIP: usize = 256;
        const VARIANCE_TOLERANCE: f32 = 0.01;

        let width = self.width;
        let height = self.height;
        let objects = &self.objects;
        let material_mgr = &self.material_mgr;

        self.image
            .par_iter_mut()
            .zip(self.pixels.par_chunks_mut(4))
            .zip(self.samples_per_pixel.par_iter_mut())
            .zip(self.image_sq.par_iter_mut())
            .zip(self.converged.par_iter_mut())
            .enumerate()
            .for_each(
                |(
                    i,
                    ((((image_pixel, pixel_out), sample_count), image_sq_pixel), pixel_converged),
                )| {
                    if *pixel_converged {
                        return;
                    }

                    // recover which sample number this pixel is on from the frame counter
                    let mut rng = rand::thread_rng();
                    let x = i % width;
                    let y = i / width;

                    for _ in 0..SAMPLES_PER_FRAME {
                        // Do at least a few samples, variance and skip if converged
                        if *sample_count > MIN_SAMPLES_BEFORE_SKIP {
                            let interval = confidence_interval(
                                *image_pixel,
                                *image_sq_pixel,
                                *sample_count as f32,
                            );
                            if interval < VARIANCE_TOLERANCE {
                                *pixel_converged = true;
                                break;
                            }
                        }

                        let jitter_x = x as f32 + rng.gen_range(-0.5..0.5);
                        let jitter_y = y as f32 + rng.gen_range(-0.5..0.5);
                        let ray =
                            self.camera
                                .get_ray(jitter_x, jitter_y, width as f32, height as f32);

                        let sample = trace(
                            &ray,
                            0,
                            &mut nalgebra_glm::Vec3::new(1.0, 1.0, 1.0),
                            objects,
                            material_mgr,
                        );

                        *sample_count += 1;
                        let n = *sample_count as f32;

                        // Raw mean for display
                        let delta = sample - *image_pixel;
                        *image_pixel += delta / n;

                        // Mean of squares for variance
                        *image_sq_pixel += delta.component_mul(&(sample - *image_pixel));
                    }

                    let pixel_hdr = tonemap(*image_pixel);
                    pixel_out[0] = (pixel_hdr.x * 255.0) as u8;
                    pixel_out[1] = (pixel_hdr.y * 255.0) as u8;
                    pixel_out[2] = (pixel_hdr.z * 255.0) as u8;
                    pixel_out[3] = 255;
                },
            );

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

fn confidence_interval(mean: nalgebra_glm::Vec3, m2: nalgebra_glm::Vec3, n: f32) -> f32 {
    let variance = m2 / (n - 1.0).max(1.0);
    let std_dev = nalgebra_glm::vec3(variance.x.sqrt(), variance.y.sqrt(), variance.z.sqrt());
    luminance(std_dev) * (1.96 / n.sqrt()) / (luminance(mean).abs().sqrt() + 1e-6)
}

fn luminance(v: nalgebra_glm::Vec3) -> f32 {
    0.2126 * v.x + 0.7152 * v.y + 0.0722 * v.z
}

impl Tracer {
    pub fn new(app: &App) -> Self {
        const TILE_SIZE: i32 = 20;

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

        // Setup materials
        let mut material_mgr = MaterialMgr::new();
        let emissive = material_mgr.add(
            Box::new(Emissive {
                color: nalgebra_glm::vec3(1.0, 0.8, 0.5) * 100.0,
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

        // Setup objects
        let objects: Vec<Box<dyn Object>> = vec![
            Box::new(MaterialMesh::new(
                QUAD_XY_DATA,
                emissive,
                nalgebra_glm::translation(&nalgebra_glm::vec3(0.0, 5.001, 0.0))
                    * nalgebra_glm::rotation(
                        std::f32::consts::FRAC_PI_2,
                        &nalgebra_glm::vec3(1.0, 0.0, 0.0),
                    )
                    * nalgebra_glm::scaling(&nalgebra_glm::vec3(1.0, 1.0, 1.0)),
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
                QUAD_XY_DATA, // top back
                lambert_white,
                nalgebra_glm::translation(&nalgebra_glm::vec3(5.75, 5.0, 0.0))
                    * nalgebra_glm::rotation(
                        std::f32::consts::FRAC_PI_2,
                        &nalgebra_glm::vec3(1.0, 0.0, 0.0),
                    )
                    * nalgebra_glm::scaling(&nalgebra_glm::vec3(5.0, 5.0, 1.0)),
            )),
            Box::new(MaterialMesh::new(
                QUAD_XY_DATA, // top front
                lambert_white,
                nalgebra_glm::translation(&nalgebra_glm::vec3(-5.75, 5.0, 0.0))
                    * nalgebra_glm::rotation(
                        std::f32::consts::FRAC_PI_2,
                        &nalgebra_glm::vec3(1.0, 0.0, 0.0),
                    )
                    * nalgebra_glm::scaling(&nalgebra_glm::vec3(5.0, 5.0, 1.0)),
            )),
            Box::new(MaterialMesh::new(
                QUAD_XY_DATA, // top left
                lambert_white,
                nalgebra_glm::translation(&nalgebra_glm::vec3(0.0, 5.0, 5.75))
                    * nalgebra_glm::rotation(
                        std::f32::consts::FRAC_PI_2,
                        &nalgebra_glm::vec3(1.0, 0.0, 0.0),
                    )
                    * nalgebra_glm::scaling(&nalgebra_glm::vec3(5.0, 5.0, 1.0)),
            )),
            Box::new(MaterialMesh::new(
                QUAD_XY_DATA, // top right
                lambert_white,
                nalgebra_glm::translation(&nalgebra_glm::vec3(0.0, 5.0, -5.75))
                    * nalgebra_glm::rotation(
                        std::f32::consts::FRAC_PI_2,
                        &nalgebra_glm::vec3(1.0, 0.0, 0.0),
                    )
                    * nalgebra_glm::scaling(&nalgebra_glm::vec3(5.0, 5.0, 1.0)),
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
            Box::new(MaterialSphere::new(
                nalgebra_glm::vec3(0.0, -2.5, 0.0),
                2.5,
                glossy,
            )),
        ];

        Tracer {
            width,
            height,
            tile_size: TILE_SIZE as usize,
            camera,
            texture_id,
            image: vec![nalgebra_glm::vec3(1.0, 0.0, 0.0); width * height],
            image_sq: vec![nalgebra_glm::vec3(1.0, 1.0, 1.0); width * height],
            converged: vec![false; width * height],
            samples_per_pixel: vec![0; width * height],
            pixels,
            material_mgr,
            objects,
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
    if depth > 4 {
        return nalgebra_glm::Vec3::zeros();
    }
    if depth > 3 {
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

fn aces(x: f32) -> f32 {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    ((x * (a * x + b)) / (x * (c * x + d) + e)).clamp(0.0, 1.0)
}

fn tonemap(v: nalgebra_glm::Vec3) -> nalgebra_glm::Vec3 {
    nalgebra_glm::vec3(aces(v.x), aces(v.y), aces(v.z))
}
