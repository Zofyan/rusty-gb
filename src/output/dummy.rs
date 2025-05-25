use std::fmt::Debug;
use crate::output::Output;

pub struct Dummy {}

impl Output for Dummy {}
impl Dummy {
    pub fn new() -> Self {
        Dummy {}
    }
}