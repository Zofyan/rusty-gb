use macroquad::miniquad::window::set_window_size;
use macroquad::prelude::*;
use std::fmt::Debug;
use crate::output::Output;

pub struct LCD {
    size: f64,
    pixels: Vec<Vec<u8>>,
}
impl Output for LCD {
    fn write_pixel(&mut self, x: u16, y: u16, color: u8, pallette: bool, _: u8) {
        if x >= 256 {
            return;
        }
        let colors = match pallette {
            false => [0, 25, 50, 75],
            true => [75, 50, 25, 0],
        };
        let c = colors[color as usize] as f32 / 100.0;

        draw_rectangle(
            (x as f64 * self.size) as f32,
            (y as f64 * self.size) as f32,
            self.size as f32,
            self.size as f32,
            Color::new(c, c * 1.33, c, 1.00),
        );
    }

    fn refresh(&mut self) {
        //clear_background(RED);
    }
}

impl LCD {
    pub fn new(scale: f64) -> Self {
        set_window_size(160 * scale as u32, 152 * scale as u32);
        LCD {
            size: scale,
            pixels: vec![vec![0; 256]; 256],
        }
    }
}
