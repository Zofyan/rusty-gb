use macroquad::miniquad::window::set_window_size;
use macroquad::prelude::*;
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
        for x in 0..200 {
            for y in 0..200 {
                let c = self.pixels[x][y].0;
                let debug = self.pixels[x][y].1;
                if debug == 0 {
                    draw_rectangle(
                        (x as f64 * self.size) as f32,
                        (y as f64 * self.size) as f32,
                        self.size as f32,
                        self.size as f32,
                        Color::new(c, c * 1.25, c, 1.00),
                    );
                } else if debug == 1 {
                    draw_rectangle(
                        (x as f64 * self.size) as f32,
                        (y as f64 * self.size) as f32,
                        self.size as f32,
                        self.size as f32,
                        Color::new(c, c, c * 1.25, 1.00),
                    );
                } else {
                    draw_rectangle(
                        (x as f64 * self.size) as f32,
                        (y as f64 * self.size) as f32,
                        self.size as f32,
                        self.size as f32,
                        Color::new(c * 1.25, c, c, 1.00),
                    );
                }
            }
        }
        true
    }
}
impl LCDD {
    pub fn new(size: f64) -> Self {
        set_window_size(160 * size as u32, 144 * size as u32);
        LCDD {
            size,
            pixels: vec![vec![(0.0, 0); 200]; 200],
        }
    }
}