pub mod dummy;
pub mod lcd;
pub mod lcdd;
pub mod terminal;

use macroquad::prelude::*;
use std::fmt::Debug;
use std::io::{Write};
use async_trait::async_trait;
use colored::{Colorize};

#[async_trait]
pub trait Output {
    fn write_pixel(&mut self, _: u16, _: u16, _: u8, _: bool, _: u8) {}
    fn refresh(&mut self) {}
    fn render_frame(&self) {}
    fn set_diagnostics(&mut self, diagnostics: String) {}
}
