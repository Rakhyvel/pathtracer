use apricot::{
    app::{App, Scene},
    camera::{Camera, ProjectionKind},
    opengl::{Texture, create_program},
    ray::Ray,
    rectangle::Rectangle,
    render_core::TextureId,
};
use rand::{Rng, SeedableRng, rngs::SmallRng};
use rayon::prelude::*;

use crate::{
    dielectric::Dielectric, emissive::Emissive, glossy::Glossy, hit_info::HitInfo,
    lambertian::Lambertian, material::MaterialEnum, material_mgr::MaterialMgr, metallic::Metallic,
    object::ObjectEnum, plane::MaterialPlane, sphere::MaterialSphere,
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
    objects: Vec<ObjectEnum>,
}

#[allow(unused)]
pub const QUAD_XY_DATA: &[u8] = include_bytes!("../res/quad-xy.obj");
#[allow(unused)]
pub const ICO_DATA: &[u8] = include_bytes!("../res/ico-sphere.obj");
#[allow(unused)]
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
        const MIN_SAMPLES_BEFORE_SKIP: usize = 256;
        const VARIANCE_TOLERANCE: f32 = 0.1;

        let objects = &self.objects;
        let material_mgr = &self.material_mgr;

        let width = self.width;
        let height = self.height;

        let start = std::time::Instant::now();
        let max_duration = std::time::Duration::from_millis(16);

        // split your image into chunks of rows
        self.image
            .par_chunks_mut(width) // each thread gets a full row
            .zip(self.pixels.par_chunks_mut(width * 4))
            .zip(self.samples_per_pixel.par_chunks_mut(width))
            .zip(self.image_sq.par_chunks_mut(width))
            .zip(self.converged.par_chunks_mut(width))
            .enumerate()
            .for_each(
                |(y, ((((image_row, pixel_row), sample_row), sq_row), conv_row))| {
                    let mut rng = SmallRng::from_entropy(); // one RNG per row/thread

                    for x in 0..width {
                        if conv_row[x] {
                            continue;
                        }

                        let sample_count = &mut sample_row[x];
                        let image_pixel = &mut image_row[x];
                        let image_sq_pixel = &mut sq_row[x];
                        let pixel_out = &mut pixel_row[x * 4..x * 4 + 4];
                        let pixel_converged = &mut conv_row[x];

                        while start.elapsed() < max_duration {
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

                            let ray = self.camera.get_ray(
                                jitter_x,
                                jitter_y,
                                width as f32,
                                height as f32,
                            );

                            let sample = trace(
                                &ray,
                                0,
                                &mut nalgebra_glm::vec3(1.0, 1.0, 1.0),
                                objects,
                                material_mgr,
                                &mut rng,
                            );

                            *sample_count += 1;
                            let n = *sample_count as f32;

                            let delta = sample - *image_pixel;
                            *image_pixel += delta / n;
                            *image_sq_pixel += delta.component_mul(&(sample - *image_pixel));
                        }

                        let pixel_hdr = tonemap(*image_pixel);
                        pixel_out[0] = (pixel_hdr.x * 255.0) as u8;
                        pixel_out[1] = (pixel_hdr.y * 255.0) as u8;
                        pixel_out[2] = (pixel_hdr.z * 255.0) as u8;
                        pixel_out[3] = 255;
                    }
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

        // Setup materials
        let mut material_mgr = MaterialMgr::new();
        #[allow(unused)]
        let emissive = material_mgr.add(
            MaterialEnum::Emissive(Emissive {
                color: nalgebra_glm::vec3(1.0, 1.0, 1.0) * 1000.0,
            }),
            Some("emissive"),
        );
        #[allow(unused)]
        let lambert_white = material_mgr.add(
            MaterialEnum::Lambertian(Lambertian {
                albedo: nalgebra_glm::vec3(0.9, 0.9, 0.9),
            }),
            Some("lambert_white"),
        );
        #[allow(unused)]
        let lambert_blue = material_mgr.add(
            MaterialEnum::Lambertian(Lambertian {
                albedo: nalgebra_glm::vec3(0.0, 0.3, 0.7),
            }),
            Some("lambert_blue"),
        );
        #[allow(unused)]
        let lambert_red = material_mgr.add(
            MaterialEnum::Lambertian(Lambertian {
                albedo: nalgebra_glm::vec3(0.9, 0.0, 0.0),
            }),
            Some("lambert_red"),
        );
        #[allow(unused)]
        let lambert_green = material_mgr.add(
            MaterialEnum::Lambertian(Lambertian {
                albedo: nalgebra_glm::vec3(0.0, 0.7, 0.0),
            }),
            Some("lambert_green"),
        );
        #[allow(unused)]
        let dielectric_blue = material_mgr.add(
            MaterialEnum::Dielectric(Dielectric {
                ior: 1.5,
                tint: nalgebra_glm::vec3(0.95, 0.98, 1.0),
            }),
            Some("dielectric_blue"),
        );
        #[allow(unused)]
        let dielectric_green = material_mgr.add(
            MaterialEnum::Dielectric(Dielectric {
                ior: 3.01,
                tint: nalgebra_glm::vec3(0.7, 0.95, 0.7),
            }),
            Some("dielectric_green"),
        );
        #[allow(unused)]
        let copper = material_mgr.add(
            MaterialEnum::Metallic(Metallic {
                roughness: 0.0,
                albedo: nalgebra_glm::vec3(0.95, 0.64, 0.54),
            }),
            Some("copper"),
        );
        #[allow(unused)]
        let silver = material_mgr.add(
            MaterialEnum::Metallic(Metallic {
                roughness: 0.1,
                albedo: nalgebra_glm::vec3(0.97, 0.96, 0.91),
            }),
            Some("silver"),
        );
        #[allow(unused)]
        let cobalt = material_mgr.add(
            MaterialEnum::Metallic(Metallic {
                roughness: 0.1,
                albedo: nalgebra_glm::vec3(0.2, 0.35, 0.8),
            }),
            Some("cobalt"),
        );
        #[allow(unused)]
        let ceramic = material_mgr.add(
            MaterialEnum::Glossy(Glossy {
                roughness: 0.2,
                albedo: nalgebra_glm::vec3(0.65, 0.8, 0.75),
            }),
            Some("ceramic"),
        );

        // Setup objects
        let objects: Vec<ObjectEnum> = vec![
            ObjectEnum::Plane(MaterialPlane::new(
                nalgebra_glm::vec3(1.0, 0.0, 1.0),
                100.0,
                emissive,
            )),
            // room
            ObjectEnum::Plane(MaterialPlane::new(
                nalgebra_glm::vec3(1.0, 0.0, 0.0), // back
                -5.0,
                lambert_white,
            )),
            ObjectEnum::Plane(MaterialPlane::new(
                nalgebra_glm::vec3(0.0, 1.0, 0.0), // bottom
                2.5,
                lambert_white,
            )),
            ObjectEnum::Plane(MaterialPlane::new(
                nalgebra_glm::vec3(0.0, 1.0, 0.0), // top
                -7.5,
                lambert_white,
            )),
            ObjectEnum::Plane(MaterialPlane::new(
                nalgebra_glm::vec3(0.0, 0.0, 1.0), // left
                -5.0,
                lambert_red,
            )),
            ObjectEnum::Plane(MaterialPlane::new(
                nalgebra_glm::vec3(0.0, 0.0, 1.0), // right
                5.0,
                lambert_blue,
            )),
            // objects
            ObjectEnum::Sphere(MaterialSphere::new(
                nalgebra_glm::vec3(0.0, 0.0, 0.0),
                2.5,
                silver,
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
    objects: &[ObjectEnum],
    material_mgr: &MaterialMgr,
    rng: &mut SmallRng,
) -> nalgebra_glm::Vec3 {
    if depth > 10 {
        return nalgebra_glm::Vec3::zeros();
    }
    if depth > 3 {
        let survive_prob = luminance(*throughput).clamp(0.05, 1.0);
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

    match hit_material.scatter(ray, &hit, rng) {
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
                rng,
            );
            emitted + scatter.attenuation.component_mul(&incoming)
        }
    }
}

fn cast_ray(ray: &Ray, objects: &[ObjectEnum]) -> Option<HitInfo> {
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
