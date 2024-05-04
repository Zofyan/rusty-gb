pub trait Output {
    fn write_pixel(&self, x: u16, y: u16, color: u8){}
}

pub struct LCD {

}

impl Output for LCD {

}