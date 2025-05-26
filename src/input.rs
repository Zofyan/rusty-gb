use std::fmt::Debug;
use gilrs::{Gilrs, Button, Event, GamepadId, Gamepad};
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
pub struct Controller {
}
impl Controller {
    pub fn new() -> Self{
        let girls = Gilrs::new().unwrap();
        for (id, gamepad) in girls.gamepads() {
            println!("Gamepad with id {} and name {} is connected",
                     id, gamepad.name());
        }
        Controller {
        }
    }
}
impl Input for Controller{
    fn check_input(&mut self, bus: &mut Bus) {
        bus.reset_joypad_buttons();
        bus.set_int_request_joypad(false);
    }
}

pub struct Keyboard {
}
impl Keyboard {
    pub fn new() -> Self{
        let girls = Gilrs::new().unwrap();
        for (id, gamepad) in girls.gamepads() {
            println!("Gamepad with id {} and name {} is connected",
                     id, gamepad.name());
        }
        Keyboard {
        }
    }
}
impl Input for Keyboard {
    fn check_input(&mut self, bus: &mut Bus) {
        bus.reset_joypad_buttons();
        bus.set_int_request_joypad(false);
    }
}

