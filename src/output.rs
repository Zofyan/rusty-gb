use piston_window::*;
use piston_window::types::ColorComponent;

pub trait Output {
    fn write_pixel(&mut self, _: u16, _: u16, _: u8) {}
    fn refresh(&mut self) {}
    fn new(size: f64) -> Self where Self: Sized;
}

pub struct Dummy {}

impl Output for Dummy {
    fn new(size: f64) -> Self where Self: Sized { Dummy {} }
}
pub struct LCD {
    window: PistonWindow,
    size: f64,
    pixels: Vec<[u16; 3]>,
}

impl Output for LCD {
    fn write_pixel(&mut self, x: u16, y: u16, color: u8) {
        self.pixels.push([x, y, color as u16])
    }
    fn refresh(&mut self) {
        let Some(event) = self.window.next() else { panic!("test") };
        self.window.draw_2d(&event, |context, graphics, _device| {
            clear([0.0; 4], graphics);
        });
        self.window.draw_2d(&event, |context, graphics, _device| {
            for pixel in &self.pixels {
                rectangle([(pixel[2] as f64 * 0.3) as ColorComponent,
                              (pixel[2] as f64 * 0.3) as ColorComponent,
                              (pixel[2] as f64 * 0.3) as ColorComponent, 1.0], // red
                          [pixel[0] as f64 * self.size, pixel[1] as f64 * self.size, self.size, self.size],
                          context.transform,
                          graphics);
            }
        });
        self.pixels.clear();
    }
    fn new(size: f64) -> Self where Self: Sized {
        LCD {
            window: WindowSettings::new("Hello Piston!", [size * 160.0, size * 144.0])
                .exit_on_esc(true).build().unwrap(),
            size,
            pixels: Vec::new(),
        }
    }
}
