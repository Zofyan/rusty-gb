use std::fmt::Debug;
use crate::output::Output;

pub struct LCDD {
    size: f64,
    pixels: Vec<Vec<(f32, u8)>>,
}
impl Output for LCDD {
    fn write_pixel(&mut self, x: u16, y: u16, color: u8, pallette: bool, debug: u8) {
        if x >= 200 {
            return;
        }
        let colors = match pallette {
            false => [10, 30, 60, 80],
            true => [80, 60, 30, 10],
        };
        let c = colors[color as usize] as f32 / 100.0;
        self.pixels[x as usize][y as usize] = (c, debug);


    }

    fn refresh(&mut self) -> bool {

        true
    }
}
impl LCDD {
    pub fn new(size: f64) -> Self {
        LCDD {
            size,
            pixels: vec![vec![(0.0, 0); 200]; 200],
        }
    }
}