use alloc::vec;
use alloc::vec::Vec;
use crate::output::{Output, PX_COLOR, PX_PALETTE, PX_SPRITE, SCREEN_HEIGHT, SCREEN_WIDTH};

pub struct LCDD {
    size: f64,
    /// (brightness, layer) per pixel; layer 2 marks a sprite pixel.
    pixels: Vec<Vec<(f32, u8)>>,
}
impl Output for LCDD {
    fn write_line(&mut self, y: u16, line: &[u8; SCREEN_WIDTH]) {
        if y as usize >= SCREEN_HEIGHT {
            return;
        }
        const LEVELS: [u8; 8] = [10, 30, 60, 80, 80, 60, 30, 10];
        for (x, px) in line.iter().enumerate() {
            let c = LEVELS[(px & (PX_COLOR | PX_PALETTE)) as usize] as f32 / 100.0;
            let layer = if px & PX_SPRITE != 0 { 2 } else { 0 };
            self.pixels[x][y as usize] = (c, layer);
        }
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
