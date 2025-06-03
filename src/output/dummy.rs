use alloc::string::String;
use crate::output::Output;

pub struct Dummy {}

impl Output for Dummy {
    fn set_diagnostics(&mut self, diagnostics: String) {
        usbd_serial::SerialPort
    }
}
impl Dummy {
    pub fn new() -> Self {
        Dummy {}
    }
}