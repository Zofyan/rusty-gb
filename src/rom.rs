use alloc::string::String;

pub trait ROM {
    fn read_single(&self, addr: u16) -> u8 {
        0
    }

    fn read(&self, addr: u16, buf: &mut [u8]) {

    }
}

struct Cartridge {

}

impl ROM for Cartridge {
    fn read_single(&self, addr: u16) -> u8 {
        0
    }

    fn read(&self, addr: u16, buf: &mut [u8]) {

    }
}

pub struct Flash {
}

impl Flash {
    pub fn new(rom: String) -> Self {
        Flash {}
    }
}
impl ROM for Flash {
    fn read_single(&self, addr: u16) -> u8 {
        0
    }

    fn read(&self, addr: u16, buf: &mut [u8]) {

    }
}
