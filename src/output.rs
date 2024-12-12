use macroquad::prelude::*;


pub trait Output {
    fn write_pixel(&mut self, _: u16, _: u16, _: u8, _: bool) {}
    fn refresh(&mut self) {}
}

pub struct Dummy {}

impl Output for Dummy {}
impl Dummy {
    pub fn new() -> Self {
        Dummy {}
    }
}
pub struct LCD {
    size: f64,
    pixels: Vec<[u16; 3]>,
}
impl Output for LCD {
    fn write_pixel(&mut self, x: u16, y: u16, color: u8, pallette: bool) {
        let colors = match pallette {
            false => [0, 33, 66, 100],
            true => [100, 66, 33, 0],
        };
        self.pixels.push([50, 50, 33])
    }
    #[macroquad::main("BasicShapes")]
    fn refresh(&mut self) {
        for p in self.pixels.iter() {

        }
        self.pixels.clear();
    }
}

impl LCD {
    pub fn new(size: f64) -> Self {

        LCD {
            size,
            pixels: Vec::new(),
        }
    }
}

pub struct Terminal {
    pixels: [[u16; 256]; 256],
}

impl Output for Terminal {
    fn write_pixel(&mut self, x: u16, y: u16, color: u8, pallette: bool) {
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
