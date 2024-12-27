use macroquad::miniquad::window::set_window_size;
use macroquad::prelude::*;
use std::fmt::Debug;

pub trait Output {
    fn write_pixel(&mut self, _: u16, _: u16, _: u8, _: bool, _: u8) {}
    fn refresh(&mut self) {}
}

pub struct Dummy {}

impl Output for Dummy {}
impl Dummy {
    pub fn new() -> Self {
        Dummy {}
    }
}
pub struct LCDD {
    size: f64,
    pixels: Vec<Vec<u8>>,
}
impl Output for LCDD {
    fn write_pixel(&mut self, x: u16, y: u16, color: u8, pallette: bool, debug: u8) {
        if x >= 256 {
            return;
        }
        let colors = match pallette {
            false => [10, 30, 60, 80],
            true => [80, 60, 30, 10],
        };
        let c = colors[color as usize] as f32 / 100.0;

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

    fn refresh(&mut self) {
    }
}
impl LCDD {
    pub fn new(size: f64) -> Self {
        set_window_size(160 * size as u32, 144 * size as u32);
        LCDD {
            size,
            pixels: vec![vec![0; 256]; 256],
        }
    }
}
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
    pub fn new(size: f64) -> Self {
        set_window_size(160 * size as u32, 144 * size as u32);
        LCD {
            size,
            pixels: vec![vec![0; 256]; 256],
        }
    }
}

pub struct Terminal {
    pixels: [[u16; 256]; 256],
}

impl Output for Terminal {
    fn write_pixel(&mut self, x: u16, y: u16, color: u8, pallette: bool, debug: u8) {
        let colors = match pallette {
            false => [0, 33, 66, 100],
            true => [100, 66, 33, 0],
        };
        self.pixels[x as usize][y as usize] = colors[color as usize]
    }
    fn refresh(&mut self) {
        for i in 0..256 {
            for j in 0..256 {
                if self.pixels[i][j] != 0 {
                    //print!(".")
                } else {
                    //print!(" ")
                }
                self.pixels[i][j] = 0;
            }
            //println!();
        }
    }
}

impl Terminal {
    pub fn new(size: f64) -> Self {
        Terminal {
            pixels: [[0u16; 256]; 256],
        }
    }
}
