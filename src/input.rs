use std::fmt::Debug;
use gamepads::{Button, Gamepads};
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
    gamepads: Gamepads
}
impl Controller {
    pub fn new() -> Self{
        Controller {
            gamepads: Gamepads::new()
        }
    }
}
impl Input for Controller{
    fn check_input(&mut self, bus: &mut Bus) {
        bus.reset_joypad_buttons();
        bus.set_int_request_joypad(false);
        self.gamepads.poll();
        let select_buttons = bus.get_joypad_select_buttons();
        let dpad_buttons = bus.get_joypad_dpad_buttons();
        for gamepad in self.gamepads.all() {
            for button in gamepad.all_currently_pressed() {
                match button {
                    Button::ActionDown => {
                        if select_buttons == false {
                            bus.set_int_request_joypad(true);
                            bus.set_joypad_set_b_left()
                        }}
                    Button::ActionRight => {
                        if select_buttons == false {
                            bus.set_int_request_joypad(true);
                            bus.set_joypad_set_a_right()
                        }}
                    Button::LeftCenterCluster => {
                        if select_buttons == false {
                            bus.set_int_request_joypad(true);
                            bus.set_joypad_set_select_up()
                        }}
                    Button::RightCenterCluster => {
                        if select_buttons == false {
                            bus.set_int_request_joypad(true);
                            bus.set_joypad_set_start_down()
                        }}
                    Button::DPadUp => {
                        if dpad_buttons == false {
                            bus.set_int_request_joypad(true);
                            bus.set_joypad_set_select_up()
                        }
                    }
                    Button::DPadDown => {
                        if dpad_buttons == false {
                            bus.set_int_request_joypad(true);
                            bus.set_joypad_set_start_down()
                        }}
                    Button::DPadLeft => {
                        if dpad_buttons == false {
                            bus.set_int_request_joypad(true);
                            bus.set_joypad_set_b_left()
                        }}
                    Button::DPadRight => {
                        if dpad_buttons == false {
                            bus.set_int_request_joypad(true);
                            bus.set_joypad_set_a_right()
                        }}
                    _ => {}
                }
                //println!("Pressed button: {:?}", button);
            }
        }
    }
}

