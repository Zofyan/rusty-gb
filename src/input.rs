use alloc::string::ToString;
use defmt::println;
use crate::bus::Bus;
use crate::mbc::MBC;

pub trait Input {
    fn check_input(&mut self, _: &mut Bus) {}
}

pub struct Dummy {
}
impl Dummy {
    pub fn new() -> Self{
        Dummy{}
    }
}
impl Input for Dummy {
    fn check_input(&mut self, bus: &mut Bus) {
        bus.reset_joypad_buttons();
        bus.set_int_request_joypad(false);
    }
}

