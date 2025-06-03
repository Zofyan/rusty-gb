pub mod dummy;
//pub mod lcd;
//pub mod lcdd;
//pub mod terminal;

use alloc::string::String;
use async_trait::async_trait;

#[async_trait]
pub trait Output {
    fn write_pixel(&mut self, _: u16, _: u16, _: u8, _: bool, _: u8) {}
    fn refresh(&mut self) -> bool {
        true
    }
    fn set_diagnostics(&mut self, diagnostics: String) {}
}
