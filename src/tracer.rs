use apricot::app::{App, Scene};

pub struct Tracer {}

impl Scene for Tracer {
    fn update(&mut self, app: &App) {}
    fn render(&mut self, app: &App) {}
}

impl Tracer {
    pub fn new(app: &App) -> Self {
        Tracer {}
    }
}
