use std::fs;
use std::io::{Read, Seek, SeekFrom};

pub trait ROM {
    fn read_single(&self, addr: u16) -> u8 {
        0
    }

    fn read(&mut self, addr: u16, buf: &mut [u8]) {

    }
}

struct Cartridge {

}

impl ROM for Cartridge {
    fn read_single(&self, addr: u16) -> u8 {
        0
    }

    fn read(&mut self, addr: u16, buf: &mut [u8]) {

    }
}

pub struct File {
    reader: fs::File
}

impl File {
    pub fn new(rom: String) -> Self {
        File { reader: fs::File::open(rom).expect("Could not open rom") }
    }
}
impl ROM for File {
    fn read_single(&self, addr: u16) -> u8 {
        0
    }

    fn read(&mut self, addr: u16, buf: &mut [u8]) {
        self.reader.seek(SeekFrom::Start(addr as u64)).unwrap();
        self.reader.read_exact(buf).unwrap()
    }
}
pub struct Included {
    data: Vec<u8>
}

impl Included {
    pub fn new(data: &[u8]) -> Self {
        Included { data: data.to_vec() }
    }
}
impl ROM for Included {
    fn read_single(&self, addr: u16) -> u8 {
        self.data[addr as usize]
    }

    fn read(&mut self, addr: u16, buf: &mut [u8]) {
        buf.copy_from_slice(&self.data[addr as usize..buf.len() + addr as usize])
    }
}
