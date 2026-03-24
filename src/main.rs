mod dielectric;
mod emissive;
mod hit_info;
mod lambertian;
mod material;
mod material_mgr;
mod object;
mod plane;
mod sphere;
mod tracer;

use std::cell::RefCell;

use apricot::app::run;
use tracer::Tracer;

fn main() -> Result<(), String> {
    // Start Apricot's game loop
    run(
        nalgebra_glm::I32Vec2::new(800, 600),
        "Path Tracer",
        &|app| RefCell::new(Box::new(Tracer::new(app))),
    )
}
