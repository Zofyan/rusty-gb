use std::fs;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use cloneable_file::CloneableFile;
use crate::ROM;

pub trait ROM {
    fn read_single(&self, addr: u16) -> u8 {
        0
    }

    fn read(&mut self, addr: usize, buf: &mut [u8]) {

    }
}

struct Cartridge {

}

impl ROM for Cartridge {
    fn read_single(&self, addr: u16) -> u8 {
        0
    }

    fn read(&mut self, addr: usize, buf: &mut [u8]) {

    }
}

pub struct File {
    pub reader: BufReader<fs::File>
}

impl File {
    pub fn new(rom: String) -> Self {
        File { reader: BufReader::new(fs::File::open(rom,).expect("Could not open rom")) }
    }
}
impl ROM for File {
    fn read_single(&self, _: u16) -> u8 {
        0
    }

    fn read(&mut self, addr: usize, buf: &mut [u8]) {
        self.reader.seek(SeekFrom::Start(addr as u64)).unwrap();
        self.reader.read_exact(buf).unwrap();
    }
}
pub struct Included {
    data: &'static[u8]
}

impl Included {
    pub fn new(data: &'static[u8]) -> Self {
        Included { data }
    }
}
impl ROM for Included {
    fn read_single(&self, addr: u16) -> u8 {
        self.data[addr as usize]
    }

    fn read(&mut self, addr: usize, buf: &mut [u8]) {
        buf.copy_from_slice(&self.data[addr..addr+buf.len()]);
    }
}
